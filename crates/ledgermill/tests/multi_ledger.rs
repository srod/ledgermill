//! Several ledgers in one `load`: each Beancount file is its own ledger and all hledger files
//! together are one (spec/beancount/several-files.md). The lists are merged by date, then by the
//! position of the item's file in the manifest, then by each ledger's own order.
//!
//! Expected values are what the reference tools print for each probe file alone (`bean-query` 0.2.0,
//! `hledger` 1.52.4; spec/beancount/probes/several-files/) plus the merge rule, never what ledgermill
//! computes.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Holding, HoldingsQuery, Quantity, SourceFile, load};
use serde_json::json;
use support::file;

macro_rules! probe {
    ($name:literal) => {
        file(
            $name,
            include_str!(concat!(
                "../../../spec/beancount/probes/several-files/",
                $name
            )),
        )
    };
}

fn a() -> SourceFile {
    probe!("a.beancount")
}
fn b() -> SourceFile {
    probe!("b.beancount")
}
fn c() -> SourceFile {
    probe!("c.journal")
}

/// `date description` of each transaction, in order.
fn titles(l: &ledgermill::Ledger) -> Vec<String> {
    l.transactions()
        .iter()
        .map(|t| format!("{} {}", t.date, t.description))
        .collect()
}

#[test]
fn two_beancount_files_list_their_transactions_by_date_then_manifest_position() {
    let expected = [
        "2026-01-03 b buys early",
        "2026-01-05 a buys on the shared day",
        "2026-01-05 b spends on the shared day",
        "2026-01-10 a buys again",
    ];
    assert_eq!(titles(&load(&[a(), b()])), expected);
    // The shared day swaps with the manifest.
    let swapped = [
        "2026-01-03 b buys early",
        "2026-01-05 b spends on the shared day",
        "2026-01-05 a buys on the shared day",
        "2026-01-10 a buys again",
    ];
    assert_eq!(titles(&load(&[b(), a()])), swapped);
}

#[test]
fn each_transaction_keeps_its_own_file_and_line() {
    let l = load(&[a(), b()]);
    let at: Vec<(&str, u32)> = l
        .transactions()
        .iter()
        .map(|t| (t.source.file.as_str(), t.source.line))
        .collect();
    assert_eq!(
        at,
        [
            ("b.beancount", 9),
            ("a.beancount", 5),
            ("b.beancount", 5),
            ("a.beancount", 9)
        ]
    );
}

#[test]
fn a_mix_of_hledger_and_beancount_merges_the_same_way() {
    let expected = [
        "2026-01-03 b buys early",
        "2026-01-05 a buys on the shared day",
        "2026-01-05 b spends on the shared day",
        "2026-01-05 c pays on the shared day",
        "2026-01-07 c buys",
        "2026-01-10 a buys again",
    ];
    assert_eq!(titles(&load(&[a(), b(), c()])), expected);
    // The hledger file in the middle of the manifest: the shared day is a, c, b.
    let middle = [
        "2026-01-03 b buys early",
        "2026-01-05 a buys on the shared day",
        "2026-01-05 c pays on the shared day",
        "2026-01-05 b spends on the shared day",
        "2026-01-07 c buys",
        "2026-01-10 a buys again",
    ];
    assert_eq!(titles(&load(&[a(), c(), b()])), middle);
}

#[test]
fn hledger_files_stay_one_ledger_even_when_beancount_sits_between_them() {
    // Two hledger files with Beancount between them: the hledger files are read together (their
    // own order: date, then file), the Beancount file merges by its position.
    let first = file("h1.journal", "2026-01-05 h1\n    x  1 EUR\n    y\n");
    let second = file("h2.journal", "2026-01-05 h2\n    x  1 EUR\n    y\n");
    let l = load(&[first, b(), second]);
    let order: Vec<&str> = l
        .transactions()
        .iter()
        .map(|t| t.description.as_str())
        .collect();
    assert_eq!(
        order,
        ["b buys early", "h1", "b spends on the shared day", "h2"]
    );
}

#[test]
fn postings_follow_the_transactions() {
    let l = load(&[a(), b(), c()]);
    let rows: Vec<(&str, &str, u32, &str, &str, &str)> = l
        .postings()
        .iter()
        .map(|p| {
            (
                p.date.as_str(),
                p.file.as_str(),
                p.line,
                p.account.as_str(),
                p.number.as_str(),
                p.currency.as_str(),
            )
        })
        .collect();
    let expected = [
        (
            "2026-01-03",
            "b.beancount",
            10,
            "Assets:Stock",
            "5",
            "STOCK",
        ),
        ("2026-01-03", "b.beancount", 11, "Assets:Bank", "-45", "EUR"),
        ("2026-01-05", "a.beancount", 6, "Assets:Stock", "2", "STOCK"),
        ("2026-01-05", "a.beancount", 7, "Assets:Bank", "-20", "EUR"),
        ("2026-01-05", "b.beancount", 6, "Expenses:Food", "3", "EUR"),
        ("2026-01-05", "b.beancount", 7, "Assets:Bank", "-3", "EUR"),
        ("2026-01-05", "c.journal", 2, "Assets:Bank", "-4", "EUR"),
        ("2026-01-05", "c.journal", 3, "Expenses:Food", "4", "EUR"),
        ("2026-01-07", "c.journal", 6, "Assets:Stock", "4", "STOCK"),
        ("2026-01-07", "c.journal", 7, "Assets:Bank", "-41.00", "EUR"),
        (
            "2026-01-10",
            "a.beancount",
            10,
            "Assets:Stock",
            "1",
            "STOCK",
        ),
        ("2026-01-10", "a.beancount", 11, "Assets:Bank", "-12", "EUR"),
    ];
    assert_eq!(rows, expected);
}

#[test]
fn prices_are_merged_by_date_then_manifest_position() {
    let rows = |files: &[SourceFile]| -> Vec<(String, String, String)> {
        load(files)
            .prices()
            .iter()
            .map(|p| {
                (
                    p.date.clone(),
                    p.file.clone(),
                    format!("{} {}", p.rate.quantity, p.rate.commodity),
                )
            })
            .collect()
    };
    let row = |d: &str, f: &str, r: &str| (d.to_string(), f.to_string(), r.to_string());
    assert_eq!(
        rows(&[a(), b(), c()]),
        [
            row("2026-01-05", "a.beancount", "11 EUR"),
            row("2026-01-05", "b.beancount", "10.5 EUR"),
            row("2026-01-05", "c.journal", "10.8 EUR"),
            row("2026-01-20", "a.beancount", "13 EUR"),
        ]
    );
    assert_eq!(
        rows(&[c(), b(), a()]),
        [
            row("2026-01-05", "c.journal", "10.8 EUR"),
            row("2026-01-05", "b.beancount", "10.5 EUR"),
            row("2026-01-05", "a.beancount", "11 EUR"),
            row("2026-01-20", "a.beancount", "13 EUR"),
        ]
    );
}

fn findings_of(files: &[SourceFile]) -> Vec<(String, String, u32)> {
    load(files)
        .check()
        .iter()
        .map(|f| (f.file.clone(), f.code.clone(), f.line))
        .collect()
}

#[test]
fn findings_are_sorted_by_the_manifest_position_of_their_file() {
    let x = probe!("unbalanced.beancount");
    let y = probe!("unbalanced.journal");
    let z = probe!("unbalanced-2.journal");
    let row = |f: &str, l: u32| (f.to_string(), "unbalanced-transaction".to_string(), l);
    // The hledger ledger comes first (its first file is first), then the Beancount one; the sort
    // puts the Beancount finding between the two hledger files'.
    assert_eq!(
        findings_of(&[y.clone(), x.clone(), z.clone()]),
        [
            row("unbalanced.journal", 1),
            row("unbalanced.beancount", 4),
            row("unbalanced-2.journal", 1),
        ]
    );
    assert_eq!(
        findings_of(&[x, y, z]),
        [
            row("unbalanced.beancount", 4),
            row("unbalanced.journal", 1),
            row("unbalanced-2.journal", 1),
        ]
    );
}

#[test]
fn a_clean_set_has_no_finding_and_a_refused_file_gives_its_own_finding_only() {
    assert!(load(&[a(), b(), c()]).check().is_empty());
    let refused = probe!("refused.beancount");
    let l = load(&[a(), refused, b()]);
    let found: Vec<(&str, &str)> = l
        .check()
        .iter()
        .map(|f| (f.file.as_str(), f.code.as_str()))
        .collect();
    assert_eq!(found, [("refused.beancount", "unsupported-syntax")]);
    assert_eq!(l.check()[0].details["construct"], json!("plugin"));
    // The refused file lists nothing; the others still load.
    assert_eq!(
        titles(&l),
        [
            "2026-01-03 b buys early",
            "2026-01-05 a buys on the shared day",
            "2026-01-05 b spends on the shared day",
            "2026-01-10 a buys again",
        ]
    );
}

fn q(commodity: &str, quantity: &str) -> Quantity {
    Quantity {
        commodity: commodity.into(),
        quantity: quantity.into(),
    }
}

fn query() -> HoldingsQuery {
    HoldingsQuery {
        commodity: "EUR".into(),
        end: None,
        today: "2026-02-01".into(),
    }
}

fn held(account: &str, commodity: &str, units: &str, lists: [&[(&str, &str)]; 3]) -> Holding {
    let list = |l: &[(&str, &str)]| l.iter().map(|(c, n)| q(c, n)).collect();
    Holding {
        account: account.into(),
        commodity: commodity.into(),
        units: units.into(),
        cost: list(lists[0]),
        value: list(lists[1]),
        cost_value: list(lists[2]),
    }
}

#[test]
fn at_cost_and_value_line_up_with_transactions() {
    let l = load(&[a(), b(), c()]);
    let titles = titles(&l);
    for (name, result) in [
        ("at_cost", l.at_cost().unwrap()),
        ("value", l.value("2026-01-31", "EUR").unwrap()),
    ] {
        let got: Vec<String> = result
            .iter()
            .map(|t| format!("{} {}", t.date, t.description))
            .collect();
        assert_eq!(got, titles, "{name}");
        for (t, listed) in result.iter().zip(l.transactions()) {
            assert_eq!(t.source, listed.source, "{name}");
        }
    }
    // Each ledger valued on its own: a at its own prices (13 EUR on the 20th), b at 10.5, the
    // hledger file at its P line (10.8); reference values from each file alone.
    let valued = l.value("2026-01-31", "EUR").unwrap();
    let stock = |name: &str| -> String {
        let t = valued.iter().find(|t| t.description == name).unwrap();
        let p = &t.postings[0];
        format!("{} {}", p.amounts[0].quantity, p.amounts[0].commodity)
    };
    assert_eq!(stock("a buys on the shared day"), "26 EUR");
    assert_eq!(stock("a buys again"), "13 EUR");
    assert_eq!(stock("b buys early"), "52.5 EUR");
    assert_eq!(stock("c buys"), "43.2 EUR");
    let costed = l.at_cost().unwrap();
    let first = &costed
        .iter()
        .find(|t| t.description == "a buys again")
        .unwrap()
        .postings[0]
        .amounts[0];
    assert_eq!(
        (first.quantity.as_str(), first.commodity.as_str()),
        ("12", "EUR")
    );
}

#[test]
fn an_error_of_the_first_ledger_wins_and_a_refused_file_makes_every_valuation_fail() {
    let l = load(&[a(), b()]);
    assert_eq!(
        l.value("2026-02-30", "EUR").unwrap_err().code,
        "invalid-argument"
    );
    let mut bad = query();
    bad.end = Some("soon".into());
    assert_eq!(
        l.holdings(&bad).unwrap_err().details,
        json!({ "argument": "end", "value": "soon" })
    );
    // A refused Beancount file: the error is its refusal, even though the others are fine.
    let l = load(&[a(), probe!("refused.beancount"), b()]);
    let refusal = l.value("2026-01-31", "EUR").unwrap_err();
    assert_eq!(refusal.file, "refused.beancount");
    assert_eq!(refusal.details, json!({ "construct": "plugin" }));
    assert_eq!(l.at_cost().unwrap_err().file, "refused.beancount");
    assert_eq!(l.holdings(&query()).unwrap_err().file, "refused.beancount");
    // Two refused files: the first in ledger order.
    let l = load(&[
        probe!("refused.beancount"),
        file(
            "other.beancount",
            "option \"operating_currency\" \"EUR\"\ninclude \"x\"\n",
        ),
    ]);
    assert_eq!(l.at_cost().unwrap_err().file, "refused.beancount");
}

#[test]
fn holdings_sum_the_rows_of_a_shared_account_and_commodity() {
    let l = load(&[a(), b(), c()]);
    let rows = l.holdings(&query()).unwrap();
    // Bank: -32 (a) + -48 (b) + -45 (hledger file) in units, cost and value alike; Stock: 3 + 5 + 4
    // units; cost 32 + 45 + 41; value 39 + 52.5 + 43.2 (each ledger at its own prices); Food: only b
    // and c have it, 3 + 4.
    let expected = [
        held(
            "Assets:Bank",
            "EUR",
            "-125",
            [&[("EUR", "-125")], &[("EUR", "-125")], &[("EUR", "-125")]],
        ),
        held(
            "Assets:Stock",
            "STOCK",
            "12",
            [&[("EUR", "118")], &[("EUR", "134.7")], &[("EUR", "118")]],
        ),
        held(
            "Expenses:Food",
            "EUR",
            "7",
            [&[("EUR", "7")], &[("EUR", "7")], &[("EUR", "7")]],
        ),
    ];
    assert_eq!(rows, expected);
}

#[test]
fn a_row_in_one_ledger_only_is_passed_through_unchanged() {
    // Only the hledger file has `p` and `q`; its own strings (two places) stay as they are.
    let only_c = file("only.journal", "2026-01-01 t\n    p  1.50 EUR\n    q\n");
    let l = load(&[a(), only_c]);
    let rows = l.holdings(&query()).unwrap();
    let p = rows.iter().find(|h| h.account == "p").unwrap();
    assert_eq!(p.units, "1.50");
    assert_eq!(p.cost, [q("EUR", "1.50")]);
    // Ordered with the rows of the other ledger: by account segments.
    let accounts: Vec<&str> = rows.iter().map(|h| h.account.as_str()).collect();
    assert_eq!(accounts, ["Assets:Bank", "Assets:Stock", "p", "q"]);
}

fn pair(name: &str, amount: &str) -> SourceFile {
    let other = amount
        .strip_prefix('-')
        .map_or_else(|| format!("-{amount}"), str::to_string);
    file(
        name,
        &format!(
            "2026-01-01 open Assets:X\n2026-01-01 open Assets:Y\n\n2026-01-02 * \"t\"\n  Assets:X  {amount} EUR\n  Assets:Y  {other} EUR\n"
        ),
    )
}

#[test]
fn sums_keep_the_places_of_their_most_precise_term() {
    let l = load(&[pair("p.beancount", "1.50"), pair("q.beancount", "2.5")]);
    let rows = l.holdings(&query()).unwrap();
    assert_eq!(
        rows[0],
        held(
            "Assets:X",
            "EUR",
            "4.00",
            [&[("EUR", "4.00")], &[("EUR", "4.00")], &[("EUR", "4.00")]]
        )
    );
    // The hledger side is rounded to ten places by its own engine and the sum is not rounded again.
    let tiny = file(
        "t.journal",
        "2026-01-02 t\n    Assets:X  0.00000000014 EUR\n    Assets:Y\n",
    );
    let l = load(&[pair("p.beancount", "1.50"), tiny]);
    assert_eq!(l.holdings(&query()).unwrap()[0].units, "1.5000000001");
}

#[test]
fn a_sum_of_exactly_zero_is_not_listed_unless_something_else_is_held() {
    let l = load(&[pair("p.beancount", "5"), pair("q.beancount", "-5")]);
    assert!(l.holdings(&query()).unwrap().is_empty());
    // Units cancel but a cost does not: the row stays with units "0".
    let sold = file(
        "s.beancount",
        "2026-01-01 open Assets:S\n2026-01-01 open Assets:C\n\n2026-01-02 * \"t\"\n  Assets:S  -2 STOCK {3 EUR}\n  Assets:C  6 EUR\n",
    );
    let bought = file(
        "b.beancount",
        "2026-01-01 open Assets:S\n2026-01-01 open Assets:C\n\n2026-01-02 * \"t\"\n  Assets:S  2 STOCK {2 EUR}\n  Assets:C  -4 EUR\n",
    );
    let rows = load(&[sold, bought]).holdings(&query()).unwrap();
    let stock = rows.iter().find(|h| h.account == "Assets:S").unwrap();
    assert_eq!(stock.units, "0");
    assert_eq!(stock.cost, [q("EUR", "-2")]);
}
