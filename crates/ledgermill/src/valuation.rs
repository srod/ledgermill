//! `Ledger::value`: every amount converted to one commodity at the prices of one date, as
//! `hledger print --value=DATE,COMM` does. See `spec/hledger/value-prices.md` and
//! `spec/hledger/value-arithmetic.md`.

use crate::Ledger;
use crate::date::Date;
use crate::decimal::{DIVISION_PLACES, Decimal, ZeroTie};
use crate::model::{Amount, Finding, Posting, Severity, Transaction};
use crate::parse::report_amount;
use crate::posting::{CostKind, WrittenAmount, WrittenCost};
use crate::prices::Graph;
use crate::problem::unsupported_finding;
use std::cmp::Ordering;
use std::collections::HashMap;

impl Ledger {
    /// A Beancount ledger lists nothing when a construct was refused, so a valuation of it would
    /// look complete and be empty: the first refusal is the answer instead.
    pub(crate) fn beancount_refusal(&self) -> Option<Finding> {
        if !self.beancount {
            return None;
        }
        self.findings
            .iter()
            .find(|f| f.code == "unsupported-syntax" && f.severity == Severity::Error)
            .cloned()
    }
}

/// One amount of a posting on its way to the output: the commodity it ends in and its value
/// at full precision.
pub(crate) struct Term<'a> {
    pub commodity: &'a str,
    pub value: Decimal,
    /// The amount itself when it was left as it is (kept with its cost and lot if nothing joins
    /// it).
    pub original: Option<&'a WrittenAmount>,
}

/// What the terms of one posting that are one amount to the reference tool add up to.
struct Merged<'a> {
    commodity: &'a str,
    /// The cost the amounts share; a total cost is the sum of the totals.
    cost: Option<WrittenCost>,
    sum: Decimal,
    /// How many terms went in.
    count: usize,
    original: Option<&'a WrittenAmount>,
}

/// Two amounts are one when they end in the same commodity and have no cost, or the same kind
/// of cost in the same commodity and, for a unit cost, the same value.
fn same_cost(a: Option<&WrittenCost>, b: Option<&WrittenCost>) -> bool {
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

/// The order amounts of one commodity are listed in: no cost first, then by the cost's
/// commodity, a total cost before a unit cost, then by its value.
fn cmp_cost(a: Option<&WrittenCost>, b: Option<&WrittenCost>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(x), Some(y)) => {
            let rank = |c: &WrittenCost| u8::from(c.kind == CostKind::Unit);
            x.commodity
                .cmp(&y.commodity)
                .then(rank(x).cmp(&rank(y)))
                .then_with(|| x.value.cmp_value(&y.value))
        }
    }
}

/// The amounts a posting reports once its amounts are converted: the terms that are one amount
/// (see [`same_cost`]) are summed in order (a zero gives the other term back with its places),
/// a sum is rounded to ten places only here, and the list is ordered by commodity, the empty one
/// first, then by cost. A term left as it is stays whole, with its cost and lot, when nothing
/// else joined it; a lot does not survive a sum.
pub(crate) fn merge<'a>(terms: impl IntoIterator<Item = Term<'a>>) -> Vec<Amount> {
    let mut merged: Vec<Merged> = Vec::new();
    for t in terms {
        let cost = t.original.and_then(|a| a.cost.as_ref());
        let same = |m: &Merged| m.commodity == t.commodity && same_cost(m.cost.as_ref(), cost);
        match merged.iter_mut().find(|m| same(m)) {
            Some(m) => {
                m.sum = m.sum.combine(&t.value, ZeroTie::KeepEarlier);
                if let (Some(held), Some(c)) = (m.cost.as_mut(), cost)
                    && held.kind == CostKind::Total
                {
                    held.value = held.value.combine(&c.value, ZeroTie::KeepEarlier);
                }
                m.count += 1;
                m.original = None;
            }
            None => merged.push(Merged {
                commodity: t.commodity,
                cost: cost.cloned(),
                sum: t.value,
                count: 1,
                original: t.original,
            }),
        }
    }
    merged.sort_by(|a, b| {
        a.commodity
            .cmp(b.commodity)
            .then_with(|| cmp_cost(a.cost.as_ref(), b.cost.as_ref()))
    });
    merged
        .into_iter()
        .map(|m| match m.original {
            Some(a) if m.count == 1 => report_amount(a),
            _ => report_amount(&WrittenAmount {
                commodity: m.commodity.to_string(),
                display_places: m.sum.places(),
                quantity: m.sum,
                cost: m.cost,
                lot: None,
            }),
        })
        .collect()
}

/// The amounts of a posting valued in `target`, merged per commodity.
///
/// Each amount is converted at full precision (no price, no lot); an amount in the target
/// commodity already, or with no path to it, is left as it is.
fn value_posting(
    amounts: &[WrittenAmount],
    target: &str,
    rates: &mut HashMap<String, Option<Decimal>>,
    graph: &Graph,
) -> Vec<Amount> {
    merge(amounts.iter().map(|a| {
        let rate = if a.commodity == target {
            None
        } else {
            rates
                .entry(a.commodity.clone())
                .or_insert_with(|| graph.rate(&a.commodity, target))
                .as_ref()
        };
        match rate {
            Some(rate) => Term {
                commodity: target,
                value: a.quantity.mul(rate).rounded(DIVISION_PLACES).normalized(),
                original: None,
            },
            None => Term {
                commodity: a.commodity.as_str(),
                value: a.quantity.clone(),
                original: Some(a),
            },
        }
    }))
}

impl Ledger {
    /// The transactions as `hledger print --value=DATE,COMMODITY` gives them: every amount of
    /// every posting converted to `commodity` at the prices (`P` lines) known on `date`, in the
    /// same shape and order as [`Ledger::transactions`]. The date of a transaction plays no
    /// part. `date` is `YYYY-MM-DD`; `commodity` is taken as given, a quoted name without its
    /// quotes.
    ///
    /// An amount already in `commodity`, or with no path of prices to it, is left as it is,
    /// cost and lot kept. Any other becomes an amount in `commodity` with neither: the product
    /// of its quantity and the rates along the path, at full precision, rounded half to even to
    /// ten places only when it has more (the ten places are kept). The amounts of a posting that
    /// end in the same commodity (and have the same cost, if they kept one) are added first,
    /// then rounded. The valued transaction need not balance.
    ///
    /// As [`Ledger::transactions`], a ledger with error findings answers for the transactions it
    /// lists.
    ///
    /// # Errors
    ///
    /// A finding with code `invalid-argument` when `date` is not a real `YYYY-MM-DD` date. A
    /// finding with code `unsupported-syntax` (details `construct`): `value-without-commodity`
    /// for an empty `commodity` (hledger reads `--value=DATE,` as another mode that the spec
    /// does not cover), and for a Beancount ledger the first `unsupported-syntax` finding of
    /// [`Ledger::check`], if any; also `price-both-ways` for a Beancount ledger with a price of zero in a
    /// pair priced both ways (spec/beancount/convert-rates.md).
    ///
    /// Beancount (`convert(position, commodity, date)`: spec/beancount/convert-rates.md and
    /// `convert-positions.md`): rates come from the `price` directives only, never from `@`, `@@` or a
    /// cost; the latest entry of the pair, direct or inverse (`1 / price`), dated on or before `date`
    /// (the day itself counts); no chain of pairs. A position with a cost that has no rate of its own
    /// goes through its cost's currency when both rates exist. A converted amount has neither cost nor
    /// lot; its quantity prints as `number()` does (exponent included, `-0` for a negative zero). A
    /// position already in `commodity` or with no rate stays whole.
    pub fn value(&self, date: &str, commodity: &str) -> Result<Vec<Transaction>, Finding> {
        let Some(date) = Date::from_iso(date) else {
            return Err(Finding {
                code: "invalid-argument".into(),
                severity: Severity::Error,
                file: String::new(),
                line: 0,
                column: None,
                details: serde_json::json!({ "argument": "date", "value": date }),
                message: "The date is not a YYYY-MM-DD date.".into(),
            });
        };
        if let Some(refusal) = self.beancount_refusal() {
            return Err(refusal);
        }
        if commodity.is_empty() {
            return Err(unsupported_finding(
                "value-without-commodity",
                "Valuing in no commodity is not supported.",
                "",
                0,
            ));
        }
        if self.beancount {
            if let Some(refusal) = crate::beancount::both_ways_refusal(&self.price_list) {
                return Err(refusal);
            }
            return Ok(crate::beancount::value(
                &self.transactions,
                &self.price_list,
                commodity,
                &date.to_string(),
            ));
        }
        let graph = Graph::at(&self.prices, date);
        let mut rates = HashMap::new();
        Ok(self
            .transactions
            .iter()
            .zip(&self.exact)
            .map(|(t, exact)| Transaction {
                postings: t
                    .postings
                    .iter()
                    .zip(&exact.postings)
                    .map(|(p, e)| Posting {
                        amounts: value_posting(&e.amounts, commodity, &mut rates, &graph),
                        ..p.clone()
                    })
                    .collect(),
                ..t.clone()
            })
            .collect())
    }
}
