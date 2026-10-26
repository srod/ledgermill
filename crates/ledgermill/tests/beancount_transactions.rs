//! Expected values are what `bean-query` 0.2.0 (Beancount 3.2.3) reports for the probe journals
//! (see spec/beancount/*.md); they are written out by hand, not computed.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::json;
use support::*;

#[test]
fn payee_and_narration_stay_apart_and_postings_have_their_own_lines() {
    let journal = include_str!("../../../harness/journals/app/proj008-payee-narration.beancount");
    assert_eq!(
        beancount_rows("a.beancount", journal),
        json!([[
            "2026-01-05",
            "*",
            "Merchant",
            "Narration text",
            4,
            [
                ["Expenses:Food", "10.00", "EUR", 5, null, null],
                ["Assets:Bank", "-10.00", "EUR", 6, null, null]
            ]
        ]])
    );
    assert_eq!(findings("a.beancount", journal), json!([]));
}

#[test]
fn one_string_is_the_narration_and_flags_are_kept_literally() {
    let journal = include_str!("../../../spec/beancount/probes/header-flags.beancount");
    let rows = beancount_rows("flags.beancount", journal);
    let flags: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| json!([r[1], r[2], r[3]]))
        .collect();
    assert_eq!(
        flags,
        vec![
            json!(["*", null, "star"]),
            json!(["!", null, "bang"]),
            json!(["*", null, "txn word reads as a star"]),
            json!(["#", null, "hash"]),
            json!(["P", null, "letter P"]),
            json!(["S", null, "letter S"]),
            json!(["&", null, "ampersand"]),
            json!(["?", null, "question mark"]),
            json!(["%", null, "percent"]),
        ]
    );
}

#[test]
fn status_follows_the_flag() {
    let journal = include_str!("../../../spec/beancount/probes/header-flags.beancount");
    let l = ledgermill::load(&[file("flags.beancount", journal)]);
    let statuses: Vec<_> = l
        .transactions()
        .iter()
        .map(|t| serde_json::to_value(t.status).unwrap())
        .collect();
    assert_eq!(
        statuses,
        vec![
            json!("*"),
            json!("!"),
            json!("*"),
            json!(""),
            json!(""),
            json!(""),
            json!(""),
            json!(""),
            json!("")
        ]
    );
}

#[test]
fn a_posting_keeps_its_own_flag() {
    let journal = include_str!("../../../spec/beancount/probes/posting-flags.beancount");
    let l = ledgermill::load(&[file("p.beancount", journal)]);
    let flags: Vec<Vec<Option<&str>>> = l
        .transactions()
        .iter()
        .map(|t| t.postings.iter().map(|p| p.flag.as_deref()).collect())
        .collect();
    assert_eq!(
        flags,
        vec![
            vec![Some("!"), None],
            vec![Some("*"), Some("#"), Some("P"), Some("&")],
            vec![None, Some("!")],
        ]
    );
}

#[test]
fn a_unit_price_is_per_unit_and_a_total_price_is_divided() {
    let journal =
        include_str!("../../../harness/journals/app/verbatim-price-annotations.beancount");
    let rows = beancount_rows("p.beancount", journal);
    assert_eq!(
        rows[0][5][0],
        json!(["Assets:Btc", "1", "BTC", 5, "100.00 EUR", null])
    );
    assert_eq!(
        rows[1][5][0],
        json!(["Assets:Btc", "2", "BTC", 9, "100.50 EUR", null])
    );
}

#[test]
fn a_lot_cost_has_its_date_and_label() {
    let journal = include_str!("../../../spec/beancount/probes/lots-costs.beancount");
    let rows = beancount_rows("l.beancount", journal);
    let lots: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[5][0][5].clone())
        .collect();
    assert_eq!(
        lots,
        vec![
            json!("100 EUR 2026-01-02"),
            json!("100.00 EUR 2026-01-03"),
            json!("3.333333333333333333333333333 EUR 2026-01-04"),
            json!("101.25 EUR 2026-01-05"),
            json!("1.50 EUR 2025-12-31 lbl"),
            json!("2 EUR 2026-01-07 a;b"),
        ]
    );
}

#[test]
fn a_transaction_without_postings_is_kept() {
    let rows = beancount_rows(
        "z.beancount",
        include_str!("../../../harness/journals/app/zero-posting.beancount"),
    );
    assert_eq!(
        rows,
        json!([["2026-01-06", "*", null, "Zero posting", 1, []]])
    );
}

#[test]
fn transactions_come_by_date_then_line_with_one_file_name() {
    let journal = include_str!("../../../spec/beancount/probes/ordering.beancount");
    let rows = beancount_rows("o.beancount", journal);
    let order: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| json!([r[0], r[4]]))
        .collect();
    assert_eq!(
        order,
        vec![
            json!(["2026-01-01", 13]),
            json!(["2026-01-05", 4]),
            json!(["2026-01-05", 7]),
            json!(["2026-01-05", 16]),
            json!(["2026-03-01", 1]),
        ]
    );
    let l = ledgermill::load(&[file("o.beancount", journal)]);
    assert_eq!(l.transactions()[0].source.file, "o.beancount");
}

#[test]
fn an_elided_amount_gets_the_places_of_the_other_amounts() {
    let journal = include_str!("../../../spec/beancount/probes/interpolation.beancount");
    let rows = beancount_rows("i.beancount", journal);
    let elided = |n: usize| rows[n][5].as_array().unwrap().last().unwrap()[1].clone();
    assert_eq!(elided(0), json!("-6.67"));
    assert_eq!(elided(1), json!("-3.370370367"));
    assert_eq!(elided(2), json!("-6.667"));
    assert_eq!(elided(3), json!("-2.246913"));
    assert_eq!(elided(4), json!("-15"));
    assert_eq!(elided(5), json!("-1000.5"));
    assert_eq!(elided(6), json!("-3.00"));
}

#[test]
fn a_quotient_with_exponent_prints_like_the_reference() {
    let journal = include_str!("../../../spec/beancount/probes/prices-exponent.beancount");
    let rows = beancount_rows("e.beancount", journal);
    let price = |n: usize| rows[n][5][0][4].clone();
    assert_eq!(price(0), json!("2.0E+2 EUR"));
    assert_eq!(price(1), json!("4E+2 EUR"));
    assert_eq!(price(2), json!("2.1E+2 EUR"));
    assert_eq!(price(3), json!("2.00E+4 EUR"));
    assert_eq!(price(4), json!("15 EUR"));
    assert_eq!(price(5), json!("0E+1 EUR"));
    assert_eq!(price(6), json!("10.0 EUR"));
    // A cost from a total is plain.
    assert_eq!(rows[7][5][0][5], json!("200 EUR 2026-01-09"));
    assert_eq!(rows[8][5][0][4], json!("1E-7 EUR"));
}

#[test]
fn an_elided_posting_is_split_by_currency() {
    let journal =
        include_str!("../../../spec/beancount/probes/interpolation-several-currencies.beancount");
    let rows = beancount_rows("s.beancount", journal);
    let quantities: Vec<_> = rows[0][5]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!([p[1], p[2], p[3]]))
        .collect();
    assert_eq!(
        quantities,
        vec![
            json!(["10.00", "EUR", 5]),
            json!(["-3.3333", "EUR", 6]),
            json!(["-6.67", "EUR", 8]),
            json!(["5.5", "USD", 7]),
            json!(["-5.5", "USD", 8]),
        ]
    );
}

#[test]
fn an_elided_amount_that_rounds_to_zero_is_kept_with_its_sign() {
    // interpolation-rounds-to-zero: only an exact zero drops the elided posting.
    let journal =
        include_str!("../../../spec/beancount/probes/interpolation-rounds-to-zero.beancount");
    let rows = beancount_rows("z.beancount", journal);
    let postings = |n: usize| -> Vec<_> {
        rows[n][5]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!([p[0], p[1], p[2]]))
            .collect()
    };
    assert_eq!(
        postings(0),
        vec![
            json!(["Assets:A", "1.004", "EUR"]),
            json!(["Assets:B", "-1.00", "EUR"]),
            json!(["Assets:C", "-0.00", "EUR"]),
        ]
    );
    assert_eq!(postings(1)[2], json!(["Assets:C", "0.00", "EUR"]));
    assert_eq!(postings(2)[2], json!(["Assets:C", "-0.00", "EUR"]));
    assert_eq!(postings(3).len(), 2);
    assert_eq!(
        postings(4),
        vec![
            json!(["Assets:A", "1.004", "EUR"]),
            json!(["Assets:B", "-1.00", "EUR"]),
            json!(["Assets:C", "-0.00", "EUR"]),
            json!(["Assets:A", "1", "USD"]),
            json!(["Assets:B", "-1", "USD"]),
        ]
    );
}

#[test]
fn a_sum_that_reaches_zero_forgets_the_places_of_its_terms() {
    // interpolation-sum-through-zero: the elided posting of each transaction is its last row.
    let journal =
        include_str!("../../../spec/beancount/probes/interpolation-sum-through-zero.beancount");
    let rows = beancount_rows("t.beancount", journal);
    let elided = |n: usize| rows[n][5].as_array().unwrap().last().unwrap()[1].clone();
    assert_eq!(elided(0), json!("-7"));
    assert_eq!(elided(1), json!("-7"));
    assert_eq!(elided(2), json!("-3.0"));
    assert_eq!(elided(3), json!("-7.0"));
    assert_eq!(elided(4), json!("-6.0"));
}
