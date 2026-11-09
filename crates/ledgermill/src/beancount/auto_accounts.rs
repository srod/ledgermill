//! The `beancount.plugins.auto_accounts` plugin: every account that is named but never opened gets
//! an `open` on the day of its first use (spec/beancount/auto-accounts.md).

use super::check::Kept;
use super::plain::{Day, Item};
use foldhash::{HashMap, HashSet};

/// The module name that turns the plugin on.
pub(super) const MODULE: &str = "beancount.plugins.auto_accounts";

fn record_use<'a>(first_use: &mut HashMap<&'a str, Day>, account: &'a str, date: Day) {
    first_use
        .entry(account)
        .and_modify(|first| *first = (*first).min(date))
        .or_insert(date);
}

/// The `open` directives the plugin adds, by date and then account name.
///
/// An account is used by a posting of a transaction booking kept, by either account of a `pad`, and
/// by a `balance`, `note`, `document` or `close`. `price`, `event`, `custom` and metadata values name
/// no account for it. An account with an `open` anywhere in the file is left alone, whatever the
/// date of that `open`: the errors for a use before it stay.
pub(super) fn opens(items: &[Item], kept: &[Kept]) -> Vec<Item> {
    let mut opened: HashSet<&str> = HashSet::default();
    let mut first_use: HashMap<&str, Day> = HashMap::default();
    let mut touch = |account, date| record_use(&mut first_use, account, date);
    for item in items {
        match item {
            Item::Open { account, .. } => {
                opened.insert(account);
            }
            Item::Close { account, date, .. }
            | Item::Note { account, date, .. }
            | Item::Document { account, date, .. }
            | Item::Balance { account, date, .. } => touch(account, *date),
            Item::Pad {
                account,
                source,
                date,
                ..
            } => {
                touch(account, *date);
                touch(source, *date);
            }
            // A transaction counts through `kept`: one rejected by booking is not seen by the plugin.
            Item::Commodity { .. }
            | Item::Price { .. }
            | Item::Transaction(_)
            | Item::Refused { .. } => {}
        }
    }
    for transaction in kept {
        for posting in &transaction.postings {
            touch(&posting.account, transaction.date);
        }
    }
    let mut added: Vec<(Day, &str)> = first_use
        .into_iter()
        .filter(|(account, _)| !opened.contains(account))
        .map(|(account, date)| (date, account))
        .collect();
    added.sort_unstable();
    added
        .into_iter()
        .map(|(date, account)| Item::Open {
            date,
            line: 0,
            account: account.to_string(),
            currencies: Vec::new(),
            method: None,
        })
        .collect()
}
