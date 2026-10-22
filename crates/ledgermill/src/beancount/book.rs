//! Booking one transaction the way the reference tool does: per-unit prices and costs, the
//! amount of an elided posting, the balance check, and the order of the postings.
//! See spec/beancount/numbers-and-interpolation.md and spec/beancount/lots-and-booking.md.
//!
//! The lot matching itself is the booking crate's (through `lima.rs`); the figures are ours.

use super::lima::{Books, Feed, FeedCost};
use super::plain::{BookFailure, Day, Lot, RawPosting, RawPrice, RawTransaction};
use super::pydec::Num as Decimal;
use std::collections::HashMap;

/// A posting after booking: one row of the postings table.
#[derive(Debug, Clone)]
pub(super) struct BookedPosting {
    pub account: String,
    pub flag: Option<char>,
    pub line: u32,
    pub units: Decimal,
    pub currency: String,
    pub lot: Option<Lot>,
    /// The price per unit and its currency.
    pub price: Option<(Decimal, String)>,
}

pub(super) struct Booked {
    pub postings: Vec<BookedPosting>,
    /// Currencies whose postings do not add up to zero, with the residual.
    pub residuals: Vec<(String, Decimal)>,
}

/// `number / |units|`, the per-unit figure of a total.
fn per_unit(total: &Decimal, units: &Decimal) -> Option<Decimal> {
    total.div(&units.abs())
}

fn feed_of(p: &RawPosting) -> Result<Feed, BookFailure> {
    let cost = match &p.cost {
        None => None,
        Some(c) => {
            let units = p.units.as_ref();
            let from_total = match (&c.total, units) {
                (Some(total), Some(units)) => {
                    // A cost per unit is written plainly, unlike a price per unit.
                    Some(
                        per_unit(total, units)
                            .ok_or(BookFailure::TooPrecise)?
                            .without_positive_exponent(),
                    )
                }
                _ => None,
            };
            let per = match (&c.per_unit, from_total) {
                (Some(p), Some(t)) => Some(p.add(&t)),
                (Some(p), None) => Some(p.clone()),
                (None, t) => t,
            };
            Some(FeedCost {
                per_unit: per,
                currency: c.currency.clone(),
                date: c.date,
                label: c.label.clone(),
            })
        }
    };
    let price = match &p.price {
        None => None,
        Some(RawPrice {
            number: Some(number),
            total,
            currency: Some(currency),
        }) => {
            if *total {
                let units = p.units.as_ref().ok_or(BookFailure::TooPrecise)?;
                let value = per_unit(number, units).ok_or(BookFailure::TooPrecise)?;
                Some((value, currency.clone()))
            } else {
                Some((number.clone(), currency.clone()))
            }
        }
        // `infer` fills every part in before a posting is fed.
        Some(_) => return Err(BookFailure::Unsupported("inferred-price")),
    };
    Ok(Feed {
        account: p.account.clone(),
        units: p.units.clone(),
        currency: p.currency.clone(),
        cost,
        price,
    })
}

fn is_auto(p: &RawPosting) -> bool {
    p.units.is_none() && p.currency.is_none() && p.cost.is_none() && p.price.is_none()
}

/// The posting the reference blames when several have no amount: the last of them
/// (spec/beancount/check-errors.md, "Which line a booking error is on").
fn last_auto(postings: &[RawPosting]) -> usize {
    postings.iter().rposition(is_auto).unwrap_or(0)
}

/// A row's currency for balancing: its lot's cost currency, else its price's, else its own.
fn weight_currency(
    row_currency: &str,
    lot: Option<&Lot>,
    price: Option<&(Decimal, String)>,
) -> String {
    match (lot, price) {
        (Some(lot), _) => lot.currency.clone(),
        (None, Some((_, currency))) => currency.clone(),
        (None, None) => row_currency.to_string(),
    }
}

/// A cost with no number at all (`{}`, `{EUR}`, `{"label"}`, `{date}`).
fn open_cost(p: &RawPosting) -> bool {
    p.cost
        .as_ref()
        .is_some_and(|c| c.per_unit.is_none() && c.total.is_none())
}

/// A price missing its number, its currency or both.
fn open_price(p: &RawPosting) -> bool {
    p.price
        .as_ref()
        .is_some_and(|p| p.number.is_none() || p.currency.is_none())
}

/// The currency a posting balances in and its weight; a cost or price must be complete.
fn weight_of(p: &RawPosting) -> Result<(String, Decimal), BookFailure> {
    let (Some(units), Some(currency)) = (&p.units, &p.currency) else {
        return Err(BookFailure::Booking("too-many-missing-numbers"));
    };
    let feed = feed_of(p)?;
    match (feed.cost, feed.price) {
        (
            Some(FeedCost {
                per_unit: Some(value),
                currency: Some(cost_currency),
                ..
            }),
            _,
        ) => Ok((cost_currency, units.mul(&value))),
        (Some(_), _) => Err(BookFailure::Unsupported("inferred-cost")),
        (None, Some((value, price_currency))) => Ok((price_currency, units.mul(&value))),
        (None, None) => Ok((currency.clone(), units.clone())),
    }
}

/// Work out the one cost (on a posting that adds a lot) or price that is left to balancing, as
/// the figure that makes the other postings of its currency add up to zero
/// (spec/beancount/inferred-cost-and-price.md). Returns the transaction with it filled in.
fn infer(books: &Books, tx: &RawTransaction) -> Result<RawTransaction, BookFailure> {
    let mut out = tx.clone();
    // The lots of an AVERAGE account are not tracked, and `book` refuses them anyway.
    if tx.postings.iter().any(|p| books.averages(&p.account)) {
        return Ok(out);
    }
    let wanted: Vec<usize> = tx
        .postings
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            open_price(p)
                || (open_cost(p)
                    && match (&p.units, &p.currency) {
                        (Some(units), Some(currency)) => {
                            books.adds_lot(&p.account, currency, units)
                        }
                        _ => true,
                    })
        })
        .map(|(i, _)| i)
        .collect();
    let Some(&at) = wanted.first() else {
        return Ok(out);
    };
    if wanted.len() > 1 {
        return Err(BookFailure::OnPosting("too-many-missing-numbers", at));
    }
    let target = &tx.postings[at];
    let is_cost = open_cost(target);
    let construct = if is_cost {
        "inferred-cost"
    } else {
        "inferred-price"
    };
    let Some(units) = &target.units else {
        return Err(BookFailure::OnPosting("too-many-missing-numbers", at));
    };
    let others = || tx.postings.iter().enumerate().filter(|(i, _)| *i != at);
    // A reduction with an open cost weighs what its lots cost, known only after matching.
    if others().any(|(_, p)| open_cost(p)) {
        return Err(BookFailure::Unsupported(construct));
    }
    // The reference blames the first posting of the group that lacks a number.
    if let Some((first, _)) = others().find(|(_, p)| is_auto(p) || p.units.is_none()) {
        return Err(BookFailure::OnPosting(
            if is_cost {
                "cannot-infer-currency"
            } else {
                "too-many-missing-numbers"
            },
            at.min(first),
        ));
    }
    let mut sums: Vec<(String, Decimal)> = Vec::new();
    for (index, p) in others() {
        let (currency, weight) = weight_of(p).map_err(|e| e.on(at.min(index)))?;
        match sums.iter_mut().find(|(c, _)| *c == currency) {
            Some((_, sum)) => *sum = sum.add(&weight),
            None => sums.push((currency, weight)),
        }
    }
    let written = if is_cost {
        target
            .cost
            .as_ref()
            .and_then(|c| c.currency.clone())
            .or_else(|| target.price.as_ref().and_then(|p| p.currency.clone()))
    } else {
        target.price.as_ref().and_then(|p| p.currency.clone())
    };
    let group = match (written, sums.as_slice()) {
        (Some(currency), _) => currency,
        (None, [(currency, _)]) => currency.clone(),
        _ => return Err(BookFailure::OnPosting("cannot-infer-currency", at)),
    };
    let sum = sums
        .iter()
        .find(|(c, _)| *c == group)
        .map_or_else(Decimal::zero, |(_, s)| s.clone());
    // A price written with its number needs no figure, only the currency.
    let needs_figure = is_cost || target.price.as_ref().is_some_and(|p| p.number.is_none());
    let value = if !needs_figure {
        Decimal::zero()
    } else if units.is_zero() {
        // Nothing is added, so a payment is left unbalanced (the reference says so). With a price,
        // or with nothing to pay, the reference crashes or was not probed: refuse.
        if is_cost && !sum.is_zero() {
            Decimal::zero()
        } else {
            return Err(BookFailure::Unsupported(construct));
        }
    } else if sum.is_zero() {
        Decimal::zero()
    } else {
        sum.negated().div(units).ok_or(BookFailure::TooPrecise)?
    };
    let slot = &mut out.postings[at];
    if is_cost {
        if value.is_negative() {
            return Err(BookFailure::OnPosting("negative-cost", at));
        }
        if let Some(cost) = slot.cost.as_mut() {
            cost.per_unit = Some(value);
            cost.currency = Some(group);
        }
    } else if let Some(price) = slot.price.as_mut() {
        if price.number.is_none() {
            price.number = Some(value.abs());
            price.total = false;
        }
        price.currency = Some(group);
    }
    Ok(out)
}

/// Lines of the postings with zero units and a cost written per unit (`0 BTC {10 EUR}`): the
/// reference rejects each as a booking error but still lists the transaction
/// (spec/beancount/check-errors.md, "Zero units at a cost").
///
/// A cost with a total (`{{10 EUR}}`, or `{1 # 5 EUR}`) is left out on purpose: the reference
/// crashes on zero units with a total cost, so the rule was never probed for it. Such a posting
/// cannot reach here anyway, because `feed_of` divides the total by the units and refuses
/// (`number-too-precise`) when they are zero. We refuse what we cannot check rather than guess.
pub(super) fn zero_units_at_cost(tx: &RawTransaction) -> impl Iterator<Item = u32> + '_ {
    tx.postings
        .iter()
        .filter(|p| {
            p.units.as_ref().is_some_and(Decimal::is_zero)
                && p.cost
                    .as_ref()
                    .is_some_and(|c| c.per_unit.is_some() && c.total.is_none())
        })
        .map(|p| p.line)
}

pub(super) fn book(
    books: &mut Books,
    date: Day,
    tx: &RawTransaction,
) -> Result<Booked, BookFailure> {
    let filled = infer(books, tx)?;
    // The reference does not list a posting of zero units whose cost was left to work out (such a
    // transaction is always rejected, as nothing balances the payment: inferred-cost-and-price.md).
    let empty_lots: Vec<usize> = tx
        .postings
        .iter()
        .enumerate()
        .filter(|(_, p)| open_cost(p) && p.units.as_ref().is_some_and(Decimal::is_zero))
        .map(|(i, _)| i)
        .collect();
    let tx = &filled;
    if tx.postings.iter().all(is_auto) {
        // Nothing to balance: a lone elided posting vanishes, two cannot be told apart.
        return match tx.postings.len() {
            0 | 1 => Ok(Booked {
                postings: Vec::new(),
                residuals: Vec::new(),
            }),
            _ => Err(BookFailure::OnPosting(
                "ambiguous-auto-posting",
                last_auto(&tx.postings),
            )),
        };
    }
    let mut feeds: Vec<Feed> = tx.postings.iter().map(feed_of).collect::<Result<_, _>>()?;
    // Which posting as written each fed posting comes from.
    let mut origins: Vec<usize> = (0..feeds.len()).collect();
    let mut rows = books.book(date, &feeds);
    if let Err(BookFailure::SeveralCurrencies(currencies)) = &rows {
        // The elided posting stands for one posting per currency still to balance.
        let mut currencies = currencies.clone();
        currencies.sort();
        let at = tx.postings.iter().position(is_auto);
        if let Some(at) = at {
            let mut expanded = Vec::new();
            let mut expanded_origins = Vec::new();
            for (i, feed) in feeds.iter().enumerate() {
                if i == at {
                    for currency in &currencies {
                        expanded.push(Feed {
                            currency: Some(currency.clone()),
                            ..feed.clone()
                        });
                        expanded_origins.push(i);
                    }
                } else {
                    expanded.push(feed.clone());
                    expanded_origins.push(i);
                }
            }
            feeds = expanded;
            origins = expanded_origins;
            rows = books.book(date, &feeds);
        }
    }
    // The crate counts the postings it was fed; the reference blames them as written.
    let rows = rows.map_err(|failure| match failure {
        BookFailure::OnPosting("ambiguous-auto-posting", _) => {
            BookFailure::OnPosting("ambiguous-auto-posting", last_auto(&tx.postings))
        }
        BookFailure::OnPosting(reason, fed) => {
            BookFailure::OnPosting(reason, origins.get(fed).copied().unwrap_or(fed))
        }
        other => other,
    })?;

    // Weights, by the currency they balance in.
    struct Slot {
        origin: usize,
        units: Decimal,
        currency: String,
        lot: Option<Lot>,
        price: Option<(Decimal, String)>,
        group: String,
        elided: bool,
    }
    let mut slots: Vec<Slot> = rows
        .into_iter()
        .map(|row| {
            let origin = origins[row.origin];
            let raw = &tx.postings[origin];
            let feed = &feeds[row.origin];
            let elided = raw.units.is_none() && raw.cost.is_none() && raw.price.is_none();
            let group = weight_currency(&row.currency, row.lot.as_ref(), feed.price.as_ref());
            Slot {
                origin,
                units: row.units,
                currency: row.currency,
                lot: row.lot,
                price: feed.price.clone(),
                group,
                elided,
            }
        })
        .collect();

    let weight = |s: &Slot| -> Decimal {
        match (&s.lot, &s.price) {
            (Some(lot), _) => s.units.mul(&lot.per_unit),
            (None, Some((price, _))) => s.units.mul(price),
            (None, None) => s.units.clone(),
        }
    };

    // Precision of each currency: the fewest places among the amounts written in it.
    let mut places: HashMap<&str, u32> = HashMap::new();
    for p in &tx.postings {
        if let (Some(units), Some(currency)) = (&p.units, &p.currency)
            && units.places() > 0
        {
            let entry = places.entry(currency.as_str()).or_insert(units.places());
            *entry = (*entry).min(units.places());
        }
    }

    let mut sums: HashMap<String, Decimal> = HashMap::new();
    for s in slots.iter().filter(|s| !s.elided) {
        let w = weight(s);
        let sum = sums.entry(s.group.clone()).or_insert_with(Decimal::zero);
        *sum = sum.add(&w);
    }
    for s in slots.iter_mut().filter(|s| s.elided) {
        let sum = sums.get(&s.group).cloned().unwrap_or_else(Decimal::zero);
        let value = sum.negated();
        s.units = match places.get(s.group.as_str()) {
            Some(&p) => value.quantized(p),
            None => value,
        };
        let entry = sums.entry(s.group.clone()).or_insert_with(Decimal::zero);
        *entry = entry.add(&s.units);
    }
    // An elided posting that comes out zero is not kept.
    slots.retain(|s| !(s.elided && s.units.is_zero()) && !empty_lots.contains(&s.origin));

    // Balance: every currency within half of its last written place; none written means exact.
    let mut residuals: Vec<(String, Decimal)> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for s in &slots {
        if !seen.contains(&s.group.as_str()) {
            seen.push(&s.group);
        }
    }
    for currency in seen {
        let residual = sums.get(currency).cloned().unwrap_or_else(Decimal::zero);
        let magnitude = residual.abs();
        let over = match places.get(currency) {
            // |residual| > 5 * 10^-(places + 1)
            Some(&p) => {
                let five = Decimal::new(5.into(), -i64::from(p + 1));
                magnitude.cmp_value(&five) == std::cmp::Ordering::Greater
            }
            None => !magnitude.is_zero(),
        };
        if over {
            residuals.push((currency.to_string(), residual));
        }
    }

    // Postings of one currency stay together, in the order the currencies first appear among the
    // postings that have an amount.
    let mut order: Vec<String> = Vec::new();
    for s in slots
        .iter()
        .filter(|s| !s.elided)
        .chain(slots.iter().filter(|s| s.elided))
    {
        if !order.contains(&s.group) {
            order.push(s.group.clone());
        }
    }
    let rank = |group: &str| order.iter().position(|g| g == group).unwrap_or(usize::MAX);
    slots.sort_by_key(|s| rank(&s.group));

    let postings = slots
        .into_iter()
        .map(|s| {
            let raw = &tx.postings[s.origin];
            BookedPosting {
                account: raw.account.clone(),
                flag: raw.flag,
                line: raw.line,
                units: s.units,
                currency: s.currency,
                lot: s.lot,
                price: s.price,
            }
        })
        .collect();
    Ok(Booked {
        postings,
        residuals,
    })
}
