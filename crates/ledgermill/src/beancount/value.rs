//! Beancount valuation: what `convert(position, V [, date])`, `cost(position)` and the sums of the
//! holdings query give. See `spec/beancount/convert-rates.md`, `convert-positions.md`,
//! `cost-of-position.md` and `holdings.md`.
//!
//! Everything is read back from the typed values the ledger already lists (`transactions()` and
//! `prices()`): their numbers are `Num`'s own output, so they parse back exactly.

use super::pydec::Num;
use crate::model::{Amount, Holding, Posting, PriceDirective, Quantity, Transaction};
use num_bigint::BigInt;
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

/// The rates the `price` directives make: for each ordered pair, its entries in the order of the
/// prices table (spec/beancount/convert-rates.md, "A pair seen both ways").
///
/// Each pair starts with its own prices. Then, one pair after another, the inverses (`1 / price`,
/// none for a price of zero) of what a pair holds *at that moment* are appended to the list of the
/// opposite pair; so a pair whose opposite was handled first holds the inverses of inverses of its
/// own prices as well, and for one date the entry appended last is the one that counts. The pairs
/// are handled shortest list first, and the pair that appeared last first among equals.
struct Rates {
    pairs: HashMap<String, HashMap<String, Entries>>,
}

/// The `(date, rate)` entries of one pair, `YYYY-MM-DD` dates.
type Entries = Vec<(String, Num)>;

impl Rates {
    fn new(prices: &[PriceDirective]) -> Rates {
        let one = Num::from_scaled(BigInt::from(1u8), 0);
        let mut index: HashMap<(String, String), usize> = HashMap::new();
        let mut lists: Vec<((String, String), Entries)> = Vec::new();
        let mut at = |lists: &mut Vec<_>, key: (String, String)| -> usize {
            *index.entry(key.clone()).or_insert_with(|| {
                lists.push((key, Vec::new()));
                lists.len() - 1
            })
        };
        for p in prices {
            let Some(number) = Num::parse(&p.rate.quantity) else {
                continue;
            };
            let i = at(&mut lists, (p.commodity.clone(), p.rate.commodity.clone()));
            if let Some((_, list)) = lists.get_mut(i) {
                list.push((p.date.clone(), number));
            }
        }
        let mut order: Vec<usize> = (0..lists.len()).collect();
        order.sort_by_key(|&i| (lists.get(i).map_or(0, |l| l.1.len()), Reverse(i)));
        for i in order {
            let Some(((from, to), list)) = lists.get(i) else {
                continue;
            };
            let (opposite, inverses): (_, Entries) = (
                (to.clone(), from.clone()),
                list.iter()
                    .filter_map(|(date, rate)| one.div(rate).map(|back| (date.clone(), back)))
                    .collect(),
            );
            let j = at(&mut lists, opposite);
            if let Some((_, other)) = lists.get_mut(j) {
                other.extend(inverses);
            }
        }
        let mut pairs: HashMap<String, HashMap<String, Entries>> = HashMap::new();
        for ((from, to), mut list) in lists {
            list.sort_by(|a, b| a.0.cmp(&b.0));
            pairs.entry(from).or_default().insert(to, list);
        }
        Rates { pairs }
    }

    /// The rate from `from` to `to`: the last entry dated on or before `on`, or the last there is.
    /// No chain of pairs.
    fn rate(&self, from: &str, to: &str, on: Option<&str>) -> Option<&Num> {
        let entries = self.pairs.get(from)?.get(to)?;
        entries
            .iter()
            .rev()
            .find(|(date, _)| on.is_none_or(|on| date.as_str() <= on))
            .map(|(_, rate)| rate)
    }
}

/// A posting's amount, read back: its units, and its cost per unit with the cost's currency.
struct Leg<'a> {
    units: Num,
    commodity: &'a str,
    cost: Option<(Num, &'a str)>,
}

/// `None` for a number that does not read back, which the ledger's own output never is.
fn leg(posting: &Posting) -> Option<Leg<'_>> {
    let amount = posting.amounts.first()?;
    let cost = match amount.lot.as_ref().and_then(|lot| lot.cost.as_ref()) {
        Some(cost) => Some((Num::parse(&cost.quantity)?, cost.commodity.as_str())),
        None => None,
    };
    Some(Leg {
        units: Num::parse(&amount.quantity)?,
        commodity: &amount.commodity,
        cost,
    })
}

/// An amount worked out. Decimal keeps the sign of a zero product, and `number()` prints it.
struct Value {
    commodity: String,
    number: Num,
    negative: bool,
}

impl Value {
    fn new(commodity: &str, number: Num, negative: bool) -> Value {
        Value {
            commodity: commodity.to_string(),
            number,
            negative,
        }
    }

    /// `number()` of the amount: `-0` for a negative zero.
    fn text(&self) -> String {
        if self.number.is_zero() && self.negative {
            format!("-{}", self.number)
        } else {
            self.number.to_string()
        }
    }

    fn times(&self, rate: &Num) -> Value {
        Value {
            commodity: self.commodity.clone(),
            number: self.number.mul(rate),
            negative: self.negative != rate.is_negative(),
        }
    }
}

impl Leg<'_> {
    fn units(&self) -> Value {
        Value::new(self.commodity, self.units.clone(), self.units.is_negative())
    }

    /// `cost(position)`: the units times the cost per unit, in the cost's currency; the units
    /// without a cost.
    fn cost(&self) -> Value {
        match &self.cost {
            Some((per_unit, currency)) => {
                let units = self.units();
                Value {
                    commodity: (*currency).to_string(),
                    ..units.times(per_unit)
                }
            }
            None => self.units(),
        }
    }
}

/// `convert(amount, V)`: a rate to `V`, or nothing (`None`: the amount stays as it is).
fn convert_amount(amount: &Value, target: &str, rates: &Rates, on: Option<&str>) -> Option<Value> {
    if amount.commodity == target {
        return None;
    }
    let rate = rates.rate(&amount.commodity, target, on)?;
    Some(Value {
        commodity: target.to_string(),
        ..amount.times(rate)
    })
}

/// `convert(position, V)`: the units' rate to `V`; else, for a position with a cost, the units
/// in the cost's currency and then that amount in `V` (two products, each rounded); else `None`.
fn convert_position(leg: &Leg, target: &str, rates: &Rates, on: Option<&str>) -> Option<Value> {
    let units = leg.units();
    if let Some(value) = convert_amount(&units, target, rates, on) {
        return Some(value);
    }
    if leg.commodity == target {
        return None;
    }
    let (_, through) = leg.cost.as_ref()?;
    let first = rates.rate(leg.commodity, through, on)?;
    let second = rates.rate(through, target, on)?;
    let in_cost_currency = units.times(first);
    Some(Value {
        commodity: target.to_string(),
        ..in_cost_currency.times(second)
    })
}

fn with_amount(posting: &Posting, value: &Value) -> Posting {
    Posting {
        amounts: vec![Amount {
            commodity: value.commodity.clone(),
            quantity: value.text(),
            cost: None,
            lot: None,
        }],
        ..posting.clone()
    }
}

/// The transactions with every position as `convert(position, target, on)` gives it; a position
/// that stays is left whole, cost and lot kept.
pub(crate) fn value(
    transactions: &[Transaction],
    prices: &[PriceDirective],
    target: &str,
    on: &str,
) -> Vec<Transaction> {
    let rates = Rates::new(prices);
    map_postings(transactions, |posting| {
        let leg = leg(posting)?;
        convert_position(&leg, target, &rates, Some(on)).map(|value| with_amount(posting, &value))
    })
}

/// The transactions with every posting that has a cost as `cost(position)` gives it.
pub(crate) fn at_cost(transactions: &[Transaction]) -> Vec<Transaction> {
    map_postings(transactions, |posting| {
        let leg = leg(posting)?;
        leg.cost.as_ref()?;
        Some(with_amount(posting, &leg.cost()))
    })
}

/// Each posting replaced by what `change` makes of it, or kept when it says `None`.
fn map_postings(
    transactions: &[Transaction],
    change: impl Fn(&Posting) -> Option<Posting>,
) -> Vec<Transaction> {
    transactions
        .iter()
        .map(|t| Transaction {
            postings: t
                .postings
                .iter()
                .map(|p| change(p).unwrap_or_else(|| p.clone()))
                .collect(),
            ..t.clone()
        })
        .collect()
}

/// A cell of the holdings query: one sum per commodity, dropped as soon as it is exactly zero,
/// so what follows starts again with its own places.
#[derive(Default)]
struct Cell(BTreeMap<String, Num>);

impl Cell {
    fn add(&mut self, commodity: &str, number: &Num) {
        let sum = match self.0.get(commodity) {
            Some(held) => held.add(number),
            None => number.clone(),
        };
        if sum.is_zero() {
            self.0.remove(commodity);
        } else {
            self.0.insert(commodity.to_string(), sum);
        }
    }

    fn quantities(&self) -> Vec<Quantity> {
        self.0
            .iter()
            .map(|(commodity, sum)| Quantity {
                commodity: commodity.clone(),
                quantity: sum.plain(),
            })
            .collect()
    }
}

/// What one `(account, commodity)` group of the holdings query sums.
#[derive(Default)]
struct Group {
    units: Cell,
    cost: Cell,
    value: Cell,
    cost_value: Cell,
}

/// The holdings of every account and commodity over the postings of the transactions dated up to
/// and including `through` (all when `None`), valued in `target` at the latest prices there are.
pub(crate) fn holdings(
    transactions: &[Transaction],
    prices: &[PriceDirective],
    target: &str,
    through: Option<&str>,
) -> Vec<Holding> {
    let rates = Rates::new(prices);
    let mut groups: BTreeMap<(Vec<&str>, &str), Group> = BTreeMap::new();
    let counted = transactions
        .iter()
        .filter(|t| through.is_none_or(|through| t.date.as_str() <= through));
    for t in counted {
        for posting in &t.postings {
            let Some(leg) = leg(posting) else {
                continue;
            };
            let group = groups
                .entry((posting.account.split(':').collect(), leg.commodity))
                .or_default();
            group.units.add(leg.commodity, &leg.units);
            let cost = leg.cost();
            group.cost.add(&cost.commodity, &cost.number);
            match convert_amount(&cost, target, &rates, None) {
                Some(v) => group.cost_value.add(&v.commodity, &v.number),
                None => group.cost_value.add(&cost.commodity, &cost.number),
            }
            match convert_position(&leg, target, &rates, None) {
                Some(v) => group.value.add(&v.commodity, &v.number),
                None => group.value.add(leg.commodity, &leg.units),
            }
        }
    }
    groups
        .into_iter()
        .filter(|(_, g)| {
            [&g.units, &g.cost, &g.value, &g.cost_value]
                .iter()
                .any(|cell| !cell.0.is_empty())
        })
        .map(|((segments, commodity), g)| Holding {
            account: segments.join(":"),
            commodity: commodity.to_string(),
            units: g
                .units
                .0
                .get(commodity)
                .map_or_else(|| "0".to_string(), Num::plain),
            cost: g.cost.quantities(),
            value: g.value.quantities(),
            cost_value: g.cost_value.quantities(),
        })
        .collect()
}
