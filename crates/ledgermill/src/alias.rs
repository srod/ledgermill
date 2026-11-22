//! `alias OLD = NEW` and `alias /REGEX/ = REPLACEMENT`. See `spec/hledger/aliases-and-parents.md`.

use crate::chars::{is_blank, trim_blank};
use crate::problem::{Parsed, Problem, Unsupported, invalid};
use regex_lite::{Regex, RegexBuilder};
use std::collections::HashMap;

/// Maximum compiled NFA size per regex alias (64 KiB).
const MAX_REGEX_NFA_BYTES: usize = 64 * 1024;
/// Maximum byte length of an account name made by `apply account` nesting or alias expansion
/// (64 KiB).
pub(crate) const MAX_ACCOUNT_LEN: usize = 64 * 1024;

// The limits above bound memory, not time: 243 cheap aliases scanning a 32 KiB name, which twelve
// doubling aliases build from 8 bytes, cost 0.6 s per posting from a 3.7 KB journal, and 20,000
// plain aliases that each look at a 60,000-byte name cost 27 ms per posting. So one load may
// make its aliases look at only this much text: a floor, plus a share per input byte, as a
// legitimate journal's work grows with its size. An alias applied to a name is charged the
// name's length, times the NFA size class for a regex (1, 4, 16 or 64 for up to 1, 4, 16 or 64
// KiB), as the scan time per byte grows with the NFA: 20 ns for a small one, 8 us for one near
// the limit. The parser charges the length of every finished account name too, with or without
// aliases, so the account names of a load cannot add up to more than the budget either (a 1 MB
// journal of `a  1` lines under a 60 KB `apply account` prefix, or with an alias that makes `a`
// 60 KB, would otherwise hold gigabytes of names).
//
// A name that was resolved before is not resolved again (see `AliasSet`), so a journal pays for
// its distinct account names, not for its postings: 150 aliases and 500 accounts in a 1 MB
// journal use 1.8 units per input byte, and 5,000 accounts 13.5. Measured in release, a unit costs
// up to 0.08 us, so the floor is about 20 ms, and a hostile journal that names a different
// account on every posting is capped at about 1.3 s per MB, linear in its size (it was 2.5 s with
// 32 units per byte, before names were remembered).
const ALIAS_WORK_FLOOR: usize = 256 * 1024;
const ALIAS_WORK_PER_INPUT_BYTE: usize = 16;
/// What the finished names an `AliasSet` remembers may take up between them, with an allowance
/// per name for the map's own bookkeeping (4 MiB).
const MAX_REMEMBERED_BYTES: usize = 4 * 1024 * 1024;
const REMEMBERED_NAME_OVERHEAD: usize = 64;

/// What the aliases and account names of a load may look at, given the bytes of all its files.
pub(crate) fn work_budget(input_bytes: usize) -> usize {
    input_bytes
        .saturating_mul(ALIAS_WORK_PER_INPUT_BYTE)
        .saturating_add(ALIAS_WORK_FLOOR)
}

/// Takes `cost` from `budget`, or refuses as `refusal` when that is more than is left.
pub(crate) fn charge(budget: &mut usize, cost: usize, refusal: Unsupported) -> Parsed<()> {
    *budget = budget
        .checked_sub(cost)
        .ok_or(Problem::Unsupported(refusal))?;
    Ok(())
}

/// The aliases in force in one file, and the names they have already been applied to. Whatever
/// changes the aliases forgets those names, so a remembered name is always what the aliases in
/// force would give now.
#[derive(Debug, Default)]
pub(crate) struct AliasSet {
    list: Vec<Alias>,
    regex_count: usize,
    /// Name before the aliases to name after, for names that came out without a problem.
    resolved: HashMap<String, String>,
    resolved_bytes: usize,
}

impl AliasSet {
    pub(crate) fn push(&mut self, alias: Alias) {
        if matches!(alias, Alias::Regex { .. }) {
            self.regex_count += 1;
        }
        self.list.push(alias);
        self.forget();
    }

    pub(crate) fn clear(&mut self) {
        self.list.clear();
        self.regex_count = 0;
        self.forget();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn regex_count(&self) -> usize {
        self.regex_count
    }

    fn forget(&mut self) {
        self.resolved.clear();
        self.resolved_bytes = 0;
    }

    /// `name` with every alias applied, the last defined first. `budget` is what the load may
    /// still look at.
    pub(crate) fn resolve(&mut self, name: String, budget: &mut usize) -> Parsed<String> {
        if self.is_empty() {
            return Ok(name);
        }
        if let Some(full) = self.resolved.get(&name) {
            return Ok(full.clone());
        }
        let mut full = name.clone();
        for alias in self.list.iter().rev() {
            alias.apply(&mut full, budget)?;
        }
        self.remember(name, &full);
        Ok(full)
    }

    /// Only successes are remembered, so a refused name is refused again, and reported, on every
    /// line that uses it.
    fn remember(&mut self, name: String, full: &str) {
        let size = name.len() + full.len() + REMEMBERED_NAME_OVERHEAD;
        if size > MAX_REMEMBERED_BYTES {
            return;
        }
        if self.resolved_bytes + size > MAX_REMEMBERED_BYTES {
            self.forget();
        }
        self.resolved_bytes += size;
        self.resolved.insert(name, full.to_string());
    }
}

#[derive(Debug)]
pub(crate) enum Alias {
    /// Replaces `old` when it is the whole name or its leading segments.
    Plain { old: String, new: String },
    Regex {
        regex: Regex,
        /// How many times dearer than the smallest a scan with this regex is (see above).
        weight: usize,
        replacement: String,
    },
}

/// Characters that mean something in a regular expression; escaped, they stand for themselves.
const META: &str = "\\.+*?()|[]{}^$#&-~/";

/// Turn the journal's regular expression into one `regex_lite` reads the same way: matching is
/// case-insensitive, `\b`, `\<` and `\>` are word boundaries, and a backslash before any other
/// letter or digit gives that character itself (`\d` is `d`).
fn translate(pattern: &str) -> Parsed<String> {
    if ["+?", "*?", "??", "}?", "(?"]
        .iter()
        .any(|s| pattern.contains(s))
    {
        return invalid("This regular expression is not supported by the reference tool.");
    }
    let mut out = String::from("(?i)");
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b' | '<' | '>') => out.push_str("\\b"),
            Some(e) if META.contains(e) => {
                out.push('\\');
                out.push(e);
            }
            Some(e) => out.push_str(&regex_lite::escape(&e.to_string())),
            None => return invalid("A regular expression cannot end with a backslash."),
        }
    }
    Ok(out)
}

/// Parse what follows `alias`.
pub(crate) fn parse_alias(text: &str) -> Parsed<Alias> {
    let text = text.trim_start_matches(is_blank);
    if let Some(rest) = text.strip_prefix('/') {
        let mut pattern = String::new();
        let mut chars = rest.char_indices();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some((_, '/')) => pattern.push('/'),
                    Some((_, e)) => {
                        pattern.push('\\');
                        pattern.push(e);
                    }
                    None => break,
                },
                '/' => {
                    end = Some(i + 1);
                    break;
                }
                c => pattern.push(c),
            }
        }
        let Some(end) = end else {
            return invalid("The regular expression of this alias is not closed.");
        };
        let after = rest.get(end..).unwrap_or("").trim_start_matches(is_blank);
        let Some(replacement) = after.strip_prefix('=') else {
            return invalid("An alias needs `=`.");
        };
        let pattern = translate(&pattern)?;
        // The smallest size class the pattern compiles in, 1 KiB up to the limit.
        let mut weight = 1;
        let regex = loop {
            match RegexBuilder::new(&pattern)
                .size_limit(weight * 1024)
                .build()
            {
                Ok(regex) => break regex,
                Err(err) if err.to_string().contains("size limit") => {
                    if weight * 1024 >= MAX_REGEX_NFA_BYTES {
                        return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
                    }
                    weight *= 4;
                }
                Err(_) => return invalid("This regular expression is not valid."),
            }
        };
        return Ok(Alias::Regex {
            regex,
            weight,
            replacement: replacement.trim_start_matches(is_blank).to_string(),
        });
    }
    let Some((old, new)) = text.split_once('=') else {
        return invalid("An alias needs `=`.");
    };
    Ok(Alias::Plain {
        old: trim_blank(old).to_string(),
        new: trim_blank(new).to_string(),
    })
}

impl Alias {
    /// Applies the alias to `account` in place. `budget` is what the load's aliases may still
    /// look at; looking at the name takes its share, matching or not.
    fn apply(&self, account: &mut String, budget: &mut usize) -> Parsed<()> {
        match self {
            Alias::Plain { old, new } => {
                charge(budget, account.len().max(1), Unsupported::AliasTooLarge)?;
                let rest_len = match account.strip_prefix(old.as_str()) {
                    Some(rest) if rest.is_empty() || rest.starts_with(':') => rest.len(),
                    _ => return Ok(()),
                };
                if new.len().saturating_add(rest_len) > MAX_ACCOUNT_LEN {
                    return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
                }
                account.replace_range(..old.len(), new);
            }
            Alias::Regex {
                regex,
                weight,
                replacement,
            } => {
                charge(
                    budget,
                    account.len().saturating_mul(*weight),
                    Unsupported::AliasTooLarge,
                )?;
                *account = replace_all(regex, replacement, account)?;
            }
        }
        Ok(())
    }
}

fn push_bounded(out: &mut String, piece: &str) -> Parsed<()> {
    if out.len().saturating_add(piece.len()) > MAX_ACCOUNT_LEN {
        return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
    }
    out.push_str(piece);
    Ok(())
}

/// Replace every match. Unlike `Regex::replace_all`, an empty match right after another match
/// is replaced too (`.*` on `ab` gives two replacements), as the reference tool does.
fn replace_all(regex: &Regex, replacement: &str, text: &str) -> Parsed<String> {
    let mut out = String::new();
    let mut copied = 0;
    let mut pos = 0;
    while let Some(caps) = regex.captures_at(text, pos) {
        let Some(whole) = caps.get(0) else { break };
        push_bounded(&mut out, text.get(copied..whole.start()).unwrap_or(""))?;
        expand(replacement, &caps, &mut out)?;
        copied = whole.end();
        pos = whole.end();
        if whole.is_empty() {
            let Some(c) = text.get(pos..).and_then(|rest| rest.chars().next()) else {
                break;
            };
            if out.len().saturating_add(c.len_utf8()) > MAX_ACCOUNT_LEN {
                return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
            }
            out.push(c);
            pos += c.len_utf8();
            copied = pos;
        }
    }
    push_bounded(&mut out, text.get(copied..).unwrap_or(""))?;
    Ok(out)
}

/// `\N` in a replacement is the Nth group (`\0` the whole match); any other backslash is kept.
fn expand(replacement: &str, caps: &regex_lite::Captures, out: &mut String) -> Parsed<()> {
    let mut chars = replacement.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek().is_some_and(char::is_ascii_digit)) {
            ('\\', true) => {
                let mut index: usize = 0;
                while let Some(d) = chars.peek().and_then(|d| d.to_digit(10)) {
                    chars.next();
                    let d = usize::try_from(d).unwrap_or(usize::MAX);
                    index = index.saturating_mul(10).saturating_add(d);
                }
                if index >= caps.len() {
                    return invalid("This alias refers to a group its regular expression lacks.");
                }
                push_bounded(out, caps.get(index).map_or("", |m| m.as_str()))?;
            }
            _ => {
                if out.len().saturating_add(c.len_utf8()) > MAX_ACCOUNT_LEN {
                    return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
                }
                out.push(c);
            }
        }
    }
    Ok(())
}
