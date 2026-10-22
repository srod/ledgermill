//! Line-by-line reading of one source file into transactions and findings.
//!
//! A line is either understood exactly, or it produces a finding: `parse-error` when the
//! reference tool rejects it too, `unsupported-syntax` when it is something ledgermill does not
//! read yet. A transaction with any such line is dropped, never half-reported.

use crate::alias::{Alias, parse_alias};
use crate::amount::{Default, Styles, read_amount, read_symbol};
use crate::balance::{Draft, Unbalanced};
use crate::chars::{is_blank, trim_blank};
use crate::cost;
use crate::cursor::Cursor;
use crate::date::{Date, YearSource, parse_date};
use crate::exact::{Exact, ExactPosting};
use crate::model::{
    Amount, Cost, Finding, Lot, Posting, PostingKind, Quantity, Severity, Source, SourceFile,
    Status, Transaction,
};
use crate::number::Mark;
use crate::period::{self, Verdict};
use crate::posting::{CostKind, Mode, WrittenAmount, parse_posting};
use crate::prices::Price;
use crate::problem::{Parsed, Problem, Unsupported, invalid};
use crate::tags::{check_posting_comment, posting_date, tags};
use crate::walk::{self, Assertion, Issue, PostingInfo, Tx};
use regex_lite::Regex;
use serde_json::json;
use std::sync::OnceLock;

pub(crate) struct Output {
    /// `(transaction, what `at_cost` and `value` need of it, date)`; sorted by the caller.
    pub transactions: Vec<(Transaction, Exact, Date)>,
    /// Every `P` line, in parse order: the files as given, each from top to bottom.
    pub prices: Vec<Price>,
    /// `(index of the file in load order, finding)`; sorted by the caller.
    pub findings: Vec<(usize, Finding)>,
}

struct PostingDraft {
    account: String,
    kind: PostingKind,
    line: u32,
    comment_lines: Vec<String>,
    amount: Option<WrittenAmount>,
    assertion: Option<Assertion>,
}

struct TxDraft {
    line: u32,
    date: Date,
    status: Status,
    description: String,
    comment_lines: Vec<String>,
    postings: Vec<PostingDraft>,
    /// A line of this transaction was refused: it is reported through findings only.
    refused: bool,
}

enum State {
    Top,
    InTransaction(TxDraft),
    /// The indented lines after a directive: only comments, plus `format` under `commodity`
    /// and `type:` tags under `account`.
    InDirective(DirectiveBody),
    /// Inside the indented lines of something refused or ignored; they belong to it.
    Skipping,
    /// Between `comment` and `end comment`.
    CommentBlock,
    /// The indented lines of an automated or periodic rule.
    InRule(RuleDraft),
}

/// A rule being read. Nothing of it is kept: hledger applies rules only when asked to with
/// `--auto` or `--forecast`, which ledgermill never does.
struct RuleDraft {
    mode: Mode,
    /// The styles of the file, but for values: no rule's number is read.
    styles: Styles,
    /// Where a date without a year in a posting comment gets its year.
    comment_year: YearSource,
    /// The same for the date of a lot.
    lot_year: YearSource,
    /// A posting has been read: comment lines are read from then on.
    has_posting: bool,
}

enum DirectiveBody {
    Plain,
    Account(String),
    Commodity(String),
    /// The code after `python`: any indented or blank lines.
    Python,
    /// The lines under `payee` and `tag`: any text, indented.
    Free,
}

/// What a file has set up so far; directives reach only the lines after them in their file.
#[derive(Default)]
struct FileState {
    styles: Styles,
    year: Option<u32>,
    aliases: Vec<Alias>,
    parents: Vec<String>,
    /// Accounts declared with type `V` (Conversion).
    conversion_accounts: Vec<String>,
}

/// Conversion accounts by name, used while the file declares none.
fn conversion_name() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        #[expect(clippy::expect_used, reason = "the pattern is a constant")]
        Regex::new(r"(?i)^equity:(conversion|trades?|trading)(:|$)").expect("valid regex")
    })
}

const ACCOUNT_TYPES: [&str; 18] = [
    "a",
    "l",
    "e",
    "r",
    "x",
    "c",
    "v",
    "g",
    "u",
    "asset",
    "liability",
    "equity",
    "revenue",
    "expense",
    "cash",
    "conversion",
    "gain",
    "unrealisedgain",
];

struct FileParser<'a> {
    file_index: usize,
    file: &'a SourceFile,
    out: &'a mut Output,
    st: FileState,
    /// Transactions read without a refused line, in file order, waiting for the file's walk.
    done: Vec<TxDraft>,
    /// Conversion-account flags of `done`'s postings, decided when each transaction ended.
    conversion: Vec<Vec<bool>>,
    /// Findings reported while reading lines: the file was not read completely if there are any.
    reported: usize,
    /// Every line of the file, for the directives that read past their own.
    lines: &'a [&'a str],
    /// Index of the first line not yet read: lines before it were taken by an earlier one.
    consumed: usize,
}

pub(crate) fn parse_file(file_index: usize, file: &SourceFile, out: &mut Output) {
    let text = file
        .content
        .strip_prefix('\u{feff}')
        .unwrap_or(&file.content);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut parser = FileParser {
        file_index,
        file,
        out,
        st: FileState::default(),
        done: Vec::new(),
        conversion: Vec::new(),
        reported: 0,
        lines: &lines,
        consumed: 0,
    };
    // The reference tool reads these by their extension; ledgermill reads journals.
    if let Some(kind) = other_format(&file.name) {
        parser.report(1, None, Problem::Unsupported(kind), None);
        parser.conclude();
        return;
    }
    let mut state = State::Top;
    let last = lines.len() - 1;
    let mut i = 0;
    while let Some(raw) = lines.get(i) {
        let line_no = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        // The reference tool fails on a final line of only blanks that has no newline after it.
        if i == last && !raw.is_empty() && line.chars().all(|c| c == ' ' || c == '\t') {
            parser.finish(state);
            parser.report(
                line_no,
                None,
                Problem::Invalid("The file ends inside a line."),
                None,
            );
            parser.conclude();
            return;
        }
        state = parser.line(state, line_no, line);
        // A directive may have taken the lines after its own (a plain `alias`).
        i = (i + 1).max(parser.consumed);
    }
    parser.finish(state);
    parser.conclude();
}

/// The kind of a file that is not a journal, by its extension, however it is capitalised.
fn other_format(name: &str) -> Option<Unsupported> {
    let (_, extension) = name.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "timeclock" => Some(Unsupported::Timeclock),
        "timedot" => Some(Unsupported::Timedot),
        "csv" | "tsv" | "ssv" => Some(Unsupported::Csv),
        _ => None,
    }
}

fn finding(
    file: &str,
    line: u32,
    column: Option<u32>,
    problem: Problem,
    text: Option<&str>,
) -> Finding {
    let (code, details, message) = match problem {
        Problem::Invalid(reason) => ("parse-error", json!({}), reason),
        Problem::Unsupported(u) => (
            "unsupported-syntax",
            json!({ "construct": u.id() }),
            u.message(),
        ),
    };
    let mut details = details;
    if let Some(text) = text {
        details["text"] = json!(text);
    }
    Finding {
        code: code.into(),
        severity: Severity::Error,
        file: file.into(),
        line,
        column,
        details,
        message: message.into(),
    }
}

/// Join comment lines the way the reference tool reports them, then trim.
fn join_comment(lines: &[String]) -> String {
    trim_blank(&lines.join("\n")).to_string()
}

fn to_quantity(commodity: &str, value: &crate::decimal::Decimal) -> Quantity {
    Quantity {
        commodity: commodity.to_string(),
        quantity: value.output(),
    }
}

/// `*` and nothing else (blanks aside): a line `!` alone is an error instead.
fn is_lone_star(content: &str) -> bool {
    content
        .strip_prefix('*')
        .is_some_and(|rest| trim_blank(rest).is_empty())
}

impl FileParser<'_> {
    fn report(&mut self, line_no: u32, column: Option<u32>, problem: Problem, text: Option<&str>) {
        self.reported += 1;
        self.out.findings.push((
            self.file_index,
            finding(&self.file.name, line_no, column, problem, text),
        ));
    }

    fn line(&mut self, state: State, line_no: u32, line: &str) -> State {
        if let State::CommentBlock = state {
            let Some(after) = line.strip_prefix("end comment") else {
                return State::CommentBlock;
            };
            if !trim_blank(after).is_empty() {
                self.report(
                    line_no,
                    None,
                    Problem::Invalid("Unexpected text after end comment."),
                    None,
                );
            }
            return State::Top;
        }
        // Blank line: ends a transaction or a directive's indented lines (not a python block).
        if line.chars().all(|c| c == ' ' || c == '\t') {
            if let State::InDirective(DirectiveBody::Python) = state {
                return state;
            }
            self.finish(state);
            return State::Top;
        }
        let first = line.chars().next().unwrap_or(' ');
        if matches!(first, ';' | '#' | '*') {
            self.finish(state);
            return State::Top;
        }
        if first == ' ' || first == '\t' {
            return self.indented(state, line_no, line);
        }
        self.finish(state);
        if line.contains('\r') {
            self.report(
                line_no,
                None,
                Problem::Unsupported(Unsupported::ControlCharacter),
                None,
            );
            return State::Skipping;
        }
        if first.is_ascii_digit() {
            return match self.header(line_no, line) {
                Ok(draft) => State::InTransaction(draft),
                Err(p) => {
                    self.report(line_no, None, p, None);
                    State::Skipping
                }
            };
        }
        if matches!(first, '=' | '~') {
            return self.rule_header(line_no, line);
        }
        match self.directive(line_no, line) {
            Ok(body) => match body {
                Some(body) => State::InDirective(body),
                None => State::CommentBlock,
            },
            Err(p) => {
                self.report(line_no, None, p, None);
                State::Skipping
            }
        }
    }

    /// `= QUERY` or `~ PERIOD  DESCRIPTION`. The query of an automated rule is not read (it is
    /// only an error when the rule is applied); the period of a periodic one is.
    fn rule_header(&mut self, line_no: u32, line: &str) -> State {
        let automated = line.starts_with('=');
        if !automated {
            let expression = period::header_expression(line.get(1..).unwrap_or(""));
            let problem = match period::judge(expression) {
                Verdict::Valid => None,
                Verdict::Invalid => Some(Problem::Invalid(
                    "A periodic rule needs a valid period expression.",
                )),
                Verdict::Unsettled => Some(Problem::Unsupported(Unsupported::PeriodExpression)),
            };
            if let Some(problem) = problem {
                self.report(line_no, None, problem, None);
                return State::Skipping;
            }
        }
        let (construct, message) = if automated {
            (
                "automated-rule",
                "An automated rule is read and left out: it only applies with --auto.",
            )
        } else {
            (
                "periodic-rule",
                "A periodic rule is read and left out: it only applies with --forecast.",
            )
        };
        self.out.findings.push((
            self.file_index,
            Finding {
                code: "ignored-syntax".into(),
                severity: Severity::Warning,
                file: self.file.name.clone(),
                line: line_no,
                column: None,
                details: json!({ "construct": construct }),
                message: message.into(),
            },
        ));
        // An automated rule has no year at all; a periodic one takes the `Y` directive's, or
        // else the current year.
        let lot_year = self.st.year.map_or(YearSource::Current, YearSource::Fixed);
        State::InRule(RuleDraft {
            mode: if automated {
                Mode::AutomatedRule
            } else {
                Mode::PeriodicRule
            },
            styles: Styles {
                shape_only: true,
                ..self.st.styles.clone()
            },
            comment_year: if automated {
                YearSource::Missing
            } else {
                lot_year
            },
            lot_year,
            has_posting: false,
        })
    }

    /// An indented line of a rule: a posting, or a comment (read once a posting is above it).
    fn rule_line(&mut self, mut rule: RuleDraft, line_no: u32, content: &str) -> State {
        let comment = if let Some(comment) = content.strip_prefix(';') {
            rule.has_posting.then(|| comment.to_string())
        } else {
            rule.has_posting = true;
            match parse_posting(content, &rule.styles, rule.lot_year, rule.mode) {
                Ok(posting) => posting.comment,
                Err(problem) => {
                    self.report(line_no, None, problem, Some(content));
                    None
                }
            }
        };
        if let Some(Err(problem)) = comment.map(|c| check_posting_comment(&c, rule.comment_year)) {
            self.report(line_no, None, problem, None);
        }
        State::InRule(rule)
    }

    fn header(&self, line_no: u32, line: &str) -> Parsed<TxDraft> {
        let mut cur = Cursor::new(line);
        let date = parse_date(&mut cur, self.st.year)?;
        if cur.eat('=') {
            parse_date(&mut cur, Some(date.year()))?;
        }
        if !cur.at_end() && !cur.peek().is_some_and(is_blank) {
            return invalid("Unexpected text after the date.");
        }
        cur.take_while(is_blank);
        let mut status = Status::Unmarked;
        if let Some(mark) = cur.peek().filter(|c| matches!(c, '*' | '!')) {
            cur.bump();
            status = if mark == '*' {
                Status::Cleared
            } else {
                Status::Pending
            };
            cur.take_while(is_blank);
        }
        if cur.eat('(') {
            let code = cur.take_while(|c| c != ')');
            if !cur.eat(')') {
                return invalid("A transaction code is not closed.");
            }
            let _ = code;
            cur.take_while(is_blank);
        }
        let rest = cur.rest();
        let (description, comment) = match rest.split_once(';') {
            Some((d, c)) => (d, Some(trim_blank(c).to_string())),
            None => (rest, None),
        };
        Ok(TxDraft {
            line: line_no,
            date,
            status,
            description: description.trim_end_matches(is_blank).to_string(),
            comment_lines: comment.into_iter().collect(),
            postings: Vec::new(),
            refused: false,
        })
    }

    /// A column-0 line that is not a transaction or a comment. `Ok(None)` opens a comment block.
    fn directive(&mut self, line_no: u32, line: &str) -> Parsed<Option<DirectiveBody>> {
        // A line that starts with two dashes is read and dropped, whatever follows.
        if line.starts_with("--") {
            return Ok(Some(DirectiveBody::Plain));
        }
        let (word, rest) = match line.find(is_blank) {
            Some(i) => (&line[..i], &line[i..]),
            None => (line, ""),
        };
        let arg = trim_blank(rest);
        let plain = Ok(Some(DirectiveBody::Plain));
        match word {
            "comment" if arg.is_empty() => Ok(None),
            "include" => Err(Problem::Unsupported(Unsupported::Include)),
            "!include" => invalid("!include is not a directive; the reference tool rejects it."),
            "P" => self.price_line(rest),
            w if w.starts_with('P') && w[1..].starts_with(|c: char| c.is_ascii_digit()) => {
                self.price_line(&line[1..])
            }
            "D" => self
                .default_commodity(rest)
                .map(|()| Some(DirectiveBody::Plain)),
            "decimal-mark" => {
                let mark = arg.chars().next().and_then(Mark::of);
                let Some(mark) = mark else {
                    return invalid("decimal-mark takes `.` or `,`.");
                };
                self.st.styles.decimal_mark = Some(mark);
                plain
            }
            "commodity" => self.commodity(rest),
            "account" => self.account(rest),
            "alias" => self.alias(line_no, rest),
            "end" => self.end(rest),
            "apply" => self.apply(rest),
            "Y" | "year" => self.year(arg),
            w if w.starts_with('Y') && w[1..].starts_with(|c: char| c.is_ascii_digit()) => {
                self.year(&line[1..])
            }
            w if w.starts_with("year") && w[4..].starts_with(|c: char| c.is_ascii_digit()) => {
                self.year(&line[4..])
            }
            "payee" => {
                // The name is what comes before a comment.
                if trim_blank(rest.split(';').next().unwrap_or("")).is_empty() {
                    return invalid("payee needs a name.");
                }
                Ok(Some(DirectiveBody::Free))
            }
            "tag" => {
                // One word, then at most a comment.
                let name = rest.trim_start_matches([' ', '\t']);
                let after = name.trim_start_matches(|c| c != ' ' && c != '\t');
                let after = after.trim_start_matches([' ', '\t']);
                if name.is_empty() {
                    return invalid("tag needs a name.");
                }
                if !(after.is_empty() || after.starts_with(';')) {
                    return invalid("A tag name is one word.");
                }
                Ok(Some(DirectiveBody::Free))
            }
            // These two need a space after the word, and take anything.
            "bucket" | "A" if rest.starts_with(' ') => plain,
            "bucket" | "A" => invalid("This directive needs an account."),
            "N" => {
                let mut cur = Cursor::new(rest);
                cur.take_while(is_blank);
                read_symbol(&mut cur)
                    .transpose()?
                    .ok_or(Problem::Invalid("N needs a commodity symbol."))?;
                plain
            }
            "C" => self
                .conversion_rate(rest)
                .map(|()| Some(DirectiveBody::Plain)),
            // Any word that starts this way: the indented lines after it are code.
            w if w.starts_with("python") => Ok(Some(DirectiveBody::Python)),
            // Ledger's directives, which hledger reads and drops; any word that starts this way.
            w if [
                "assert", "capture", "check", "define", "eval", "expr", "value",
            ]
            .iter()
            .any(|d| w.starts_with(d)) =>
            {
                plain
            }
            _ => invalid("This line is not a transaction or a directive."),
        }
    }

    /// `C AMOUNT = AMOUNT`: checked only; what follows the second amount is not read.
    fn conversion_rate(&self, rest: &str) -> Parsed<()> {
        let mut cur = Cursor::new(rest);
        cur.take_while(is_blank);
        read_amount(&mut cur, &self.st.styles, Default::Ignore)?;
        cur.take_while(is_blank);
        if !cur.eat('=') {
            return invalid("C needs an `=` between its two amounts.");
        }
        cur.take_while(is_blank);
        read_amount(&mut cur, &self.st.styles, Default::Ignore)?;
        Ok(())
    }

    /// `alias OLD = NEW`. A plain name reads on past its line to the first `=` of the file, so
    /// the lines in between belong to it; with no `=` left the error is at the end of the file.
    fn alias(&mut self, line_no: u32, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let text = rest.trim_start_matches([' ', '\t']);
        if rest.is_empty() || text.starts_with('=') {
            return invalid("An alias needs something before the `=`.");
        }
        if text.starts_with('/') || text.contains('=') {
            self.st.aliases.push(parse_alias(rest)?);
            return Ok(Some(DirectiveBody::Plain));
        }
        let mut old = text.to_string();
        for (i, later) in self
            .lines
            .iter()
            .enumerate()
            .skip(usize::try_from(line_no).unwrap_or(usize::MAX))
        {
            old.push('\n');
            if let Some((before, new)) = later.split_once('=') {
                old.push_str(before);
                self.consumed = i + 1;
                // Nothing with a line break in it can name an account.
                let old = old.trim_end_matches(is_blank);
                if !old.is_empty() && !old.contains('\n') {
                    self.st.aliases.push(Alias::Plain {
                        old: old.to_string(),
                        new: trim_blank(new).to_string(),
                    });
                }
                return Ok(Some(DirectiveBody::Plain));
            }
            old.push_str(later);
        }
        self.consumed = self.lines.len();
        let last = u32::try_from(self.lines.len()).unwrap_or(u32::MAX);
        self.report(last, None, Problem::Invalid("An alias needs `=`."), None);
        Ok(Some(DirectiveBody::Plain))
    }

    /// `end ...`. What may follow depends on the word.
    fn end(&mut self, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let plain = Ok(Some(DirectiveBody::Plain));
        let name = rest.trim_start_matches(is_blank);
        let comment_or_nothing = |t: &str| {
            let t = t.trim_start_matches(is_blank);
            t.is_empty() || t.starts_with(';')
        };
        if let Some(after) = name.strip_prefix("aliases") {
            if !comment_or_nothing(after) {
                return invalid("Unexpected text after end aliases.");
            }
            self.st.aliases.clear();
            return plain;
        }
        if let Some(after) = name.strip_prefix("tag") {
            if !trim_blank(after).is_empty() {
                return invalid("Unexpected text after end tag.");
            }
            return plain;
        }
        if let Some(after) = name
            .strip_prefix("apply")
            .filter(|a| a.starts_with(is_blank))
        {
            let word = after.trim_start_matches(is_blank);
            if let Some(after) = word.strip_prefix("account") {
                if !comment_or_nothing(after) {
                    return invalid("Unexpected text after end apply account.");
                }
                if self.st.parents.pop().is_none() {
                    return invalid("end apply account without apply account.");
                }
                return plain;
            }
            // These three read nothing after their words, and want one blank after `end`.
            let literal = rest.starts_with(" apply year") || rest.starts_with(" apply fixed");
            if literal || (rest.starts_with(" apply") && word.starts_with("tag")) {
                return plain;
            }
        }
        invalid("Unknown end directive.")
    }

    fn year(&mut self, arg: &str) -> Parsed<Option<DirectiveBody>> {
        let arg = trim_blank(arg.split(';').next().unwrap_or(""));
        if arg.len() < 4 || !arg.chars().all(|c| c.is_ascii_digit()) {
            return invalid("Y takes a year of at least four digits.");
        }
        self.st.year = Some(arg.parse().or(invalid("Y takes a year."))?);
        Ok(Some(DirectiveBody::Plain))
    }

    /// `apply account NAME`, `apply year YEAR`, `apply fixed ...`, `apply tag ...`. Only
    /// `account` may have more than one blank before it.
    fn apply(&mut self, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let plain = Ok(Some(DirectiveBody::Plain));
        if let Some(name) = rest.trim_start_matches(is_blank).strip_prefix("account") {
            if !(name.is_empty() || name.starts_with(is_blank)) {
                return invalid("apply account needs a blank before the account.");
            }
            let name = trim_blank(name);
            if name.is_empty() {
                return invalid("apply account needs an account.");
            }
            if name.contains("  ") {
                return invalid("Unexpected text after the account.");
            }
            let full = self.full_account(name);
            self.st.parents.push(full);
            return plain;
        }
        let one_space = rest.strip_prefix(' ').filter(|r| !r.starts_with(is_blank));
        match one_space {
            Some(r) if r.starts_with("fixed") || r.starts_with("tag") => plain,
            Some(r) if r.starts_with("year") => self.year(&r[4..]),
            _ => invalid("This apply directive is not known."),
        }
    }

    /// A `P` line: kept for `value`; raw output does not show prices.
    fn price_line(&mut self, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let price = self.price_directive(rest)?;
        self.out.prices.push(price);
        Ok(Some(DirectiveBody::Plain))
    }

    /// `P DATE [TIME] COMMODITY AMOUNT`. The time of day is read and ignored.
    fn price_directive(&self, rest: &str) -> Parsed<Price> {
        let mut cur = Cursor::new(rest);
        cur.take_while(is_blank);
        let date = parse_date(&mut cur, self.st.year)?;
        if cur.take_while(is_blank).is_empty() {
            return invalid("A space was expected after the date.");
        }
        if cur.peek().is_some_and(|c| c.is_ascii_digit()) {
            time_of_day(&mut cur)?;
            if cur.take_while(is_blank).is_empty() {
                return invalid("A space was expected after the time.");
            }
        }
        let Some(symbol) = read_symbol(&mut cur).transpose()? else {
            return invalid("A commodity symbol was expected.");
        };
        if !cur.peek().is_some_and(is_blank) {
            return invalid("A space was expected after the commodity.");
        }
        cur.take_while(is_blank);
        let price = read_amount(&mut cur, &self.st.styles, Default::Apply)?;
        Ok(Price {
            date,
            from: symbol,
            to: price.commodity,
            rate: price.quantity,
        })
    }

    /// The sample amount of a `commodity` or `D` directive. It is read like any amount, so the
    /// marks already in force decide a lone `.`/`,`; it must end up with a decimal mark.
    fn sample_mark(&self, text: &str, trailing_ok: bool) -> Parsed<(String, Option<Mark>)> {
        let mut cur = Cursor::new(text);
        cur.take_while(is_blank);
        let amount = read_amount(&mut cur, &self.st.styles, Default::Ignore)?;
        if amount.mark.is_none() {
            return invalid("A commodity directive's number needs a decimal mark.");
        }
        let after = trim_blank(cur.rest());
        if !(trailing_ok || after.is_empty() || after.starts_with(';')) {
            return invalid("Unexpected text after the amount.");
        }
        Ok((amount.commodity, amount.mark))
    }

    fn default_commodity(&mut self, rest: &str) -> Parsed<()> {
        let text = rest.split(';').next().unwrap_or("");
        // Text after the amount is not read.
        let (commodity, mark) = self.sample_mark(text, true)?;
        self.st.styles.default_commodity = Some((commodity, mark));
        Ok(())
    }

    fn commodity(&mut self, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let text = rest.split(';').next().unwrap_or("");
        let arg = trim_blank(text);
        if arg.is_empty() {
            return invalid("commodity needs a symbol or an amount.");
        }
        // A lone `,` is a commodity symbol; a mark is part of a number only before a digit.
        let starts_number = arg.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+'))
            || (arg.starts_with(['.', ',']) && arg[1..].starts_with(|c: char| c.is_ascii_digit()));
        let quoted_then_number = {
            let mut c = Cursor::new(arg);
            matches!(read_symbol(&mut c), Some(Ok(_))) && {
                c.take_while(is_blank);
                !c.at_end()
            }
        };
        if starts_number || quoted_then_number {
            let (commodity, mark) = self.sample_mark(arg, false)?;
            if let Some(mark) = mark {
                self.st.styles.commodity_marks.insert(commodity, mark);
            }
            return Ok(Some(DirectiveBody::Plain));
        }
        let mut cur = Cursor::new(arg);
        let symbol = read_symbol(&mut cur)
            .transpose()?
            .ok_or(Problem::Invalid("commodity needs a symbol."))?;
        Ok(Some(DirectiveBody::Commodity(symbol)))
    }

    fn account(&mut self, rest: &str) -> Parsed<Option<DirectiveBody>> {
        let rest = rest.trim_start_matches(is_blank);
        if rest.is_empty() {
            return invalid("account needs a name.");
        }
        let (name, after) = match find_double_blank(rest) {
            Some(i) => (&rest[..i], trim_blank(&rest[i..])),
            None => (rest, ""),
        };
        // The brackets that make a posting virtual do not belong to a declared name.
        if name.starts_with(['(', '[']) {
            return invalid("A declared account cannot be virtual.");
        }
        let name = self.full_account(trim_blank(name));
        if !after.is_empty() {
            let Some(comment) = after.strip_prefix(';') else {
                return invalid("Unexpected text after the account name.");
            };
            self.account_comment(&name, comment)?;
        }
        Ok(Some(DirectiveBody::Account(name)))
    }

    /// `type:` tags on an account: the first one counts; its value must be a known type.
    fn account_comment(&mut self, account: &str, comment: &str) -> Parsed<()> {
        let Some((_, value)) = tags(comment).into_iter().find(|(n, _)| *n == "type") else {
            return Ok(());
        };
        let lower = value.to_lowercase();
        if !ACCOUNT_TYPES.contains(&lower.as_str()) {
            return invalid("This account type is not one the reference tool knows.");
        }
        if (lower == "v" || lower == "conversion")
            && !self.st.conversion_accounts.iter().any(|a| a == account)
        {
            self.st.conversion_accounts.push(account.to_string());
        }
        Ok(())
    }

    /// The account with `apply account` parents and aliases applied (aliases see the parents).
    fn full_account(&self, name: &str) -> String {
        let mut full = match self.st.parents.last() {
            Some(parent) => format!("{parent}:{name}"),
            None => name.to_string(),
        };
        for alias in self.st.aliases.iter().rev() {
            if let Ok(next) = alias.apply(&full) {
                full = next;
            }
        }
        full
    }

    fn indented(&mut self, state: State, line_no: u32, line: &str) -> State {
        let content = line.trim_start_matches(is_blank);
        if content.is_empty() {
            // Only blanks, but some are not spaces or tabs: the reference tool rejects it.
            self.report(
                line_no,
                None,
                Problem::Invalid("A line of only blanks."),
                None,
            );
            return Self::mark_refused(state);
        }
        if line.contains('\r') {
            self.report(
                line_no,
                None,
                Problem::Unsupported(Unsupported::ControlCharacter),
                None,
            );
            return Self::mark_refused(state);
        }
        match state {
            State::Skipping | State::CommentBlock => state,
            State::Top => {
                if !content.starts_with([';', '#', '*']) {
                    self.report(
                        line_no,
                        None,
                        Problem::Invalid("An indented line outside a transaction."),
                        None,
                    );
                }
                State::Top
            }
            State::InDirective(body) => self.directive_line(body, line_no, content),
            // A status mark with nothing after it is no posting: it ends the transaction or the
            // rule, and is a comment line (spec/hledger/posting-star-line.md).
            State::InRule(_) if is_lone_star(content) => State::Top,
            State::InTransaction(_) if is_lone_star(content) => {
                self.finish(state);
                State::Top
            }
            State::InRule(rule) => self.rule_line(rule, line_no, content),
            State::InTransaction(mut tx) => {
                if let Some(comment) = content.strip_prefix(';') {
                    match tx.postings.last_mut() {
                        Some(posting) => {
                            if let Err(p) =
                                check_posting_comment(comment, YearSource::Fixed(tx.date.year()))
                            {
                                self.report(line_no, None, p, None);
                                tx.refused = true;
                            }
                            posting.comment_lines.push(trim_blank(comment).to_string());
                        }
                        None => tx.comment_lines.push(trim_blank(comment).to_string()),
                    }
                } else {
                    self.posting(&mut tx, line_no, line, content);
                }
                State::InTransaction(tx)
            }
        }
    }

    fn directive_line(&mut self, body: DirectiveBody, line_no: u32, content: &str) -> State {
        if content.starts_with(['#', '*']) {
            return State::InDirective(body);
        }
        if let Some(comment) = content.strip_prefix(';') {
            if let DirectiveBody::Account(name) = &body {
                let name = name.clone();
                if let Err(p) = self.account_comment(&name, comment) {
                    self.report(line_no, None, p, None);
                }
            }
            return State::InDirective(body);
        }
        if let (DirectiveBody::Commodity(symbol), Some(format)) =
            (&body, content.strip_prefix("format"))
        {
            match self.sample_mark(format, false) {
                Ok((commodity, Some(mark))) if &commodity == symbol => {
                    self.st.styles.commodity_marks.insert(commodity, mark);
                }
                Ok(_) => self.report(
                    line_no,
                    None,
                    Problem::Invalid("The format's commodity differs from the directive's."),
                    None,
                ),
                Err(p) => self.report(line_no, None, p, None),
            }
            return State::InDirective(body);
        }
        match body {
            DirectiveBody::Account(_)
            | DirectiveBody::Commodity(_)
            | DirectiveBody::Python
            | DirectiveBody::Free => State::InDirective(body),
            DirectiveBody::Plain => {
                self.report(
                    line_no,
                    None,
                    Problem::Invalid("This directive takes no indented lines."),
                    None,
                );
                State::Skipping
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

    fn posting(&mut self, tx: &mut TxDraft, line_no: u32, line: &str, content: &str) {
        let parsed = parse_posting(
            content,
            &self.st.styles,
            YearSource::Fixed(tx.date.year()),
            Mode::Transaction,
        );
        let p = match parsed {
            Ok(p) => p,
            Err(problem) => {
                self.report(line_no, None, problem, Some(content));
                tx.refused = true;
                return;
            }
        };
        let assertion = p.assertion.map(|written| {
            let indent = line.get(..line.len() - content.len()).unwrap_or("");
            let before = content.get(..written.offset).unwrap_or("");
            Assertion {
                column: column_after(indent, before),
                written,
            }
        });
        if let Some(Err(problem)) = p
            .comment
            .as_deref()
            .map(|c| check_posting_comment(c, YearSource::Fixed(tx.date.year())))
        {
            self.report(line_no, None, problem, None);
            tx.refused = true;
        }
        let account = self.full_account(&p.account);
        tx.postings.push(PostingDraft {
            account,
            kind: p.kind,
            line: line_no,
            assertion,
            comment_lines: p
                .comment
                .map(|c| trim_blank(&c).to_string())
                .into_iter()
                .collect(),
            amount: p.amount,
        });
    }

    fn is_conversion_account(&self, account: &str) -> bool {
        if self.st.conversion_accounts.is_empty() {
            return conversion_name().is_match(account);
        }
        self.st.conversion_accounts.iter().any(|a| {
            account == a
                || account
                    .strip_prefix(a.as_str())
                    .is_some_and(|r| r.starts_with(':'))
        })
    }

    /// End of a transaction: keep it for the file's walk, unless a line of it was refused.
    fn finish(&mut self, state: State) {
        let State::InTransaction(tx) = state else {
            return;
        };
        if tx.refused {
            return;
        }
        let flags = tx
            .postings
            .iter()
            .map(|p| self.is_conversion_account(&p.account))
            .collect();
        self.conversion.push(flags);
        self.done.push(tx);
    }

    /// End of the file: balance its transactions and walk its postings (see `walk`), report what
    /// that finds, and keep the transactions. The findings come in hledger's order: what the
    /// lines gave, then the problems of plain transactions, then the walk's.
    fn conclude(&mut self) {
        let mut done = std::mem::take(&mut self.done);
        let conversion = std::mem::take(&mut self.conversion);
        // Which postings `--cost` drops is decided on the transaction as written.
        let plans: Vec<cost::Plan> = done
            .iter()
            .zip(&conversion)
            .map(|(tx, flags)| {
                let assignment = tx
                    .postings
                    .iter()
                    .any(|p| p.assertion.is_some() && p.amount.is_none());
                cost::plan(&drafts_of(tx, flags), assignment)
            })
            .collect();
        let mut txs: Vec<Tx> = done
            .iter_mut()
            .zip(&conversion)
            .map(|(tx, flags)| Tx {
                date: tx.date,
                line: tx.line,
                dropped: false,
                info: tx
                    .postings
                    .iter_mut()
                    .map(|p| PostingInfo {
                        account: p.account.clone(),
                        line: p.line,
                        date: posting_date(&p.comment_lines, tx.date.year()),
                        assertion: p.assertion.take(),
                    })
                    .collect(),
                drafts: drafts_of(tx, flags),
            })
            .collect();
        let issues = walk::process(&mut txs, self.reported == 0);
        for issue in &issues {
            let finding = self.issue_finding(issue);
            self.out.findings.push((self.file_index, finding));
        }
        for ((tx, walked), plan) in done.into_iter().zip(txs).zip(plans) {
            if walked.dropped {
                continue;
            }
            let exact = Exact {
                postings: tx
                    .postings
                    .iter()
                    .zip(&walked.drafts)
                    .zip(&plan.tagged)
                    .map(|((p, d), &tagged)| ExactPosting {
                        amounts: d.amounts.clone().unwrap_or_default(),
                        conversion_tagged: tagged
                            || p.comment_lines
                                .iter()
                                .any(|c| tags(c).iter().any(|(n, _)| *n == "_conversion-posting")),
                    })
                    .collect(),
                cost_refused: plan.refused,
            };
            let postings = tx
                .postings
                .into_iter()
                .zip(walked.drafts)
                .map(|(p, d)| Posting {
                    account: p.account,
                    kind: p.kind,
                    comment: join_comment(&p.comment_lines),
                    amounts: d
                        .amounts
                        .unwrap_or_default()
                        .iter()
                        .map(report_amount)
                        .collect(),
                    flag: None,
                    line: None,
                })
                .collect();
            self.out.transactions.push((
                Transaction {
                    date: tx.date.to_string(),
                    status: tx.status,
                    flag: None,
                    payee: None,
                    description: tx.description,
                    comment: join_comment(&tx.comment_lines),
                    source: Source {
                        file: self.file.name.clone(),
                        line: tx.line,
                    },
                    postings,
                },
                exact,
                tx.date,
            ));
        }
    }

    fn issue_finding(&self, issue: &Issue) -> Finding {
        let file = self.file.name.clone();
        match issue {
            Issue::Unbalanced { line, problem } => unbalanced_finding(&file, *line, problem),
            Issue::AssertionFailed {
                line,
                column,
                account,
                commodity,
                expected,
                actual,
                total,
                inclusive,
            } => Finding {
                code: "assertion-failed".into(),
                severity: Severity::Error,
                file,
                line: *line,
                column: Some(*column),
                details: json!({
                    "account": account,
                    "commodity": commodity,
                    "expected": expected.to_string(),
                    "actual": actual.to_string(),
                    "total": total,
                    "inclusive": inclusive,
                }),
                message: format!(
                    "The balance of {account} in {commodity} is {actual}, not {expected}."
                ),
            },
            Issue::AssignmentPostingDate { line, account } => Finding {
                code: "assignment-with-posting-date".into(),
                severity: Severity::Error,
                file,
                line: *line,
                column: None,
                details: json!({ "account": account }),
                message: "A balance assignment cannot have a posting date.".into(),
            },
        }
    }
}

/// The finding for a transaction that does not balance.
fn unbalanced_finding(file: &str, line: u32, u: &Unbalanced) -> Finding {
    let at = |code: &str, details: serde_json::Value, message: &str| Finding {
        code: code.into(),
        severity: Severity::Error,
        file: file.into(),
        line,
        column: None,
        details,
        message: message.into(),
    };
    match u {
        Unbalanced::UnsupportedConversion => finding(
            file,
            line,
            None,
            Problem::Unsupported(Unsupported::ConversionPostings),
            None,
        ),
        Unbalanced::ConversionWithCost => at(
            "unbalanced-transaction",
            json!({ "reason": "conversion-posting-with-cost" }),
            "Conversion postings must not have a price.",
        ),
        Unbalanced::TwoElided { group } => at(
            "unbalanced-transaction",
            json!({ "reason": "two-elided-amounts", "group": group.id() }),
            "At most one posting per group may leave out its amount.",
        ),
        Unbalanced::Residual { group, residual } => at(
            "unbalanced-transaction",
            json!({
                "group": group.id(),
                "residual": residual
                    .iter()
                    .map(|(c, q)| json!({ "commodity": c, "quantity": q }))
                    .collect::<Vec<_>>(),
            }),
            "The postings do not add up to zero.",
        ),
    }
}

/// The drafts balancing starts from: the postings as written.
fn drafts_of(tx: &TxDraft, conversion_flags: &[bool]) -> Vec<Draft> {
    tx.postings
        .iter()
        .zip(conversion_flags)
        .map(|(p, &conversion_account)| Draft {
            kind: p.kind,
            conversion_account,
            amounts: p.amount.clone().map(|a| vec![a]),
        })
        .collect()
}

/// An amount as the public model reports it: quantities rounded to ten places.
pub(crate) fn report_amount(a: &WrittenAmount) -> Amount {
    Amount {
        commodity: a.commodity.clone(),
        quantity: a.quantity.output(),
        cost: a.cost.as_ref().map(|c| {
            let rounded = c.value.rounded(crate::decimal::OUTPUT_PLACES);
            // An inferred unit price loses its trailing zeros when its exact value fits in ten
            // places; otherwise it keeps all ten (inferred-prices.md).
            let exact = c.value.normalized().places() <= crate::decimal::OUTPUT_PLACES;
            let value = if c.inferred_unit && exact {
                rounded.normalized()
            } else {
                rounded
            };
            let amount = Quantity {
                commodity: c.commodity.clone(),
                quantity: value.to_string(),
            };
            match c.kind {
                CostKind::Unit => Cost::Unit { amount },
                CostKind::Total => Cost::Total { amount },
            }
        }),
        lot: a.lot.as_ref().map(|l| Lot {
            cost: l.cost.as_ref().map(|c| to_quantity(&c.commodity, &c.value)),
            date: l.date.clone(),
            label: l.label.clone(),
        }),
    }
}

/// `HH:MM` or `HH:MM:SS`, 24-hour, two digits each.
fn time_of_day(cur: &mut Cursor) -> Parsed<()> {
    fn part(cur: &mut Cursor, max: u32) -> Parsed<()> {
        let digits = cur.take_while(|c| c.is_ascii_digit());
        match digits.parse::<u32>() {
            Ok(v) if digits.len() == 2 && v <= max => Ok(()),
            _ => invalid("This is not a valid time."),
        }
    }
    part(cur, 23)?;
    if !cur.eat(':') {
        return invalid("This is not a valid time.");
    }
    part(cur, 59)?;
    if cur.eat(':') {
        part(cur, 59)?;
    }
    Ok(())
}

/// The 1-based column after `indent` and `before`, as the reference tool counts: a tab moves to
/// the next multiple of eight.
fn column_after(indent: &str, before: &str) -> u32 {
    let mut column = 0u32;
    for c in indent.chars().chain(before.chars()) {
        column = if c == '\t' {
            column.saturating_add(8) / 8 * 8
        } else {
            column.saturating_add(1)
        };
    }
    column.saturating_add(1)
}

/// Byte offset of the first of two blanks in a row (blanks may be several bytes long).
fn find_double_blank(text: &str) -> Option<usize> {
    let mut previous: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (is_blank(c), previous) {
            (true, Some(start)) => return Some(start),
            (true, None) => previous = Some(i),
            (false, _) => previous = None,
        }
    }
    None
}
