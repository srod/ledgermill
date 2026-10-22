//! `balance` assertions: when they are checked, what they sum, their tolerance and how a failure
//! is reported (spec/beancount/balance-directive.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

fn probe(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../spec/beancount/probes/{name}.beancount",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn an_assertion_that_holds_adds_nothing_and_reports_nothing() {
    for name in [
        "balance-directive",
        "balance-directive-subaccounts",
        "balance-directive-lots",
        "balance-directive-tolerance",
    ] {
        let journal = probe(name);
        assert_eq!(findings("b.beancount", &journal), json!([]), "{name}");
    }
    let journal = probe("balance-directive");
    let ledger = load(&[file("b.beancount", &journal)]);
    assert_eq!(ledger.transactions().len(), 1);
    assert_eq!(ledger.postings().len(), 2);
}

#[test]
fn an_assertion_sees_the_transactions_before_its_day_wherever_they_are_written() {
    let journal = probe("balance-directive-day");
    assert_eq!(findings("b.beancount", &journal), json!([]));
}

#[test]
fn a_failed_assertion_is_reported_at_its_line_in_the_order_bean_check_prints() {
    let journal = probe("balance-directive-fails");
    // By date, not by line.
    assert_eq!(
        short_findings("b.beancount", &journal),
        json!([
            [29, "assertion-failed", null],
            [28, "assertion-failed", null],
            [27, "assertion-failed", null],
            [26, "assertion-failed", null],
            [25, "assertion-failed", null],
            [24, "assertion-failed", null],
            [23, "assertion-failed", null],
            [30, "assertion-failed", null],
            [31, "assertion-failed", null],
        ])
    );
}

#[test]
fn a_failed_assertion_names_the_account_the_amounts_and_the_difference() {
    let journal = probe("balance-directive-fails");
    let found = findings("b.beancount", &journal);
    let by_line = |line: u64| {
        found
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["line"] == json!(line))
            .unwrap()
            .clone()
    };
    assert_eq!(by_line(29)["severity"], json!("error"));
    assert_eq!(
        by_line(29)["details"],
        json!({
            "account": "Assets:G", "currency": "USD", "expected": "5",
            "accumulated": "0", "difference": "-5"
        })
    );
    assert_eq!(
        by_line(28)["details"],
        json!({
            "account": "Assets:F", "currency": "EUR", "expected": "-500.00",
            "accumulated": "500", "difference": "1000.00"
        })
    );
    assert_eq!(by_line(24)["details"]["accumulated"], json!("499.989"));
    assert_eq!(by_line(24)["details"]["difference"], json!("-0.011"));
    assert_eq!(by_line(31)["details"]["expected"], json!("500.0"));
    assert_eq!(by_line(31)["details"]["difference"], json!("0.11"));
}

#[test]
fn a_failed_assertion_leaves_the_transactions_alone() {
    let journal = probe("balance-directive-fails");
    assert_eq!(
        load(&[file("b.beancount", &journal)]).transactions().len(),
        1
    );
}

#[test]
fn an_assertion_in_a_currency_never_held_sums_zero() {
    let journal = "2026-01-01 open Assets:A\n2026-01-02 balance Assets:A  0 USD\n2026-01-03 balance Assets:A  1 USD\n";
    assert_eq!(
        short_findings("b.beancount", journal),
        json!([[3, "assertion-failed", null]])
    );
}

#[test]
fn balance_is_no_longer_unsupported_syntax() {
    let journal = probe("balance-directive");
    let found = short_findings("b.beancount", &journal);
    assert!(found.as_array().unwrap().is_empty());
}
