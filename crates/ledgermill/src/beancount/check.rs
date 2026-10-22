//! The checks the reference tool runs on the entries once they are booked: padding, balance
//! assertions, and the account, currency and duplicate checks.
//!
//! The order matters: `check()` reports problems in the order `bean-check` prints them, so the
//! first one is the reference's first (spec/beancount/check-order.md).

use super::plain::{Day, Item, Lot};
use super::pydec::Num;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// One posting of a kept transaction: what an account gained, and the lot it went into.
pub(super) struct Posted {
    pub account: String,
    pub currency: String,
    pub units: Num,
    pub lot: Option<Lot>,
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

/// The transaction a `pad` adds for one `balance` (spec/beancount/pad.md).
pub(super) struct Padding {
    /// The pad's date and line: where the transaction stands.
    pub date: Day,
    pub line: u32,
    /// The balance's line: where its postings stand.
    pub balance_line: u32,
    pub account: String,
    pub source: String,
    pub currency: String,
    pub expected: Num,
    /// What `account` gains and `source` loses.
    pub difference: Num,
}

impl Padding {
    fn kept(&self) -> Kept {
        let posted = |account: &str, units: Num| Posted {
            account: account.to_string(),
            currency: self.currency.clone(),
            units,
            lot: None,
        };
        Kept {
            date: self.date,
            line: self.line,
            postings: vec![
                posted(&self.account, self.difference.clone()),
                posted(&self.source, self.difference.negated()),
            ],
        }
    }
}

pub(super) struct Checked {
    pub paddings: Vec<Padding>,
    pub problems: Vec<Problem>,
}

/// A directive, or a transaction with its rank among the padding transactions of one pad (0 for
/// a transaction that was written).
enum Entry<'a> {
    Directive(&'a Item, Day, u32),
    Transaction(&'a Kept, usize),
}

impl Entry<'_> {
    /// Where the entry stands in a walk: by date, then `open`, `balance`, everything else and
    /// last `close` and `document`, each group by line; a padding transaction right after its pad.
    fn key(&self) -> (Day, u8, u32, usize) {
        match self {
            Entry::Directive(item, date, line) => {
                let group = match item {
                    Item::Open { .. } => 0,
                    Item::Balance { .. } => 1,
                    _ => 2,
                };
                (*date, group, *line, 0)
            }
            Entry::Transaction(kept, rank) => (kept.date, 2, kept.line, *rank),
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

fn same_lot(a: &Lot, b: &Lot) -> bool {
    a.currency == b.currency
        && a.date == b.date
        && a.label == b.label
        && a.per_unit.cmp_value(&b.per_unit) == Ordering::Equal
}

/// What every account holds, by currency, as the walk goes on.
#[derive(Default)]
struct Sums {
    units: BTreeMap<(String, String), Num>,
    /// Units held in lots with a cost: account, currency, lot.
    lots: Vec<(String, String, Lot, Num)>,
}

impl Sums {
    fn apply(&mut self, postings: &[Posted]) {
        for p in postings {
            let sum = self
                .units
                .entry((p.account.clone(), p.currency.clone()))
                .or_insert_with(Num::zero);
            *sum = sum.add(&p.units);
            let Some(lot) = &p.lot else { continue };
            let held = self.lots.iter_mut().find(|(account, currency, other, _)| {
                *account == p.account && *currency == p.currency && same_lot(other, lot)
            });
            match held {
                Some((.., units)) => *units = units.add(&p.units),
                None => self.lots.push((
                    p.account.clone(),
                    p.currency.clone(),
                    lot.clone(),
                    p.units.clone(),
                )),
            }
        }
    }

    /// The units of `currency` held by `root` and everything below it.
    fn total(&self, root: &str, currency: &str) -> Num {
        self.units
            .iter()
            .filter(|((account, c), _)| c == currency && within(account, root))
            .fold(Num::zero(), |sum, (_, units)| sum.add(units))
    }

    /// Whether `root` or anything below it holds `currency` in a lot with a cost.
    fn holds_cost(&self, root: &str, currency: &str) -> bool {
        self.lots.iter().any(|(account, c, _, units)| {
            c == currency && within(account, root) && !units.is_zero()
        })
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
pub(super) fn run(items: &[Item], kept: &[Kept]) -> Checked {
    let mut entries: Vec<Entry> = items
        .iter()
        .filter_map(|item| {
            item.place()
                .map(|(date, line)| Entry::Directive(item, date, line))
        })
        .chain(kept.iter().map(|k| Entry::Transaction(k, 0)))
        .collect();
    // Stable: entries of one group on one line keep their order.
    entries.sort_by_key(Entry::key);

    let mut problems = Vec::new();
    let paddings = pad_step(&entries, &mut problems);

    // From here on the padding transactions are entries like the others.
    let padding_kept: Vec<Kept> = paddings.iter().map(Padding::kept).collect();
    entries.extend(
        padding_kept
            .iter()
            .enumerate()
            .map(|(rank, k)| Entry::Transaction(k, rank + 1)),
    );
    entries.sort_by_key(Entry::key);
    balance_assertions(&entries, &mut problems);
    Checked { paddings, problems }
}

/// A `pad` waiting for assertions.
struct PadState<'a> {
    account: &'a str,
    source: &'a str,
    date: Day,
    line: u32,
    /// The currencies it has seen an assertion for, whether or not it padded.
    handled: Vec<&'a str>,
    made: bool,
}

/// Step 4: add the padding transactions, then report the pads that did nothing.
fn pad_step(entries: &[Entry], problems: &mut Vec<Problem>) -> Vec<Padding> {
    let mut sums = Sums::default();
    let mut pads: Vec<PadState> = Vec::new();
    // Indexes into `pads`, the pad of each account that is still waiting.
    let mut active: Vec<usize> = Vec::new();
    let mut paddings = Vec::new();
    for entry in entries {
        match entry {
            Entry::Transaction(kept, _) => sums.apply(&kept.postings),
            Entry::Directive(
                Item::Pad {
                    account, source, ..
                },
                date,
                line,
            ) => {
                active.retain(|&i| pads.get(i).is_none_or(|p| p.account != account));
                active.push(pads.len());
                pads.push(PadState {
                    account,
                    source,
                    date: *date,
                    line: *line,
                    handled: Vec::new(),
                    made: false,
                });
            }
            Entry::Directive(
                Item::Balance {
                    account,
                    expected,
                    tolerance: written,
                    currency,
                    ..
                },
                _,
                balance_line,
            ) => {
                let tolerance = tolerance(expected, written.as_ref());
                let mut made = Vec::new();
                for &i in &active {
                    let Some(pad) = pads.get_mut(i) else { continue };
                    if !within(account, pad.account) || pad.handled.contains(&currency.as_str()) {
                        continue;
                    }
                    pad.handled.push(currency);
                    let actual = sums.total(pad.account, currency);
                    let difference = expected.add(&actual.negated());
                    if within_tolerance(&difference, &tolerance) {
                        continue;
                    }
                    if sums.holds_cost(pad.account, currency) {
                        problems.push(Problem {
                            line: *balance_line,
                            code: "pad-with-cost",
                            details: json!({ "account": pad.account, "currency": currency }),
                            message: "The account holds lots with a cost, which a pad does not respect.",
                        });
                    }
                    pad.made = true;
                    made.push(Padding {
                        date: pad.date,
                        line: pad.line,
                        balance_line: *balance_line,
                        account: pad.account.to_string(),
                        source: pad.source.to_string(),
                        currency: currency.clone(),
                        expected: expected.clone(),
                        difference,
                    });
                }
                // Every pad of this assertion looked at the accounts as they were before it.
                for padding in &made {
                    sums.apply(&padding.kept().postings);
                }
                paddings.extend(made);
            }
            Entry::Directive(..) => {}
        }
    }
    problems.extend(pads.iter().filter(|p| !p.made).map(|p| Problem {
        line: p.line,
        code: "unused-pad",
        details: json!({ "account": p.account, "source": p.source }),
        message: "This pad never added a transaction.",
    }));
    paddings
}

/// Step 5: every `balance`, walking the entries so that it sees what came before its day.
fn balance_assertions(entries: &[Entry], problems: &mut Vec<Problem>) {
    let mut sums = Sums::default();
    for entry in entries {
        match entry {
            Entry::Transaction(kept, _) => sums.apply(&kept.postings),
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
