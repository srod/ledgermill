//! `alias OLD = NEW` and `alias /REGEX/ = REPLACEMENT`. See `spec/hledger/aliases.md`.

use crate::chars::{is_blank, trim_blank};
use crate::problem::{Parsed, invalid};
use regex_lite::Regex;

#[derive(Debug)]
pub(crate) enum Alias {
    /// Replaces `old` when it is the whole name or its leading segments.
    Plain {
        old: String,
        new: String,
    },
    Regex {
        regex: Regex,
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
        let regex = Regex::new(&translate(&pattern)?)
            .or(invalid("This regular expression is not valid."))?;
        return Ok(Alias::Regex {
            regex,
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
    pub(crate) fn apply(&self, account: &str) -> Parsed<String> {
        match self {
            Alias::Plain { old, new } => Ok(match account.strip_prefix(old.as_str()) {
                Some(rest) if rest.is_empty() || rest.starts_with(':') => format!("{new}{rest}"),
                _ => account.to_string(),
            }),
            Alias::Regex { regex, replacement } => replace_all(regex, replacement, account),
        }
    }
}

/// Replace every match. Unlike `Regex::replace_all`, an empty match right after another match
/// is replaced too (`.*` on `ab` gives two replacements), as the reference tool does.
fn replace_all(regex: &Regex, replacement: &str, text: &str) -> Parsed<String> {
    let mut out = String::new();
    let mut copied = 0;
    let mut pos = 0;
    while let Some(caps) = regex.captures_at(text, pos) {
        let Some(whole) = caps.get(0) else { break };
        out.push_str(text.get(copied..whole.start()).unwrap_or(""));
        expand(replacement, &caps, &mut out)?;
        copied = whole.end();
        pos = whole.end();
        if whole.is_empty() {
            let Some(c) = text.get(pos..).and_then(|rest| rest.chars().next()) else {
                break;
            };
            out.push(c);
            pos += c.len_utf8();
            copied = pos;
        }
    }
    out.push_str(text.get(copied..).unwrap_or(""));
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
                out.push_str(caps.get(index).map_or("", |m| m.as_str()));
            }
            _ => out.push(c),
        }
    }
    Ok(())
}
