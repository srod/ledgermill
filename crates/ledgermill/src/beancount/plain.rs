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
        account: String,
        method: Option<Method>,
    },
    Transaction(RawTransaction),
    /// A directive that changes what transactions come out and is not built yet.
    Refused {
        line: u32,
        construct: &'static str,
    },
}

/// The result of reading one file.
#[derive(Debug)]
pub(super) struct Read {
    pub items: Vec<Item>,
    /// The `booking_method` option; STRICT when absent.
    pub default_method: Method,
}

/// A line the parser rejects.
#[derive(Debug, Clone)]
pub(super) struct ParseIssue {
    pub line: u32,
    pub reason: String,
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
    /// The fork's own message, in kebab case.
    Booking(&'static str),
    TooPrecise,
    /// An elided posting with several currencies to balance: it stands for one posting each.
    SeveralCurrencies(Vec<String>),
    /// A shape the reference accepts and this does not build; the construct's name.
    Unsupported(&'static str),
    /// An account books by AVERAGE, which the booking code does not implement.
    Average(String),
}
