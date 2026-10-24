//! `format` for Beancount (`spec/beancount/format.md`): the amounts of postings, `balance` and
//! `price` lines of a whole file end in one column, and every posting has the file's usual indent.
//!
//! The line shapes come from the spec note (found by running `bean-format`); which lines they are
//! tried on comes from the loader (`amount_lines`), so text that only looks like a posting (a
//! multi-line string, say) is never touched.

use crate::model::SourceFile;
use std::collections::HashMap;

/// The file with its posting, `balance` and `price` lines rewritten; the file as it was when
/// ledgermill reads none of it.
pub(crate) fn realign(file: &SourceFile) -> String {
    let Some(read) = super::amount_lines(file) else {
        return file.content.clone();
    };
    // A CR belongs to the line ending, not to the line (rule 7).
    let lines: Vec<(&str, &str)> = file
        .content
        .split('\n')
        .map(|line| match line.strip_suffix('\r') {
            Some(body) => (body, "\r"),
            None => (line, ""),
        })
        .collect();
    let shapes: Vec<Option<Shape>> = lines
        .iter()
        .enumerate()
        .map(|(index, (body, _))| {
            let number = u32::try_from(index + 1).ok()?;
            read.contains(&number).then(|| Shape::of(body))?
        })
        .collect();

    let indent = usual_indent(shapes.iter().flatten());
    // shortcut: char count, not display width (CJK/emoji misalign), upgrade if users report it
    let mut prefix_width = 0;
    let mut number_width = 0;
    for shape in shapes.iter().flatten() {
        match shape {
            Shape::Posting {
                account, number, ..
            } => {
                prefix_width = prefix_width.max(indent + account.chars().count());
                number_width = number_width.max(number.chars().count());
            }
            Shape::Dated { prefix, number, .. } => {
                prefix_width = prefix_width.max(prefix.chars().count());
                number_width = number_width.max(number.chars().count());
            }
            Shape::Account { .. } => {}
        }
    }

    let pad = " ".repeat(indent);
    let mut out = String::with_capacity(file.content.len());
    for (index, ((body, ending), shape)) in lines.iter().zip(&shapes).enumerate() {
        if index > 0 {
            out.push('\n');
        }
        match shape {
            Some(Shape::Posting {
                account,
                number,
                rest,
                ..
            }) => {
                let head = format!("{pad}{account}");
                out.push_str(&format!(
                    "{head:<prefix_width$}  {number:>number_width$} {rest}"
                ));
            }
            Some(Shape::Dated {
                prefix,
                number,
                rest,
            }) => out.push_str(&format!(
                "{prefix:<prefix_width$}  {number:>number_width$} {rest}"
            )),
            Some(Shape::Account { rest, .. }) => {
                out.push_str(&pad);
                out.push_str(rest);
            }
            None => out.push_str(body),
        }
        out.push_str(ending);
    }
    out
}

/// The indent most account lines have, a tab counting 1; the larger one when two are tied.
fn usual_indent<'a>(shapes: impl Iterator<Item = &'a Shape<'a>>) -> usize {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for shape in shapes {
        match shape {
            Shape::Posting { indent, .. } | Shape::Account { indent, .. } => {
                *counts.entry(*indent).or_default() += 1;
            }
            Shape::Dated { .. } => {}
        }
    }
    counts
        .into_iter()
        .max_by_key(|&(indent, lines)| (lines, indent))
        .map_or(0, |(indent, _)| indent)
}

/// A line cut where `realign` needs it. Every piece is the text as written.
enum Shape<'a> {
    /// A posting with an amount: indent, `account`, `number`, then `rest` from the currency on.
    Posting {
        indent: usize,
        account: &'a str,
        number: &'a str,
        rest: &'a str,
    },
    /// A dated line with an amount (`balance`, `price`): the text before the blanks that
    /// precede `number`, then `rest` from the currency on.
    Dated {
        prefix: &'a str,
        number: &'a str,
        rest: &'a str,
    },
    /// A posting without an amount: indent, then `rest` from the account on.
    Account { indent: usize, rest: &'a str },
}

impl<'a> Shape<'a> {
    /// The shape of a line, if it has one of the three `bean-format` rewrites.
    fn of(line: &'a str) -> Option<Shape<'a>> {
        posting(line)
            .or_else(|| dated(line))
            .or_else(|| account_line(line))
    }
}

/// Spaces and tabs only: any other blank is not one for the formatter (rule 2).
fn skip_blanks(text: &str) -> &str {
    text.trim_start_matches([' ', '\t'])
}

/// Characters an account component continues with. Unicode letters and digits count.
// shortcut: Unicode "alphabetic" and "numeric" properties, not the exact general categories
// (they differ on combining marks and letter numbers); upgrade if a comparison finds a line.
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-'
}

/// What is left of `text` after an account: a capital, name characters, then one or more
/// `:component`s. The account is the longest such prefix, so a trailing `:` is not part of it.
fn after_account(text: &str) -> Option<&str> {
    let mut rest = text
        .strip_prefix(char::is_uppercase)?
        .trim_start_matches(is_name_char);
    let mut components = 0;
    while let Some(tail) = rest
        .strip_prefix(':')
        .and_then(|t| t.strip_prefix(|c: char| c.is_uppercase() || c.is_numeric()))
    {
        rest = tail.trim_start_matches(is_name_char);
        components += 1;
    }
    (components > 0).then_some(rest)
}

/// The length of a number at the start of `text`: a sign, blanks, digits and commas, then a point
/// and digits; or two of those around one operator, in parentheses.
fn number_len(text: &str) -> Option<usize> {
    let rest = match text.strip_prefix('(') {
        Some(inner) => {
            let after_left = plain_number(inner)?;
            let after_operator = skip_blanks(after_left).strip_prefix(['+', '-', '*', '/'])?;
            plain_number(after_operator)?.strip_prefix(')')?
        }
        None => plain_number(text)?,
    };
    Some(text.len() - rest.len())
}

/// What is left of `text` after `[-+]?[ \t]*[0-9,]+(\.[0-9]*)?`.
fn plain_number(text: &str) -> Option<&str> {
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let digits = skip_blanks(unsigned);
    let after_digits = digits.trim_start_matches(|c: char| c.is_ascii_digit() || c == ',');
    if after_digits.len() == digits.len() {
        return None;
    }
    Some(match after_digits.strip_prefix('.') {
        Some(fraction) => fraction.trim_start_matches(|c: char| c.is_ascii_digit()),
        None => after_digits,
    })
}

/// Whether `text` starts with a currency: a capital and capitals, digits and `' . _ -`, or the
/// `/…` form, that ends where a word ends (so `EURx` is not one).
fn starts_with_currency(text: &str) -> bool {
    fn is_currency_char(c: char) -> bool {
        c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '\'' | '.' | '_' | '-')
    }
    fn is_word(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let (slashed, body) = match text.strip_prefix('/') {
        Some(body) => (true, body),
        None => (false, text),
    };
    let run: Vec<char> = body.chars().take_while(|&c| is_currency_char(c)).collect();
    if !slashed && !run.first().is_some_and(char::is_ascii_uppercase) {
        return false;
    }
    let after_run = body.chars().nth(run.len());
    // The currency may stop before the end of the run, wherever a word boundary falls. The `/…`
    // form must end in a capital, or in a digit after a capital.
    (0..run.len()).any(|last| {
        let Some(&end) = run.get(last) else {
            return false;
        };
        let well_formed = !slashed
            || end.is_ascii_uppercase()
            || (end.is_ascii_digit() && run.iter().take(last).any(char::is_ascii_uppercase));
        let next = run.get(last + 1).copied().or(after_run);
        well_formed && is_word(end) != next.is_some_and(is_word)
    })
}

/// `number`, blanks and a currency at the start of `text` (which starts at the blanks before the
/// number): the number and the text from the currency on.
fn number_and_currency(text: &str) -> Option<(&str, &str)> {
    let after_blanks = skip_blanks(text);
    if after_blanks.len() == text.len() {
        return None;
    }
    let len = number_len(after_blanks)?;
    let (number, after_number) = after_blanks.split_at_checked(len)?;
    let rest = skip_blanks(after_number);
    if rest.len() == after_number.len() || !starts_with_currency(rest) {
        return None;
    }
    Some((number, rest))
}

/// Shape 1: indent, account, blanks, number, blanks, currency, anything.
fn posting(line: &str) -> Option<Shape<'_>> {
    let account_start = skip_blanks(line);
    let indent = line.len() - account_start.len();
    if indent == 0 {
        return None;
    }
    let after = after_account(account_start)?;
    let account = account_start.get(..account_start.len() - after.len())?;
    let (number, rest) = number_and_currency(after)?;
    Some(Shape::Posting {
        indent,
        account,
        number,
        rest,
    })
}

/// Shape 2: a digit, text without `"` or `;`, then the first blanks that a number and a currency
/// follow.
fn dated(line: &str) -> Option<Shape<'_>> {
    if !line.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut previous_blank = false;
    for (at, c) in line.char_indices().skip(1) {
        if matches!(c, '"' | ';') {
            return None;
        }
        let blank = matches!(c, ' ' | '\t');
        if blank && !previous_blank {
            let (prefix, tail) = line.split_at_checked(at)?;
            if let Some((number, rest)) = number_and_currency(tail) {
                return Some(Shape::Dated {
                    prefix,
                    number,
                    rest,
                });
            }
        }
        previous_blank = blank;
    }
    None
}

/// Shape 3: indent, then the start of an account (the first colon and the character after it
/// are enough), then anything.
fn account_line(line: &str) -> Option<Shape<'_>> {
    let account_start = skip_blanks(line);
    let indent = line.len() - account_start.len();
    if indent == 0 {
        return None;
    }
    account_start
        .strip_prefix(char::is_uppercase)?
        .trim_start_matches(is_name_char)
        .strip_prefix(':')?
        .starts_with(|c: char| c.is_uppercase() || c.is_numeric())
        .then_some(Shape::Account {
            indent,
            rest: account_start,
        })
}
