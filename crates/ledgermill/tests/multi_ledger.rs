//! Several files in one `load`: all Beancount files together are one ledger, and all hledger files
//! together are one (spec/beancount/several-files.md). The lists are sorted by date, then by the
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
fn an_account_opened_in_the_first_file_is_used_in_the_second() {
    // b.beancount opens Expenses:Food only: Assets:Bank and Assets:Stock are a's.
    let l = load(&[a(), b()]);
    assert!(l.check().is_empty());
    let rows = |file: &str| l.postings().iter().filter(|p| p.file == file).count();
    assert_eq!((rows("a.beancount"), rows("b.beancount")), (4, 4));
    // An `open` is dated: it does not matter which file comes first.
    assert!(load(&[b(), a()]).check().is_empty());
    assert!(load(&[a(), b(), c()]).check().is_empty());
}

#[test]
fn a_balance_assertion_in_the_second_file_adds_up_the_postings_of_both() {
    let first = probe!("bal-a.beancount");
    assert!(
        load(&[first.clone(), probe!("bal-b.beancount")])
            .check()
            .is_empty()
    );
    // 100 is what the first file alone holds: the failure is on the second file's line.
    let wrong = load(&[first, probe!("bal-wrong-b.beancount")]);
    assert_eq!(
        findings_of_ledger(&wrong),
        [("bal-wrong-b.beancount", "assertion-failed", 6)]
    );
    assert_eq!(
        wrong.check()[0].details,
        json!({
            "account": "Assets:Bank", "currency": "EUR", "expected": "100",
            "accumulated": "150", "difference": "50",
        })
    );
}

fn findings_of_ledger(l: &ledgermill::Ledger) -> Vec<(&str, &str, u32)> {
    l.check()
        .iter()
        .map(|f| (f.file.as_str(), f.code.as_str(), f.line))
        .collect()
}

#[test]
fn an_account_opened_twice_across_files_is_a_duplicate_open_in_the_second_file() {
    let (x, y) = (probe!("open-a.beancount"), probe!("open-b.beancount"));
    assert_eq!(
        findings_of_ledger(&load(&[x.clone(), y.clone()])),
        [
            ("open-b.beancount", "duplicate-open", 1),
            ("open-b.beancount", "duplicate-open", 2),
        ]
    );
    // On one date and one line, the file later in the set is the duplicate.
    assert_eq!(
        findings_of_ledger(&load(&[y, x])),
        [
            ("open-a.beancount", "duplicate-open", 1),
            ("open-a.beancount", "duplicate-open", 2),
        ]
    );
}

#[test]
fn the_walk_goes_by_date_and_line_before_the_file() {
    // Both open the account on one date, one on line 5 and one on line 1: the line decides, so
    // the one on line 5 is the duplicate whichever file comes first.
    let (x, y) = (
        probe!("order-open-a.beancount"),
        probe!("order-open-b.beancount"),
    );
    let expected = [("order-open-a.beancount", "duplicate-open", 5)];
    assert_eq!(findings_of_ledger(&load(&[x.clone(), y.clone()])), expected);
    assert_eq!(findings_of_ledger(&load(&[y, x])), expected);
}

#[test]
fn a_pad_in_one_file_is_filled_by_a_balance_in_another() {
    let l = load(&[probe!("pad-a.beancount"), probe!("pad-b.beancount")]);
    assert!(l.check().is_empty());
    // The transaction stands on the pad, its postings on the balance, which is in the other file.
    let [t] = l.transactions() else {
        panic!("one padding transaction")
    };
    assert_eq!(
        (t.source.file.as_str(), t.source.line),
        ("pad-a.beancount", 3)
    );
    let rows: Vec<(&str, u32, &str, &str)> = l
        .postings()
        .iter()
        .map(|p| {
            (
                p.file.as_str(),
                p.line,
                p.account.as_str(),
                p.number.as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("pad-b.beancount", 1, "Assets:Bank", "100"),
            ("pad-b.beancount", 1, "Equity:Opening", "-100"),
        ]
    );
}

#[test]
fn the_booking_method_and_plugins_of_the_first_file_serve_the_set() {
    let reduced = |l: &ledgermill::Ledger| -> Option<String> {
        let t = l
            .transactions()
            .iter()
            .find(|t| t.description == "sell one")?;
        t.postings[0].amounts[0]
            .lot
            .as_ref()?
            .cost
            .as_ref()
            .map(|c| c.quantity.clone())
    };
    // FIFO in the root takes the oldest lot, 10 EUR, for a sale in the other file.
    let served = load(&[probe!("fifo-a.beancount"), probe!("sale-b.beancount")]);
    assert!(served.check().is_empty());
    assert_eq!(reduced(&served).as_deref(), Some("10"));
    // The option of a file that is not the root is not applied: the sale is ambiguous.
    let ignored = load(&[probe!("sale-b.beancount"), probe!("fifo-a.beancount")]);
    assert_eq!(
        findings_of_ledger(&ignored)
            .iter()
            .map(|(f, c, _)| (*f, *c))
            .collect::<Vec<_>>(),
        [("sale-b.beancount", "booking-error")]
    );
    // The same for a plugin: auto_accounts of the root opens what the other file uses.
    let (x, y) = (probe!("auto-a.beancount"), probe!("auto-b.beancount"));
    assert!(load(&[x.clone(), y.clone()]).check().is_empty());
    let unopened = load(&[y, x]);
    let codes: Vec<(&str, &str)> = findings_of_ledger(&unopened)
        .iter()
        .map(|(f, c, _)| (*f, *c))
        .collect();
    assert_eq!(
        codes,
        [("auto-a.beancount", "account-not-open"); 2]
            .into_iter()
            .chain([("auto-b.beancount", "account-not-open"); 2])
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_file_with_no_entry_adds_nothing_as_root_or_not() {
    let alone = load(&[a(), b()]);
    // An empty file, and one that only has options ledgermill reads, in any position.
    for nothing in [probe!("empty.beancount"), probe!("options-only.beancount")] {
        for files in [
            [a(), nothing.clone(), b()],
            [nothing.clone(), a(), b()],
            [a(), b(), nothing],
        ] {
            let l = load(&files);
            assert!(l.check().is_empty());
            assert_eq!(titles(&l), titles(&alone));
            assert_eq!(l.postings(), alone.postings());
            assert_eq!(l.prices(), alone.prices());
        }
    }
    // On its own it is a ledger that holds nothing.
    let empty = load(&[probe!("empty.beancount")]);
    assert!(empty.check().is_empty() && empty.transactions().is_empty());
}

#[test]
fn settings_alone_in_a_file_that_is_not_the_root_change_nothing() {
    // `settings-only` has booking_method FIFO and both plugins. After `lots-a` (no settings) the
    // sale stays ambiguous, the gift's accounts stay unopened, and no price is added.
    let ignored = load(&[
        probe!("lots-a.beancount"),
        probe!("settings-only.beancount"),
    ]);
    let mut found: Vec<(&str, u32)> = findings_of_ledger(&ignored)
        .iter()
        .map(|(f, c, line)| {
            assert!(*f == "lots-a.beancount");
            (*c, *line)
        })
        .collect();
    found.sort_unstable();
    assert_eq!(
        found,
        [
            ("account-not-open", 18),
            ("account-not-open", 18),
            ("booking-error", 14)
        ]
    );
    assert!(ignored.prices().is_empty());
    // As the root they apply to the other file: no finding, and the lots' prices are added.
    let served = load(&[
        probe!("settings-only.beancount"),
        probe!("lots-a.beancount"),
    ]);
    assert!(served.check().is_empty());
    let prices: Vec<(&str, u32, &str)> = served
        .prices()
        .iter()
        .map(|p| (p.date.as_str(), p.line, p.rate.quantity.as_str()))
        .collect();
    assert_eq!(prices, [("2026-01-02", 6, "10"), ("2026-01-03", 10, "20")]);
}

#[test]
fn implicit_prices_of_the_root_cover_the_transactions_of_every_file() {
    let (x, y) = (
        probe!("implicit-a.beancount"),
        probe!("implicit-b.beancount"),
    );
    let rows = |l: &ledgermill::Ledger| -> Vec<(String, String, u32, String)> {
        l.prices()
            .iter()
            .map(|p| {
                (
                    p.date.clone(),
                    p.file.clone(),
                    p.line,
                    p.rate.quantity.clone(),
                )
            })
            .collect()
    };
    let row = |d: &str, f: &str, l: u32, r: &str| (d.to_string(), f.to_string(), l, r.to_string());
    assert_eq!(
        rows(&load(&[x.clone(), y.clone()])),
        [
            // The table lists by line within a day (and a valuation takes the later one: 10).
            row("2026-01-05", "implicit-b.beancount", 4, "11"),
            row("2026-01-05", "implicit-a.beancount", 6, "10"),
            row("2026-01-07", "implicit-b.beancount", 8, "11.5"),
            row("2026-01-09", "implicit-a.beancount", 10, "12"),
        ]
    );
    // Not the root: its plugin line does nothing, and only the written price is listed.
    assert_eq!(
        rows(&load(&[y, x])),
        [row("2026-01-07", "implicit-b.beancount", 8, "11.5")]
    );
}

#[test]
fn a_mixed_set_keeps_the_hledger_part_as_it_is_alone() {
    let alone = load(&[c()]);
    let mixed = load(&[a(), c(), b()]);
    assert!(mixed.check().is_empty());
    let hledger: Vec<_> = mixed
        .transactions()
        .iter()
        .filter(|t| t.source.file == "c.journal")
        .cloned()
        .collect();
    assert_eq!(hledger, alone.transactions());
    let rows: Vec<_> = mixed
        .postings()
        .iter()
        .filter(|p| p.file == "c.journal")
        .cloned()
        .collect();
    assert_eq!(rows, alone.postings());
    let prices: Vec<_> = mixed
        .prices()
        .iter()
        .filter(|p| p.file == "c.journal")
        .cloned()
        .collect();
    assert_eq!(prices, alone.prices());
}

#[test]
fn a_refused_construct_refuses_the_beancount_ledger_and_leaves_the_hledger_one() {
    let refused = probe!("refused.beancount");
    let l = load(&[a(), refused.clone(), b()]);
    assert_eq!(
        findings_of_ledger(&l),
        [("refused.beancount", "unsupported-syntax", 1)]
    );
    assert_eq!(l.check()[0].details["construct"], json!("plugin"));
    // Beancount files are one ledger: it lists nothing, with the good files next to the bad one.
    assert!(l.transactions().is_empty() && l.postings().is_empty() && l.prices().is_empty());
    // The hledger ledger is another one.
    assert_eq!(titles(&load(&[a(), refused, c()])), titles(&load(&[c()])));
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
    // The Beancount files are one ledger with the prices of both (the latest on the 31st is 13 EUR,
    // a's), the hledger file is valued at its own P line (10.8).
    let valued = l.value("2026-01-31", "EUR").unwrap();
    let stock = |name: &str| -> String {
        let t = valued.iter().find(|t| t.description == name).unwrap();
        let p = &t.postings[0];
        format!("{} {}", p.amounts[0].quantity, p.amounts[0].commodity)
    };
    assert_eq!(stock("a buys on the shared day"), "26 EUR");
    assert_eq!(stock("a buys again"), "13 EUR");
    assert_eq!(stock("b buys early"), "65 EUR");
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
    assert_eq!(
        refusal.details,
        json!({ "construct": "plugin", "name": "beancount.plugins.check_commodity" })
    );
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
    // units; cost 32 + 45 + 41; value (3 + 5) * 13 at the Beancount ledger's latest price, then 43.2
    // for the hledger file at its own; Food: only b and c have it, 3 + 4.
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
            [&[("EUR", "118")], &[("EUR", "147.2")], &[("EUR", "118")]],
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

fn hledger_pair(amount: &str) -> SourceFile {
    file(
        "h.journal",
        &format!("2026-01-02 t\n    Assets:X  {amount} EUR\n    Assets:Y\n"),
    )
}

#[test]
fn sums_keep_the_places_of_their_most_precise_term() {
    let l = load(&[pair("p.beancount", "1.50"), hledger_pair("2.5")]);
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
    let l = load(&[pair("p.beancount", "5"), hledger_pair("-5")]);
    assert!(l.holdings(&query()).unwrap().is_empty());
    // Units cancel but a cost does not: the row stays with units "0".
    let bought = file(
        "b.beancount",
        "2026-01-01 open Assets:S\n2026-01-01 open Assets:C\n\n2026-01-02 * \"t\"\n  Assets:S  2 STOCK {2 EUR}\n  Assets:C  -4 EUR\n",
    );
    let sold = file(
        "s.journal",
        "2026-01-02 t\n    Assets:S  -2 STOCK @ 3 EUR\n    Assets:C\n",
    );
    let rows = load(&[bought, sold]).holdings(&query()).unwrap();
    let stock = rows.iter().find(|h| h.account == "Assets:S").unwrap();
    assert_eq!(stock.units, "0");
    assert_eq!(stock.cost, [q("EUR", "-2")]);
}
