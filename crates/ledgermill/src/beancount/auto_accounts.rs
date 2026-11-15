//! The `beancount.plugins.auto_accounts` plugin: an `open` for every account that is used and has
//! none (spec/beancount/auto-accounts.md).

use super::check::Kept;
use super::plain::{Day, Item};
use std::collections::{BTreeMap, HashSet};

/// The module name that turns the plugin on.
pub(super) const MODULE: &str = "beancount.plugins.auto_accounts";

/// Note a use of `account` on `date`, keeping the earliest.
fn used<'a>(first_use: &mut BTreeMap<&'a str, Day>, account: &'a str, date: Day) {
    first_use
        .entry(account)
        .and_modify(|first| *first = (*first).min(date))
        .or_insert(date);
}

/// The `open` directives the plugin adds, each on the day its account is first used. A use is a
/// posting of a transaction booking kept (one it rejected is not seen), either account of a `pad`,
/// and a `balance`, `note`, `document` or `close`. An account with an `open` anywhere in the files
/// is left alone, even if the `open` comes late: the error that follows is the reference's own.
pub(super) fn opens(files: &[&[Item]], kept: &[Kept]) -> Vec<Item> {
    let mut first_use: BTreeMap<&str, Day> = BTreeMap::new();
    let mut declared: HashSet<&str> = HashSet::new();
    for item in files.iter().flat_map(|items| items.iter()) {
        match item {
            Item::Open { account, .. } => {
                declared.insert(account);
            }
            Item::Pad {
                date,
                account,
                source,
                ..
            } => {
                used(&mut first_use, account, *date);
                used(&mut first_use, source, *date);
            }
            Item::Balance { date, account, .. }
            | Item::Note { date, account, .. }
            | Item::Document { date, account, .. }
            | Item::Close { date, account, .. } => used(&mut first_use, account, *date),
            Item::Commodity { .. }
            | Item::Price { .. }
            | Item::Transaction(_)
            | Item::Refused { .. } => {}
        }
    }
    for transaction in kept {
        for posting in &transaction.postings {
            used(&mut first_use, &posting.account, transaction.date);
        }
    }
    let mut added = Vec::new();
    for (account, date) in first_use {
        if declared.contains(account) {
            continue;
        }
        added.push(Item::Open {
            date,
            // The reference gives the entries it adds no line of the file.
            line: 0,
            account: account.to_owned(),
            currencies: vec![],
            method: None,
        });
    }
    added
}
