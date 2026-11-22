//! `alias OLD = NEW` and `alias /REGEX/ = REPLACEMENT`. See `spec/hledger/aliases-and-parents.md`.

use crate::chars::{is_blank, trim_blank};
use crate::problem::{Parsed, Problem, Unsupported, invalid};
use regex_lite::{Regex, RegexBuilder};

/// Maximum compiled NFA size per regex alias (64 KiB).
const MAX_REGEX_NFA_BYTES: usize = 64 * 1024;
/// Maximum byte length of an account name produced by alias expansion (64 KiB).
const MAX_ACCOUNT_LEN: usize = 64 * 1024;

// The limits above bound memory, not time: 243 cheap aliases scanning a 32 KiB name, which twelve
// doubling aliases build from 8 bytes, cost 0.6 s per posting from a 3.7 KB journal. So one load
// may feed its regex aliases only this much text: a floor, plus a share per input byte, as a
// legitimate journal's work grows with its size. A scan is charged `text length x weight`, the
// weight being the NFA size class (1, 4, 16 or 64 for up to 1, 4, 16 or 64 KiB), because the
// scan time per byte grows with the NFA: 20 ns for a small one, 8 us for one near the limit.
// Measured in release, a unit costs up to 0.1 us, so the floor is about 25 ms and the hostile
// journal above runs out of it within its first posting. The corpus journals use under 0.4 units
// per input byte; 32 leaves room for 20 aliases that expand one-letter names in a 1 MB journal,
// and still caps a hostile 1 MB journal at about 3 s, linear in its size.
const ALIAS_WORK_FLOOR: usize = 256 * 1024;
const ALIAS_WORK_PER_INPUT_BYTE: usize = 32;

/// What the regex aliases of a load may scan, given the bytes of all its files.
pub(crate) fn work_budget(input_bytes: usize) -> usize {
    input_bytes
        .saturating_mul(ALIAS_WORK_PER_INPUT_BYTE)
        .saturating_add(ALIAS_WORK_FLOOR)
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
    /// `budget` is what the load's regex aliases may still scan; a regex alias takes its share.
    pub(crate) fn apply(&self, account: &str, budget: &mut usize) -> Parsed<String> {
        match self {
            Alias::Plain { old, new } => match account.strip_prefix(old.as_str()) {
                Some(rest) if rest.is_empty() || rest.starts_with(':') => {
                    if new.len().saturating_add(rest.len()) > MAX_ACCOUNT_LEN {
                        return Err(Problem::Unsupported(Unsupported::AliasTooLarge));
                    }
                    Ok(format!("{new}{rest}"))
                }
                _ => Ok(account.to_string()),
            },
            Alias::Regex {
                regex,
                weight,
                replacement,
            } => {
                let cost = account.len().saturating_mul(*weight);
                *budget = budget
                    .checked_sub(cost)
                    .ok_or(Problem::Unsupported(Unsupported::AliasTooLarge))?;
                replace_all(regex, replacement, account)
            }
        }
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
