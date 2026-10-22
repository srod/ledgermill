//! Expected values are what `bean-query` 0.2.0 (Beancount 3.2.3) reports for the probe journals
//! (see spec/beancount/postings.md); they are written out by hand, not computed.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::file;

fn probe(name: &str) -> String {
    std::fs::read_to_string(format!("../../spec/beancount/probes/{name}.beancount")).unwrap()
}

/// `[date, flag, payee, narration, account, number, currency, line]` per row.
fn rows(name: &str, content: &str) -> Vec<Value> {
    let ledger = load(&[file(name, content)]);
    assert_eq!(ledger.check().len(), 0, "{name}");
    ledger
        .postings()
        .iter()
        .map(|p| {
            assert_eq!(p.file, name);
            json!([
                p.date,
                p.flag,
                p.payee,
                p.narration,
                p.account,
                p.number,
                p.currency,
                p.line
            ])
        })
        .collect()
}

#[test]
fn a_posting_split_over_lots_gives_one_row_per_lot_in_booking_order() {
    let rows = rows("b.beancount", &probe("postings-lot-ties"));
    let sales: Vec<(&str, i64, i64)> = rows
        .iter()
        .filter(|r| r[0] == "2026-01-10")
        .map(|r| {
            let number: i64 = r[5].as_str().unwrap().parse().unwrap();
            (r[4].as_str().unwrap(), number, r[7].as_i64().unwrap())
        })
        .collect();
    assert_eq!(
        sales,
        [
            ("Assets:Lifo", -3, 24),
            ("Assets:Lifo", -2, 24),
            ("Assets:Lifo", -1, 24),
            ("Assets:Hifo", -1, 25),
            ("Assets:Hifo", -3, 25),
            ("Assets:Hifo", -2, 25),
            ("Assets:Fifo", -1, 26),
            ("Assets:Fifo", -2, 26),
            ("Assets:Fifo", -3, 26),
            ("Assets:Bank", 330, 27),
        ]
    );
}

#[test]
fn rows_follow_date_then_line_and_carry_the_transaction_header() {
    let rows = rows("b.beancount", &probe("postings-order"));
    let wanted = |line: i64| rows.iter().find(|r| r[7] == line).unwrap().clone();
    // Written last, dated earliest: first.
    assert_eq!(rows[0], wanted(28));
    assert_eq!(
        rows[0],
        json!([
            "2026-01-04",
            "P",
            null,
            "written last, earlier date, other flag",
            "Assets:A",
            "8",
            "EUR",
            28
        ])
    );
    // A payee, then no payee with a pending flag and an elided posting keeping its places.
    assert_eq!(wanted(6)[2], "Shop");
    assert_eq!(
        wanted(10),
        json!([
            "2026-01-05",
            "!",
            null,
            "same date, no payee, pending",
            "Assets:B",
            "-2.50",
            "EUR",
            10
        ])
    );
    // `txn` reads as `*`; a flag on a posting is not the row's flag.
    assert_eq!(wanted(13)[1], "*");
    assert_eq!(wanted(16)[1], "*");
    // No narration, an empty one: both empty. Two strings: payee then narration.
    assert_eq!(wanted(19)[3], "");
    assert_eq!(wanted(22)[3], "");
    assert_eq!(
        (wanted(25)[2].clone(), wanted(25)[3].clone()),
        (json!("Payee only?"), json!(""))
    );
    // The transaction without a posting (line 11) has no row, and the sale at line 36 gives two.
    assert!(rows.iter().all(|r| r[7] != 11));
    let keys: Vec<(&str, i64)> = rows
        .iter()
        .map(|r| (r[0].as_str().unwrap(), r[7].as_i64().unwrap()))
        .collect();
    assert!(keys.is_sorted());
    let lines: Vec<i64> = keys.iter().map(|k| k.1).collect();
    assert_eq!(lines.iter().filter(|l| **l == 36).count(), 2);
    assert_eq!(lines.iter().filter(|l| **l == 31).count(), 1);
    assert_eq!(rows.last().unwrap()[4], "Assets:A");
}

#[test]
fn rows_of_a_transaction_follow_the_lines_not_the_currency_groups() {
    let rows = rows("b.beancount", &probe("interpolation-groups"));
    let lines = |date: &str| -> Vec<i64> {
        rows.iter()
            .filter(|r| r[0] == date)
            .map(|r| r[7].as_i64().unwrap())
            .collect()
    };
    assert_eq!(lines("2026-01-04"), [15, 16, 17, 18]);
    assert_eq!(lines("2026-01-05"), [20, 21, 22, 23]);
}

#[test]
fn hledger_gives_no_postings_yet() {
    let ledger = load(&[file("a.journal", "2026-01-01 x\n  a  1\n  b\n")]);
    assert_eq!(ledger.transactions().len(), 1);
    assert!(ledger.postings().is_empty());
}

#[test]
fn a_refused_journal_gives_no_postings() {
    let ledger = load(&[file(
        "b.beancount",
        "2026-01-01 open Assets:A\n2026-01-02 pad Assets:A Equity:O\n",
    )]);
    assert!(!ledger.check().is_empty());
    assert!(ledger.postings().is_empty());
}
