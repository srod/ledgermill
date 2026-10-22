//! The checks the reference tool runs on the entries once they are booked: balance assertions,
//! and (in later steps of the same walk) padding and the account, currency and duplicate checks.
//!
//! The order matters: `check()` reports problems in the order `bean-check` prints them, so the
//! first one is the reference's first (spec/beancount/check-order.md).

use super::plain::{Day, Item};
use super::pydec::Num;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// One posting of a kept transaction: what an account gained, and the lot it went into.
pub(super) struct Posted {
    pub account: String,
    pub currency: String,
    pub units: Num,
}

/// A transaction the reference keeps (one that booking rejected is dropped before any check).
pub(super) struct Kept {
    pub date: Day,
    pub line: u32,
    pub postings: Vec<Posted>,
}

/// A problem found here, before it is given its file and severity.
pub(super) struct Problem {
    pub line: u32,
    pub code: &'static str,
    pub details: Value,
    pub message: &'static str,
}

enum Entry<'a> {
    Directive(&'a Item, Day, u32),
    Transaction(&'a Kept),
}

impl Entry<'_> {
    /// Where the entry stands in a walk: by date, then `open`, `balance`, everything else and
    /// last `close` and `document`, each group by line.
    fn key(&self) -> (Day, u8, u32) {
        match self {
            Entry::Directive(item, date, line) => {
                let group = match item {
                    Item::Open { .. } => 0,
                    Item::Balance { .. } => 1,
                    _ => 2,
                };
                (*date, group, *line)
            }
            Entry::Transaction(kept) => (kept.date, 2, kept.line),
        }
    }
}

/// Whether `account` is `root` or lies below it.
fn within(account: &str, root: &str) -> bool {
    account == root
        || account
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with(':'))
}

/// What every account holds, by currency, as the walk goes on.
#[derive(Default)]
struct Sums {
    units: BTreeMap<(String, String), Num>,
}

impl Sums {
    fn apply(&mut self, postings: &[Posted]) {
        for p in postings {
            let sum = self
                .units
                .entry((p.account.clone(), p.currency.clone()))
                .or_insert_with(Num::zero);
            *sum = sum.add(&p.units);
        }
    }

    /// The units of `currency` held by `root` and everything below it.
    fn total(&self, root: &str, currency: &str) -> Num {
        self.units
            .iter()
            .filter(|((account, c), _)| c == currency && within(account, root))
            .fold(Num::zero(), |sum, (_, units)| sum.add(units))
    }
}

/// How far an account may be from an assertion: the places written in the number, or the
/// explicit `~` amount (spec/beancount/balance-directive.md).
fn tolerance(expected: &Num, written: Option<&Num>) -> Num {
    match written {
        Some(t) => t.clone(),
        None if expected.places() == 0 => Num::zero(),
        None => Num::new(1.into(), -i64::from(expected.places())),
    }
}

fn within_tolerance(difference: &Num, tolerance: &Num) -> bool {
    difference.abs().cmp_value(tolerance) != Ordering::Greater
}

/// Run the checks that come after booking, in the order the reference prints them.
pub(super) fn run(items: &[Item], kept: &[Kept]) -> Vec<Problem> {
    let mut entries: Vec<Entry> = items
        .iter()
        .filter_map(|item| {
            item.place()
                .map(|(date, line)| Entry::Directive(item, date, line))
        })
        .chain(kept.iter().map(Entry::Transaction))
        .collect();
    // Stable: entries of one group on one line keep their order.
    entries.sort_by_key(Entry::key);

    let mut problems = Vec::new();
    balance_assertions(&entries, &mut problems);
    problems
}

/// Step 5: every `balance`, walking the entries so that it sees what came before its day.
fn balance_assertions(entries: &[Entry], problems: &mut Vec<Problem>) {
    let mut sums = Sums::default();
    for entry in entries {
        match entry {
            Entry::Transaction(kept) => sums.apply(&kept.postings),
            Entry::Directive(
                Item::Balance {
                    line,
                    account,
                    expected,
                    tolerance: written,
                    currency,
                    ..
                },
                ..,
            ) => {
                let accumulated = sums.total(account, currency);
                let difference = accumulated.add(&expected.negated());
                if !within_tolerance(&difference, &tolerance(expected, written.as_ref())) {
                    problems.push(Problem {
                        line: *line,
                        code: "assertion-failed",
                        details: json!({
                            "account": account,
                            "currency": currency,
                            "expected": expected.to_string(),
                            "accumulated": accumulated.to_string(),
                            "difference": difference.to_string(),
                        }),
                        message: "The account does not hold what the assertion says.",
                    });
                }
            }
            Entry::Directive(..) => {}
        }
    }
}
