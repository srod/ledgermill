//! What hledger does with one file's transactions once they are parsed: balance the plain ones,
//! then walk every posting in date order with running balances, checking assertions and working
//! out assignments. See `spec/hledger/assertion-order.md`, `assignment-processing.md` and
//! `assertion-failures.md`.
//!
//! The issues come back in hledger's own order, so that the first one is the first error it
//! prints: the problems of plain transactions, in file order, then the walk's.

use crate::balance::{Draft, Unbalanced, balance};
use crate::balances::Balances;
use crate::date::Date;
use crate::decimal::Decimal;
use crate::posting::WrittenAssertion;

/// What the walk needs to know about a posting besides its amount (which is in the draft).
pub(crate) struct PostingInfo {
    pub account: String,
    pub line: u32,
    /// The posting's own date, from its comment.
    pub date: Option<Date>,
    pub assertion: Option<Assertion>,
}

pub(crate) struct Assertion {
    pub written: WrittenAssertion,
    /// Where the first `=` is in its line, counting a tab as moving to a multiple of eight.
    pub column: u32,
}

/// One transaction of a file, as the walk sees it. `drafts` is parallel to `info`.
pub(crate) struct Tx {
    pub date: Date,
    pub line: u32,
    pub info: Vec<PostingInfo>,
    pub drafts: Vec<Draft>,
    /// Left out of the result: its balancing is something ledgermill does not do yet.
    pub dropped: bool,
}

impl Tx {
    /// A posting with an assertion and no amount works out its own amount.
    fn is_assignment(&self, posting: usize) -> bool {
        self.info
            .get(posting)
            .is_some_and(|i| i.assertion.is_some())
            && self
                .drafts
                .get(posting)
                .is_some_and(|d| d.amounts.is_none())
    }

    fn has_assignment(&self) -> bool {
        (0..self.info.len()).any(|i| self.is_assignment(i))
    }
}

pub(crate) enum Issue {
    /// A transaction does not balance.
    Unbalanced { line: u32, problem: Unbalanced },
    /// An assertion does not hold.
    AssertionFailed {
        line: u32,
        column: u32,
        account: String,
        commodity: String,
        expected: Decimal,
        actual: Decimal,
        total: bool,
        inclusive: bool,
    },
    /// An assignment posting carries a date of its own.
    AssignmentPostingDate { line: u32, account: String },
}

/// Where in the walk something happens: a plain posting at its own date, or a whole
/// transaction with an assignment at the date of the transaction. Ties go to the one written
/// first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Step {
    date: Date,
    tx: usize,
    /// `None`: the whole transaction.
    posting: Option<usize>,
}

fn steps(txs: &[Tx]) -> Vec<Step> {
    let mut steps = Vec::new();
    for (tx_index, tx) in txs.iter().enumerate().filter(|(_, t)| !t.dropped) {
        if tx.has_assignment() {
            steps.push(Step {
                date: tx.date,
                tx: tx_index,
                posting: None,
            });
            continue;
        }
        for (i, info) in tx.info.iter().enumerate() {
            steps.push(Step {
                date: info.date.unwrap_or(tx.date),
                tx: tx_index,
                posting: Some(i),
            });
        }
    }
    steps.sort();
    steps
}

/// Check the assertion of a posting against the balances now.
fn check(balances: &Balances, info: &PostingInfo, issues: &mut Vec<Issue>) {
    let Some(assertion) = &info.assertion else {
        return;
    };
    let w = &assertion.written;
    if let Some(m) = balances.check(&info.account, w.total, w.inclusive, &w.amount) {
        issues.push(Issue::AssertionFailed {
            line: info.line,
            column: assertion.column,
            account: info.account.clone(),
            commodity: m.commodity,
            expected: m.expected,
            actual: m.actual,
            total: w.total,
            inclusive: w.inclusive,
        });
    }
}

/// Balance every transaction and walk the file. The balancing fills in the elided amounts and
/// the assignments of `txs`. `walk_is_known` is false when part of the file was not read, so the
/// balances cannot be trusted and the walk reports nothing.
pub(crate) fn process(txs: &mut [Tx], walk_is_known: bool) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut known = walk_is_known;
    for tx in txs.iter_mut().filter(|t| !t.has_assignment()) {
        if let Err(problem) = balance(&mut tx.drafts) {
            if matches!(problem, Unbalanced::UnsupportedConversion) {
                tx.dropped = true;
                known = false;
            }
            issues.push(Issue::Unbalanced {
                line: tx.line,
                problem,
            });
        }
    }
    let mut walked = Vec::new();
    let mut balances = Balances::default();
    for step in steps(txs) {
        let Some(tx) = txs.get_mut(step.tx) else {
            continue;
        };
        match step.posting {
            Some(i) => {
                if let (Some(info), Some(draft)) = (tx.info.get(i), tx.drafts.get(i)) {
                    for amount in draft.amounts.iter().flatten() {
                        balances.add(&info.account, amount, false);
                    }
                    check(&balances, info, &mut walked);
                }
            }
            None => {
                if !walk_transaction(tx, &mut balances, &mut walked) {
                    known = false;
                }
            }
        }
    }
    if known {
        issues.extend(walked);
    }
    issues
}

/// A transaction with an assignment, taken as a block (assignment-processing.md). Returns false
/// when it had to be left out.
fn walk_transaction(tx: &mut Tx, balances: &mut Balances, issues: &mut Vec<Issue>) -> bool {
    let elided: Vec<bool> = (0..tx.info.len())
        .map(|i| tx.drafts.get(i).is_some_and(|d| d.amounts.is_none()) && !tx.is_assignment(i))
        .collect();
    // A posting dated after the transaction waits for the end of the block: the postings before
    // it do not see it, but it is in the balance when the block is over.
    let later: Vec<bool> = (0..tx.info.len())
        .map(|i| {
            !tx.is_assignment(i)
                && tx
                    .info
                    .get(i)
                    .and_then(|p| p.date)
                    .is_some_and(|d| d > tx.date)
        })
        .collect();
    for i in 0..tx.info.len() {
        if later.get(i).copied().unwrap_or(false) {
            continue;
        }
        if tx.is_assignment(i) {
            let (Some(info), Some(draft)) = (tx.info.get(i), tx.drafts.get_mut(i)) else {
                continue;
            };
            let Some(assertion) = &info.assertion else {
                continue;
            };
            if info.date.is_some() {
                issues.push(Issue::AssignmentPostingDate {
                    line: info.line,
                    account: info.account.clone(),
                });
            }
            let w = &assertion.written;
            draft.amounts = Some(balances.assign(&info.account, w.total, w.inclusive, &w.amount));
        } else if let (Some(info), Some(draft)) = (tx.info.get(i), tx.drafts.get(i)) {
            if draft.amounts.is_none() {
                continue;
            }
            for amount in draft.amounts.iter().flatten() {
                balances.add(&info.account, amount, true);
            }
            check(balances, info, issues);
        }
    }
    for i in (0..later.len()).filter(|&i| later[i]) {
        if let (Some(info), Some(draft)) = (tx.info.get(i), tx.drafts.get(i)) {
            for amount in draft.amounts.iter().flatten() {
                balances.add(&info.account, amount, true);
            }
            check(balances, info, issues);
        }
    }
    let outcome = balance(&mut tx.drafts);
    let mut kept = true;
    if let Err(problem) = outcome {
        if matches!(problem, Unbalanced::UnsupportedConversion) {
            tx.dropped = true;
            kept = false;
        }
        issues.push(Issue::Unbalanced {
            line: tx.line,
            problem,
        });
    }
    // The elided amounts are known only now, and join the balances only now.
    for (i, was_elided) in elided.into_iter().enumerate() {
        if let (true, Some(info), Some(draft)) = (was_elided, tx.info.get(i), tx.drafts.get(i)) {
            for amount in draft.amounts.iter().flatten() {
                balances.add(&info.account, amount, true);
            }
        }
    }
    kept
}
