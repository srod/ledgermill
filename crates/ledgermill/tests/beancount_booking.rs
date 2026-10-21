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
