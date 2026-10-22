//! The `P` lines of a ledger and the graph of them that is in force on a date, with the path
//! of prices from one commodity to another. See `spec/hledger/value-prices.md`.

use crate::date::Date;
use crate::decimal::{DIVISION_PLACES, Decimal};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// One `P` line: on `date`, one unit of `from` was worth `rate` of `to`.
#[derive(Debug, Clone)]
pub(crate) struct Price {
    pub date: Date,
    pub from: String,
    pub to: String,
    pub rate: Decimal,
}

/// `commodity -> neighbour -> rate`; the neighbours come out sorted by name (code point order).
type Edges = BTreeMap<String, BTreeMap<String, Decimal>>;

/// The prices known on a date: for each ordered pair the latest line dated on or before it, the
/// one written last on a tie. Each pair is a forward edge, and also an inverse edge the other
/// way, with rate `1/price` to 255 places (0 for a price of zero).
pub(crate) struct Graph {
    forward: Edges,
    inverse: Edges,
}

impl Graph {
    /// `prices` are in parse order. A price of a commodity in itself is no edge.
    pub(crate) fn at(prices: &[Price], date: Date) -> Graph {
        let mut latest: BTreeMap<(&str, &str), (Date, &Decimal)> = BTreeMap::new();
        for p in prices.iter().filter(|p| p.date <= date && p.from != p.to) {
            let key = (p.from.as_str(), p.to.as_str());
            if latest.get(&key).is_none_or(|(d, _)| *d <= p.date) {
                latest.insert(key, (p.date, &p.rate));
            }
        }
        let mut graph = Graph {
            forward: Edges::new(),
            inverse: Edges::new(),
        };
        let one = Decimal::new(1u8.into(), 0);
        for ((from, to), (_, rate)) in latest {
            let back = one
                .divided(rate, DIVISION_PLACES)
                .unwrap_or_else(Decimal::zero);
            graph
                .forward
                .entry(from.to_string())
                .or_default()
                .insert(to.to_string(), rate.clone());
            graph
                .inverse
                .entry(to.to_string())
                .or_default()
                .insert(from.to_string(), back);
        }
        graph
    }

    /// The rate that turns one `from` into `to`, or `None` when no path of prices leads there.
    ///
    /// A path of forward edges only is looked for first, then one with inverse edges too; the
    /// shortest wins, and a tie goes to the first found by a breadth-first search that tries
    /// each commodity's forward neighbours (by name), then its inverse ones. The rate is folded
    /// from the start, rounded to 255 places at each product.
    pub(crate) fn rate(&self, from: &str, to: &str) -> Option<Decimal> {
        let path = self
            .path(from, to, false)
            .or_else(|| self.path(from, to, true))?;
        let mut rates = path.into_iter();
        let first = rates.next()?;
        Some(rates.fold(first.clone(), |rate, next| {
            rate.mul(next).rounded(DIVISION_PLACES)
        }))
    }

    /// The rates along the path, from `from` to `to`.
    fn path(&self, from: &str, to: &str, inverse: bool) -> Option<Vec<&Decimal>> {
        let mut seen: HashSet<&str> = HashSet::from([from]);
        let mut came_by: HashMap<&str, (&str, &Decimal)> = HashMap::new();
        let mut queue: VecDeque<&str> = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            let inverse_edges = inverse.then(|| self.inverse.get(at)).flatten();
            let edges = self.forward.get(at).into_iter().chain(inverse_edges);
            for (next, rate) in edges.flatten() {
                if !seen.insert(next.as_str()) {
                    continue;
                }
                came_by.insert(next.as_str(), (at, rate));
                if next == to {
                    let mut rates = Vec::new();
                    let mut here = to;
                    while let Some(&(before, rate)) = came_by.get(here) {
                        rates.push(rate);
                        here = before;
                    }
                    rates.reverse();
                    return Some(rates);
                }
                queue.push_back(next);
            }
        }
        None
    }
}
