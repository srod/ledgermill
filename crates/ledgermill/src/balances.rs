//! Running balances of accounts, what a balance assertion compares them with and the amounts a
//! balance assignment gets. See `spec/hledger/assertions.md`, `assignments.md` and
//! `assignment-lots-and-costs.md`.
//!
//! A balance is kept per account and commodity as a list of entries. A posting adds to the entry
//! without a price (its own price and lot never count: two postings in one commodity are one
//! balance), so that list holds one entry, unless an assignment with a price in its asserted
//! amount put priced entries there.

use crate::decimal::{Decimal, ZeroTie};
use crate::posting::{CostKind, WrittenAmount, WrittenCost, WrittenLot};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

/// The price of a balance entry; a total price is its total, signed like the quantity.
#[derive(Debug, Clone)]
struct Price {
    kind: CostKind,
    commodity: String,
    value: Decimal,
}

impl Price {
    fn of(cost: &WrittenCost) -> Price {
        Price {
            kind: cost.kind,
            commodity: cost.commodity.clone(),
            value: cost.value.clone(),
        }
    }

    fn written(&self) -> WrittenCost {
        WrittenCost {
            kind: self.kind,
            commodity: self.commodity.clone(),
            value: self.value.clone(),
            inferred_unit: false,
        }
    }
}

/// Whether two entries are one amount to hledger's sums: no price, or the same kind of price in
/// the same commodity and, for a unit price, the same value.
fn same_group(a: &Option<Price>, b: &Option<Price>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            x.kind == y.kind
                && x.commodity == y.commodity
                && (x.kind == CostKind::Total || x.value.cmp_value(&y.value).is_eq())
        }
        _ => false,
    }
}

/// The order amounts of one commodity are listed in: no price first, then by price commodity,
/// a total price before a unit price, then by price value.
fn cmp_group(a: &Option<Price>, b: &Option<Price>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(x), Some(y)) => {
            let rank = |p: &Price| u8::from(p.kind == CostKind::Unit);
            x.commodity
                .cmp(&y.commodity)
                .then(rank(x).cmp(&rank(y)))
                .then_with(|| x.value.cmp_value(&y.value))
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    quantity: Decimal,
    price: Option<Price>,
    /// The lot of the one posting that made this entry; adding a second posting drops it.
    lot: Option<WrittenLot>,
}

/// What one price group of a commodity holds: the sum of its entries.
struct Group {
    price: Option<Price>,
    quantity: Decimal,
    /// The lot, when exactly one entry made the group.
    lot: Option<WrittenLot>,
}

fn groups(entries: &[&Entry]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|g| same_group(&g.price, &e.price)) {
            Some(g) => {
                g.quantity = g.quantity.combine(&e.quantity, ZeroTie::KeepLater);
                if let (Some(p), Some(q)) = (g.price.as_mut(), e.price.as_ref())
                    && p.kind == CostKind::Total
                {
                    p.value = p.value.combine(&q.value, ZeroTie::KeepLater);
                }
                g.lot = None;
            }
            None => out.push(Group {
                price: e.price.clone(),
                quantity: e.quantity.clone(),
                lot: e.lot.clone(),
            }),
        }
    }
    out.sort_by(|a, b| cmp_group(&a.price, &b.price));
    out
}

/// One amount of a difference.
struct Piece {
    price: Option<Price>,
    quantity: Decimal,
    lot: Option<WrittenLot>,
}

impl Piece {
    /// Nothing to list: no quantity, and no total price either.
    fn is_zero(&self) -> bool {
        self.quantity.is_zero()
            && self
                .price
                .as_ref()
                .is_none_or(|p| p.kind == CostKind::Unit || p.value.is_zero())
    }
}

/// `asserted - held`, per price group, listed in group order. Where one side is zero the other
/// comes back with its own places; the amounts of a group nothing is asserted for are negated.
fn difference(asserted: Option<&WrittenAmount>, held: &[Group]) -> Vec<Piece> {
    let asserted_price = asserted.and_then(|a| a.cost.as_ref().map(Price::of));
    let mut out = Vec::new();
    let mut used = false;
    for g in held {
        match asserted {
            Some(a) if same_group(&asserted_price, &g.price) => {
                used = true;
                let price = asserted_price.clone().map(|mut p| {
                    if let (CostKind::Total, Some(held_price)) = (p.kind, &g.price) {
                        p.value = p
                            .value
                            .combine(&held_price.value.negated(), ZeroTie::KeepEarlier);
                    }
                    p
                });
                out.push(Piece {
                    price,
                    quantity: a
                        .quantity
                        .combine(&g.quantity.negated(), ZeroTie::KeepEarlier),
                    lot: None,
                });
            }
            _ => out.push(Piece {
                price: g.price.clone().map(|mut p| {
                    if p.kind == CostKind::Total {
                        p.value = p.value.negated();
                    }
                    p
                }),
                quantity: g.quantity.negated(),
                lot: g.lot.clone(),
            }),
        }
    }
    if let (Some(a), false) = (asserted, used) {
        out.push(Piece {
            price: asserted_price,
            quantity: a.quantity.clone(),
            lot: None,
        });
    }
    out.sort_by(|a, b| cmp_group(&a.price, &b.price));
    out
}

/// The amount the account holds in `commodity` is not the asserted one.
#[derive(Debug)]
pub(crate) struct Mismatch {
    pub commodity: String,
    pub expected: Decimal,
    pub actual: Decimal,
}

/// The balances of every account, as one file's walk builds them.
#[derive(Debug, Default)]
pub(crate) struct Balances {
    accounts: BTreeMap<String, BTreeMap<String, Vec<Entry>>>,
}

impl Balances {
    /// A posting's amount joins the balance of its account; its price and lot never count, and a
    /// second posting in a commodity drops the lot of the first.
    pub(crate) fn add(&mut self, account: &str, amount: &WrittenAmount) {
        let entries = self
            .accounts
            .entry(account.to_string())
            .or_default()
            .entry(amount.commodity.clone())
            .or_default();
        match entries.iter_mut().find(|e| e.price.is_none()) {
            Some(e) => {
                e.quantity = e.quantity.combine(&amount.quantity, ZeroTie::KeepLater);
                e.lot = None;
            }
            None => entries.insert(
                0,
                Entry {
                    quantity: amount.quantity.clone(),
                    price: None,
                    lot: amount.lot.clone(),
                },
            ),
        }
    }

    /// The entries of the sub-accounts (`a:` prefix), in name order.
    fn below(&self, account: &str) -> Vec<&BTreeMap<String, Vec<Entry>>> {
        let prefix = format!("{account}:");
        self.accounts
            .range::<str, _>((Bound::Included(prefix.as_str()), Bound::Unbounded))
            .take_while(|(name, _)| name.starts_with(&prefix))
            .map(|(_, held)| held)
            .collect()
    }

    /// The accounts whose entries count: `account`, then, if `inclusive`, everything below it.
    fn scope(&self, account: &str, inclusive: bool) -> Vec<&BTreeMap<String, Vec<Entry>>> {
        let mut scope: Vec<_> = self.accounts.get(account).into_iter().collect();
        if inclusive {
            scope.extend(self.below(account));
        }
        scope
    }

    fn entries<'a>(scope: &[&'a BTreeMap<String, Vec<Entry>>], commodity: &str) -> Vec<&'a Entry> {
        scope
            .iter()
            .filter_map(|held| held.get(commodity))
            .flatten()
            .collect()
    }

    /// What the account holds in `commodity`, priced entries included; zero if it holds none.
    pub(crate) fn total(&self, account: &str, inclusive: bool, commodity: &str) -> Decimal {
        Self::entries(&self.scope(account, inclusive), commodity)
            .into_iter()
            .fold(None, |sum: Option<Decimal>, e| {
                Some(match sum {
                    Some(sum) => sum.combine(&e.quantity, ZeroTie::KeepLater),
                    None => e.quantity.clone(),
                })
            })
            .unwrap_or_else(Decimal::zero)
    }

    /// The commodities the account holds, in code point order.
    fn commodities(&self, account: &str, inclusive: bool) -> BTreeSet<String> {
        self.scope(account, inclusive)
            .into_iter()
            .flat_map(|held| held.keys().cloned())
            .collect()
    }

    /// Compare the account with an assertion; `None` when it holds. With `total`, every other
    /// commodity must be at zero too, and the first (in code point order) that is not is named.
    pub(crate) fn check(
        &self,
        account: &str,
        total: bool,
        inclusive: bool,
        asserted: &WrittenAmount,
    ) -> Option<Mismatch> {
        let actual = self.total(account, inclusive, &asserted.commodity);
        if !actual.cmp_value(&asserted.quantity).is_eq() {
            return Some(Mismatch {
                commodity: asserted.commodity.clone(),
                expected: asserted.quantity.clone(),
                actual,
            });
        }
        if !total {
            return None;
        }
        self.commodities(account, inclusive)
            .into_iter()
            .filter(|c| *c != asserted.commodity)
            .find_map(|commodity| {
                let actual = self.total(account, inclusive, &commodity);
                (!actual.is_zero()).then(|| Mismatch {
                    commodity,
                    expected: Decimal::zero(),
                    actual,
                })
            })
    }

    /// Work out the amounts of a balance assignment and set the balance as it leaves it
    /// (assignments.md). The amounts are listed by commodity, then by price group.
    pub(crate) fn assign(
        &mut self,
        account: &str,
        total: bool,
        inclusive: bool,
        asserted: &WrittenAmount,
    ) -> Vec<WrittenAmount> {
        let mut commodities = BTreeSet::from([asserted.commodity.clone()]);
        if total {
            commodities.extend(self.commodities(account, inclusive));
        }
        let mut amounts = Vec::new();
        let mut replacements: Vec<(String, Vec<Entry>)> = Vec::new();
        for commodity in commodities {
            let target = (commodity == asserted.commodity).then_some(asserted);
            let held = groups(&Self::entries(&self.scope(account, inclusive), &commodity));
            for piece in difference(target, &held) {
                if !piece.is_zero() {
                    amounts.push(WrittenAmount {
                        commodity: commodity.clone(),
                        quantity: piece.quantity,
                        cost: piece.price.as_ref().map(Price::written),
                        lot: if target.is_some() { None } else { piece.lot },
                    });
                }
            }
            // What the account itself holds afterwards: the asserted amount less what the
            // sub-accounts hold (all of it, for `=` and `==`, which look at no sub-account).
            let below = if inclusive {
                groups(&Self::entries(&self.below(account), &commodity))
            } else {
                Vec::new()
            };
            let left = difference(target, &below)
                .into_iter()
                .filter(|p| target.is_some() || !p.is_zero())
                .map(|p| Entry {
                    quantity: p.quantity,
                    price: p.price,
                    lot: None,
                })
                .collect();
            replacements.push((commodity, left));
        }
        let own = self.accounts.entry(account.to_string()).or_default();
        for (commodity, entries) in replacements {
            if entries.is_empty() {
                own.remove(&commodity);
            } else {
                own.insert(commodity, entries);
            }
        }
        amounts
    }
}
