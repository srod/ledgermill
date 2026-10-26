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
            display_places: self.value.places(),
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
    /// The display precision of the balance: the most places among the amounts added to it.
    shown: u32,
    price: Option<Price>,
    /// The lot of the one posting that made this entry; adding a second posting drops it.
    lot: Option<WrittenLot>,
}

/// What one price group of a commodity holds: the sum of its entries.
struct Group {
    price: Option<Price>,
    quantity: Decimal,
    shown: u32,
    /// The lot, when exactly one entry made the group.
    lot: Option<WrittenLot>,
}

fn groups(entries: &[&Entry]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for e in entries {
        match out.iter_mut().find(|g| same_group(&g.price, &e.price)) {
            Some(g) => {
                g.quantity = g.quantity.combine(&e.quantity, ZeroTie::KeepLater);
                g.shown = g.shown.max(e.shown);
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
                shown: e.shown,
                lot: e.lot.clone(),
            }),
        }
    }
    out.sort_by(|a, b| cmp_group(&a.price, &b.price));
    out
}

/// `held - taken`, per price group; a group `taken` has none of is left as it is.
fn subtract(held: &mut [Group], taken: &[Group]) {
    for g in held.iter_mut() {
        let Some(t) = taken.iter().find(|t| same_group(&t.price, &g.price)) else {
            continue;
        };
        // Observed places (assignments.md): nothing is taken off when the account holds a
        // zero; otherwise plain arithmetic, a zero whole being no exception, except that a share
        // equal to the account's own balance has the places of that balance.
        g.shown = g.shown.max(t.shown);
        if !t.quantity.is_zero() {
            let share = g.quantity.add(&t.quantity.negated());
            g.quantity = if share.cmp_value(&t.quantity).is_eq() {
                share.rounded(t.quantity.places())
            } else {
                share
            };
        }
        if let (Some(p), Some(q)) = (g.price.as_mut(), t.price.as_ref())
            && p.kind == CostKind::Total
        {
            p.value = p.value.combine(&q.value.negated(), ZeroTie::KeepEarlier);
        }
    }
}

/// One amount of a difference.
struct Piece {
    price: Option<Price>,
    quantity: Decimal,
    /// The places it is listed with, at least those of the quantity (see `WrittenAmount`).
    shown: u32,
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
                    shown: a.display_places.max(g.shown),
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
                shown: g.shown,
                lot: g.lot.clone(),
            }),
        }
    }
    if let (Some(a), false) = (asserted, used) {
        out.push(Piece {
            price: asserted_price,
            quantity: a.quantity.clone(),
            shown: a.display_places,
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
    /// A posting's amount joins the balance of its account; a second posting in a commodity drops
    /// the lot of the first. Its price never counts in a sum, but a posting of a transaction with
    /// an assignment (`keep_price`) keeps it: it makes an entry of its own price group, which the
    /// amounts of a later assignment subtract as they are (assignment-processing.md).
    pub(crate) fn add(&mut self, account: &str, amount: &WrittenAmount, keep_price: bool) {
        let price = if keep_price {
            amount.cost.as_ref().map(Price::of)
        } else {
            None
        };
        let entries = self
            .accounts
            .entry(account.to_string())
            .or_default()
            .entry(amount.commodity.clone())
            .or_default();
        match entries.iter_mut().find(|e| same_group(&e.price, &price)) {
            Some(e) => {
                e.quantity = e.quantity.combine(&amount.quantity, ZeroTie::KeepLater);
                e.shown = e.shown.max(amount.display_places);
                if let (Some(held), Some(added)) = (e.price.as_mut(), price.as_ref())
                    && held.kind == CostKind::Total
                {
                    held.value = held.value.combine(&added.value, ZeroTie::KeepLater);
                }
                e.lot = None;
            }
            None => {
                entries.push(Entry {
                    quantity: amount.quantity.clone(),
                    shown: amount.display_places,
                    price,
                    lot: amount.lot.clone(),
                });
                entries.sort_by(|a, b| cmp_group(&a.price, &b.price));
            }
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
        // Only the commodities the account holds itself are looked at: one held only by a
        // sub-account is not, even with a star.
        self.commodities(account, false)
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
    ///
    /// The asserted commodity is compared with what the account holds (and, with `inclusive`,
    /// what its sub-accounts hold). `total` adds every other commodity of the account; `inclusive`
    /// alone adds the other commodities of the sub-accounts, which the account itself then takes
    /// back (assignments.md, "Other commodities with a star").
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
        } else if inclusive {
            commodities.extend(self.below(account).iter().flat_map(|h| h.keys().cloned()));
        }
        let mut amounts = Vec::new();
        let mut replacements: Vec<(String, Vec<Entry>)> = Vec::new();
        for commodity in commodities {
            let is_asserted = commodity == asserted.commodity;
            let target = is_asserted.then_some(asserted);
            let own = Self::entries(&self.scope(account, false), &commodity);
            let below = if inclusive {
                Self::entries(&self.below(account), &commodity)
            } else {
                Vec::new()
            };
            let mut all_entries = own.clone();
            all_entries.extend(&below);
            let mut held = groups(&all_entries);
            if !total && !is_asserted {
                // Another commodity, with a star only: what the sub-accounts hold, worked out as
                // the whole less the account's own, which can give other places than their sum.
                subtract(&mut held, &groups(&own));
            }
            let own_groups = groups(&own);
            let below_groups = groups(&below);
            // With a star and a price-less assertion, the price-less part of the asserted commodity
            // is (asserted - the sub-accounts' price-less sum) - own, which has other places than
            // asserted - (own + sub-accounts) when that whole is zero; priced entries beside it are
            // only negated (assignments.md).
            let in_two_steps = inclusive && asserted.cost.is_none() && is_asserted;
            // Zero pieces are not listed on the posting, but they still carry their places into
            // the balance (assignments.md, "Places of the balance after a star").
            let mut pieces: Vec<Piece> = difference(target, &held);
            let plain = |pieces: Vec<Piece>| pieces.into_iter().find(|p| p.price.is_none());
            if in_two_steps
                && let Some(after_subs) = plain(difference(target, &below_groups))
                && let Some(two_steps) = plain(difference(
                    Some(&WrittenAmount {
                        commodity: commodity.clone(),
                        display_places: after_subs.quantity.places().max(after_subs.shown),
                        quantity: after_subs.quantity,
                        cost: None,
                        lot: None,
                    }),
                    &own_groups,
                ))
                && let Some(slot) = pieces.iter_mut().find(|p| p.price.is_none())
            {
                *slot = two_steps;
            }
            let left: Vec<Entry> = if total || is_asserted {
                // What the account itself holds afterwards: the asserted amount less what the
                // sub-accounts hold (all of it, for `=` and `==`, which look at no sub-account).
                difference(target, &groups(&below))
                    .into_iter()
                    .map(|p| Entry {
                        quantity: p.quantity,
                        shown: p.shown,
                        price: p.price,
                        lot: None,
                    })
                    .collect()
            } else if pieces.is_empty() {
                own.into_iter().cloned().collect()
            } else {
                let mut left: Vec<Entry> = own.into_iter().cloned().collect();
                for p in &pieces {
                    match left.iter_mut().find(|e| same_group(&e.price, &p.price)) {
                        Some(e) => {
                            e.quantity = e.quantity.combine(&p.quantity, ZeroTie::KeepLater);
                            e.shown = e.shown.max(p.shown);
                            if let (Some(held), Some(add)) = (e.price.as_mut(), p.price.as_ref())
                                && held.kind == CostKind::Total
                            {
                                held.value = held.value.combine(&add.value, ZeroTie::KeepLater);
                            }
                            e.lot = None;
                        }
                        None => left.push(Entry {
                            quantity: p.quantity.clone(),
                            shown: p.shown,
                            price: p.price.clone(),
                            lot: None,
                        }),
                    }
                }
                left.sort_by(|a, b| cmp_group(&a.price, &b.price));
                left
            };
            for piece in pieces.into_iter().filter(|p| !p.is_zero()) {
                amounts.push(WrittenAmount {
                    commodity: commodity.clone(),
                    display_places: piece.quantity.places().max(piece.shown),
                    quantity: piece.quantity,
                    cost: piece.price.as_ref().map(Price::written),
                    lot: if is_asserted { None } else { piece.lot },
                });
            }
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
