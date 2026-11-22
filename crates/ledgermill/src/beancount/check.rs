//! The checks the reference tool runs on the entries once they are booked: padding, balance
//! assertions, and the account, currency and duplicate checks.
//!
//! The order matters: `check()` reports problems in the order `bean-check` prints them, so the
//! first one is the reference's first (spec/beancount/check-order.md).
//!
//! The entries of several files are one list: the walk sorts by date, group and line, and only
//! then by the position of the file, as the reference does (spec/beancount/several-files.md).

use super::plain::{Day, Item, Lot};
use super::pydec::Num;
use foldhash::{HashMap, HashSet};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::Bound;

/// Where an entry stands: the position of its file in the set, and its line.
#[derive(Clone, Copy)]
pub(super) struct At {
    pub file: usize,
    pub line: u32,
}

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
    pub at: At,
    pub postings: Vec<Posted>,
}

/// A problem found here, before it is given its severity.
pub(super) struct Problem {
    pub at: At,
    pub code: &'static str,
    pub details: Value,
    pub message: &'static str,
}

/// The transaction a `pad` adds for one `balance` (spec/beancount/pad.md).
pub(super) struct Padding {
    /// The pad's date and place: where the transaction stands.
    pub date: Day,
    pub at: At,
    /// The balance's place: where its postings stand, which may be in another file.
    pub balance_at: At,
    pub account: String,
    pub source: String,
    pub currency: String,
    pub expected: Num,
    /// What `account` gains and `source` loses.
    pub difference: Num,
}

impl Padding {
    /// The two postings, in order: the padded account gains, the source loses. The one place that
    /// says so; the transaction we report and the one the later checks walk are both built from it.
    pub(super) fn legs(&self) -> [(&str, Num); 2] {
        [
            (&self.account, self.difference.clone()),
            (&self.source, self.difference.negated()),
        ]
    }

    fn kept(&self) -> Kept {
        Kept {
            date: self.date,
            at: self.at,
            postings: self
                .legs()
                .into_iter()
                .map(|(account, units)| Posted {
                    account: account.to_string(),
                    currency: self.currency.clone(),
                    units,
                    lot: None,
                })
                .collect(),
        }
    }
}

pub(super) struct Checked {
    pub paddings: Vec<Padding>,
    /// The `document` directives: the reference looks for their files, we do not.
    pub documents: Vec<At>,
    pub problems: Vec<Problem>,
}

/// A directive, or a transaction with its rank among the padding transactions of one pad (0 for
/// a transaction that was written).
enum Entry<'a> {
    Directive(&'a Item, Day, At),
    Transaction(&'a Kept, usize),
}

impl Entry<'_> {
    /// Where the entry stands in a walk: by date, then `open`, `balance`, everything else and
    /// last `close` and `document`, each group by line, a line shared by two files in file order; a
    /// padding transaction right after its pad.
    fn key(&self) -> (Day, u8, u32, usize, usize) {
        match self {
            Entry::Directive(item, date, at) => {
                let group = match item {
                    Item::Open { .. } => 0,
                    Item::Balance { .. } => 1,
                    Item::Close { .. } | Item::Document { .. } => 3,
                    _ => 2,
                };
                (*date, group, at.line, at.file, 0)
            }
            Entry::Transaction(kept, rank) => (kept.date, 2, kept.at.line, kept.at.file, *rank),
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

/// Key of a lot held inside an account: currency, lot cost currency, date, label, canonical cost.
type LotKey = (String, String, Day, Option<String>, (String, i64));

/// What every account holds, by currency, as the walk goes on.
struct Sums {
    /// By account, then currency: ordered like (account, currency) pairs, and a posting of a pair
    /// already seen looks it up without copying its names.
    units: BTreeMap<String, BTreeMap<String, Num>>,
    /// Units held in lots with a cost, keyed by account then lot key (populated only when
    /// `track_lots` is true, i.e. during `pad_step`).
    lots: BTreeMap<String, HashMap<LotKey, Num>>,
    track_lots: bool,
}

impl Sums {
    fn new(track_lots: bool) -> Sums {
        Sums {
            units: BTreeMap::new(),
            lots: BTreeMap::new(),
            track_lots,
        }
    }

    fn apply(&mut self, postings: &[Posted]) {
        for p in postings {
            if !self.units.contains_key(&p.account) {
                self.units.insert(p.account.clone(), BTreeMap::new());
            }
            let Some(currencies) = self.units.get_mut(&p.account) else {
                continue;
            };
            if !currencies.contains_key(&p.currency) {
                currencies.insert(p.currency.clone(), Num::zero());
            }
            let Some(sum) = currencies.get_mut(&p.currency) else {
                continue;
            };
            *sum = sum.add(&p.units);
            if !self.track_lots {
                continue;
            }
            let Some(lot) = &p.lot else { continue };
            let key: LotKey = (
                p.currency.clone(),
                lot.currency.clone(),
                lot.date,
                lot.label.clone(),
                lot.per_unit.canonical(),
            );
            let account_lots = self.lots.entry(p.account.clone()).or_default();
            match account_lots.get_mut(&key) {
                Some(units) => {
                    *units = units.add(&p.units);
                    if units.is_zero() {
                        account_lots.remove(&key);
                    }
                }
                None if !p.units.is_zero() => {
                    account_lots.insert(key, p.units.clone());
                }
                None => {}
            }
        }
    }

    /// The units of `currency` held by `root` and everything below it.
    fn total(&self, root: &str, currency: &str) -> Num {
        let mut sum = self
            .units
            .get(root)
            .and_then(|held| held.get(currency))
            .cloned()
            .unwrap_or_else(Num::zero);
        let prefix = format!("{root}:");
        for (account, held) in self
            .units
            .range::<str, _>((Bound::Included(prefix.as_str()), Bound::Unbounded))
            .take_while(|(account, _)| account.starts_with(&prefix))
        {
            debug_assert!(within(account, root));
            if let Some(units) = held.get(currency) {
                sum = sum.add(units);
            }
        }
        sum
    }

    /// Whether `root` or anything below it holds `currency` in a lot with a cost.
    fn holds_cost(&self, root: &str, currency: &str) -> bool {
        let has_in = |lots: &HashMap<LotKey, Num>| {
            lots.iter()
                .any(|((c, ..), units)| c == currency && !units.is_zero())
        };
        if self.lots.get(root).is_some_and(has_in) {
            return true;
        }
        let prefix = format!("{root}:");
        self.lots
            .range::<str, _>((Bound::Included(prefix.as_str()), Bound::Unbounded))
            .take_while(|(account, _)| account.starts_with(&prefix))
            .any(|(_, lots)| has_in(lots))
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

/// Run the checks that come after booking, in the order the reference prints them. `added` are
/// the directives a plugin made; they are checked like the ones written.
pub(super) fn run(files: &[&[Item]], added: &[Item], kept: &[Kept]) -> Checked {
    let mut entries: Vec<Entry> = files
        .iter()
        .enumerate()
        .flat_map(|(file, items)| items.iter().map(move |item| (file, item)))
        // The reference gives the entries a plugin makes no place of their own.
        .chain(added.iter().map(|item| (0, item)))
        .filter_map(|(file, item)| {
            item.place()
                .map(|(date, line)| Entry::Directive(item, date, At { file, line }))
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
    let accounts = Accounts::of(&entries);
    balance_assertions(&entries, &accounts, &mut problems);
    open_and_close(&entries, &mut problems);
    accounts_used(&entries, &accounts, &mut problems);
    currencies_allowed(&entries, &accounts, &mut problems);
    duplicate_assertions(&entries, &mut problems);
    duplicate_commodities(&entries, &mut problems);
    let documents = entries
        .iter()
        .filter_map(|e| match e {
            Entry::Directive(Item::Document { .. }, _, at) => Some(*at),
            _ => None,
        })
        .collect();
    Checked {
        paddings,
        documents,
        problems,
    }
}

/// What the first `open` and the first `close` of each account say about when it is active.
struct Accounts<'a> {
    opens: HashMap<&'a str, (Day, &'a [String])>,
    closes: HashMap<&'a str, Day>,
}

/// Why an account cannot be used on a date.
enum Inactive {
    NeverOpened,
    NotYetOpen,
    Closed,
}

impl Inactive {
    fn name(&self) -> &'static str {
        match self {
            Inactive::NeverOpened => "never-opened",
            Inactive::NotYetOpen => "not-yet-open",
            Inactive::Closed => "closed",
        }
    }
}

impl<'a> Accounts<'a> {
    fn of(entries: &[Entry<'a>]) -> Accounts<'a> {
        let mut accounts = Accounts {
            opens: HashMap::default(),
            closes: HashMap::default(),
        };
        for entry in entries {
            match entry {
                Entry::Directive(
                    Item::Open {
                        account,
                        currencies,
                        ..
                    },
                    date,
                    _,
                ) => {
                    accounts.opens.entry(account).or_insert((*date, currencies));
                }
                Entry::Directive(Item::Close { account, .. }, date, _) => {
                    accounts.closes.entry(account).or_insert(*date);
                }
                _ => {}
            }
        }
        accounts
    }

    /// Whether `account` may be named on `date`; only a transaction or a pad also stops at the close.
    fn inactive(&self, account: &str, date: Day, until_close: bool) -> Option<Inactive> {
        let Some((opened, _)) = self.opens.get(account) else {
            return Some(Inactive::NeverOpened);
        };
        if date < *opened {
            return Some(Inactive::NotYetOpen);
        }
        (until_close
            && self
                .closes
                .get(account)
                .is_some_and(|closed| date > *closed))
        .then_some(Inactive::Closed)
    }

    /// The currencies `account` allows; empty means any.
    fn currencies(&self, account: &str) -> &'a [String] {
        self.opens.get(account).map_or(&[], |(_, c)| c)
    }
}

fn not_open(account: &str, why: &Inactive, at: At) -> Problem {
    Problem {
        at,
        code: "account-not-open",
        details: json!({ "account": account, "reason": why.name() }),
        message: "This account is not open on this date.",
    }
}

fn currency_not_allowed(account: &str, currency: &str, directive: &str, at: At) -> Problem {
    Problem {
        at,
        code: "currency-not-allowed",
        details: json!({ "account": account, "currency": currency, "directive": directive }),
        message: "The account does not allow this currency.",
    }
}

/// A `pad` waiting for assertions.
struct PadState<'a> {
    account: &'a str,
    source: &'a str,
    date: Day,
    at: At,
    /// The currencies it has seen an assertion for, whether or not it padded.
    handled: HashSet<&'a str>,
    made: bool,
}

/// Step 4: add the padding transactions, then report the pads that did nothing.
fn pad_step(entries: &[Entry], problems: &mut Vec<Problem>) -> Vec<Padding> {
    let mut sums = Sums::new(true);
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
                at,
            ) => {
                active.retain(|&i| pads.get(i).is_none_or(|p| p.account != account));
                active.push(pads.len());
                pads.push(PadState {
                    account,
                    source,
                    date: *date,
                    at: *at,
                    handled: HashSet::default(),
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
                balance_at,
            ) => {
                let tolerance = tolerance(expected, written.as_ref());
                let mut made = Vec::new();
                for &i in &active {
                    let Some(pad) = pads.get_mut(i) else { continue };
                    if !within(account, pad.account) || !pad.handled.insert(currency.as_str()) {
                        continue;
                    }
                    let actual = sums.total(pad.account, currency);
                    let difference = expected.add(&actual.negated());
                    if within_tolerance(&difference, &tolerance) {
                        continue;
                    }
                    if sums.holds_cost(pad.account, currency) {
                        problems.push(Problem {
                            at: *balance_at,
                            code: "pad-with-cost",
                            details: json!({ "account": pad.account, "currency": currency }),
                            message: "The account holds lots with a cost, which a pad does not respect.",
                        });
                    }
                    pad.made = true;
                    made.push(Padding {
                        date: pad.date,
                        at: pad.at,
                        balance_at: *balance_at,
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
        at: p.at,
        code: "unused-pad",
        details: json!({ "account": p.account, "source": p.source }),
        message: "This pad never added a transaction.",
    }));
    paddings
}

/// Step 5: every `balance`, walking the entries so that it sees what came before its day.
fn balance_assertions(entries: &[Entry], accounts: &Accounts, problems: &mut Vec<Problem>) {
    let mut sums = Sums::new(false);
    for entry in entries {
        match entry {
            Entry::Transaction(kept, _) => sums.apply(&kept.postings),
            Entry::Directive(
                Item::Balance {
                    account,
                    expected,
                    tolerance: written,
                    currency,
                    ..
                },
                date,
                at,
            ) => {
                if let Some(why @ (Inactive::NeverOpened | Inactive::NotYetOpen)) =
                    accounts.inactive(account, *date, false)
                {
                    problems.push(not_open(account, &why, *at));
                }
                let allowed = accounts.currencies(account);
                if !allowed.is_empty() && !allowed.contains(currency) {
                    problems.push(currency_not_allowed(account, currency, "balance", *at));
                }
                let accumulated = sums.total(account, currency);
                let difference = accumulated.add(&expected.negated());
                if !within_tolerance(&difference, &tolerance(expected, written.as_ref())) {
                    problems.push(Problem {
                        at: *at,
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

/// Step 6.1: `open` and `close` directives, walking the entries.
fn open_and_close(entries: &[Entry], problems: &mut Vec<Problem>) {
    let mut opened: HashSet<&str> = HashSet::default();
    let mut closed: HashSet<&str> = HashSet::default();
    for entry in entries {
        match entry {
            Entry::Directive(Item::Open { account, .. }, _, at) => {
                if !opened.insert(account) {
                    problems.push(Problem {
                        at: *at,
                        code: "duplicate-open",
                        details: json!({ "account": account }),
                        message: "This account was opened before.",
                    });
                }
            }
            Entry::Directive(Item::Close { account, .. }, _, at) => {
                if !opened.contains(account.as_str()) {
                    problems.push(Problem {
                        at: *at,
                        code: "close-without-open",
                        details: json!({ "account": account }),
                        message: "This account is closed without having been opened.",
                    });
                } else if !closed.insert(account) {
                    problems.push(Problem {
                        at: *at,
                        code: "duplicate-close",
                        details: json!({ "account": account }),
                        message: "This account was closed before.",
                    });
                }
            }
            _ => {}
        }
    }
}

/// Step 6.2: the accounts that transactions, pads, notes and documents name. A `balance` was
/// looked at in step 5, and a padding transaction names what its pad names.
fn accounts_used<'a>(entries: &[Entry<'a>], accounts: &Accounts, problems: &mut Vec<Problem>) {
    // One buffer for every call: the accounts of the entry that were already looked at.
    let mut seen: Vec<&str> = Vec::new();
    let mut check = |names: &mut dyn Iterator<Item = &'a str>, date: Day, at: At, until_close| {
        seen.clear();
        for name in names {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
            if let Some(why) = accounts.inactive(name, date, until_close) {
                problems.push(not_open(name, &why, at));
            }
        }
    };
    for entry in entries {
        match entry {
            Entry::Transaction(kept, 0) => check(
                &mut kept.postings.iter().map(|p| p.account.as_str()),
                kept.date,
                kept.at,
                true,
            ),
            Entry::Directive(
                Item::Pad {
                    account, source, ..
                },
                date,
                at,
            ) => check(
                &mut [account.as_str(), source.as_str()].into_iter(),
                *date,
                *at,
                true,
            ),
            Entry::Directive(
                Item::Note { account, .. } | Item::Document { account, .. },
                date,
                at,
            ) => check(&mut std::iter::once(account.as_str()), *date, *at, false),
            _ => {}
        }
    }
}

/// Step 6.3: postings in a currency their account does not allow.
fn currencies_allowed(entries: &[Entry], accounts: &Accounts, problems: &mut Vec<Problem>) {
    for entry in entries {
        let Entry::Transaction(kept, _) = entry else {
            continue;
        };
        for p in &kept.postings {
            let allowed = accounts.currencies(&p.account);
            if !allowed.is_empty() && !allowed.contains(&p.currency) {
                problems.push(currency_not_allowed(
                    &p.account,
                    &p.currency,
                    "transaction",
                    kept.at,
                ));
            }
        }
    }
}

/// Step 6.4: assertions on one account, currency and day that differ from the first of them.
fn duplicate_assertions(entries: &[Entry], problems: &mut Vec<Problem>) {
    let mut first: HashMap<(&str, &str, Day), &Num> = HashMap::default();
    for entry in entries {
        let Entry::Directive(
            Item::Balance {
                account,
                currency,
                expected,
                ..
            },
            date,
            at,
        ) = entry
        else {
            continue;
        };
        let earlier = *first
            .entry((account.as_str(), currency.as_str(), *date))
            .or_insert(expected);
        if earlier.cmp_value(expected) != Ordering::Equal {
            problems.push(Problem {
                at: *at,
                code: "duplicate-assertion",
                details: json!({
                    "account": account,
                    "currency": currency,
                    "expected": expected.to_string(),
                    "first": earlier.to_string(),
                }),
                message: "Another assertion of the same day expects a different amount.",
            });
        }
    }
}

/// Step 6.5: a `commodity` declared again.
fn duplicate_commodities(entries: &[Entry], problems: &mut Vec<Problem>) {
    let mut seen: HashSet<&str> = HashSet::default();
    for entry in entries {
        if let Entry::Directive(Item::Commodity { currency, .. }, _, at) = entry
            && !seen.insert(currency)
        {
            problems.push(Problem {
                at: *at,
                code: "duplicate-commodity",
                details: json!({ "currency": currency }),
                message: "This commodity was declared before.",
            });
        }
    }
}
