//! The typed values `Ledger` answers with. Their JSON form is the contract the comparison
//! harness diffs against.

use serde::{Deserialize, Serialize};

/// One source file handed to `load`. Files are processed in the order given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    #[serde(rename = "")]
    Unmarked,
    #[serde(rename = "!")]
    Pending,
    #[serde(rename = "*")]
    Cleared,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub file: String,
    pub line: u32,
}

/// `(account)` postings are virtual and never balanced; `[account]` postings are balanced among
/// themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostingKind {
    Regular,
    Virtual,
    BalancedVirtual,
}

/// A commodity and an exact quantity: a decimal string that keeps its decimal places, e.g.
/// `"-12.50"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity {
    pub commodity: String,
    pub quantity: String,
}

/// The price written (or inferred) for an amount: `@` gives a unit price, `@@` a total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Cost {
    Unit { amount: Quantity },
    Total { amount: Quantity },
}

/// A lot annotation: `{cost}` (a total `{{T}}` is reported per unit), `[date]` and `(label)`.
/// `cost` is `None` for `{}` or when only a date or label is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    pub cost: Option<Quantity>,
    pub date: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub commodity: String,
    pub quantity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lot: Option<Lot>,
}

/// One posting.
///
/// `flag` and `line` are only set for Beancount, whose postings carry their own flag and are
/// reported at their own source line; hledger gives neither. A Beancount posting has exactly one
/// amount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub account: String,
    pub kind: PostingKind,
    pub comment: String,
    pub amounts: Vec<Amount>,
    /// The posting's own flag (Beancount), e.g. `"!"`. Absent when the posting has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    /// The line of the posting itself (Beancount). Several postings share a line when one posting
    /// was booked against several lots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// One transaction, from either format.
///
/// A single type serves both formats on purpose: a caller reading transactions treats hledger and
/// Beancount results the same way, and the shared fields mean the same thing in both. What only one
/// format has is optional and left out of the JSON when absent (`payee`, `flag`, and the posting's
/// `flag` and `line`), so the hledger JSON is unchanged. A tagged variant per format would make
/// every caller branch before reading a date or an account.
///
/// For Beancount, `description` is the narration, `comment` is empty, and `status` follows the
/// flag: `*` is cleared, `!` is pending, any other flag is unmarked (read the exact one in `flag`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// `YYYY-MM-DD`
    pub date: String,
    pub status: Status,
    /// The exact transaction flag, one character (Beancount only); `txn` reads as `*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    /// The payee when the header has two strings (Beancount only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
    pub description: String,
    pub comment: String,
    pub source: Source,
    pub postings: Vec<Posting>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub file: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    pub details: serde_json::Value,
    /// Short English sentence, our own words.
    pub message: String,
}
