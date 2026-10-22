//! ledgermill core: loads ledger sources from memory and answers through typed values.
//!
//! Portable by design: no file access, no JS types. `load` never fails; problems become findings.

mod alias;
mod amount;
mod balance;
mod balances;
mod beancount;
mod chars;
mod cursor;
mod date;
mod decimal;
mod model;
mod number;
mod parse;
mod posting;
mod problem;
mod tags;
mod walk;

pub use model::*;

/// A loaded ledger. Freed by drop.
#[derive(Debug, Default)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    postings: Vec<FlatPosting>,
    findings: Vec<Finding>,
}

impl Ledger {
    /// Transactions in the order the reference tool prints them: by date, then source order.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// One row per booked posting, by date, then file and line (Beancount only).
    ///
    /// Empty for hledger: its flat postings are not built yet (planned separately).
    pub fn postings(&self) -> &[FlatPosting] {
        &self.postings
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
    /// balances would be wrong.
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
        transactions: Vec::new(),
        postings: Vec::new(),
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
        };
    }
    let mut out = parse::Output {
        transactions: Vec::new(),
        findings: Vec::new(),
    };
    for (index, file) in files.iter().enumerate() {
        parse::parse_file(index, file, &mut out);
    }
    // The sort is stable: same date keeps file order, then line order. Findings stay in the order
    // they were made in, which is hledger's: file by file; in each file what the lines gave, the
    // problems of plain transactions, then the walk's.
    out.transactions.sort_by_key(|(_, date)| *date);
    Ledger {
        transactions: out.transactions.into_iter().map(|(t, _)| t).collect(),
        postings: Vec::new(),
        findings: out.findings.into_iter().map(|(_, f)| f).collect(),
    }
}
