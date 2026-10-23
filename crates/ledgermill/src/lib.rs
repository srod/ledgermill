//! ledgermill core: loads ledger sources from memory and answers through typed values.
//!
//! Portable by design: no file access, no JS types. `load` never fails; problems become findings.

mod alias;
mod amount;
mod balance;
mod balances;
mod beancount;
mod chars;
mod cost;
mod cursor;
mod date;
mod decimal;
mod exact;
mod flat;
mod holdings;
mod model;
mod number;
mod parse;
mod period;
mod posting;
mod prices;
mod problem;
mod tags;
mod valuation;
mod walk;

use exact::Exact;
pub use model::*;

/// A loaded ledger. Freed by drop.
///
/// Besides the public lists it keeps what `at_cost`, `value` and `holdings` need (hledger only):
/// every `P` line and the exact amounts and dates of each transaction, so that those questions read
/// nothing again.
#[derive(Debug, Default)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    postings: Vec<FlatPosting>,
    findings: Vec<Finding>,
    /// Parallel to `transactions` (hledger): the exact amounts, dates and cost tags, for `at_cost`,
    /// `value` and `holdings`.
    exact: Vec<Exact>,
    /// Every `P` line, in parse order (hledger).
    prices: Vec<prices::Price>,
    /// The same lines as `prices()` gives them: sorted by date.
    price_list: Vec<PriceDirective>,
    beancount: bool,
}

impl Ledger {
    /// Transactions in the order the reference tool prints them: by date, then source order.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// One row per posting and commodity, with the header fields of its transaction.
    ///
    /// Beancount: booked postings, by date, then file and line. hledger: by date, then the order
    /// the files were read; the postings of a transaction as written; the commodities of a
    /// posting in code point order (spec/hledger/postings-flat.md). A transaction with a line that
    /// was refused, or that was dropped, gives no rows, as in `transactions()`.
    pub fn postings(&self) -> &[FlatPosting] {
        &self.postings
    }

    /// The `P` directives of the files, as `hledger prices` lists them (hledger only).
    ///
    /// Sorted by date; directives with the same date stay in the order they were read, which is
    /// the files as given, each from top to bottom (spec/hledger/prices-list.md). Prices are not
    /// taken from the transactions. A directive whose rate is followed by a price, a lot or a label
    /// is refused with an `unsupported-syntax` finding (`price-annotation`) and not listed.
    ///
    /// Beancount: the `price` directives, sorted by date, the same date in the order written (a stable
    /// sort, as the `prices` table lists them: spec/beancount/price-table.md). Nothing is dropped:
    /// a directive written twice is listed twice. `rate.quantity` is the number as `number(amount)`
    /// prints it, so a rate below 0.000001 has an exponent (`1E-7`). Empty when the file has an
    /// `unsupported-syntax` finding: a Beancount file with a refused construct lists nothing.
    pub fn prices(&self) -> &[PriceDirective] {
        &self.price_list
    }

    /// Problems found while loading, in the order the reference tool meets them, which is not line
    /// order: the first error is therefore the reference's first error.
    ///
    /// hledger: file by file, as given (hledger stops at the first file that has an error). In a
    /// file: what the lines gave (`parse-error`, `unsupported-syntax`), in line order; then the
    /// problems of the transactions without a balance assignment (`unbalanced-transaction`), in
    /// file order; then the walk over the postings by date, ties by position in the file
    /// (spec/hledger/assertion-failures.md): the `assertion-failed` findings of plain postings
    /// and, for a transaction with an assignment, taken as a block at its date, its
    /// `assignment-with-posting-date`, its `assertion-failed` and then its own
    /// `unbalanced-transaction`. A file with a line that was not read is not walked, as its
    /// balances would be wrong. A `=` or `~` rule is read and left out, with an `ignored-syntax`
    /// warning on its header line (`automated-rule`, `periodic-rule`), in line order with the
    /// errors of that file; it never changes the verdict (spec/hledger/rules-ignored.md).
    ///
    /// Beancount: in the order `bean-check` prints them, which is not line order: by step (parsing,
    /// booking, pads, balance assertions, then the validations), and within a step by date, with a
    /// `balance` before the rest of its day (spec/beancount/check-order.md). A warning
    /// (`ignored-syntax`) comes before the errors.
    pub fn check(&self) -> &[Finding] {
        &self.findings
    }
}

/// A set of files ledgermill cannot read together yet: no transaction, one finding.
fn refuse_set(construct: &str, at: &SourceFile) -> Ledger {
    Ledger {
        findings: vec![Finding {
            code: "unsupported-syntax".into(),
            severity: Severity::Error,
            file: at.name.clone(),
            line: 1,
            column: None,
            details: serde_json::json!({ "construct": construct }),
            message: if construct == "mixed-formats" {
                "hledger and Beancount files cannot be loaded together yet.".into()
            } else {
                "Several Beancount files cannot be loaded together yet.".into()
            },
        }],
        ..Ledger::default()
    }
}

/// Parse the given files, in order.
///
/// `.beancount` and `.bean` files are Beancount, everything else is hledger. One Beancount file
/// is read on its own; several of them, or a mix of both formats, are refused.
pub fn load(files: &[SourceFile]) -> Ledger {
    let beancount: Vec<&SourceFile> = files
        .iter()
        .filter(|f| beancount::is_beancount(&f.name))
        .collect();
    if let Some(first) = beancount.first() {
        if beancount.len() < files.len() {
            return refuse_set("mixed-formats", first);
        }
        if let Some(second) = beancount.get(1) {
            return refuse_set("several-beancount-files", second);
        }
        let loaded = beancount::load(first);
        return Ledger {
            postings: beancount::flatten(&loaded.transactions),
            transactions: loaded.transactions,
            findings: loaded.findings,
            price_list: loaded.prices,
            beancount: true,
            ..Ledger::default()
        };
    }
    let mut out = parse::Output {
        transactions: Vec::new(),
        prices: Vec::new(),
        findings: Vec::new(),
    };
    for (index, file) in files.iter().enumerate() {
        parse::parse_file(index, file, &mut out);
    }
    // The sort is stable: same date keeps file order, then line order. Findings stay in the order
    // they were made in, which is hledger's: file by file; in each file what the lines gave, the
    // problems of plain transactions, then the walk's.
    out.transactions.sort_by_key(|(_, _, _, date)| *date);
    let mut transactions = Vec::new();
    let mut exact = Vec::new();
    let mut postings = Vec::new();
    for (transaction, kept, rows, _) in out.transactions {
        transactions.push(transaction);
        exact.push(kept);
        postings.extend(rows);
    }
    let mut listed: Vec<&prices::Price> = out.prices.iter().collect();
    listed.sort_by_key(|p| p.date);
    let price_list = listed
        .into_iter()
        .map(|p| PriceDirective {
            date: p.date.to_string(),
            commodity: p.from.clone(),
            rate: Quantity {
                commodity: p.to.clone(),
                quantity: p.listed.to_string(),
            },
            file: p.file.clone(),
            line: p.line,
        })
        .collect();
    Ledger {
        transactions,
        postings,
        exact,
        prices: out.prices,
        price_list,
        findings: out.findings.into_iter().map(|(_, f)| f).collect(),
        ..Ledger::default()
    }
}
