//! `Ledger::value`: every amount converted to one commodity at the prices of one date, as
//! `hledger print --value=DATE,COMM` does. See `spec/hledger/value-prices.md` and
//! `spec/hledger/value-arithmetic.md`.

use crate::Ledger;
use crate::date::Date;
use crate::decimal::{DIVISION_PLACES, Decimal, OrdDecimal, ZeroTie};
use crate::model::{Amount, Finding, Posting, Severity, Transaction};
use crate::parse::report_amount;
use crate::posting::{CostKind, WrittenAmount, WrittenCost};
use crate::prices::Graph;
use crate::problem::unsupported_finding;
use std::collections::{BTreeMap, HashMap};

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
    /// Present only when a single untouched term made this entry (a second term clears it).
    original: Option<&'a WrittenAmount>,
}

/// Canonical sort/lookup key matching hledger's order: no cost first, then by cost commodity,
/// total cost before unit cost, then by unit cost value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CostKey {
    None,
    Cost {
        commodity: String,
        is_unit: bool,
        unit_value: OrdDecimal,
    },
}

impl CostKey {
    fn of(cost: Option<&WrittenCost>) -> CostKey {
        match cost {
            None => CostKey::None,
            Some(c) => CostKey::Cost {
                commodity: c.commodity.clone(),
                is_unit: c.kind == CostKind::Unit,
                unit_value: OrdDecimal(if c.kind == CostKind::Unit {
                    c.value.clone()
                } else {
                    Decimal::zero()
                }),
            },
        }
    }
}

/// The amounts a posting reports once its amounts are converted: the terms that are one amount
/// are summed in order (a zero gives the other term back with its places), a sum is rounded to ten
/// places only here, and the list is ordered by commodity, the empty one first, then by cost. A
/// term left as it is stays whole, with its cost and lot, when nothing else joined it; a lot does
/// not survive a sum.
pub(crate) fn merge<'a>(terms: impl IntoIterator<Item = Term<'a>>) -> Vec<Amount> {
    let mut merged: BTreeMap<(&'a str, CostKey), Merged<'a>> = BTreeMap::new();
    for t in terms {
        let cost = t.original.and_then(|a| a.cost.as_ref());
        let key = (t.commodity, CostKey::of(cost));
        match merged.get_mut(&key) {
            Some(m) => {
                m.sum = m.sum.combine(&t.value, ZeroTie::KeepEarlier);
                if let (Some(held), Some(c)) = (m.cost.as_mut(), cost)
                    && held.kind == CostKind::Total
                {
                    held.value = held.value.combine(&c.value, ZeroTie::KeepEarlier);
                }
                m.original = None;
            }
            None => {
                merged.insert(
                    key,
                    Merged {
                        commodity: t.commodity,
                        cost: cost.cloned(),
                        sum: t.value,
                        original: t.original,
                    },
                );
            }
        }
    }
    merged
        .into_values()
        .map(|m| match m.original {
            Some(a) => report_amount(a),
            None => report_amount(&WrittenAmount {
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
) -> Result<Vec<Amount>, Finding> {
    let mut terms = Vec::with_capacity(amounts.len());
    for a in amounts {
        let rate = if a.commodity == target {
            None
        } else {
            graph.cached_rate(rates, &a.commodity, target)?
        };
        terms.push(match rate {
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
        });
    }
    Ok(merge(terms))
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
    /// does not cover), `number-too-large` for a chain of prices whose product passes 65,536
    /// bits, and for a Beancount ledger the first `unsupported-syntax` finding of
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
    ///
    /// A set of several ledgers (see [`load`](crate::load)) values each ledger on its own, at its own
    /// prices, and puts the results in the order of [`Ledger::transactions`], so result `i` is for
    /// transaction `i`. The error is the first ledger's, ledgers being ordered by the position of
    /// their first file.
    pub fn value(&self, date: &str, commodity: &str) -> Result<Vec<Transaction>, Finding> {
        if !self.parts.is_empty() {
            return self.ask_parts(|part| part.value(date, commodity));
        }
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
        let mut out = Vec::with_capacity(self.transactions.len());
        for (t, exact) in self.transactions.iter().zip(&self.exact) {
            let mut postings = Vec::with_capacity(t.postings.len());
            for (p, e) in t.postings.iter().zip(&exact.postings) {
                postings.push(Posting {
                    amounts: value_posting(&e.amounts, commodity, &mut rates, &graph)?,
                    ..p.clone()
                });
            }
            out.push(Transaction {
                postings,
                ..t.clone()
            });
        }
        Ok(out)
    }
}
