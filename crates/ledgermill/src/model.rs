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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostingKind {
    Regular,
}

/// `quantity` is an exact decimal string that keeps the decimal places as written, e.g. `"-12.50"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    pub commodity: String,
    pub quantity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub account: String,
    pub kind: PostingKind,
    pub comment: String,
    pub amounts: Vec<Amount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// `YYYY-MM-DD`
    pub date: String,
    pub status: Status,
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
