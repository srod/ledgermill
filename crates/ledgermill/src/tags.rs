//! What the reference tool reads inside comments: `name: value` tags, and `[DATE]` brackets in
//! posting comments. See `spec/hledger/comment-dates.md` and `spec/hledger/conversion.md`.

use crate::chars::{is_blank, trim_blank};
use crate::cursor::Cursor;
use crate::date::{Date, YearSource, check_date, parse_date};
use crate::problem::{Parsed, Problem, invalid};

/// Tags of one comment line, in order: `(name, value)`. Scanning left to right, a tag name is
/// the word (letters, digits, `-`, `_`) right before a `:`; its value runs to the next comma,
/// and the next tag can only start after that comma.
pub(crate) fn tags(text: &str) -> Vec<(&str, &str)> {
    tags_at(text)
        .into_iter()
        .map(|(_, name, value)| (name, value))
        .collect()
}

/// [`tags`] with the byte offset of each tag name in `text`.
fn tags_at(text: &str) -> Vec<(usize, &str, &str)> {
    let is_name = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(colon) = rest.find(':') {
        let before = rest.get(..colon).unwrap_or("");
        let name_start = before.rfind(|c: char| !is_name(c)).map_or(0, |i| {
            i + before
                .get(i..)
                .and_then(|s| s.chars().next())
                .map_or(1, char::len_utf8)
        });
        let name = before.get(name_start..).unwrap_or("");
        let preceded_ok = before
            .get(..name_start)
            .and_then(|s| s.chars().next_back())
            .is_none_or(is_blank);
        let after = rest.get(colon + 1..).unwrap_or("");
        let (value, next) = match after.find(',') {
            Some(i) => (
                after.get(..i).unwrap_or(""),
                after.get(i + 1..).unwrap_or(""),
            ),
            None => (after, ""),
        };
        if !name.is_empty() && preceded_ok {
            out.push((name_start, name, trim_blank(value)));
        }
        rest = next;
    }
    out
}

/// A date at the start of `text` (blanks first allowed); what follows it does not matter.
fn date_prefix(text: &str, year: u32) -> Parsed<Date> {
    let mut cur = Cursor::new(text);
    cur.skip_ws();
    parse_date(&mut cur, Some(year))
}

/// [`date_prefix`] for checking only, with the year coming from `source`.
fn check_date_prefix(text: &str, source: YearSource) -> Parsed<()> {
    let mut cur = Cursor::new(text);
    cur.skip_ws();
    check_date(&mut cur, source).map(|_| ())
}

fn is_bracket_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '-' | '/' | '.' | '=')
}

/// `[DATE]`, `[DATE=DATE]` or `[=DATE]`, strictly; `inner` is the text between the brackets.
fn bracket_dates(inner: &str, source: YearSource) -> Parsed<()> {
    let mut cur = Cursor::new(inner);
    let mut second = source;
    if !cur.at_end() && cur.peek() != Some('=') {
        // The second date takes the year the first one settled on.
        if let Some(year) = check_date(&mut cur, source)? {
            second = YearSource::Fixed(year);
        }
    }
    if cur.eat('=') {
        check_date(&mut cur, second)?;
    }
    if !cur.at_end() {
        return invalid("A bracketed date was expected.");
    }
    Ok(())
}

/// Check the dates a posting comment line holds: `date:`/`date2:` tags and bracketed dates,
/// which the reference tool reads and rejects when invalid. Dates without a year take theirs
/// from `source`.
pub(crate) fn check_posting_comment(text: &str, source: YearSource) -> Parsed<()> {
    for (name, value) in tags(text) {
        if name == "date" || name == "date2" {
            check_date_prefix(value, source).map_err(|p| match p {
                Problem::Unsupported(_) => p,
                Problem::Invalid(_) => {
                    Problem::Invalid("This posting date tag is not a valid date.")
                }
            })?;
        }
    }
    for (_, run) in date_brackets(text) {
        bracket_dates(run, source)?;
    }
    Ok(())
}

/// The brackets of a comment line that count as dates: they hold nothing but date characters,
/// with a date separator. `(offset of the `[`, text between the brackets)`.
fn date_brackets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('[') {
        rest = rest.get(at + 1..).unwrap_or("");
        let run_len = rest.find(|c| !is_bracket_char(c)).unwrap_or(rest.len());
        let run = rest.get(..run_len).unwrap_or("");
        let closed = rest.get(run_len..).is_some_and(|r| r.starts_with(']'));
        if closed && run.contains(['-', '/', '.']) {
            out.push((text.len() - rest.len() - 1, run));
        }
    }
    out
}

/// The date a posting's comment gives it, if any: the first `date:` tag or `[DATE]` bracket in
/// the text, line by line (spec/hledger/assertion-order.md). `[=DATE2]` and `date2:` give none.
/// Dates without a year take `year`. The dates were checked by [`check_posting_comment`].
pub(crate) fn posting_date(lines: &[String], year: u32) -> Option<Date> {
    lines.iter().find_map(|line| {
        let tag = tags_at(line)
            .into_iter()
            .filter(|(_, name, _)| *name == "date")
            .find_map(|(at, _, value)| Some((at, date_prefix(value, year).ok()?)));
        let bracket = date_brackets(line).into_iter().find_map(|(at, run)| {
            let mut cur = Cursor::new(run);
            if cur.peek() == Some('=') {
                return None;
            }
            Some((at, parse_date(&mut cur, Some(year)).ok()?))
        });
        match (tag, bracket) {
            (Some((a, d)), Some((b, e))) => Some(if a < b { d } else { e }),
            (Some((_, d)), None) | (None, Some((_, d))) => Some(d),
            (None, None) => None,
        }
    })
}
