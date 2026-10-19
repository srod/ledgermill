//! Line-by-line reading of one source file into transactions and findings.
//!
//! The rule that shapes everything here: a line is either understood exactly, or it produces an
//! `unsupported-syntax` finding. A transaction with any refused line is dropped, never
//! half-reported.

use crate::amount::{ParsedAmount, parse_amount};
use crate::model::{
    Amount, Finding, Posting, PostingKind, Severity, Source, SourceFile, Status, Transaction,
};
use crate::number::Decimal;
use serde_json::json;

/// What was refused, as a stable machine name plus a short sentence.
#[derive(Debug, Clone, Copy)]
enum Construct {
    Directive,
    PeriodicOrAutoTransaction,
    Date,
    HeaderSyntax,
    Code,
    ControlCharacter,
    SpacesAtEndOfFile,
    PostingDate,
    IndentedLine,
    ElidedAmount,
    AccountName,
    Amount,
    MultipleCommodities,
    NumberTooLarge,
}

impl Construct {
    fn id(self) -> &'static str {
        match self {
            Construct::Directive => "directive",
            Construct::PeriodicOrAutoTransaction => "periodic-or-auto-transaction",
            Construct::Date => "date",
            Construct::HeaderSyntax => "header-syntax",
            Construct::Code => "transaction-code",
            Construct::ControlCharacter => "control-character",
            Construct::SpacesAtEndOfFile => "spaces-at-end-of-file",
            Construct::PostingDate => "posting-date",
            Construct::IndentedLine => "indented-line",
            Construct::ElidedAmount => "elided-amount",
            Construct::AccountName => "account-name",
            Construct::Amount => "amount",
            Construct::MultipleCommodities => "multiple-commodities",
            Construct::NumberTooLarge => "number-too-large",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Construct::Directive => {
                "Directives and other non-transaction lines are not supported yet."
            }
            Construct::PeriodicOrAutoTransaction => {
                "Periodic and auto-posting rules are not supported yet."
            }
            Construct::Date => "Only valid dates written YYYY-MM-DD are supported.",
            Construct::HeaderSyntax => "This transaction header form is not supported yet.",
            Construct::Code => "Transaction codes are not supported yet.",
            Construct::ControlCharacter => {
                "Tabs and unusual whitespace are only supported in indentation."
            }
            Construct::SpacesAtEndOfFile => {
                "A last line of only spaces with no newline after it is not supported."
            }
            Construct::PostingDate => {
                "Dates inside posting comments are only supported as date:YYYY-MM-DD."
            }
            Construct::IndentedLine => "An indented line outside a transaction is not supported.",
            Construct::ElidedAmount => "Postings without an amount are not supported yet.",
            Construct::AccountName => "This account name form is not supported yet.",
            Construct::Amount => "This amount form is not supported yet.",
            Construct::MultipleCommodities => {
                "Several commodities in one transaction are not supported yet."
            }
            Construct::NumberTooLarge => "This number is too large to handle exactly.",
        }
    }
}

pub(crate) struct Output {
    pub transactions: Vec<Transaction>,
    /// `(index of the file in load order, finding)`; sorted by the caller.
    pub findings: Vec<(usize, Finding)>,
}

struct PostingDraft {
    line: u32,
    account: String,
    comment_lines: Vec<String>,
    amount: ParsedAmount,
}

struct TxDraft {
    line: u32,
    date: String,
    status: Status,
    description: String,
    comment_lines: Vec<String>,
    postings: Vec<PostingDraft>,
    /// A line of this transaction was refused: it is reported through findings only.
    refused: bool,
}

enum State {
    /// Between transactions.
    Top,
    InTransaction(TxDraft),
    /// Inside the indented lines of a refused column-0 line; they belong to it.
    Refused,
}

struct FileParser<'a> {
    file_index: usize,
    file: &'a SourceFile,
    out: &'a mut Output,
}

pub(crate) fn parse_file(file_index: usize, file: &SourceFile, out: &mut Output) {
    let mut parser = FileParser {
        file_index,
        file,
        out,
    };
    let text = file
        .content
        .strip_prefix('\u{feff}')
        .unwrap_or(&file.content);
    let mut state = State::Top;
    let last = text.split('\n').count() - 1;
    for (i, raw) in text.split('\n').enumerate() {
        let line_no = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        // The reference tool fails on a final line of only blanks that has no newline after it.
        if i == last && !raw.is_empty() && line.bytes().all(|b| b == b' ') {
            parser.refuse(line_no, line, 0, Construct::SpacesAtEndOfFile, None);
            parser.finish(state);
            return;
        }
        state = parser.line(state, line_no, line);
    }
    parser.finish(state);
}

/// Byte offset of the first character that is a control character or whitespace other than a
/// plain space.
fn odd_char(text: &str) -> Option<usize> {
    text.char_indices()
        .find(|&(_, c)| c.is_control() || (c.is_whitespace() && c != ' '))
        .map(|(i, _)| i)
}

fn valid_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| -> Option<u32> {
        let s = &text[r];
        if s.bytes().all(|b| b.is_ascii_digit()) {
            s.parse().ok()
        } else {
            None
        }
    };
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1000..=9999).contains(&year) && (1..=days).contains(&day)
}

/// The reference tool reads `date:` and `date2:` tags and `[...]` dates in *posting* comments and
/// fails on a bad one. Only well-formed tags are accepted here; they change nothing we report.
fn posting_comment_ok(comment: &str) -> bool {
    if comment.contains('[') {
        return false;
    }
    ["date:", "date2:"].iter().all(|tag| {
        comment.match_indices(tag).all(|(at, _)| {
            let value = comment[at + tag.len()..].trim_start_matches(' ');
            valid_date(value.split([' ', ',']).next().unwrap_or(""))
        })
    })
}

/// Join comment lines the way the reference tool reports them, then trim.
fn join_comment(lines: &[String]) -> String {
    lines.join("\n").trim().to_string()
}

impl FileParser<'_> {
    /// Refuse the text at `byte_offset` of `line`.
    fn refuse(
        &mut self,
        line_no: u32,
        line: &str,
        byte_offset: usize,
        construct: Construct,
        text: Option<&str>,
    ) {
        let column = line
            .get(..byte_offset)
            .map(|s| u32::try_from(s.chars().count() + 1).unwrap_or(u32::MAX));
        self.refuse_at(line_no, column, construct, text);
    }

    fn refuse_at(
        &mut self,
        line_no: u32,
        column: Option<u32>,
        construct: Construct,
        text: Option<&str>,
    ) {
        let mut details = json!({ "construct": construct.id() });
        if let Some(text) = text {
            details["text"] = json!(text);
        }
        self.out.findings.push((
            self.file_index,
            Finding {
                code: "unsupported-syntax".into(),
                severity: Severity::Error,
                file: self.file.name.clone(),
                line: line_no,
                column,
                details,
                message: construct.message().into(),
            },
        ));
    }

    fn line(&mut self, state: State, line_no: u32, line: &str) -> State {
        // Blank line: ends a transaction.
        if line.bytes().all(|b| b == b' ') {
            self.finish(state);
            return State::Top;
        }
        let first = line.as_bytes()[0];

        // Column-0 comment: ignored, and ends a transaction.
        if matches!(first, b';' | b'#' | b'*') {
            self.finish(state);
            if let Some(at) = line.find('\r') {
                self.refuse(line_no, line, at, Construct::ControlCharacter, None);
            }
            return State::Top;
        }

        if first == b' ' || first == b'\t' {
            return self.indented(state, line_no, line);
        }

        // Any other column-0 line starts something new.
        self.finish(state);
        if first.is_ascii_digit() {
            match Self::header(line_no, line) {
                Ok(draft) => State::InTransaction(draft),
                Err((at, construct)) => {
                    self.refuse(line_no, line, at, construct, None);
                    State::Refused
                }
            }
        } else {
            let construct = if first == b'~' || first == b'=' {
                Construct::PeriodicOrAutoTransaction
            } else {
                Construct::Directive
            };
            self.refuse(line_no, line, 0, construct, None);
            State::Refused
        }
    }

    fn header(line_no: u32, line: &str) -> Result<TxDraft, (usize, Construct)> {
        let date = line
            .get(..10)
            .filter(|d| valid_date(d))
            .ok_or((0, Construct::Date))?;
        let rest = &line[10..];
        if let Some(at) = odd_char(rest) {
            return Err((10 + at, Construct::ControlCharacter));
        }
        if !rest.is_empty() && !rest.starts_with(' ') {
            return Err((10, Construct::HeaderSyntax));
        }
        let mut body = rest.trim_start_matches(' ');
        let mut status = Status::Unmarked;
        if let Some(mark) = body.chars().next().filter(|c| matches!(c, '*' | '!')) {
            let after = &body[1..];
            if !after.is_empty() && !after.starts_with(' ') {
                return Err((line.len() - body.len(), Construct::HeaderSyntax));
            }
            status = if mark == '*' {
                Status::Cleared
            } else {
                Status::Pending
            };
            body = after.trim_start_matches(' ');
        }
        if body.starts_with('(') {
            return Err((line.len() - body.len(), Construct::Code));
        }
        let (description, comment) = match body.split_once(';') {
            Some((description, comment)) => (description, Some(comment.trim().to_string())),
            None => (body, None),
        };
        Ok(TxDraft {
            line: line_no,
            date: date.to_string(),
            status,
            description: description.trim_end_matches(' ').to_string(),
            comment_lines: comment.into_iter().collect(),
            postings: Vec::new(),
            refused: false,
        })
    }

    fn indented(&mut self, state: State, line_no: u32, line: &str) -> State {
        let content = line.trim_start_matches([' ', '\t']);
        let indent = line.len() - content.len();
        if content.is_empty() {
            self.refuse(
                line_no,
                line,
                indent.saturating_sub(1),
                Construct::ControlCharacter,
                None,
            );
            return Self::mark_refused(state);
        }
        if let Some(at) = odd_char(content) {
            self.refuse(
                line_no,
                line,
                indent + at,
                Construct::ControlCharacter,
                None,
            );
            return Self::mark_refused(state);
        }
        match state {
            State::Refused => State::Refused,
            State::Top => {
                // An indented comment outside a transaction is ignored; anything else is out of place.
                if !content.starts_with(';') {
                    self.refuse(line_no, line, indent, Construct::IndentedLine, None);
                }
                State::Top
            }
            State::InTransaction(mut tx) => {
                if let Some(comment) = content.strip_prefix(';') {
                    let comment = comment.trim().to_string();
                    match tx.postings.last_mut() {
                        Some(_) if !posting_comment_ok(&comment) => {
                            self.refuse(line_no, line, indent, Construct::PostingDate, None);
                            tx.refused = true;
                        }
                        Some(posting) => posting.comment_lines.push(comment),
                        None => tx.comment_lines.push(comment),
                    }
                } else {
                    self.posting(&mut tx, line_no, line, indent);
                }
                State::InTransaction(tx)
            }
        }
    }

    fn mark_refused(state: State) -> State {
        match state {
            State::InTransaction(mut tx) => {
                tx.refused = true;
                State::InTransaction(tx)
            }
            other => other,
        }
    }

    fn posting(&mut self, tx: &mut TxDraft, line_no: u32, line: &str, indent: usize) {
        let content = &line[indent..];
        let (account, rest) = match content.find("  ") {
            Some(at) => (&content[..at], content[at..].trim_start_matches(' ')),
            None => (content, ""),
        };
        let valid_account = account.chars().next().is_some_and(char::is_alphanumeric)
            && account.chars().all(|c| {
                c.is_alphanumeric()
                    || matches!(
                        c,
                        ':' | '-' | '_' | '.' | '&' | '\'' | '/' | ',' | '+' | ' '
                    )
            });
        if !valid_account {
            self.refuse(line_no, line, indent, Construct::AccountName, None);
            tx.refused = true;
            return;
        }
        let (amount_text, comment) = match rest.split_once(';') {
            Some((amount, comment)) => (
                amount.trim_end_matches(' '),
                Some(comment.trim().to_string()),
            ),
            None => (rest.trim_end_matches(' '), None),
        };
        let amount_at = line.len() - rest.len();
        if comment.as_deref().is_some_and(|c| !posting_comment_ok(c)) {
            self.refuse(line_no, line, indent, Construct::PostingDate, None);
            tx.refused = true;
            return;
        }
        if amount_text.is_empty() {
            self.refuse(line_no, line, indent, Construct::ElidedAmount, None);
            tx.refused = true;
            return;
        }
        let Some(amount) = parse_amount(amount_text) else {
            let construct = if amount_text.bytes().filter(u8::is_ascii_digit).count() > 15 {
                Construct::NumberTooLarge
            } else {
                Construct::Amount
            };
            self.refuse(line_no, line, amount_at, construct, Some(amount_text));
            tx.refused = true;
            return;
        };
        tx.postings.push(PostingDraft {
            line: line_no,
            account: account.to_string(),
            comment_lines: comment.into_iter().collect(),
            amount,
        });
    }

    /// End of a transaction: check it and, if nothing was refused, keep it.
    fn finish(&mut self, state: State) {
        let State::InTransaction(mut tx) = state else {
            return;
        };

        // One commodity per transaction in this slice.
        let first = tx.postings.first().map(|p| p.amount.commodity.clone());
        if let Some(first) = first {
            let offending: Vec<u32> = tx
                .postings
                .iter()
                .filter(|p| p.amount.commodity != first)
                .map(|p| p.line)
                .collect();
            for line_no in offending {
                self.refuse_at(line_no, None, Construct::MultipleCommodities, None);
                tx.refused = true;
            }
        }
        if tx.refused {
            return;
        }

        let numbers: Vec<Decimal> = tx.postings.iter().map(|p| p.amount.number).collect();
        match Decimal::sum(&numbers) {
            None => {
                self.refuse_at(tx.line, None, Construct::NumberTooLarge, None);
                return;
            }
            Some(residual) if !residual.is_zero() => {
                let commodity = tx
                    .postings
                    .first()
                    .map(|p| p.amount.commodity.clone())
                    .unwrap_or_default();
                self.out.findings.push((
                    self.file_index,
                    Finding {
                        code: "unbalanced-transaction".into(),
                        severity: Severity::Error,
                        file: self.file.name.clone(),
                        line: tx.line,
                        column: None,
                        details: json!({ "commodity": commodity, "residual": residual.to_string() }),
                        message: "The postings do not add up to zero.".into(),
                    },
                ));
            }
            Some(_) => {}
        }

        let postings = tx
            .postings
            .into_iter()
            .map(|p| Posting {
                account: p.account,
                kind: PostingKind::Regular,
                comment: join_comment(&p.comment_lines),
                amounts: vec![Amount {
                    commodity: p.amount.commodity,
                    quantity: p.amount.number.to_string(),
                }],
            })
            .collect();
        self.out.transactions.push(Transaction {
            date: tx.date,
            status: tx.status,
            description: tx.description,
            comment: join_comment(&tx.comment_lines),
            source: Source {
                file: self.file.name.clone(),
                line: tx.line,
            },
            postings,
        });
    }
}
