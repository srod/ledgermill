//! `pad`: the transaction it adds at a `balance`, which pads it, in what order the results come
//! out, and the problems it causes (spec/beancount/pad.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::*;

fn probe(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../spec/beancount/probes/{name}.beancount",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// The padding transactions, in the order `transactions()` gives them:
/// `[line, narration, [[account, quantity, line]]]`.
fn paddings(name: &str) -> Vec<Value> {
    let journal = probe(name);
    beancount_rows("p.beancount", &journal)
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t[1] == json!("P"))
        .map(|t| {
            let postings: Vec<Value> = t[5]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| json!([p[0], p[1], p[3]]))
                .collect();
            json!([t[4], t[3], postings])
        })
        .collect()
}

fn narration(expected: &str, difference: &str) -> String {
    format!("(Padding inserted for Balance of {expected} for difference {difference})")
}

#[test]
fn a_pad_adds_a_p_flagged_transaction_on_the_pad_line_with_postings_on_the_balance_line() {
    let journal = probe("pad");
    let rows = beancount_rows("p.beancount", &journal);
    assert_eq!(
        rows,
        json!([[
            "2026-01-02",
            "P",
            null,
            narration("500.00 EUR", "500.00 EUR"),
            4,
            [
                ["Assets:Bank", "500.00", "EUR", 5, null, null],
                ["Equity:Opening", "-500.00", "EUR", 5, null, null]
            ]
        ]])
    );
    let ledger = load(&[file("p.beancount", &journal)]);
    assert_eq!(findings("p.beancount", &journal), json!([]));
    assert_eq!(
        ledger.transactions()[0].status,
        ledgermill::Status::Unmarked
    );
    assert!(
        ledger
            .postings()
            .iter()
            .all(|r| r.flag == "P" && r.line == 5)
    );
}

#[test]
fn the_padded_amount_is_what_the_assertion_needs_over_what_the_account_holds() {
    assert_eq!(
        paddings("pad-amount"),
        vec![
            json!([
                9,
                narration("500.00 EUR", "510.00 EUR"),
                [
                    ["Assets:A", "510.00", 13],
                    ["Equity:Opening", "-510.00", 13]
                ]
            ]),
            json!([
                18,
                narration("500 EUR", "-300 EUR"),
                [["Assets:B", "-300", 19], ["Expenses:Food", "300", 19]]
            ]),
            json!([
                24,
                narration("500.1 EUR", "489.845 EUR"),
                [
                    ["Assets:C", "489.845", 25],
                    ["Equity:Opening", "-489.845", 25]
                ]
            ]),
            json!([
                27,
                narration("500.1 EUR", "500.1 EUR"),
                [["Assets:D", "500.1", 28], ["Equity:Opening", "-500.1", 28]]
            ]),
            json!([
                33,
                narration("500.51 EUR", "0.51 EUR"),
                [["Assets:E", "0.51", 34], ["Equity:Opening", "-0.51", 34]]
            ]),
        ]
    );
}

#[test]
fn numbers_in_the_narration_are_plain_decimals_with_their_places() {
    let rows: Vec<String> = paddings("pad-narration-numbers")
        .iter()
        .map(|t| t[1].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        rows,
        vec![
            narration("1000.50 EUR", "1000.50 EUR"),
            narration("7.50 EUR", "7.50 EUR"),
            narration("-12.0 EUR", "-12.0 EUR"),
            narration("5 EUR", "5 EUR"),
            narration("0.0000005 EUR", "0.0000005 EUR"),
        ]
    );
    // The postings keep the number as the reference's query prints it.
    let last = paddings("pad-narration-numbers").pop().unwrap();
    assert_eq!(
        last[2],
        json!([["Assets:E", "5E-7", 18], ["Equity:Opening", "-5E-7", 18]])
    );
}

#[test]
fn transactions_of_the_pad_date_all_count_and_those_of_the_balance_date_do_not() {
    let journal = probe("pad-same-day-order");
    let rows = beancount_rows("p.beancount", &journal);
    let lines: Vec<(Value, Value)> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t[1].clone(), t[4].clone()))
        .collect();
    // The padding transaction stands where the pad is, between A and B.
    assert_eq!(
        lines,
        vec![
            (json!("*"), json!(5)),
            (json!("P"), json!(8)),
            (json!("*"), json!(9)),
            (json!("*"), json!(13))
        ]
    );
    assert_eq!(
        paddings("pad-same-day-order")[0][1],
        json!(narration("100.00 EUR", "103.00 EUR"))
    );
    assert_eq!(findings("p.beancount", &journal), json!([]));
}

#[test]
fn flat_postings_of_a_padding_sort_by_the_balance_line() {
    let journal = probe("pad-same-day-order");
    let ledger = load(&[file("p.beancount", &journal)]);
    let rows: Vec<(String, u32)> = ledger
        .postings()
        .iter()
        .map(|r| (r.flag.clone(), r.line))
        .collect();
    // After B (lines 10, 11): the padding rows carry the balance's line, 12.
    assert_eq!(
        rows,
        vec![
            ("*".into(), 6),
            ("*".into(), 7),
            ("*".into(), 10),
            ("*".into(), 11),
            ("P".into(), 12),
            ("P".into(), 12),
            ("*".into(), 14),
            ("*".into(), 15)
        ]
    );
}

#[test]
fn each_currency_is_padded_once_and_the_orders_of_the_two_lists_differ() {
    let journal = probe("pad-two-currencies");
    let tx: Vec<String> = paddings("pad-two-currencies")
        .iter()
        .map(|t| t[1].as_str().unwrap().to_string())
        .collect();
    // The USD balance is on the earlier date, so its transaction was made first.
    assert_eq!(
        tx,
        vec![
            narration("20 USD", "20 USD"),
            narration("600.00 EUR", "600.00 EUR")
        ]
    );
    let ledger = load(&[file("p.beancount", &journal)]);
    let flat: Vec<(String, u32)> = ledger
        .postings()
        .iter()
        .map(|r| (r.currency.clone(), r.line))
        .collect();
    assert_eq!(
        flat,
        vec![
            ("EUR".into(), 5),
            ("EUR".into(), 5),
            ("USD".into(), 6),
            ("USD".into(), 6)
        ]
    );
}

#[test]
fn a_new_pad_on_the_account_starts_afresh() {
    let again = paddings("pad-again");
    assert_eq!(again.len(), 2);
    assert_eq!(again[0][1], json!(narration("100.00 EUR", "100.00 EUR")));
    assert_eq!(again[1][1], json!(narration("150.00 EUR", "50.00 EUR")));
    assert_eq!(findings("p.beancount", &probe("pad-again")), json!([]));
}

#[test]
fn a_pad_sums_its_account_and_everything_below_but_not_what_the_same_balance_adds() {
    let rows = paddings("pad-subaccounts");
    let summary: Vec<(Value, Value, Value)> = rows
        .iter()
        .map(|t| (t[0].clone(), t[1].clone(), t[2][0][0].clone()))
        .collect();
    assert_eq!(
        summary,
        vec![
            (
                json!(16),
                json!(narration("100.00 EUR", "100.00 EUR")),
                json!("Assets:Q:Sub")
            ),
            (
                json!(13),
                json!(narration("500.00 EUR", "399.00 EUR")),
                json!("Assets:Parent")
            ),
            (
                json!(17),
                json!(narration("100.00 EUR", "100.00 EUR")),
                json!("Assets:Q")
            ),
        ]
    );
}

#[test]
fn a_pad_that_never_made_a_transaction_is_unused_and_reported_in_pad_order() {
    let found = short_findings("p.beancount", &probe("pad-unused"));
    assert_eq!(
        found,
        json!([
            [9, "unused-pad", null],
            [13, "unused-pad", null],
            [18, "unused-pad", null],
            [20, "unused-pad", null],
            [23, "unused-pad", null],
            [28, "unused-pad", null]
        ])
    );
    let one = &findings("p.beancount", &probe("pad-unused"))[0];
    assert_eq!(one["severity"], json!("error"));
    assert_eq!(
        one["details"],
        json!({ "account": "Assets:A", "source": "Equity:Opening" })
    );
    // Only the pad that padded something made a transaction.
    assert_eq!(paddings("pad-unused").len(), 1);
}

#[test]
fn a_pad_handles_a_currency_only_once_even_when_the_first_assertion_needed_nothing() {
    let journal = probe("pad-consumed");
    assert_eq!(
        short_findings("p.beancount", &journal),
        json!([
            [8, "unused-pad", null],
            [7, "assertion-failed", null],
            [10, "assertion-failed", null]
        ])
    );
    assert_eq!(paddings("pad-consumed").len(), 1);
}

#[test]
fn a_pad_dated_the_day_of_the_balance_is_not_active_yet() {
    let journal = probe("pad-same-date-as-balance");
    assert_eq!(
        short_findings("p.beancount", &journal),
        json!([[4, "unused-pad", null], [5, "assertion-failed", null]])
    );
}

#[test]
fn only_a_pad_on_the_account_or_above_it_pads() {
    assert_eq!(
        short_findings("p.beancount", &probe("pad-child-balance-parent")),
        json!([[8, "unused-pad", null], [9, "assertion-failed", null]])
    );
    assert_eq!(
        short_findings("p.beancount", &probe("pad-parent-balance-child")),
        json!([[12, "assertion-failed", null]])
    );
    assert_eq!(paddings("pad-parent-balance-child").len(), 1);
}

#[test]
fn a_transaction_the_reference_leaves_out_does_not_count_but_an_unbalanced_one_does() {
    let rows = paddings("pad-rejected-transactions");
    assert_eq!(rows[0][1], json!(narration("500 EUR", "490 EUR")));
    let codes: Vec<Value> = findings("p.beancount", &probe("pad-rejected-transactions"))
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].clone())
        .collect();
    assert_eq!(
        codes,
        vec![json!("booking-error"), json!("unbalanced-transaction")]
    );
}

#[test]
fn padding_an_account_that_holds_lots_is_reported_on_the_balance_line_but_still_made() {
    let found = findings("p.beancount", &probe("pad-lots-at-cost"));
    assert_eq!(
        short_findings("p.beancount", &probe("pad-lots-at-cost")),
        json!([[13, "pad-with-cost", null]])
    );
    assert_eq!(
        found[0]["details"],
        json!({ "account": "Assets:Stock", "currency": "BTC" })
    );
    let rows = paddings("pad-lots-at-cost");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][1], json!(narration("7 BTC", "2 BTC")));
    // Plain units: no cost on the padding posting.
    let all = beancount_rows("p.beancount", &probe("pad-lots-at-cost"));
    let posting = &all[2][5][0];
    assert_eq!(posting[5], json!(null));
}
