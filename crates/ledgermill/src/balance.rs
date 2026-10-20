//! Balancing one transaction: conversion postings, inferred prices, the elided amount, and the
//! check that each group sums to zero. See `spec/hledger/balancing.md`, `elided-amounts.md`,
//! `inferred-prices.md` and `conversion.md`.

use crate::decimal::{DIVISION_PLACES, Decimal, OUTPUT_PLACES, ZeroTie};
use crate::model::PostingKind;
use crate::posting::{CostKind, WrittenAmount, WrittenCost, WrittenLot};
use std::collections::BTreeMap;

/// A posting as balancing sees it.
#[derive(Debug, Clone)]
pub(crate) struct Draft {
    pub kind: PostingKind,
    pub conversion_account: bool,
    /// `None`: the amount was left out. `Some(vec![])` never happens on input.
    pub amounts: Option<Vec<WrittenAmount>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unbalanced {
    /// The residual, one `(commodity, quantity)` per commodity that does not balance.
    Residual {
        group: Group,
        residual: Vec<(String, String)>,
    },
    TwoElided {
        group: Group,
    },
    ConversionWithCost,
    /// Conversion postings in a shape whose rule we have not pinned down (conversion.md).
    UnsupportedConversion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    Real,
    BalancedVirtual,
}

impl Group {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Group::Real => "real",
            Group::BalancedVirtual => "balanced-virtual",
        }
    }

    fn of(kind: PostingKind) -> Option<Group> {
        match kind {
            PostingKind::Regular => Some(Group::Real),
            PostingKind::BalancedVirtual => Some(Group::BalancedVirtual),
            PostingKind::Virtual => None,
        }
    }
}

/// What an amount contributes to its group's sum: the cost if it has one, else itself.
fn contribution(a: &WrittenAmount) -> (String, Decimal) {
    match &a.cost {
        Some(c) => (c.commodity.clone(), cost_value(a, c)),
        None => (a.commodity.clone(), a.quantity.clone()),
    }
}

fn cost_value(a: &WrittenAmount, c: &WrittenCost) -> Decimal {
    match c.kind {
        CostKind::Unit => a.quantity.mul(&c.value).normalized(),
        CostKind::Total => c.value.clone(),
    }
}

fn priced(p: &Draft) -> bool {
    p.amounts
        .as_deref()
        .is_some_and(|a| a.iter().any(|x| x.cost.is_some()))
}

/// `value` is within half a unit of `target` at `places` decimal places.
fn within_half_unit(value: &Decimal, target: &Decimal, places: u32) -> bool {
    let gap = value.add(&target.negated());
    let gap = if gap.is_negative() {
        gap.negated()
    } else {
        gap
    };
    gap.cmp_value(&Decimal::new(5u8.into(), places + 1)).is_le()
}

/// A pair goes with a priced posting when its two amounts are the negated quantity and the
/// cost, in either order, by number only, each read at its own written places.
fn pair_fits(a: &WrittenAmount, cost: &WrittenCost, x: &WrittenAmount, y: &WrittenAmount) -> bool {
    let neg = a.quantity.negated();
    let value = cost_value(a, cost);
    let fits = |q: &WrittenAmount, c: &WrittenAmount| {
        within_half_unit(&q.quantity, &neg, q.quantity.places())
            && within_half_unit(&c.quantity, &value, c.quantity.places())
    };
    fits(x, y) || fits(y, x)
}

/// A posting without a price whose quantity is the priced quantity, by number only and at the
/// priced quantity's places, turns the leading-run rule off.
fn cancels_leading_run(a: &WrittenAmount, p: &Draft) -> bool {
    !p.conversion_account
        && p.amounts.as_deref().is_some_and(|amounts| {
            amounts.iter().all(|x| x.cost.is_none())
                && amounts
                    .iter()
                    .any(|x| within_half_unit(&x.quantity, &a.quantity, a.quantity.places()))
        })
}

/// Postings balanced by their quantity rather than their cost because of the conversion
/// postings around them; see `conversion.md`. Transactions with several priced postings whose
/// rule is not pinned down are `UnsupportedConversion`.
fn conversion_exempt(postings: &[Draft]) -> Result<Vec<bool>, Unbalanced> {
    let n = postings.len();
    let is_conv = |i: usize| postings.get(i).is_some_and(|p| p.conversion_account);
    // Adjacent pairs, found greedily left to right.
    let mut pairs = Vec::new();
    let mut i = 0;
    while i + 1 < n {
        if is_conv(i) && is_conv(i + 1) {
            pairs.push(i);
            i += 2;
        } else {
            i += 1;
        }
    }
    if pairs
        .iter()
        .any(|&p| priced(&postings[p]) || priced(&postings[p + 1]))
    {
        return Err(Unbalanced::ConversionWithCost);
    }
    let priced_at: Vec<usize> = (0..n).filter(|&k| priced(&postings[k])).collect();
    let mut exempt = vec![false; n];
    if pairs.is_empty() || priced_at.is_empty() {
        return Ok(exempt);
    }
    let unsupported = Err(Unbalanced::UnsupportedConversion);
    if priced_at.iter().any(|&k| is_conv(k)) {
        return unsupported;
    }
    // Where the odd conversion run right before a priced posting starts.
    let run_start = |k: usize| (0..k).rev().take_while(|&i| is_conv(i)).last().unwrap_or(k);
    // With several priced postings, an odd run preceded only by conversion and priced postings
    // is a shape whose rule is not pinned down.
    let unclear = |k: usize| {
        let start = run_start(k);
        (k - start) % 2 == 1 && (0..start).all(|i| is_conv(i) || priced(&postings[i]))
    };
    if priced_at.len() > 1 && priced_at.iter().any(|&k| unclear(k)) {
        return unsupported;
    }
    let one = |i: usize| match postings[i].amounts.as_deref() {
        Some([a]) => Some(a),
        _ => None,
    };
    let mut eligible: Vec<(usize, &WrittenAmount, &WrittenCost)> = Vec::new();
    for &k in &priced_at {
        let Some(a) = one(k) else { return unsupported };
        let Some(cost) = &a.cost else {
            return unsupported;
        };
        let run_start = run_start(k);
        if (k - run_start) % 2 == 1 {
            // An odd run of conversion postings right before the priced posting: it balances
            // by quantity only when the run opens the transaction, whatever the amounts, and
            // no posting repeats its quantity; the pairs are not consulted.
            exempt[k] = run_start == 0 && !postings.iter().any(|p| cancels_leading_run(a, p));
        } else {
            eligible.push((k, a, cost));
        }
    }
    // Each pair goes with the one priced posting it fits; a pair fitting several goes with none.
    // A pair with an elided amount fits nothing.
    for &q in &pairs {
        let (Some(x), Some(y)) = (one(q), one(q + 1)) else {
            continue;
        };
        let mut fit = eligible.iter().filter(|(_, a, c)| pair_fits(a, c, x, y));
        if let (Some((k, _, _)), None) = (fit.next(), fit.next()) {
            exempt[*k] = true;
        }
    }
    Ok(exempt)
}

/// Balance a transaction in place: fill the elided amount, infer prices, and report what does
/// not balance.
pub(crate) fn balance(postings: &mut [Draft]) -> Result<(), Unbalanced> {
    let exempt = conversion_exempt(postings)?;
    for group in [Group::Real, Group::BalancedVirtual] {
        balance_group(postings, group, &exempt)?;
    }
    for p in postings.iter_mut() {
        if p.kind == PostingKind::Virtual && p.amounts.is_none() {
            p.amounts = Some(vec![WrittenAmount {
                commodity: String::new(),
                quantity: Decimal::zero(),
                cost: None,
                lot: None,
            }]);
        }
    }
    Ok(())
}

fn balance_group(postings: &mut [Draft], group: Group, exempt: &[bool]) -> Result<(), Unbalanced> {
    let members: Vec<usize> = (0..postings.len())
        .filter(|&i| Group::of(postings[i].kind) == Some(group))
        .collect();
    let elided: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| postings[i].amounts.is_none())
        .collect();
    if elided.len() > 1 {
        return Err(Unbalanced::TwoElided { group });
    }
    if let Some(&e) = elided.first() {
        // An elided amount next to a matched conversion pair: the reference tool rejects
        // the transaction (conversion.md).
        if members.iter().any(|&i| exempt[i]) {
            return Err(Unbalanced::Residual {
                group,
                residual: Vec::new(),
            });
        }
        let amounts = elided_amounts(postings, &members, exempt);
        postings[e].amounts = Some(amounts);
        return Ok(());
    }
    if !members.is_empty() {
        infer_prices(postings, &members);
    }
    let everyone: Vec<usize> = (0..postings.len()).collect();
    let residual = residual(postings, &members, &everyone, exempt);
    if residual.is_empty() {
        Ok(())
    } else {
        Err(Unbalanced::Residual { group, residual })
    }
}

/// Sum per commodity of each member's contribution, in posting order. Exact; a zero term gives
/// back the other term, places included, as the reference tool adds amounts.
fn sums(postings: &[Draft], members: &[usize], exempt: &[bool]) -> BTreeMap<String, Decimal> {
    let mut sums: BTreeMap<String, Decimal> = BTreeMap::new();
    for &i in members {
        for a in postings[i].amounts.iter().flatten() {
            let (commodity, value) = if exempt[i] {
                (a.commodity.clone(), a.quantity.clone())
            } else {
                contribution(a)
            };
            match sums.get_mut(&commodity) {
                Some(sum) => *sum = sum.combine(&value, ZeroTie::KeepLater),
                None => {
                    sums.insert(commodity, value);
                }
            }
        }
    }
    sums
}

/// Commodities whose sum is not zero at the precision their amounts were written with.
/// Precision comes from every posting of the transaction, virtual ones included.
fn residual(
    postings: &[Draft],
    members: &[usize],
    everyone: &[usize],
    exempt: &[bool],
) -> Vec<(String, String)> {
    let precision = precisions(postings, everyone);
    sums(postings, members, exempt)
        .into_iter()
        .filter(|(commodity, sum)| {
            let places = precision.get(commodity).copied().unwrap_or(0);
            !sum.rounded(places).is_zero()
        })
        .map(|(commodity, sum)| (commodity, sum.output()))
        .collect()
}

/// The places a commodity is checked to: the most places among the amounts written in it, or,
/// if none is, among the prices written in it. Capped at ten.
fn precisions(postings: &[Draft], members: &[usize]) -> BTreeMap<String, u32> {
    let mut written: BTreeMap<String, u32> = BTreeMap::new();
    let mut priced: BTreeMap<String, u32> = BTreeMap::new();
    for &i in members {
        for a in postings[i].amounts.iter().flatten() {
            let w = written.entry(a.commodity.clone()).or_insert(0);
            *w = (*w).max(a.quantity.places());
            if let Some(c) = &a.cost {
                let p = priced.entry(c.commodity.clone()).or_insert(0);
                *p = (*p).max(c.value.places());
            }
        }
    }
    for (commodity, places) in priced {
        written.entry(commodity).or_insert(places);
    }
    written
        .into_iter()
        .map(|(c, p)| (c, p.min(OUTPUT_PLACES)))
        .collect()
}

/// The amounts of an elided posting: per commodity, the negated sum of the others, worked out
/// in two steps (elided-amounts.md). First amounts are summed per group: same commodity, same
/// kind of cost, same cost commodity and, for unit prices, the same price value. Then each group
/// is converted (a unit price multiplies, a total is its own value) and the results are summed
/// per commodity: costless groups first, then by commodity, totals before unit prices. In both
/// sums a zero term gives back the other term, decimal places included.
fn elided_amounts(postings: &[Draft], members: &[usize], exempt: &[bool]) -> Vec<WrittenAmount> {
    // (costed, commodity, kind: 0 none / 1 total / 2 unit, cost commodity, unit price)
    type Key = (bool, String, u8, String, Decimal);
    struct Group {
        quantity: Decimal,
        total: Option<Decimal>,
        lots: Vec<Option<WrittenLot>>,
    }
    let mut groups: Vec<(Key, Group)> = Vec::new();
    for &i in members {
        for a in postings[i].amounts.iter().flatten() {
            let cost = if exempt[i] { None } else { a.cost.as_ref() };
            let key: Key = match cost {
                None => (
                    false,
                    a.commodity.clone(),
                    0,
                    String::new(),
                    Decimal::zero(),
                ),
                Some(c) if c.kind == CostKind::Total => (
                    true,
                    a.commodity.clone(),
                    1,
                    c.commodity.clone(),
                    Decimal::zero(),
                ),
                Some(c) => (
                    true,
                    a.commodity.clone(),
                    2,
                    c.commodity.clone(),
                    c.value.normalized(),
                ),
            };
            let total = cost
                .filter(|c| c.kind == CostKind::Total)
                .map(|c| c.value.clone());
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, g)) => {
                    g.quantity = g.quantity.combine(&a.quantity, ZeroTie::KeepLater);
                    if let Some(t) = total {
                        g.total = Some(match &g.total {
                            Some(sum) => sum.combine(&t, ZeroTie::KeepLater),
                            None => t,
                        });
                    }
                    g.lots.push(a.lot.clone());
                }
                None => groups.push((
                    key,
                    Group {
                        quantity: a.quantity.clone(),
                        total,
                        lots: vec![a.lot.clone()],
                    },
                )),
            }
        }
    }
    groups.sort_by(|(a, _), (b, _)| {
        (a.0, &a.1, a.2, &a.3)
            .cmp(&(b.0, &b.1, b.2, &b.3))
            .then_with(|| a.4.cmp_value(&b.4))
    });
    let mut result: BTreeMap<String, (Decimal, Vec<Option<WrittenLot>>)> = BTreeMap::new();
    for ((costed, commodity, kind, cost_commodity, unit), g) in groups {
        let (target, value, lots) = match (costed, kind, g.total) {
            (true, 1, Some(total)) => (cost_commodity, total, vec![None, None]),
            (true, _, _) => (
                cost_commodity,
                g.quantity.mul(&unit).normalized(),
                vec![None, None],
            ),
            _ => (commodity, g.quantity, g.lots),
        };
        match result.get_mut(&target) {
            Some((sum, all)) => {
                *sum = sum.combine(&value, ZeroTie::KeepEarlier);
                all.extend(lots);
            }
            None => {
                result.insert(target, (value, lots));
            }
        }
    }
    result
        .into_iter()
        .map(|(commodity, (sum, lots))| WrittenAmount {
            commodity,
            quantity: sum.negated(),
            cost: None,
            // A lot survives only when exactly one amount contributed.
            lot: match lots.as_slice() {
                [Some(lot)] => Some(lot.clone()),
                _ => None,
            },
        })
        .collect()
}

/// Price inference: in a group with no written price, two commodities that do not balance give
/// the first one a price in the other (inferred-prices.md).
fn infer_prices(postings: &mut [Draft], members: &[usize]) {
    let has_cost = members.iter().any(|&i| {
        postings[i]
            .amounts
            .iter()
            .flatten()
            .any(|a| a.cost.is_some())
    });
    if has_cost {
        return;
    }
    let none = vec![false; postings.len()];
    let sums = sums(postings, members, &none);
    let open: Vec<&String> = sums
        .iter()
        .filter(|(_, s)| !s.is_zero())
        .map(|(c, _)| c)
        .collect();
    let [x, y] = open.as_slice() else { return };
    // The commodity of the first amount (in posting order) among the two gets the price.
    let first = members.iter().find_map(|&i| {
        postings[i]
            .amounts
            .iter()
            .flatten()
            .map(|a| &a.commodity)
            .find(|c| c == x || c == y)
    });
    let Some(first) = first.cloned() else { return };
    let other = if &first == *x {
        (*y).clone()
    } else {
        (*x).clone()
    };
    let holders: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| {
            postings[i]
                .amounts
                .iter()
                .flatten()
                .any(|a| a.commodity == first)
        })
        .collect();
    let single = |i: usize| postings[i].amounts.as_ref().is_some_and(|a| a.len() == 1);
    let other_holders_single = members
        .iter()
        .filter(|&&i| {
            postings[i]
                .amounts
                .iter()
                .flatten()
                .any(|a| a.commodity == other)
        })
        .all(|&i| single(i));
    if !holders.iter().all(|&i| single(i)) || !other_holders_single {
        return;
    }
    let (Some(this_sum), Some(other_sum)) = (sums.get(&first), sums.get(&other)) else {
        return;
    };
    if this_sum.is_negative() == other_sum.is_negative() {
        return;
    }
    let price_total = other_sum.negated();
    let cost = if holders.len() == 1 {
        WrittenCost {
            kind: CostKind::Total,
            commodity: other.clone(),
            value: price_total,
            inferred_unit: false,
        }
    } else {
        let Some(unit) = price_total.divided(this_sum, DIVISION_PLACES) else {
            return;
        };
        WrittenCost {
            kind: CostKind::Unit,
            commodity: other.clone(),
            value: unit,
            inferred_unit: true,
        }
    };
    for &i in &holders {
        for a in postings[i].amounts.iter_mut().flatten() {
            if a.commodity == first {
                a.cost = Some(cost.clone());
            }
        }
    }
}
