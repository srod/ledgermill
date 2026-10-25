//! The typed values `Ledger` answers with. Their JSON form is the contract the comparison
//! harness diffs against.

use serde::{Deserialize, Serialize};

/// One source file handed to `load`. Files are processed in the order given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    /// The file name. It is reported back in every `file` field, and its extension decides the
    /// format: `.beancount` and `.bean` are Beancount, anything else is hledger.
    pub name: String,
    /// The whole text of the file.
    pub content: String,
}

/// A transaction's status. In JSON it is `""`, `"!"` or `"*"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// No mark. Beancount: any flag other than `*` and `!`, including the `P` of padding.
    #[serde(rename = "")]
    Unmarked,
    /// Marked `!`.
    #[serde(rename = "!")]
    Pending,
    /// Marked `*`; a Beancount `txn` counts as cleared.
    #[serde(rename = "*")]
    Cleared,
}

/// Where a transaction starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The file, as given to `load`.
    pub file: String,
    /// The line of the transaction's header, from 1. A Beancount padding transaction sits on the
    /// line of its `pad`.
    pub line: u32,
}

/// `(account)` postings are virtual and never balanced; `[account]` postings are balanced among
/// themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostingKind {
    /// An ordinary posting; always the case for Beancount.
    Regular,
    /// `(account)`: never balanced.
    Virtual,
    /// `[account]`: balanced among the other `[account]` postings of its transaction.
    BalancedVirtual,
}

/// A commodity and an exact quantity: a decimal string that keeps its decimal places, e.g.
/// `"-12.50"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity {
    /// The commodity; `""` when the amount has none.
    pub commodity: String,
    /// The exact decimal string, with a point as decimal mark.
    pub quantity: String,
}

/// One `P` directive of a journal: on `date`, one unit of `commodity` was worth `rate`.
///
/// What `Ledger::prices` lists. `rate.quantity` is the exact number the line gave, with the
/// places it was written with (`0.80` stays `0.80`, an exponent is expanded), read under the number
/// marks in force at that line. A rate with no commodity of its own is listed with at least the
/// places of the `D` sample, as `hledger prices` prints it.
///
/// Beancount: a `price` directive; `rate.quantity` is `number(amount)` as the reference prints it
/// (places kept, no digit-group commas, an exponent for a number below 0.000001: `1E-7`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceDirective {
    /// `YYYY-MM-DD`; a time of day on the line is dropped.
    pub date: String,
    /// The commodity being priced (its name without quotes).
    pub commodity: String,
    /// What one unit of `commodity` was worth: the commodity of the rate and an exact decimal
    /// string.
    pub rate: Quantity,
    /// The file the line is in, as given to `load`.
    pub file: String,
    /// The line of the `P` directive, from 1.
    pub line: u32,
}

/// What `Ledger::holdings` is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldingsQuery {
    /// The commodity `value` and `cost_value` are given in (`V`); not empty.
    pub commodity: String,
    /// `YYYY-MM-DD`, exclusive like `-e`: only postings dated before it count. hledger values at the day
    /// before; Beancount at the latest prices there are. `None`: every posting counts.
    #[serde(default)]
    pub end: Option<String>,
    /// `YYYY-MM-DD`, today's date. The core has no clock; it decides which `P` lines may move the
    /// valuation date when there is no `end` (hledger). Beancount does not use it.
    pub today: String,
}

/// One account's position in one commodity, as `Ledger::holdings` gives it.
///
/// `cost`, `value` and `cost_value` list one quantity per commodity, in code point order, without
/// the commodities whose sum is exactly zero. JSON field names are as written here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holding {
    /// The account, as written (a virtual posting is never counted).
    pub account: String,
    /// The commodity `C` held; `""` for amounts without one.
    pub commodity: String,
    /// The sum of the quantities of `commodity`, costs and lots ignored: exact, rounded half to even
    /// to ten places when it has more (`"0.0000000001"`). May be `"0"`: see `Ledger::holdings`.
    /// Beancount: the plain decimal digits of the sum, no exponent and no rounding to ten places.
    pub units: String,
    /// The same postings at their cost (`hledger --cost`): the costs' commodities, and `commodity`
    /// itself for the amounts without a cost.
    pub cost: Vec<Quantity>,
    /// The units valued at the end date in the query's commodity (`--value=end,V`).
    pub value: Vec<Quantity>,
    /// The cost, valued (`--cost --value=end,V`).
    pub cost_value: Vec<Quantity>,
}

/// The price written (or inferred) for an amount: `@` gives a unit price, `@@` a total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Cost {
    /// `@`: the price of one unit.
    Unit {
        /// The price of one unit.
        amount: Quantity,
    },
    /// `@@`: the price of the whole amount (hledger only).
    Total {
        /// The price of the whole amount.
        amount: Quantity,
    },
}

/// A lot annotation: `{cost}` (a total `{{T}}` is reported per unit), `[date]` and `(label)`.
/// `cost` is `None` for `{}` or when only a date or label is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    /// The cost of one unit of the lot.
    pub cost: Option<Quantity>,
    /// The lot's date, `YYYY-MM-DD`.
    pub date: Option<String>,
    /// The lot's label.
    pub label: Option<String>,
}

/// An amount of one commodity on a posting, with the price and lot it was written or booked with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amount {
    /// The commodity; `""` when the amount has none.
    pub commodity: String,
    /// The exact decimal string, with a point as decimal mark and the places as written or worked
    /// out. hledger: rounded half to even to ten places when it has more.
    pub quantity: String,
    /// The `@` or `@@` price, written or inferred. Beancount: the `@` price, as a unit price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
    /// The lot annotation. Beancount: the cost the posting was booked against (`{}`), with its
    /// date and label.
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
    /// The account name, after aliases; for a virtual posting without its `()` or `[]` (see `kind`).
    pub account: String,
    /// Whether the posting is real or virtual.
    pub kind: PostingKind,
    /// The posting's comment, trimmed; empty when it has none, and always empty for Beancount.
    pub comment: String,
    /// The amounts of the posting. hledger: one per commodity, an elided posting getting the
    /// amounts that balance its transaction (the list can be empty). Beancount: exactly one.
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
    /// The transaction's status.
    pub status: Status,
    /// The exact transaction flag, one character (Beancount only); `txn` reads as `*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    /// The payee when the header has two strings (Beancount only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
    /// The description of the header; for Beancount the narration.
    pub description: String,
    /// The comment of the header and of the comment lines under it, trimmed; empty when there is none.
    pub comment: String,
    /// The file and line the transaction starts at.
    pub source: Source,
    /// The postings, in the order written (Beancount: grouped by currency, as the reference lists
    /// them).
    pub postings: Vec<Posting>,
}

/// One row of the flat posting list: a booked posting with the header fields of its transaction.
///
/// Beancount: a posting booked against several lots gives one row per lot, all with the same
/// `line`, in the order the lots were consumed. Rows are ordered by date, then by file and line
/// (spec note `spec/beancount/postings.md`).
///
/// hledger: one row per posting and commodity (an elided posting gives one per commodity it was
/// given). Rows are ordered by the transaction's date, ties in the order the files were read; the
/// postings of a transaction in the order written; the commodities of a posting in code point
/// order, the empty one first (spec note `spec/hledger/postings-flat.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlatPosting {
    /// The transaction's date, `YYYY-MM-DD`.
    pub date: String,
    /// The transaction's flag, not the posting's own. Beancount: one character (`txn` reads as
    /// `*`). hledger: the status, `""` (unmarked), `"*"` or `"!"`.
    pub flag: String,
    /// The payee when the transaction header has two strings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee: Option<String>,
    /// The narration; empty when the header has none.
    pub narration: String,
    /// The posting's account; for a virtual posting without its `()` or `[]` (see `kind`).
    pub account: String,
    /// The kind of posting: always `Regular` for Beancount.
    pub kind: PostingKind,
    /// The units as an exact decimal string with a point as decimal mark, places as written or
    /// worked out, e.g. `"-2.50"`; hledger: never rounded, and a zero is `"0"`.
    pub number: String,
    /// The units' commodity.
    pub currency: String,
    /// The file the transaction was read from, as given to `load`.
    pub file: String,
    /// The line of the posting itself, from 1. hledger gives no line for a posting: this is the line
    /// it is written on, shared by its rows.
    pub line: u32,
}

/// How serious a finding is. In JSON it is `"error"` or `"warning"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The reference tool rejects this input, or ledgermill cannot read it.
    Error,
    /// Read and set aside; it does not change the verdict.
    Warning,
}

/// A problem found in a ledger, or something ledgermill read and set aside: what `Ledger::check`
/// lists. The `code` is stable; `spec/README.md` lists the codes and their `details`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// The stable code, e.g. `parse-error` or `unsupported-syntax`. Match on this, not on `message`.
    pub code: String,
    /// Whether it makes the ledger invalid.
    pub severity: Severity,
    /// The file, as given to `load`.
    pub file: String,
    /// The line it is reported on, from 1; `0` for a problem of the whole file (a Beancount push
    /// that is never popped).
    pub line: u32,
    /// The column, from 1 (a tab counts to the next multiple of eight), when the finding points
    /// inside the line; for example a failed hledger assertion, at its first `=`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Facts about this finding as JSON; which ones depends on the `code` (the account, the
    /// commodity, the construct refused...).
    pub details: serde_json::Value,
    /// Short English sentence, our own words.
    pub message: String,
}
