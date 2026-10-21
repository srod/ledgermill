//! ledgermill core: loads ledger sources from memory and answers through typed values.
//!
//! Portable by design: no file access, no JS types. `load` never fails; problems become findings.

mod alias;
mod amount;
mod balance;
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

pub use model::*;

/// A loaded ledger. Freed by drop.
#[derive(Debug, Default)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    findings: Vec<Finding>,
}

impl Ledger {
    /// Transactions in the order the reference tool prints them: by date, then source order.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// Problems found while loading, ordered by file (as given) then line.
    pub fn check(&self) -> &[Finding] {
        &self.findings
    }
}

/// A set of files ledgermill cannot read together yet: no transaction, one finding.
fn refuse_set(construct: &str, at: &SourceFile) -> Ledger {
    Ledger {
        transactions: Vec::new(),
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
    // Both sorts are stable: same date keeps file order, then line order.
    out.transactions.sort_by_key(|(_, date)| *date);
    out.findings
        .sort_by_key(|(file, finding)| (*file, finding.line));
    Ledger {
        transactions: out.transactions.into_iter().map(|(t, _)| t).collect(),
        findings: out.findings.into_iter().map(|(_, f)| f).collect(),
    }
}
