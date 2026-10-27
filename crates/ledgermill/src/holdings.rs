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

/// A quantity of a holding read back to be added; always possible for what `holdings` prints.
fn read_back(text: &str) -> Result<Decimal, Finding> {
    Decimal::parse(text).ok_or_else(|| {
        unsupported_finding(
            "holdings-sum",
            "A quantity of a holding could not be read back to be added.",
            "",
            0,
        )
    })
}

/// One holding's cells added up over the ledgers of a set: the exact sum of the printed strings,
/// printed with the places of its most precise term (no rounding: each term was rounded by its
/// own ledger already). A sum that is exactly zero is `"0"` for the units and left out of a list;
/// a row left with zero units and no list is not a holding (`None`), as in either engine.
fn sum_rows(rows: &[Holding]) -> Result<Option<Holding>, Finding> {
    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let mut units = Decimal::zero();
    let mut lists: [Sums; 3] = Default::default();
    for row in rows {
        units = units.add(&read_back(&row.units)?);
        for (sums, list) in lists
            .iter_mut()
            .zip([&row.cost, &row.value, &row.cost_value])
        {
            for q in list {
                sums.add(&q.commodity, &read_back(&q.quantity)?);
            }
        }
    }
    let [cost, value, cost_value] = lists.map(|sums| {
        sums.0
            .into_iter()
            .filter(|(_, sum)| !sum.is_zero())
            .map(|(commodity, sum)| Quantity {
                commodity,
                quantity: sum.to_string(),
            })
            .collect::<Vec<_>>()
    });
    if units.is_zero() && cost.is_empty() && value.is_empty() && cost_value.is_empty() {
        return Ok(None);
    }
    Ok(Some(Holding {
        account: first.account.clone(),
        commodity: first.commodity.clone(),
        units: if units.is_zero() {
            "0".to_string()
        } else {
            units.to_string()
        },
        cost,
        value,
        cost_value,
    }))
}

impl Ledger {
    /// The holdings of a set of several ledgers: each ledger's rows, the rows of one account and
    /// commodity added up (see [`sum_rows`]), the others passed through as they are.
    fn holdings_of_parts(&self, query: &HoldingsQuery) -> Result<Vec<Holding>, Finding> {
        let mut groups: BTreeMap<(Vec<String>, String), Vec<Holding>> = BTreeMap::new();
        for part in &self.parts {
            for row in part.holdings(query)? {
                let segments = row.account.split(':').map(str::to_string).collect();
                groups
                    .entry((segments, row.commodity.clone()))
                    .or_default()
                    .push(row);
            }
        }
        let mut out = Vec::with_capacity(groups.len());
        for (_, mut rows) in groups {
            if rows.len() == 1 {
                out.extend(rows.pop());
            } else {
                out.extend(sum_rows(&rows)?);
            }
        }
        Ok(out)
    }

    /// The holdings of every account, per commodity, over the real postings of the ledger:
    /// for hledger, what `balance --layout=tidy --real` gives with `-e end`, with `--cost`,
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
    /// # Several ledgers
    ///
    /// A set of several ledgers (see [`load`](crate::load)) is asked per ledger, each with its own
    /// prices, and the rows of one account and commodity are added: the units exactly, and the
    /// `cost`, `value` and `cost_value` lists per commodity. The sum is the exact sum of the strings
    /// the ledgers print, printed with the places of its most precise term and never rounded again
    /// (each term was rounded by its own engine: ten places for hledger, 28 digits for Beancount). A
    /// units sum of exactly zero is `"0"`; a commodity whose sum is exactly zero is left out of a list,
    /// and a row left with zero units and no list is not listed, as in either engine. A row present in
    /// one ledger only is passed through with its strings unchanged. The order is as above, over the
    /// union of the rows. The error is the first ledger's, ledgers being ordered by the position of
    /// their first file.
    ///
    /// # Beancount
    ///
    /// The sums of the units and holdings queries Plainledger sends
    /// (`spec/beancount/holdings.md`), per account and posting commodity. `end` is exclusive here
    /// too: the postings of transactions dated before it, which is `WHERE date <= D` with `D` the
    /// day before `end` (padding transactions count, on their `pad`). The valuation date is **not**
    /// `end - 1`: `convert(position, V)` takes the latest price of each pair there is in the
    /// ledger, whatever `end` and `today` say, and `today` is not used. `units` is the plain
    /// decimal sum (`"0"` when it is gone); `cost`, `value` and `cost_value` are `sum(cost(position))`,
    /// `sum(convert(position, V))` and `sum(convert(cost(position), V))` (convert-positions.md: a
    /// position with a cost goes through its cost's currency), each position converted and rounded
    /// to 28 digits on its own, no further rounding. A sum that is exactly zero is dropped as soon
    /// as it is reached, so what follows starts again with its own places. A holding is listed when
    /// its units or one of the three lists is not empty. Ordering as for hledger.
    ///
    /// # Errors
    ///
    /// A finding with code `invalid-argument` (details `argument`: `end` or `today`) for a date that
    /// is not a real `YYYY-MM-DD`. A finding with code `unsupported-syntax` (details `construct`):
    /// `value-without-commodity` for an empty `query.commodity`, `cost-conversion-postings` for a
    /// transaction [`Ledger::at_cost`] refuses, and for a Beancount ledger the first
    /// `unsupported-syntax` finding of [`Ledger::check`], if any, or `price-both-ways` (a price of zero in a pair
    /// priced both ways).
    pub fn holdings(&self, query: &HoldingsQuery) -> Result<Vec<Holding>, Finding> {
        if !self.parts.is_empty() {
            return self.holdings_of_parts(query);
        }
        let end = match &query.end {
            None => None,
            Some(text) => Some(Date::from_iso(text).ok_or_else(|| invalid_argument("end", text))?),
        };
        let today =
            Date::from_iso(&query.today).ok_or_else(|| invalid_argument("today", &query.today))?;
        if let Some(refusal) = self.beancount_refusal() {
            return Err(refusal);
        }
        if query.commodity.is_empty() {
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
            // `WHERE date <= D` with D the day before `end`; no day before the first one.
            let through = match end {
                None => None,
                Some(end) => match end.previous_day() {
                    Some(day) => Some(day.to_string()),
                    None => return Ok(Vec::new()),
                },
            };
            return Ok(crate::beancount::holdings(
                &self.transactions,
                &self.price_list,
                &query.commodity,
                through.as_deref(),
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
