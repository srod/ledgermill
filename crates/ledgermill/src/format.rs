//! `format`: realign the posting lines of an hledger journal (`spec/hledger/format.md`).

use crate::chars::is_blank;
use crate::model::{Finding, SourceFile};
use crate::parse::{self, PostingLayout};
use crate::{Ledger, beancount, load};

/// Posting lines start with this many spaces.
const INDENT: usize = 4;
/// The least space between an account and its amount.
const GAP: usize = 2;

/// Format an hledger journal: postings indented by 4 spaces, and in each transaction the amounts
/// of its postings ending in one column, at least 2 spaces after the account.
///
/// Only whitespace changes. Comments, directives, prices, line order, blank lines, number
/// notation and line endings (CRLF, BOM, a missing final newline) are kept; so is everything
/// ledgermill does not read as a transaction. If the result would not give the same transactions,
/// postings, prices and findings (a finding's column and message aside), the file is returned as
/// it was. Formatting the result again changes nothing.
///
/// A Beancount file (`.beancount`, `.bean`) is formatted like `bean-format` does
/// (`spec/beancount/format.md`): postings, `balance` and `price` lines get the amounts of the
/// whole file in one column and the file's usual indent. A file ledgermill refuses or cannot
/// parse is returned as it was.
pub fn format(file: &SourceFile) -> String {
    let formatted = if beancount::is_beancount(&file.name) {
        beancount::realign(file)
    } else {
        realign(file)
    };
    if formatted != file.content && same_ledger(file, &formatted) {
        formatted
    } else {
        file.content.clone()
    }
}

/// The file with its posting lines rewritten.
fn realign(file: &SourceFile) -> String {
    let (bom, text) = match file.content.strip_prefix('\u{feff}') {
        Some(text) => ("\u{feff}", text),
        None => ("", file.content.as_str()),
    };
    let mut out = parse::Output {
        transactions: Vec::new(),
        prices: Vec::new(),
        findings: Vec::new(),
        layouts: Vec::new(),
    };
    parse::parse_file(0, file, &mut out);
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for tx in &out.layouts {
        for (index, rewritten) in rewrite_transaction(&lines, tx) {
            if let Some(line) = lines.get_mut(index) {
                *line = rewritten;
            }
        }
    }
    format!("{bom}{}", lines.join("\n"))
}

/// The posting lines of one transaction, rewritten: `(index into lines, new text)`.
fn rewrite_transaction(lines: &[String], tx: &[PostingLayout]) -> Vec<(usize, String)> {
    let pieces: Option<Vec<Pieces>> = tx.iter().map(|p| Pieces::of(lines, p)).collect();
    let Some(pieces) = pieces else {
        return Vec::new();
    };
    // shortcut: char count, not display width (CJK/emoji misalign), upgrade if users report it
    let column = pieces
        .iter()
        .filter_map(|p| p.amount.map(|a| INDENT + width(p.head) + GAP + width(a)))
        .max()
        .unwrap_or(0);
    pieces
        .iter()
        .map(|p| {
            let mut text = " ".repeat(INDENT);
            text.push_str(p.head);
            match p.amount {
                Some(amount) => {
                    let pad = column - INDENT - width(p.head) - width(amount);
                    text.push_str(&" ".repeat(pad));
                    text.push_str(p.tail);
                }
                None => text.push_str(p.tail),
            }
            if p.carriage_return {
                text.push('\r');
            }
            (p.index, text)
        })
        .collect()
}

fn width(text: &str) -> usize {
    text.chars().count()
}

/// A posting line cut where `format` needs it.
struct Pieces<'a> {
    index: usize,
    /// Status, brackets and account, as written.
    head: &'a str,
    /// The amount alone, if the posting has one.
    amount: Option<&'a str>,
    /// What follows the head (no amount) or starts at the amount (its cost, comment, ...), as
    /// written, without trailing spaces and tabs.
    tail: &'a str,
    carriage_return: bool,
}

impl<'a> Pieces<'a> {
    /// `None` when the parser's offsets do not cut this line (they always should): the caller then
    /// leaves the whole transaction as written rather than misalign it.
    fn of(lines: &'a [String], p: &PostingLayout) -> Option<Pieces<'a>> {
        let index = usize::try_from(p.line).ok()?.checked_sub(1)?;
        let raw = lines.get(index)?;
        let carriage_return = raw.ends_with('\r');
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let content = line.trim_start_matches(is_blank);
        let head = content.get(..p.account_end)?;
        let (amount, from) = match p.amount {
            Some((start, end)) => (Some(content.get(start..end)?), start),
            None => (None, p.account_end),
        };
        let tail = content.get(from..)?.trim_end_matches([' ', '\t']);
        Some(Pieces {
            index,
            head,
            amount,
            tail,
            carriage_return,
        })
    }
}

/// Whether `formatted` reads as the same ledger as `file`.
fn same_ledger(file: &SourceFile, formatted: &str) -> bool {
    let before = load(std::slice::from_ref(file));
    let after = load(&[SourceFile {
        name: file.name.clone(),
        content: formatted.to_string(),
    }]);
    before.transactions() == after.transactions()
        && before.postings() == after.postings()
        && before.prices() == after.prices()
        && findings(&before) == findings(&after)
}

/// The findings without what moves with the posting lines (the column and the message), in an
/// order that does not depend on the run: Beancount's findings of one line come out in a
/// different order from one load to the next, which would make the check fail by chance.
fn findings(ledger: &Ledger) -> Vec<Finding> {
    let mut found: Vec<Finding> = ledger
        .check()
        .iter()
        .map(|f| Finding {
            column: None,
            message: String::new(),
            ..f.clone()
        })
        .collect();
    found.sort_by_cached_key(|f| (f.line, f.code.clone(), f.details.to_string()));
    found
}
