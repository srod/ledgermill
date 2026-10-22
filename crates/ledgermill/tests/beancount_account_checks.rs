//! What `check()` reports besides booking and balances: accounts that are not open, `open` and
//! `close` mistakes, currencies an account does not allow, duplicate assertions and commodities
//! (spec/beancount/check-errors.md, check-order.md). Each journal lists the findings in the order
//! `bean-check` prints them, with the errors it prints twice once.
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

/// `[line, code]` of each error, in order.
fn errors(name: &str) -> Value {
    let found = findings("c.beancount", &probe(name));
    Value::Array(
        found
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == json!("error"))
            .map(|f| json!([f["line"], f["code"]]))
            .collect(),
    )
}

fn details(name: &str, index: usize) -> Value {
    findings("c.beancount", &probe(name))[index]["details"].clone()
}

#[test]
fn an_account_that_was_never_opened_is_reported_at_the_transaction_header() {
    assert_eq!(
        errors("check-account-unknown"),
        json!([[3, "account-not-open"], [6, "account-not-open"]])
    );
    assert_eq!(
        details("check-account-unknown", 0),
        json!({ "account": "Expenses:Food", "reason": "never-opened" })
    );
    // bean-query still lists them.
    let journal = probe("check-account-unknown");
    assert_eq!(
        load(&[file("c.beancount", &journal)]).transactions().len(),
        2
    );
}

#[test]
fn an_account_used_before_its_open_or_after_its_close_is_not_active() {
    assert_eq!(
        errors("check-account-dates"),
        json!([[6, "account-not-open"], [9, "account-not-open"]])
    );
    assert_eq!(
        details("check-account-dates", 0)["reason"],
        json!("not-yet-open")
    );
    assert_eq!(details("check-account-dates", 1)["reason"], json!("closed"));
}

#[test]
fn the_days_of_the_open_and_the_close_are_fine_whatever_the_order_of_the_lines() {
    assert_eq!(errors("check-account-open-close-days"), json!([]));
}

#[test]
fn notes_documents_balances_and_pads_name_accounts_too_and_a_close_only_stops_transactions_and_pads()
 {
    assert_eq!(
        errors("check-directive-dates"),
        // The balance before the open comes from the balance step, the rest from the account walk.
        json!([
            [8, "account-not-open"],
            [6, "account-not-open"],
            [7, "account-not-open"],
            [12, "account-not-open"]
        ])
    );
    assert_eq!(
        errors("check-directive-accounts"),
        json!([[4, "account-not-open"], [5, "account-not-open"]])
    );
}

#[test]
fn a_second_open_is_a_duplicate_and_the_errors_come_in_date_order() {
    assert_eq!(
        errors("check-open-duplicate"),
        json!([
            [6, "duplicate-open"],
            [7, "duplicate-open"],
            [2, "duplicate-open"],
            [10, "duplicate-open"],
            [3, "duplicate-open"]
        ])
    );
    assert_eq!(
        details("check-open-duplicate", 0),
        json!({ "account": "Assets:C" })
    );
}

#[test]
fn a_close_needs_an_earlier_open_and_happens_once() {
    assert_eq!(
        errors("check-close-errors"),
        json!([
            [6, "close-without-open"],
            [3, "duplicate-close"],
            [4, "close-without-open"],
            [8, "duplicate-close"]
        ])
    );
}

#[test]
fn a_posting_in_a_currency_the_account_does_not_allow_is_reported_at_the_header() {
    assert_eq!(
        errors("check-currency-constraint"),
        json!([
            [6, "currency-not-allowed"],
            [9, "currency-not-allowed"],
            [12, "currency-not-allowed"]
        ])
    );
    assert_eq!(
        details("check-currency-constraint", 0),
        json!({ "account": "Expenses:Food", "currency": "USD", "directive": "transaction" })
    );
    // Cost and price currencies are not checked.
    assert_eq!(errors("check-currency-constraint-ok"), json!([]));
}

#[test]
fn a_balance_checks_its_account_and_its_currency_before_summing() {
    assert_eq!(
        errors("balance-directive-account-errors"),
        json!([
            [6, "currency-not-allowed"],
            [7, "account-not-open"],
            [8, "account-not-open"]
        ])
    );
    assert_eq!(
        details("balance-directive-account-errors", 0)["directive"],
        json!("balance")
    );
}

#[test]
fn assertions_that_differ_from_the_first_of_their_day_are_duplicates() {
    assert_eq!(
        errors("balance-directive-duplicates"),
        json!([
            [11, "assertion-failed"],
            [13, "assertion-failed"],
            [9, "duplicate-assertion"],
            [11, "duplicate-assertion"]
        ])
    );
    assert_eq!(
        details("balance-directive-duplicates", 2),
        json!({
            "account": "Assets:Bank", "currency": "EUR",
            "expected": "-10.001", "first": "-10.00"
        })
    );
}

#[test]
fn a_commodity_declared_twice_is_a_duplicate_at_the_later_directive() {
    assert_eq!(
        errors("check-commodity-duplicate"),
        json!([[2, "duplicate-commodity"], [5, "duplicate-commodity"]])
    );
    assert_eq!(
        details("check-commodity-duplicate", 0),
        json!({ "currency": "EUR" })
    );
}

#[test]
fn pad_account_and_currency_errors_come_in_the_steps_the_reference_runs_them() {
    // The balance step first (17: currency, 19: unknown), then the accounts, then the currencies.
    assert_eq!(
        errors("pad-account-errors"),
        json!([
            [17, "currency-not-allowed"],
            [19, "account-not-open"],
            [10, "account-not-open"],
            [12, "account-not-open"],
            [18, "account-not-open"],
            [14, "currency-not-allowed"],
            [16, "currency-not-allowed"]
        ])
    );
    // The padding transaction is still made.
    assert_eq!(
        load(&[file("c.beancount", &probe("pad-account-errors"))])
            .transactions()
            .len(),
        5
    );
}

#[test]
fn errors_come_in_steps_and_by_date_within_a_step() {
    assert_eq!(
        errors("check-order-dates"),
        json!([
            [18, "account-not-open"],
            [21, "account-not-open"],
            [20, "duplicate-open"],
            [12, "close-without-open"],
            [7, "account-not-open"],
            [13, "account-not-open"],
            [14, "account-not-open"],
            [17, "account-not-open"],
            [11, "account-not-open"],
            [4, "account-not-open"]
        ])
    );
}

#[test]
fn checks_run_after_booking_and_the_does_not_balance_error_is_last() {
    // A booking error (38), the unused pad (33), a failed balance (32), a duplicate open (31),
    // an inactive account (28), a currency (25), a duplicate assertion (24), a duplicate
    // commodity (19) and a transaction that does not balance (15); the parser's own errors
    // (39, 0) stop the later steps, so drop them from this journal.
    let journal: String = probe("check-order-phases")
        .lines()
        .take(38)
        .collect::<Vec<_>>()
        .join("\n");
    let lines: Vec<Value> = findings("c.beancount", &journal)
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] == json!("error"))
        .map(|f| f["line"].clone())
        .collect();
    assert_eq!(
        lines,
        [38, 33, 32, 31, 28, 25, 24, 19, 15]
            .map(|n| json!(n))
            .to_vec()
    );
}
