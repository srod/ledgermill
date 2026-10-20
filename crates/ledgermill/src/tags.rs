//! What the reference tool reads inside comments: `name: value` tags, and `[DATE]` brackets in
//! posting comments. See `spec/hledger/comment-dates.md` and `spec/hledger/conversion.md`.

use crate::chars::{is_blank, trim_blank};
use crate::cursor::Cursor;
use crate::date::parse_date;
use crate::problem::{Parsed, Problem, invalid};

/// Tags of one comment line, in order: `(name, value)`. Scanning left to right, a tag name is
/// the word (letters, digits, `-`, `_`) right before a `:`; its value runs to the next comma,
/// and the next tag can only start after that comma.
pub(crate) fn tags(text: &str) -> Vec<(&str, &str)> {
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
            out.push((name, trim_blank(value)));
        }
        rest = next;
    }
    out
}

/// A date at the start of `text` (blanks first allowed); what follows it does not matter.
fn date_prefix(text: &str, year: u32) -> Parsed<u32> {
    let mut cur = Cursor::new(text);
    cur.skip_ws();
    parse_date(&mut cur, Some(year)).map(|d| d.year())
}

fn is_bracket_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '-' | '/' | '.' | '=')
}

/// `[DATE]`, `[DATE=DATE]` or `[=DATE]`, strictly; `inner` is the text between the brackets.
fn bracket_dates(inner: &str, year: u32) -> Parsed<()> {
    let mut cur = Cursor::new(inner);
    let mut primary_year = year;
    if !cur.at_end() && cur.peek() != Some('=') {
        primary_year = parse_date(&mut cur, Some(year))?.year();
    }
    if cur.eat('=') {
        parse_date(&mut cur, Some(primary_year))?;
    }
    if !cur.at_end() {
        return invalid("A bracketed date was expected.");
    }
    Ok(())
}

/// Check the dates a posting comment line holds: `date:`/`date2:` tags and bracketed dates,
/// which the reference tool reads and rejects when invalid. Dates without a year take `year`,
/// the transaction's.
pub(crate) fn check_posting_comment(text: &str, year: u32) -> Parsed<()> {
    for (name, value) in tags(text) {
        if name == "date" || name == "date2" {
            date_prefix(value, year).map_err(|p| match p {
                Problem::Unsupported(_) => p,
                Problem::Invalid(_) => {
                    Problem::Invalid("This posting date tag is not a valid date.")
                }
            })?;
        }
    }
    // Only brackets that hold nothing but date characters, with a date separator, count.
    let mut rest = text;
    while let Some(at) = rest.find('[') {
        rest = rest.get(at + 1..).unwrap_or("");
        let run_len = rest.find(|c| !is_bracket_char(c)).unwrap_or(rest.len());
        let run = rest.get(..run_len).unwrap_or("");
        let closed = rest.get(run_len..).is_some_and(|r| r.starts_with(']'));
        if closed && run.contains(['-', '/', '.']) {
            bracket_dates(run, year)?;
        }
    }
    Ok(())
}
