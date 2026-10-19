//! ledgermill core: loads ledger sources from memory and answers through typed values.
//!
//! Portable by design: no file access, no JS types. `load` never fails; problems become findings.

mod amount;
mod model;
mod number;
mod parse;

pub use model::*;

/// A loaded ledger. Freed by drop.
#[derive(Debug, Default)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    findings: Vec<Finding>,
}

impl Ledger {
    /// Transactions in the order the reference tool prints them: by date, then source order.
    pub fn transactions(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    /// Problems found while loading, ordered by file (as given) then line.
    pub fn check(&self) -> Vec<Finding> {
        self.findings.clone()
    }
}

/// Parse the given files, in order.
pub fn load(files: &[SourceFile]) -> Ledger {
    let mut out = parse::Output {
        transactions: Vec::new(),
        findings: Vec::new(),
    };
    for (index, file) in files.iter().enumerate() {
        parse::parse_file(index, file, &mut out);
    }
    // Both sorts are stable: same date keeps file order, then line order.
    out.transactions.sort_by(|a, b| a.date.cmp(&b.date));
    out.findings
        .sort_by_key(|(file, finding)| (*file, finding.line));
    Ledger {
        transactions: out.transactions,
        findings: out.findings.into_iter().map(|(_, f)| f).collect(),
    }
}
