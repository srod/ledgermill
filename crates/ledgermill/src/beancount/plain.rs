//! What the rest of the Beancount code works with: plain values, never the types of the parser
//! and booking crates (those stay inside `lima.rs`).

use super::pydec::Num as Decimal;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct Day {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// How an account matches a reduction against its lots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    Strict,
    StrictWithSize,
    None,
    Fifo,
    Lifo,
    Hifo,
    Average,
}

/// A cost as written: `{per-unit}`, `{{total}}` or `{per-unit # total}`, plus selectors.
#[derive(Debug, Clone, Default)]
pub(super) struct RawCost {
    pub per_unit: Option<Decimal>,
    pub total: Option<Decimal>,
    pub currency: Option<String>,
    pub date: Option<Day>,
    pub label: Option<String>,
    /// `{*}` asks to merge lots.
    pub merge: bool,
}

/// A price as written: `@ number currency` (per unit) or `@@ number currency` (total). A part
/// left out is worked out from the other postings (inferred-cost-and-price.md).
#[derive(Debug, Clone)]
pub(super) struct RawPrice {
    pub number: Option<Decimal>,
    pub total: bool,
    pub currency: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct RawPosting {
    pub account: String,
    pub flag: Option<char>,
    pub line: u32,
    pub units: Option<Decimal>,
    pub currency: Option<String>,
    pub cost: Option<RawCost>,
    pub price: Option<RawPrice>,
}

#[derive(Debug, Clone)]
pub(super) struct RawTransaction {
    pub date: Day,
    pub line: u32,
    pub flag: char,
    pub payee: Option<String>,
    pub narration: Option<String>,
    pub postings: Vec<RawPosting>,
}

/// A directive that matters to transactions, in the order the reference tool processes them.
#[derive(Debug, Clone)]
pub(super) enum Item {
    Open {
        date: Day,
        line: u32,
        account: String,
        /// The currencies the account allows; empty means any.
        currencies: Vec<String>,
        method: Option<Method>,
    },
    Close {
        date: Day,
        line: u32,
        account: String,
    },
    Commodity {
        date: Day,
        line: u32,
        currency: String,
    },
    Pad {
        date: Day,
        line: u32,
        account: String,
        source: String,
    },
    /// `balance`: the tolerance is the one written after `~`, if any.
    Balance {
        date: Day,
        line: u32,
        account: String,
        expected: Decimal,
        tolerance: Option<Decimal>,
        currency: String,
    },
    /// A `price`: one unit of `currency` was worth `number` of `quote` on `date`.
    Price {
        date: Day,
        line: u32,
        currency: String,
        number: Decimal,
        quote: String,
    },
    /// A `note`; only the account it names matters.
    Note {
        date: Day,
        line: u32,
        account: String,
    },
    /// A `document`; ledgermill cannot look at files, so only the account matters.
    Document {
        date: Day,
        line: u32,
        account: String,
    },
    Transaction(RawTransaction),
    /// A directive that changes what transactions come out and is not built yet.
    Refused {
        line: u32,
        construct: &'static str,
    },
}

impl Item {
    /// The date and line of a directive that is not a transaction.
    pub(super) fn place(&self) -> Option<(Day, u32)> {
        match self {
            Item::Open { date, line, .. }
            | Item::Close { date, line, .. }
            | Item::Commodity { date, line, .. }
            | Item::Price { date, line, .. }
            | Item::Pad { date, line, .. }
            | Item::Balance { date, line, .. }
            | Item::Note { date, line, .. }
            | Item::Document { date, line, .. } => Some((*date, *line)),
            Item::Transaction(_) | Item::Refused { .. } => None,
        }
    }
}

/// The result of reading one file.
#[derive(Debug)]
pub(super) struct Read {
    pub items: Vec<Item>,
    /// The `booking_method` option; STRICT when absent.
    pub default_method: Method,
}

/// What the parser objected to.
#[derive(Debug, Clone)]
pub(super) enum IssueKind {
    /// A token or syntax error; `reason` is the parser's message.
    Syntax,
    /// An account whose first component is not one of the five roots.
    InvalidAccount(String),
    /// A `poptag` or `popmeta` of something not pushed.
    PopWithoutPush { metadata: bool, name: String },
    /// A `pushtag` or `pushmeta` still open at the end of the file.
    PushNotPopped { metadata: bool, name: String },
    /// A key given twice under one entry or one posting.
    DuplicateMetadata { key: String, posting: bool },
}

/// A line the parser rejects.
#[derive(Debug, Clone)]
pub(super) struct ParseIssue {
    /// The line the reference reports it on: line 0 for a push left open, the entry's header for
    /// a duplicate key.
    pub line: u32,
    pub reason: String,
    pub kind: IssueKind,
}

/// A booked posting: one row of the postings table. A posting reduced across several lots gives
/// several rows.
#[derive(Debug, Clone)]
pub(super) struct Row {
    /// Index of the posting as written.
    pub origin: usize,
    pub units: Decimal,
    pub currency: String,
    pub lot: Option<Lot>,
}

#[derive(Debug, Clone)]
pub(super) struct Lot {
    pub per_unit: Decimal,
    pub currency: String,
    pub date: Day,
    pub label: Option<String>,
}

/// Why booking a transaction failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BookFailure {
    /// The fork's own message, in kebab case, reported at the transaction's line.
    Booking(&'static str),
    /// The same, reported at the line of the posting with this index (as written): the reference
    /// blames the posting for what it cannot categorize or work out.
    OnPosting(&'static str, usize),
    TooPrecise,
    /// An elided posting with several currencies to balance: it stands for one posting each.
    SeveralCurrencies(Vec<String>),
    /// A shape the reference accepts and this does not build; the construct's name.
    Unsupported(&'static str),
    /// An account books by AVERAGE, which the booking code does not implement.
    Average(String),
}

impl BookFailure {
    /// The same failure, blamed on the posting with this index when it is a plain one.
    pub(super) fn on(self, posting: usize) -> BookFailure {
        match self {
            BookFailure::Booking(reason) => BookFailure::OnPosting(reason, posting),
            other => other,
        }
    }
}
