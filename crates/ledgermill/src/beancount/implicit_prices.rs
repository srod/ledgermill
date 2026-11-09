//! The `beancount.plugins.implicit_prices` plugin: a `price` entry for every price annotation and
//! every lot a posting opens (spec/beancount/implicit-prices.md).

use super::book::BookedPosting;
use super::plain::{Day, Item};
use super::pydec::Num;
use foldhash::HashSet;

/// The module name that turns the plugin on.
pub(super) const MODULE: &str = "beancount.plugins.implicit_prices";

/// What one entry is told apart by: its day, the priced commodity, the quote currency and the rate
/// as a value (`100.0` and `100.00` are one).
type Key = (Day, String, String, String);

/// The entries one run of the plugin adds, gathered a transaction at a time in the order of the
/// file.
#[derive(Default)]
pub(super) struct Run {
    seen: HashSet<Key>,
    entries: Vec<Item>,
}

/// The rate written so that equal values give equal text.
fn canonical(rate: &Num) -> String {
    let plain = rate.plain();
    let trimmed = if plain.contains('.') {
        plain.trim_end_matches('0').trim_end_matches('.')
    } else {
        plain.as_str()
    };
    if trimmed == "-0" { "0" } else { trimmed }.to_string()
}

impl Run {
    /// Add what the postings of a transaction that booking kept make: a price annotation gives its
    /// rate; failing that, a posting that opens a lot gives the cost per unit. A posting that
    /// reduces a lot gives nothing from its cost. An entry equal to one already made in this run
    /// (same day, commodities and value) is dropped, the first written staying as written.
    pub(super) fn transaction(&mut self, date: Day, line: u32, postings: &[BookedPosting]) {
        for posting in postings {
            let (rate, quote) = match (&posting.price, &posting.lot) {
                (Some((rate, quote)), _) => (rate, quote),
                (None, Some(lot)) if lot.opened => (&lot.per_unit, &lot.currency),
                _ => continue,
            };
            let key = (
                date,
                posting.currency.clone(),
                quote.clone(),
                canonical(rate),
            );
            if self.seen.insert(key) {
                self.entries.push(Item::Price {
                    date,
                    line,
                    currency: posting.currency.clone(),
                    number: rate.clone(),
                    quote: quote.clone(),
                });
            }
        }
    }

    /// The entries of `runs` runs of the plugin (one per `plugin` line, each making the same
    /// entries again), in the order they are listed in.
    pub(super) fn repeated(self, runs: usize) -> Vec<Item> {
        let mut all = Vec::with_capacity(self.entries.len() * runs);
        for _ in 0..runs {
            all.extend(self.entries.iter().cloned());
        }
        all
    }
}
