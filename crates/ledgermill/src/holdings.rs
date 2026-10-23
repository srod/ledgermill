//! `Ledger::holdings`: per account and commodity, the units, the cost, the value and the cost
//! value the four `hledger balance --layout=tidy --real` runs of a holdings page give. See
//! `spec/hledger/holdings.md`.

use crate::Ledger;
use crate::cost::at_cost_term;
use crate::date::Date;
use crate::decimal::{DIVISION_PLACES, Decimal};
use crate::model::{Finding, Holding, HoldingsQuery, PostingKind, Quantity, Severity};
use crate::posting::WrittenAmount;
use crate::prices::Graph;
use crate::problem::unsupported_finding;
use std::collections::{BTreeMap, HashMap};

/// What one `(account, commodity)` has gathered.
#[derive(Default)]
struct Position<'a> {
    units: Decimal,
    /// The amounts that count for the cost: those of the postings `--cost` keeps.
    costed: Vec<&'a WrittenAmount>,
}

/// Exact sums per commodity.
#[derive(Default)]
struct Sums(BTreeMap<String, Decimal>);

impl Sums {
    fn add(&mut self, commodity: &str, value: &Decimal) {
        let entry = self.0.entry(commodity.to_string()).or_default();
        *entry = entry.add(value);
    }

    fn any_held(&self) -> bool {
        self.0.values().any(|sum| !sum.is_zero())
    }

    /// The quantities in code point order of commodity, each rounded to ten places once; a sum
    /// that is exactly zero is left out, one that only rounds to zero stays.
    fn quantities(self) -> Vec<Quantity> {
        self.0
            .into_iter()
            .filter(|(_, sum)| !sum.is_zero())
            .map(|(commodity, sum)| Quantity {
                commodity,
                quantity: sum.output(),
            })
            .collect()
    }
}

fn invalid_argument(argument: &str, value: &str) -> Finding {
    Finding {
        code: "invalid-argument".into(),
        severity: Severity::Error,
        file: String::new(),
        line: 0,
        column: None,
        details: serde_json::json!({ "argument": argument, "value": value }),
        message: format!("The {argument} is not a YYYY-MM-DD date."),
    }
}

/// The amounts valued in `target`: one in `target`, or with no path of prices to it, stays; the
/// others are multiplied by the rate of the path at full precision. Amounts that end in the same
/// commodity are added.
fn valued<'a>(
    amounts: impl IntoIterator<Item = (&'a str, &'a Decimal)>,
    target: &str,
    graph: &Graph,
    rates: &mut HashMap<String, Option<Decimal>>,
) -> Vec<Quantity> {
    let mut sums = Sums::default();
    for (commodity, quantity) in amounts {
        let rate = if commodity == target {
            None
        } else {
            rates
                .entry(commodity.to_string())
                .or_insert_with(|| graph.rate(commodity, target))
                .as_ref()
        };
        match rate {
            Some(rate) => sums.add(
                target,
                &quantity.mul(rate).rounded(DIVISION_PLACES).normalized(),
            ),
            None => sums.add(commodity, quantity),
        }
    }
    sums.quantities()
}

impl Ledger {
    /// The holdings of every account, per commodity, over the real postings of the ledger
    /// (hledger only): what `balance --layout=tidy --real` gives with `-e end`, with `--cost`,
    /// `--value=end,V` and both, for each commodity, as `spec/hledger/holdings.md` says.
    ///
    /// A posting counts when it is real (`(a)` and `[a]` are ignored everywhere) and its own date
    /// (its `date:` tag, else its transaction's) is before `query.end`; with no `end`, all count.
    /// Values are taken at the day before `end`; with no `end`, at the latest date of the whole
    /// ledger: every transaction, every posting's own date (virtual ones too) and every `P` line
    /// not after `query.today`.
    ///
    /// Per `(account, commodity C)`: `units` is the sum of the quantities of C; `cost` converts each
    /// amount of C that has a cost to its cost, drops the conversion postings (see
    /// [`Ledger::at_cost`]) and sums per commodity; `value` values the units; `cost_value` values
    /// each commodity of the cost. A commodity with no path of prices to V stays as it is, as does V
    /// itself. Every sum is rounded to ten places, once, at the end (`units` too); a sum that is
    /// exactly zero is left out.
    ///
    /// A holding is listed when its units or one of its costs is not zero: an account whose units
    /// of C cancel can still have a cost (10 X at 2 EUR and -10 X at 3 EUR), as hledger's cost run
    /// has a row then; its `units` is `"0"`. Ordered by the account's `:`-separated names in code
    /// point order, then by commodity in code point order, `""` first. When no posting counts,
    /// there are no holdings.
    ///
    /// Filtering by account prefix, symbol aliases and the missing-cost rule are left to the caller.
    ///
    /// # Errors
    ///
    /// A finding with code `invalid-argument` (details `argument`: `end` or `today`) for a date that
    /// is not a real `YYYY-MM-DD`. A finding with code `unsupported-syntax` (details `construct`):
    /// `value-without-commodity` for an empty `query.commodity`, `beancount-valuation` for a
    /// Beancount ledger, and `cost-conversion-postings` for a transaction [`Ledger::at_cost`]
    /// refuses.
    pub fn holdings(&self, query: &HoldingsQuery) -> Result<Vec<Holding>, Finding> {
        let end = match &query.end {
            None => None,
            Some(text) => Some(Date::from_iso(text).ok_or_else(|| invalid_argument("end", text))?),
        };
        let today =
            Date::from_iso(&query.today).ok_or_else(|| invalid_argument("today", &query.today))?;
        if self.beancount {
            return Err(crate::valuation::beancount_refusal());
        }
        if query.commodity.is_empty() {
            return Err(unsupported_finding(
                "value-without-commodity",
                "Valuing in no commodity is not supported.",
                "",
                0,
            ));
        }
        if let Some((t, _)) = self
            .transactions
            .iter()
            .zip(&self.exact)
            .find(|(_, exact)| exact.cost_refused)
        {
            return Err(unsupported_finding(
                "cost-conversion-postings",
                "Which postings --cost drops is not known for conversion postings with several priced postings or an assignment.",
                &t.source.file,
                t.source.line,
            ));
        }
        let mut positions: BTreeMap<(Vec<&str>, &str), Position> = BTreeMap::new();
        for (t, exact) in self.transactions.iter().zip(&self.exact) {
            for (p, e) in t.postings.iter().zip(&exact.postings) {
                if p.kind != PostingKind::Regular || end.is_some_and(|end| e.date >= end) {
                    continue;
                }
                let dropped = e.conversion_tagged && e.amounts.iter().all(|a| a.cost.is_none());
                for a in &e.amounts {
                    let at = positions
                        .entry((p.account.split(':').collect(), a.commodity.as_str()))
                        .or_default();
                    at.units = at.units.add(&a.quantity);
                    if !dropped {
                        at.costed.push(a);
                    }
                }
            }
        }
        let Some(valuation_date) = self.valuation_date(end, today) else {
            return Ok(Vec::new());
        };
        let graph = Graph::at(&self.prices, valuation_date);
        let mut rates = HashMap::new();
        let mut out = Vec::new();
        for ((segments, commodity), position) in positions {
            let mut cost = Sums::default();
            for a in position.costed {
                let term = at_cost_term(a);
                cost.add(term.commodity, &term.value);
            }
            if position.units.is_zero() && !cost.any_held() {
                continue;
            }
            let value = valued(
                [(commodity, &position.units)],
                &query.commodity,
                &graph,
                &mut rates,
            );
            let cost_value = valued(
                cost.0.iter().map(|(c, sum)| (c.as_str(), sum)),
                &query.commodity,
                &graph,
                &mut rates,
            );
            out.push(Holding {
                account: segments.join(":"),
                commodity: commodity.to_string(),
                units: position.units.output(),
                cost: cost.quantities(),
                value,
                cost_value,
            });
        }
        Ok(out)
    }

    /// The date values are taken at (see [`Ledger::holdings`]); `None` when the ledger has no
    /// date to take it from, or `end` is the first day there is.
    fn valuation_date(&self, end: Option<Date>, today: Date) -> Option<Date> {
        if let Some(end) = end {
            return end.previous_day();
        }
        let dates = self
            .exact
            .iter()
            .flat_map(|e| std::iter::once(e.date).chain(e.postings.iter().map(|p| p.date)));
        let prices = self.prices.iter().map(|p| p.date).filter(|&d| d <= today);
        dates.chain(prices).max()
    }
}
