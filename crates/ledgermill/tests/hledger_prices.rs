//! `Ledger::prices()`: the `P` lines of the files, sorted by date. Expected values: hledger 1.52.4
//! `prices` (second run, `-c` per rate commodity) on the named probes, `spec/hledger/prices-list.md`.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

macro_rules! probe {
    ($name:literal) => {
        (
            concat!($name, ".journal"),
            include_str!(concat!("../../../spec/hledger/probes/", $name, ".journal")),
        )
    };
}

#[test]
fn only_the_p_lines_are_listed() {
    assert_eq!(
        price_lines(&[probe!("prices-not-from-costs")]),
        ["2026-01-05 ZZZ 2 EUR", "2099-01-01 FUT 2 EUR"]
    );
    assert!(price_lines(&[probe!("prices-empty")]).is_empty());
}

#[test]
fn rows_are_sorted_by_date_and_equal_dates_keep_the_order_written() {
    let (a, b) = (probe!("prices-order-a"), probe!("prices-order-b"));
    assert_eq!(
        price_lines(&[a]),
        [
            "2026-01-01 USD 0.8 EUR",
            "2026-02-01 EUR 1.1 USD",
            "2026-02-01 EUR 1.2 USD",
            "2026-02-01 EUR 1.15 USD",
            "2026-02-01 AAA 1 EUR",
            "2026-02-01 USD 0.5 EUR",
            "2026-02-01 USD 0.5 EUR",
            "2026-02-01 USD 0.50 EUR",
            "2026-03-01 USD 0.9 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[a, b]),
        [
            "2026-01-01 USD 0.8 EUR",
            "2026-01-15 USD 0.85 EUR",
            "2026-02-01 EUR 1.1 USD",
            "2026-02-01 EUR 1.2 USD",
            "2026-02-01 EUR 1.15 USD",
            "2026-02-01 AAA 1 EUR",
            "2026-02-01 USD 0.5 EUR",
            "2026-02-01 USD 0.5 EUR",
            "2026-02-01 USD 0.50 EUR",
            "2026-02-01 GBP 7 EUR",
            "2026-02-01 USD 0.6 EUR",
            "2026-03-01 USD 0.9 EUR",
            "2026-04-01 GBP 1.2 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[b, a])[2..4],
        ["2026-02-01 GBP 7 EUR", "2026-02-01 USD 0.6 EUR"]
    );
}

#[test]
fn a_rate_keeps_the_places_it_was_written_with_and_every_form_of_number() {
    assert_eq!(
        price_lines(&[probe!("prices-rate-forms")]),
        [
            "2026-01-01 USD 0.8 EUR",
            "2026-01-02 USD 0.80 EUR",
            "2026-01-03 USD 0.123456789012345678901234 EUR",
            "2026-01-04 USD 0.001 EUR",
            "2026-01-05 USD 150 EUR",
            "2026-01-06 USD 1234.56 EUR",
            "2026-01-07 USD 1234.5 EUR",
            "2026-01-08 USD -0.5 EUR",
            "2026-01-09 USD 0 EUR",
            "2026-01-10 USD 0.00 EUR",
            "2026-01-11 USD 12345678901234567890.12345 EUR",
            "2026-01-12 EUR 1 EUR",
            "2026-01-13 USD 1 EUR",
            "2026-01-14 USD 0.5 EUR",
            "2026-01-15 USD 1.5 EUR",
            "2026-01-16 USD 1.234 EUR",
            "2026-01-17 USD 1.234 EUR",
            "2026-01-18 USD 0.5 EUR",
            "2026-01-19 USD 0.3333333333333333333333333333333 EUR",
            "2026-01-20 USD 15.0 EUR",
            "2026-01-21 USD 0.000000000000000000000000000001 EUR",
            "2026-01-22 USD 2500 EUR",
            "2026-01-23 USD 10000000000000000000000000000 EUR",
            "2026-01-24 USD 0 EUR",
            "2026-01-25 USD 0.00 EUR",
            "2026-01-26 USD 150 EUR",
            "2026-01-27 USD 123.4 EUR",
            "2026-01-28 USD 1234.5 EUR",
            "2026-01-29 USD 123456789012345678901234567890123456789012345678901234567890.123456789012345678901234567890 EUR",
        ]
    );
}

#[test]
fn a_rate_with_no_commodity_has_an_empty_one_or_the_default_commodity() {
    assert_eq!(
        price_lines(&[probe!("prices-rate-bare")]),
        [
            "2026-01-03 USD 0.8",
            "2026-01-04 USD 1.5",
            "2026-01-05 USD 1.234",
            "2026-01-06 USD 1234.5",
            "2026-01-07 USD 1234.5",
            "2026-01-08 USD 3",
            "2026-01-09 USD 3",
        ]
    );
}

#[test]
fn a_bare_rate_under_d_is_listed_with_the_places_of_d() {
    assert_eq!(
        price_lines(&[probe!("prices-rate-default-commodity")]),
        [
            "2026-01-01 USD 8 EUR",
            "2026-01-02 USD 0.8 EUR",
            "2026-01-03 USD 0.8 GBP",
            "2026-01-04 USD 1234.50 EUR",
        ]
    );
    let sevens = [
        (
            "D 1,000.00 EUR\nP 2026-01-01 USD 7\n",
            "2026-01-01 USD 7.00 EUR",
        ),
        (
            "D 1,000.000 EUR\nP 2026-01-01 USD 1.5\n",
            "2026-01-01 USD 1.500 EUR",
        ),
        (
            "D 1,000.000 EUR\nP 2026-01-01 USD 1.2345\n",
            "2026-01-01 USD 1.2345 EUR",
        ),
        (
            "D 1,000.00 EUR\nP 2026-01-01 USD 7 EUR\n",
            "2026-01-01 USD 7 EUR",
        ),
    ];
    for (journal, expected) in sevens {
        assert_eq!(
            price_lines(&[("d.journal", journal)]),
            [expected],
            "{journal}"
        );
    }
}

#[test]
fn the_number_marks_in_force_before_the_line_read_the_rate() {
    assert_eq!(
        price_lines(&[probe!("prices-rate-commodity-format")]),
        [
            "2026-07-31 USD 8 EUR",
            "2026-07-31 USD 9333 EUR",
            "2026-07-31 USD 0.9333 EUR",
            "2026-07-31 USD 1234.5 EUR",
            "2026-07-31 USD 1234 EUR",
            "2026-07-31 EUR 1.1 USD",
            "2026-07-31 USD 5 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-commodity-after")]),
        ["2026-01-01 USD 0.8 EUR", "2026-01-02 USD 8 EUR"]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-commodity-block")]),
        ["2026-01-01 USD 8 EUR"]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-commodity-from")]),
        [
            "2026-01-01 USD 0.8 EUR",
            "2026-01-02 EUR 8 USD",
            "2026-01-03 USD 1.000 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-decimal-mark-comma")]),
        [
            "2026-01-01 USD 8 EUR",
            "2026-01-02 USD 0.8 EUR",
            "2026-01-03 USD 1234 EUR",
            "2026-01-04 USD 1234.5 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-decimal-mark-point")]),
        [
            "2026-01-01 USD 8 EUR",
            "2026-01-02 USD 1234 EUR",
            "2026-01-03 USD 1234.5 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-scope-a"), probe!("prices-rate-scope-b")]),
        [
            "2026-01-01 USD 0.5 EUR",
            "2026-01-02 USD 1234 EUR",
            "2026-01-03 USD 0.7 EUR",
            "2026-01-04 USD 1.234 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-rate-style-last")]),
        [
            "2026-01-01 USD 0.8 EUR",
            "2026-01-02 USD 15.0 EUR",
            "2026-01-03 USD 123 EUR",
        ]
    );
}

#[test]
fn commodity_names_come_without_their_quotes() {
    assert_eq!(
        price_lines(&[probe!("prices-names")]),
        [
            "2026-01-01 $ 0.9 €",
            "2026-01-02 € 1.1 $",
            "2026-01-03 EUR 1.25 $",
            "2026-01-04 EUR 1.5 $",
            "2026-01-05 ACME INC 12.5 Big Bank Ltd",
            "2026-01-06 ACME INC 12.5 USD",
            "2026-01-07 ACME 3 €",
            "2026-01-08 ACME -3 $",
            "2026-01-09 ACME -3 $",
            "2026-01-10 A1 3 B2",
            "2026-01-11 1x 3 EUR",
            "2026-01-12 VTI 3 EUR",
            "2026-01-13 VTI 3 EUR",
            "2026-01-14 EUR 3 EUR",
            "2026-01-15 USD 5 €",
            "2026-01-16 USD 5 C_D",
            "2026-01-17  3 EUR",
            "2026-01-18 USD 3 EUR",
            "2026-01-19 USD -3 EUR",
            "2026-01-20 USD -3 EUR",
            "2026-01-21 USD 3 a b",
            "2026-01-22 USD 3 a",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-names-prefix-side")]),
        [
            "2026-01-01 USD 3 EUR",
            "2026-01-02 USD 4 EUR",
            "2026-01-03 GBP 5 EUR"
        ]
    );
}

#[test]
fn the_line_forms_hledger_reads_are_listed_and_the_trailing_text_is_dropped() {
    assert_eq!(
        price_lines(&[probe!("prices-line-forms")]),
        [
            "2026-01-01 USD 5 EUR",
            "2026-01-02 USD 5 EUR",
            "2026-01-03 USD 5 EUR",
            "2026-01-04 USD 5 EURjunk",
            "2026-01-05 USD 5 EUR",
            "2026-01-06 USD 5 AB",
            "2026-01-07 USD 5 EUR",
            "2026-01-08 USD 5 EUR",
            "2026-01-09 USD 5 E",
            "2026-01-10 USD 1 EUR",
            "2026-01-11 USD 1 EUR",
            "2026-01-12 USD 1 EUR",
        ]
    );
}

#[test]
fn dates_are_written_in_full_and_a_time_does_not_order_rows() {
    assert_eq!(
        price_lines(&[probe!("prices-dates")]),
        [
            "2026-01-05 USD 4 EUR",
            "2026-01-09 USD 1 EUR",
            "2026-02-03 USD 2 EUR",
            "2026-03-04 USD 3 EUR",
            "2099-01-01 USD 5 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-dates-year")]),
        [
            "2024-12-31 USD 4 EUR",
            "2025-01-09 USD 1 EUR",
            "2025-02-03 USD 2 EUR",
            "2026-01-01 USD 3 EUR",
        ]
    );
    assert_eq!(
        price_lines(&[probe!("prices-dates-time")]),
        [
            "2026-01-08 USD 4 EUR",
            "2026-01-09 USD 1 EUR",
            "2026-01-09 USD 2 EUR",
            "2026-01-09 USD 3 EUR",
        ]
    );
}

#[test]
fn each_row_gives_the_file_as_loaded_and_the_line_of_its_p() {
    let (a, b) = (probe!("prices-order-a"), probe!("prices-order-b"));
    let ledger = load(&[file(a.0, a.1), file(b.0, b.1)]);
    let place = |i: usize| (ledger.prices()[i].file.as_str(), ledger.prices()[i].line);
    // Row 1 is order-b's first P (2026-01-15), row 0 is order-a's 2026-01-01 on line 2.
    assert_eq!(place(0), ("prices-order-a.journal", 2));
    assert_eq!(place(1), ("prices-order-b.journal", 3));
    assert_eq!(
        serde_json::to_value(&ledger.prices()[1]).unwrap(),
        json!({
            "date": "2026-01-15",
            "commodity": "USD",
            "rate": { "commodity": "EUR", "quantity": "0.85" },
            "file": "prices-order-b.journal",
            "line": 3,
        })
    );
}

#[test]
fn a_rate_followed_by_a_price_a_lot_or_a_label_is_refused_at_its_line() {
    let (name, text) = probe!("prices-rate-annotations");
    let ledger = load(&[file(name, text)]);
    let refused: Vec<(u32, &str)> = ledger
        .check()
        .iter()
        .map(|f| {
            assert_eq!(f.code, "unsupported-syntax");
            assert_eq!(f.details["construct"], "price-annotation");
            (f.line, f.file.as_str())
        })
        .collect();
    // Lines 3 to 14 have @ { [ or ( right after the rate (`{}` and a lone `@` too).
    let lines: Vec<u32> = refused.iter().map(|(l, _)| *l).collect();
    assert_eq!(lines, [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
    // The lines that hledger reads as plain junk are listed.
    assert_eq!(
        price_lines(&[(name, text)]),
        [
            "2026-01-13 USD 5 EUR",
            "2026-01-14 USD 5 EUR",
            "2026-01-15 USD 5 EUR",
            "2026-01-16 USD 5 EUR",
            "2026-01-17 USD 5 EUR",
            "2026-01-18 USD 5 EUR",
        ]
    );
}
