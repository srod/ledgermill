//! Booking one transaction the way the reference tool does: per-unit prices and costs, the
//! amount of an elided posting, the balance check, and the order of the postings.
//! See spec/beancount/numbers-and-interpolation.md and spec/beancount/lots-and-booking.md.
//!
//! The lot matching itself is the booking crate's (through `lima.rs`); the figures are ours.

use super::lima::{Books, Feed, FeedCost};
use super::plain::{BookFailure, Day, Lot, RawPosting, RawTransaction};
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
        Some(price) if !price.total => Some((price.number.clone(), price.currency.clone())),
        Some(price) => {
            let units = p.units.as_ref().ok_or(BookFailure::TooPrecise)?;
            let value = per_unit(&price.number, units).ok_or(BookFailure::TooPrecise)?;
            Some((value, price.currency.clone()))
        }
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

pub(super) fn book(
    books: &mut Books,
    date: Day,
    tx: &RawTransaction,
) -> Result<Booked, BookFailure> {
    if tx.postings.iter().all(is_auto) {
        // Nothing to balance: a lone elided posting vanishes, two cannot be told apart.
        return match tx.postings.len() {
            0 | 1 => Ok(Booked {
                postings: Vec::new(),
                residuals: Vec::new(),
            }),
            _ => Err(BookFailure::Booking("ambiguous-auto-posting")),
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
    let rows = rows?;

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
    slots.retain(|s| !(s.elided && s.units.is_zero()));

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
