//! A lot cost or a price left to balancing. Expected values are what `bean-check` / `bean-query`
//! 3.2.3 report for the probe journals (spec/beancount/inferred-cost-and-price.md); they are
//! written out by hand, not computed.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use std::path::Path;
use support::*;

fn probe(name: &str) -> String {
    let path = Path::new("../../spec/beancount/probes").join(name);
    std::fs::read_to_string(path).unwrap()
}

/// `[account, units, price, lot]` of every posting of the transaction at `index`.
fn postings_of(name: &str, index: usize) -> Vec<Value> {
    let rows = beancount_rows("b.beancount", &probe(name));
    rows[index][5]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!([p[0], p[1], p[4], p[5]]))
        .collect()
}

/// `[code, reason-or-construct]` of each finding.
fn codes(name: &str) -> Vec<Value> {
    short_findings("b.beancount", &probe(name))
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f[1], f[2]]))
        .collect()
}

fn no_transaction(name: &str) {
    let loaded = load(&[file("b.beancount", &probe(name))]);
    assert!(loaded.transactions().is_empty(), "{name}");
}

#[test]
fn a_cost_left_out_is_what_balances_the_payment() {
    for name in [
        "lots-inferred-cost.beancount",
        "lots-inferred-cost-currency.beancount",
        "lots-inferred-cost-two-payments.beancount",
    ] {
        let postings = postings_of(name, 0);
        assert_eq!(postings[0][3], json!("12.50 EUR 2026-01-02"), "{name}");
        assert_eq!(postings[0][1], json!("4"), "{name}");
        assert_eq!(findings("b.beancount", &probe(name)), json!([]), "{name}");
    }
}

#[test]
fn the_date_and_the_label_are_kept_as_written() {
    assert_eq!(
        postings_of("lots-inferred-cost-date-and-label.beancount", 0)[0][3],
        json!("12.50 EUR 2025-12-30 x")
    );
}

#[test]
fn an_inexact_cost_has_28_significant_digits() {
    let name = "lots-inferred-cost-division.beancount";
    assert_eq!(
        postings_of(name, 0)[0][3],
        json!("3.333333333333333333333333333 EUR 2026-01-02")
    );
    assert_eq!(codes(name), Vec::<Value>::new());
    // With a payment written without places, the 1e-27 residual shows.
    assert_eq!(
        codes("lots-inferred-cost-integers.beancount"),
        vec![json!(["unbalanced-transaction", null])]
    );
}

#[test]
fn a_cost_with_nothing_to_balance_is_zero() {
    assert_eq!(
        postings_of("lots-inferred-cost-zero-residual.beancount", 0)[0][3],
        json!("0 EUR 2026-01-02")
    );
    let name = "lots-inferred-cost-currency-without-payment.beancount";
    assert_eq!(postings_of(name, 0)[0][3], json!("0 EUR 2026-01-02"));
    assert_eq!(codes(name), vec![json!(["unbalanced-transaction", null])]);
    assert_eq!(
        codes("lots-inferred-cost-unbalanced.beancount"),
        vec![json!(["unbalanced-transaction", null])]
    );
}

#[test]
fn the_price_is_kept_and_other_prices_count_by_their_weight() {
    let with_price = postings_of("lots-inferred-cost-with-price.beancount", 0);
    assert_eq!(with_price[0][2], json!("13 EUR"));
    assert_eq!(with_price[0][3], json!("12.50 EUR 2026-01-02"));
    let currency = postings_of("lots-inferred-cost-currency-with-price.beancount", 0);
    assert_eq!(currency[0][3], json!("12.50 EUR 2026-01-02"));
    let paid = postings_of("lots-inferred-cost-paid-with-price.beancount", 0);
    assert_eq!(paid[0][3], json!("12.50 EUR 2026-01-02"));
}

#[test]
fn the_cost_currency_is_the_written_one_then_the_price_one_then_the_only_other() {
    let name = "lots-inferred-cost-price-currency.beancount";
    assert_eq!(postings_of(name, 0)[0][3], json!("0 USD 2026-01-02"));
    assert_eq!(codes(name), vec![json!(["unbalanced-transaction", null])]);
    let paid = "lots-inferred-cost-price-currency-paid.beancount";
    assert_eq!(postings_of(paid, 0)[0][3], json!("12.50 USD 2026-01-02"));
    assert_eq!(codes(paid), Vec::<Value>::new());
}

#[test]
fn negative_units_add_a_negative_lot_with_a_positive_cost() {
    let postings = postings_of("lots-inferred-cost-negative-units.beancount", 0);
    assert_eq!(postings[0][1], json!("-4"));
    assert_eq!(postings[0][3], json!("12.50 EUR 2026-01-02"));
}

#[test]
fn a_negative_cost_is_an_error() {
    let name = "lots-inferred-cost-negative-result.beancount";
    assert_eq!(codes(name), vec![json!(["booking-error", "negative-cost"])]);
}

#[test]
fn a_later_sale_reduces_inferred_lots_with_their_digits() {
    let rows = postings_of("lots-inferred-cost-then-sale.beancount", 2);
    assert_eq!(rows[0][3], json!("12.50 EUR 2026-01-02"));
    assert_eq!(
        rows[1][3],
        json!("3.333333333333333333333333333 EUR 2026-01-03")
    );
}

#[test]
fn equal_lots_show_the_digits_of_the_first_one() {
    let rows = postings_of("lots-inferred-cost-equal-lots.beancount", 3);
    assert_eq!(rows[0][3], json!("12.50 EUR 2026-01-02"));
}

#[test]
fn what_cannot_be_told_apart_is_an_error_and_no_transaction() {
    for name in [
        "lots-inferred-cost-two-currencies.beancount",
        "lots-inferred-cost-elided-posting.beancount",
        "lots-inferred-cost-two-lots.beancount",
    ] {
        let found = codes(name);
        assert!(
            found.iter().any(|f| f[0] == json!("booking-error")),
            "{name}: {found:?}"
        );
    }
    assert_eq!(
        load(&[file(
            "b.beancount",
            &probe("lots-inferred-cost-elided-posting.beancount")
        )])
        .transactions()
        .len(),
        0
    );
}

#[test]
fn zero_units_leave_the_payment_unbalanced() {
    assert_eq!(
        codes("lots-inferred-cost-zero-units.beancount"),
        vec![json!(["unbalanced-transaction", null])]
    );
}

#[test]
fn a_price_left_out_is_what_balances_the_payment() {
    let price = |name: &str| postings_of(name, 0)[0][2].clone();
    assert_eq!(
        price("prices-inferred-number.beancount"),
        json!("12.50 EUR")
    );
    assert_eq!(price("prices-inferred-total.beancount"), json!("12.50 EUR"));
    assert_eq!(price("prices-inferred-bare.beancount"), json!("12.50 EUR"));
    assert_eq!(
        price("prices-inferred-currency.beancount"),
        json!("12.5 EUR")
    );
    assert_eq!(
        price("prices-inferred-total-number-only.beancount"),
        json!("12.5 EUR")
    );
    assert_eq!(
        price("prices-inferred-division.beancount"),
        json!("3.333333333333333333333333333 EUR")
    );
    assert_eq!(
        price("prices-inferred-negative-units.beancount"),
        json!("12.50 EUR")
    );
    assert_eq!(
        postings_of("prices-inferred-conversion.beancount", 0)[0][2],
        json!("0.9 EUR")
    );
}

#[test]
fn a_price_that_cannot_balance_is_unbalanced() {
    for name in [
        "prices-inferred-wrong-sign.beancount",
        "prices-inferred-other-currency.beancount",
        "prices-inferred-two-currencies.beancount",
    ] {
        assert_eq!(
            codes(name),
            vec![json!(["unbalanced-transaction", null])],
            "{name}"
        );
    }
    assert_eq!(
        postings_of("prices-inferred-wrong-sign.beancount", 0)[0][2],
        json!("12.50 EUR")
    );
    assert_eq!(
        postings_of("prices-inferred-other-currency.beancount", 0)[0][2],
        json!("0 USD")
    );
}

#[test]
fn a_price_with_two_candidate_currencies_is_an_error() {
    let found = codes("prices-inferred-currency-two-groups.beancount");
    assert!(found.iter().any(|f| f[0] == json!("booking-error")));
}

#[test]
fn a_price_to_work_out_next_to_a_cost_stays_refused() {
    let name = "refused/inferred-price-with-cost.beancount";
    assert!(
        codes(name).contains(&json!(["unsupported-syntax", "inferred-price"])),
        "{:?}",
        codes(name)
    );
    no_transaction(name);
}

#[test]
fn zero_units_with_a_price_to_work_out_are_refused_without_a_guess() {
    let journal = "2026-01-01 open Assets:Stock BTC\n2026-01-01 open Assets:Bank EUR\n\n\
2026-01-02 * \"x\"\n  Assets:Stock  0 BTC @ EUR\n  Assets:Bank  -5.00 EUR\n";
    assert_eq!(
        short_findings("b.beancount", journal),
        json!([[4, "unsupported-syntax", "inferred-price"]])
    );
}

#[test]
fn a_comma_before_the_label_and_empty_double_braces_read_as_open_costs() {
    let comma = postings_of("lots-inferred-cost-label-comma.beancount", 0);
    assert_eq!(comma[0][3], json!("12.50 EUR 2026-01-02 lot1"));
    let double = postings_of("lots-inferred-cost-double-braces.beancount", 0);
    assert_eq!(double[0][3], json!("12.50 EUR 2026-01-02"));
}

#[test]
fn a_cost_with_one_part_of_the_hash_missing_is_refused() {
    for name in [
        "refused/cost-partial-per-unit.beancount",
        "refused/cost-partial-total.beancount",
    ] {
        assert!(
            codes(name).contains(&json!(["unsupported-syntax", "cost-partial"])),
            "{name}: {:?}",
            codes(name)
        );
        no_transaction(name);
    }
}

#[test]
fn a_total_cost_written_in_full_is_not_partial() {
    let journal = "2026-01-01 open Assets:Stock BTC\n2026-01-01 open Assets:Bank EUR\n\n\
2026-01-02 * \"x\"\n  Assets:Stock  4 BTC {{50.00 EUR}}\n  Assets:Bank  -50.00 EUR\n\
2026-01-03 * \"y\"\n  Assets:Stock  4 BTC {1.00 # 48.00 EUR}\n  Assets:Bank  -52.00 EUR\n";
    assert_eq!(findings("b.beancount", journal), json!([]));
}
