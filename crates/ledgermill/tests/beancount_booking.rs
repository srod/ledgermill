//! Expected values are what `bean-query` 0.2.0 (Beancount 3.2.3) reports for the probe journals
//! (see spec/beancount/lots-and-booking.md); they are written out by hand, not computed.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::{Value, json};
use support::*;

/// Lines and lots of the postings of the transaction at `index`: `[account, units, line, lot]`.
fn postings_of(rows: &Value, index: usize) -> Vec<Value> {
    rows[index][5]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!([p[0], p[1], p[3], p[5]]))
        .collect()
}

fn sell_two(method: &str) -> Vec<Value> {
    let path = format!("../../spec/beancount/probes/booking-{method}.beancount");
    let journal = std::fs::read_to_string(path).unwrap();
    assert_eq!(findings("b.beancount", &journal), json!([]), "{method}");
    postings_of(&beancount_rows("b.beancount", &journal), 3)
}

#[test]
fn fifo_takes_the_oldest_lots_and_gives_one_posting_per_lot() {
    assert_eq!(
        sell_two("fifo"),
        vec![
            json!(["Assets:Stock", "-1", 15, "10 EUR 2026-01-02"]),
            json!(["Assets:Stock", "-1", 15, "30 EUR 2026-01-03"]),
            json!(["Assets:Bank", "100", 16, null]),
            json!(["Income:Gain", "-60", 17, null]),
        ]
    );
}

#[test]
fn lifo_takes_the_newest_lots() {
    assert_eq!(
        sell_two("lifo"),
        vec![
            json!(["Assets:Stock", "-1", 15, "20 EUR 2026-01-04"]),
            json!(["Assets:Stock", "-1", 15, "30 EUR 2026-01-03"]),
            json!(["Assets:Bank", "100", 16, null]),
            json!(["Income:Gain", "-50", 17, null]),
        ]
    );
}

#[test]
fn hifo_takes_the_dearest_lots() {
    assert_eq!(
        sell_two("hifo"),
        vec![
            json!(["Assets:Stock", "-1", 15, "30 EUR 2026-01-03"]),
            json!(["Assets:Stock", "-1", 15, "20 EUR 2026-01-04"]),
            json!(["Assets:Bank", "100", 16, null]),
            json!(["Income:Gain", "-50", 17, null]),
        ]
    );
}

#[test]
fn the_next_sale_sees_what_the_last_one_left() {
    let journal =
        std::fs::read_to_string("../../spec/beancount/probes/booking-fifo.beancount").unwrap();
    let rows = beancount_rows("b.beancount", &journal);
    assert_eq!(
        postings_of(&rows, 4)[0],
        json!(["Assets:Stock", "-1", 19, "20 EUR 2026-01-04"])
    );
}

#[test]
fn a_method_on_the_open_directive_beats_the_option() {
    let journal = include_str!("../../../spec/beancount/probes/booking-option.beancount");
    let rows = beancount_rows("o.beancount", journal);
    // LIFO from the option takes the lot at 30; FIFO from the open directive takes the lot at 10.
    assert_eq!(postings_of(&rows, 2)[0][3], json!("30 EUR 2026-01-03"));
    assert_eq!(postings_of(&rows, 3)[0][3], json!("10 EUR 2026-01-02"));
}

#[test]
fn strict_is_the_default_and_refuses_an_ambiguous_sale() {
    for probe in [
        include_str!("../../../spec/beancount/probes/booking-default-is-strict.beancount"),
        include_str!("../../../spec/beancount/probes/booking-strict-ambiguous.beancount"),
    ] {
        let found = short_findings("s.beancount", probe);
        assert_eq!(found, json!([[9, "booking-error", "ambiguous-lot-match"]]));
    }
}

#[test]
fn strict_lets_one_lot_match_a_partial_sale_and_selectors_pick_lots() {
    let journal =
        include_str!("../../../spec/beancount/probes/booking-strict-single-lot.beancount");
    assert_eq!(findings("s.beancount", journal), json!([]));
    let rows = beancount_rows("s.beancount", journal);
    assert_eq!(postings_of(&rows, 1)[0][3], json!("10 EUR 2026-01-02"));
    assert_eq!(postings_of(&rows, 3)[0][3], json!("20 EUR 2026-01-11"));
    assert_eq!(postings_of(&rows, 4)[0][3], json!("10 EUR 2026-01-02"));
}

#[test]
fn strict_with_size_takes_the_lot_of_the_same_size() {
    let journal = include_str!("../../../spec/beancount/probes/booking-strict-with-size.beancount");
    let rows = beancount_rows("s.beancount", journal);
    assert_eq!(postings_of(&rows, 2)[0][3], json!("20 EUR 2026-01-03"));
}

#[test]
fn none_never_matches_lots() {
    let journal = include_str!("../../../spec/beancount/probes/booking-none.beancount");
    assert_eq!(findings("n.beancount", journal), json!([]));
    let rows = beancount_rows("n.beancount", journal);
    assert_eq!(postings_of(&rows, 2)[0][3], json!("15 EUR 2026-01-10"));
    assert_eq!(rows[3][5][0][4], json!("20 EUR"));
}

#[test]
fn labels_and_dates_select_lots() {
    let journal = include_str!("../../../spec/beancount/probes/booking-selectors.beancount");
    assert_eq!(findings("l.beancount", journal), json!([]));
    let rows = beancount_rows("l.beancount", journal);
    assert_eq!(
        postings_of(&rows, 3)[0][3],
        json!("10 EUR 2026-01-03 second")
    );
    assert_eq!(postings_of(&rows, 4)[0][3], json!("12 EUR 2026-01-04"));
    // A price on a reduction stays a price.
    assert_eq!(rows[5][5][0][4], json!("20 EUR"));
}

#[test]
fn a_negative_posting_with_a_cost_and_no_lot_adds_a_negative_lot() {
    let journal =
        include_str!("../../../spec/beancount/probes/lots-negative-without-lot.beancount");
    assert_eq!(findings("n.beancount", journal), json!([]));
    let rows = beancount_rows("n.beancount", journal);
    assert_eq!(postings_of(&rows, 0)[0][3], json!("2 BTC 2026-01-02"));
    assert_eq!(postings_of(&rows, 1)[0][3], json!("4 BTC 2026-01-03"));
    assert_eq!(postings_of(&rows, 2)[0][3], json!("2 BTC 2026-01-02"));
}

#[test]
fn average_booking_is_refused_without_a_panic() {
    let journal = include_str!("../../../spec/beancount/probes/refused/average-booking.beancount");
    let l = ledgermill::load(&[file("a.beancount", journal)]);
    assert!(l.transactions().is_empty());
    assert_eq!(
        short_findings("a.beancount", journal),
        json!([[5, "unsupported-syntax", "average-booking"]])
    );
}

#[test]
fn a_large_high_precision_unbalanced_cost_transaction_is_kept_and_reported_unbalanced() {
    let journal = concat!(
        "2026-01-01 open Assets:A\n",
        "2026-01-01 open Assets:Cash\n",
        "2026-01-01 * \"unbalanced\"\n",
        "  Assets:A  10000.0000 AAPL {1.0000 EUR}\n",
        "  Assets:Cash  -1.0000 EUR\n",
    );
    let ledger = ledgermill::load(&[file("u.beancount", journal)]);
    assert_eq!(ledger.transactions().len(), 1);
    assert_eq!(ledger.check().len(), 1);
    assert_eq!(ledger.check()[0].code, "unbalanced-transaction");
    assert_eq!(ledger.check()[0].line, 3);
    assert_eq!(
        ledger.check()[0].details["residual"],
        json!([{ "commodity": "EUR", "quantity": "9999.00000000" }])
    );
}

#[test]
fn a_huge_unbalanced_cost_transaction_is_kept_and_reported_unbalanced() {
    // bean-check 3.2.3 prints `(999999999999999999999999.0000 EUR)` for this journal.
    let journal = concat!(
        "2026-01-01 open Assets:A\n",
        "2026-01-01 open Assets:Cash\n",
        "\n",
        "2026-01-02 * \"big\"\n",
        "  Assets:A  (1000000000000 * 1000000000000) AAPL {1.0000 EUR}\n",
        "  Assets:Cash  -1.0000 EUR\n",
    );
    let ledger = ledgermill::load(&[file("u.beancount", journal)]);
    assert_eq!(ledger.transactions().len(), 1);
    assert_eq!(
        short_findings("u.beancount", journal),
        json!([[4, "unbalanced-transaction", null]])
    );
    assert_eq!(
        ledger.check()[0].details["residual"],
        json!([{ "commodity": "EUR", "quantity": "999999999999999999999999.0000" }])
    );
}

#[test]
fn a_huge_unbalanced_cost_transaction_with_a_negative_residual_or_many_places_is_kept() {
    // (units, cost, cash, residual): each residual is what bean-check 3.2.3 prints for the journal.
    let cases = [
        (
            "-(1000000000000 * 1000000000000)",
            "1.0000",
            "1.0000",
            "-999999999999999999999999.0000",
        ),
        (
            "(1000000000000 * 1000000000000)",
            "1.0000000000",
            "-1.0000000000",
            "999999999999999999999999.0000",
        ),
        (
            "(1000000000000 * 1000000000000)",
            "1",
            "-1",
            "999999999999999999999999",
        ),
    ];
    for (units, cost, cash, residual) in cases {
        let journal = format!(
            "2026-01-01 open Assets:A\n2026-01-01 open Assets:Cash\n\n2026-01-02 * \"big\"\n  Assets:A  {units} AAPL {{{cost} EUR}}\n  Assets:Cash  {cash} EUR\n"
        );
        let ledger = ledgermill::load(&[file("u.beancount", &journal)]);
        assert_eq!(ledger.transactions().len(), 1, "{units} {cost} {cash}");
        assert_eq!(
            ledger.check()[0].details["residual"],
            json!([{ "commodity": "EUR", "quantity": residual }]),
            "{units} {cost} {cash}"
        );
    }
}

/// Four accounts open on lines 1-4, a blank line, so the first transaction starts on line 6.
const OPENED: &str = "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n2026-01-01 open Assets:Cash\n2026-01-01 open Income:X\n\n";
/// About 5e28, which fits `Decimal` (7.9e28), written as an expression because a literal of more
/// than 22 characters is refused. Two of them do not fit.
const HALF_MAX: &str = "(50000000000000 * 1000000000000000)";
/// About 3e28: two of them fit, three do not.
const THIRD_MAX: &str = "(30000000000000 * 1000000000000000)";
/// 1e19: two of them multiplied do not fit `Decimal`.
const TEN_E19: &str = "10000000000000000000";

/// Booking must refuse a figure `Decimal` cannot hold as `number-too-precise`, as it does for
/// the product of two big operands in an expression, instead of panicking inside the booking
/// crate. The reference (bean-check 3.2.3) computes these with no limit.
fn refused_as_too_precise(body: &str, line: u32) {
    let journal = format!("{OPENED}{body}");
    let ledger = ledgermill::load(&[file("o.beancount", &journal)]);
    assert!(ledger.transactions().is_empty(), "{journal}");
    assert_eq!(
        short_findings("o.beancount", &journal),
        json!([[line, "unsupported-syntax", "number-too-precise"]]),
        "{journal}"
    );
}

#[test]
fn units_times_a_cost_per_unit_that_overflows_is_refused_not_a_panic() {
    for (units, cash) in [("", "-1"), ("-", "1")] {
        refused_as_too_precise(
            &format!(
                "2026-01-02 * \"t\"\n  Assets:A  {units}{TEN_E19} AAPL {{{TEN_E19} EUR}}\n  Assets:Cash  {cash} EUR\n"
            ),
            6,
        );
    }
    // A cost written as `per-unit # total` is a per-unit cost too.
    refused_as_too_precise(
        &format!(
            "2026-01-02 * \"t\"\n  Assets:A  {TEN_E19} AAPL {{{TEN_E19} # 1 EUR}}\n  Assets:Cash  -1 EUR\n"
        ),
        6,
    );
}

#[test]
fn units_times_a_price_that_overflows_is_refused_not_a_panic() {
    for (units, cash) in [("", "-1"), ("-", "1")] {
        refused_as_too_precise(
            &format!(
                "2026-01-02 * \"t\"\n  Assets:A  {units}{TEN_E19} AAPL @ {TEN_E19} EUR\n  Assets:Cash  {cash} EUR\n"
            ),
            6,
        );
    }
    // Decimals in both factors still need more than the 96 bits `Decimal` holds.
    refused_as_too_precise(
        "2026-01-02 * \"t\"\n  Assets:A  12345678901234567.89 AAPL @ 12345678901234567.89 EUR\n  Assets:Cash  -1 EUR\n",
        6,
    );
}

#[test]
fn several_weights_whose_sum_overflows_are_refused_not_a_panic() {
    let postings = [
        // plain amounts
        format!(
            "  Assets:A  {HALF_MAX} EUR\n  Assets:B  {HALF_MAX} EUR\n  Assets:Cash  {HALF_MAX} EUR\n"
        ),
        format!(
            "  Assets:A  -{HALF_MAX} EUR\n  Assets:B  -{HALF_MAX} EUR\n  Assets:Cash  -{HALF_MAX} EUR\n"
        ),
        // costs
        format!(
            "  Assets:A  1 AAPL {{{HALF_MAX} EUR}}\n  Assets:B  1 AAPL {{{HALF_MAX} EUR}}\n  Assets:Cash  1 AAPL {{{HALF_MAX} EUR}}\n"
        ),
        // prices
        format!(
            "  Assets:A  1 AAPL @ {HALF_MAX} EUR\n  Assets:B  1 AAPL @ {HALF_MAX} EUR\n  Assets:Cash  1 AAPL @ {HALF_MAX} EUR\n"
        ),
        // a cost and a price
        format!(
            "  Assets:A  2 AAPL {{{HALF_MAX} EUR}}\n  Assets:B  1 MSFT @ {HALF_MAX} EUR\n  Assets:Cash  -1 EUR\n"
        ),
    ];
    for body in postings {
        refused_as_too_precise(&format!("2026-01-02 * \"t\"\n{body}"), 6);
    }
}

#[test]
fn an_elided_posting_that_would_hold_a_figure_that_overflows_is_refused_not_a_panic() {
    let postings = [
        // the elided posting takes the sum of the others
        format!("  Assets:A  {HALF_MAX} EUR\n  Assets:B  {HALF_MAX} EUR\n  Assets:Cash\n"),
        format!("  Assets:A  {TEN_E19} AAPL {{{TEN_E19} EUR}}\n  Assets:Cash\n"),
        format!("  Assets:A  {TEN_E19} AAPL @ {TEN_E19} EUR\n  Assets:Cash\n"),
    ];
    for body in postings {
        refused_as_too_precise(&format!("2026-01-02 * \"t\"\n{body}"), 6);
    }
}

#[test]
fn what_an_account_holds_growing_past_decimal_is_refused_not_a_panic() {
    let day = |d: u32, units: &str, cash: &str| {
        format!("2026-01-0{d} * \"t\"\n  Assets:A  {units}\n  Assets:Cash  {cash}\n\n")
    };
    // Each transaction fits, and so do two of them; the third makes the account hold more than
    // `Decimal` can.
    let cases = [
        (format!("{THIRD_MAX} EUR"), format!("-{THIRD_MAX} EUR")),
        (
            format!("1 AAPL {{{THIRD_MAX} EUR}}"),
            format!("-{THIRD_MAX} EUR"),
        ),
        (
            format!("{THIRD_MAX} AAPL {{1 EUR}}"),
            format!("-{THIRD_MAX} EUR"),
        ),
    ];
    for (units, cash) in cases {
        let body: String = (2..=4).map(|d| day(d, &units, &cash)).collect();
        refused_as_too_precise(&body, 6 + 2 * 4);
    }
}

#[test]
fn a_sale_whose_lots_weigh_more_than_decimal_is_refused_not_a_panic() {
    // Two lots of 5e28 bought with different money: each purchase fits, the sale weighs both.
    let body = format!(
        "2026-01-02 * \"b\"\n  Assets:A  1 AAPL {{{HALF_MAX} EUR}}\n  Assets:Cash  -{HALF_MAX} EUR\n\n\
         2026-01-03 * \"b\"\n  Assets:A  1 AAPL {{{HALF_MAX} EUR, 2026-01-03}}\n  Assets:B  -{HALF_MAX} EUR\n\n\
         2026-01-04 * \"sale\"\n  Assets:A  -2 AAPL {{}}\n  Assets:Cash  1 EUR\n  Income:X\n"
    );
    refused_as_too_precise(&body, 6 + 8);
}

#[test]
fn figures_that_fit_decimal_exactly_are_still_booked() {
    let journal = format!(
        "{OPENED}2026-01-02 * \"t\"\n  Assets:A  1000000000 AAPL {{1000000000 EUR}}\n  Assets:Cash  -1000000000000000000 EUR\n\n\
         2026-01-03 * \"t\"\n  Assets:A  {HALF_MAX} EUR\n  Assets:Cash  -{HALF_MAX} EUR\n"
    );
    assert_eq!(findings("f.beancount", &journal), json!([]));
    assert_eq!(
        transactions("f.beancount", &journal)
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
