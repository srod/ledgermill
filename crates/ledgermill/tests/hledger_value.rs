//! `Ledger::value(date, commodity)`: amounts valued at the prices of a date. Expected values:
//! hledger 1.52.4 `print -O json --value=DATE,COMM` on the named probes
//! (spec/hledger/value-prices.md and value-arithmetic.md; the lines are the short form of
//! `spec/hledger/show-amounts.sh`).
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

/// `value` on one probe: the lines it gives.
fn value(probe: (&str, &str), date: &str, commodity: &str) -> Vec<String> {
    valued(&[probe], date, commodity)
}

/// The same line is expected on every date or commodity given.
fn assert_lines(probe: (&str, &str), cases: &[(&str, &str, &str)]) {
    for (date, commodity, expected) in cases {
        assert_eq!(
            value(probe, date, commodity),
            [(*expected).to_string()],
            "{} on {date} in {commodity}",
            probe.0
        );
    }
}

// ───────────── which P lines exist on a date ─────────────

#[test]
fn the_latest_price_on_or_before_the_date_wins() {
    assert_lines(
        probe!("value-p-latest"),
        &[
            ("2025-12-31", "EUR", "2026-02-01  a: X 1"),
            ("2026-01-01", "EUR", "2026-02-01  a: EUR 2"),
            ("2026-01-04", "EUR", "2026-02-01  a: EUR 7"),
            ("2026-01-05", "EUR", "2026-02-01  a: EUR 4"),
            ("2026-01-08", "EUR", "2026-02-01  a: EUR 4"),
            ("2026-01-09", "EUR", "2026-02-01  a: EUR 5"),
        ],
    );
}

#[test]
fn a_time_of_day_on_a_price_line_is_ignored() {
    let p = probe!("value-p-time");
    assert_lines(
        p,
        &[
            ("2026-01-05", "EUR", "2026-01-01  a: EUR 4"),
            ("2026-01-06", "EUR", "2026-01-01  a: EUR 7"),
        ],
    );
}

#[test]
fn prices_of_every_file_count_and_the_file_order_breaks_a_tie() {
    let (a, b) = (probe!("value-p-files-a"), probe!("value-p-files-b"));
    let run = |files: [(&str, &str); 2], date: &str| valued(&files, date, "EUR");
    assert_eq!(run([a, b], "2026-01-04"), ["2026-01-02  a: EUR 8"]);
    assert_eq!(run([b, a], "2026-01-04"), ["2026-01-02  a: EUR 8"]);
    assert_eq!(run([a, b], "2026-01-05"), ["2026-01-02  a: EUR 4"]);
    assert_eq!(run([b, a], "2026-01-05"), ["2026-01-02  a: EUR 3"]);
}

#[test]
fn each_direction_of_a_pair_is_a_price_of_its_own() {
    assert_lines(
        probe!("value-p-pair-direction"),
        &[
            ("2026-01-01", "X", "2026-01-04  a: X 1 | b: X 0.5"),
            ("2026-01-02", "X", "2026-01-04  a: X 1 | b: X 0.25"),
            ("2026-01-03", "X", "2026-01-04  a: X 1 | b: X 0.25"),
            ("2026-01-03", "EUR", "2026-01-04  a: EUR 3 | b: EUR 1"),
        ],
    );
}

#[test]
fn zero_negative_and_self_prices() {
    assert_lines(
        probe!("value-p-zero-negative-self"),
        &[
            (
                "2026-01-02",
                "EUR",
                "2026-01-02  a: EUR 0 | a: EUR -10 | a: Z 5 | a: EUR 5 | a: EUR 20",
            ),
            (
                "2026-01-02",
                "X",
                "2026-01-02  a: X 5 | a: X 0 | a: Z 5 | a: X 0 | a: X 0",
            ),
            (
                "2026-01-02",
                "Y",
                "2026-01-02  a: Y 0 | a: Y 5 | a: Z 5 | a: Y -2.5 | a: Y -10",
            ),
            (
                "2026-01-02",
                "Z",
                "2026-01-02  a: X 5 | a: Y 5 | a: Z 5 | a: EUR 5 | a: V 5",
            ),
        ],
    );
}

#[test]
fn a_price_without_commodity_is_in_the_default_commodity_or_the_empty_one() {
    assert_lines(
        probe!("value-p-no-commodity"),
        &[("2026-01-02", "X", "2026-01-02  a: X 1.5 | b: X 3")],
    );
    assert_lines(
        probe!("value-p-default-commodity"),
        &[("2026-01-02", "EUR", "2026-01-02  a: EUR 6 | b: EUR 1")],
    );
}

#[test]
fn a_price_keeps_its_places_and_only_the_result_is_rounded() {
    assert_lines(
        probe!("value-p-precision"),
        &[(
            "2026-01-02",
            "EUR",
            "2026-01-02  a: EUR 0.1234567890 | b: EUR 123456.7890123457 | c: EUR 0.0000000000",
        )],
    );
}

#[test]
fn a_quoted_commodity_is_named_without_its_quotes() {
    let p = probe!("value-p-quoted");
    assert_lines(
        p,
        &[
            ("2026-01-02", "E F", "2026-01-02  a: E F 2 | b: E F 1"),
            ("2026-01-02", "€", "2026-01-02  a: € 6 | b: € 3"),
        ],
    );
    // With the quotes it is another name, which nothing is priced in.
    let quoted = value(p, "2026-01-02", "\"E F\"");
    assert!(quoted[0].contains("a: X "), "{quoted:?}");
}

// ───────────── the path ─────────────

#[test]
fn a_forward_path_however_long_beats_an_inverse_edge() {
    assert_lines(
        probe!("value-path-forward-first"),
        &[
            ("2026-01-03", "D", "2026-01-03  a: D 30 | b: D 1"),
            ("2026-01-03", "A", "2026-01-03  a: A 1 | b: A 7"),
        ],
    );
}

#[test]
fn the_shortest_forward_path_wins_then_the_smaller_names() {
    assert_lines(
        probe!("value-path-forward-shortest"),
        &[
            ("2026-01-02", "E", "2026-01-02  x: E 5"),
            ("2026-01-02", "G", "2026-01-02  x: G 77"),
            ("2026-01-02", "H", "2026-01-02  x: H 437"),
        ],
    );
}

#[test]
fn without_a_forward_path_the_shortest_path_with_inverse_edges_wins() {
    assert_lines(
        probe!("value-path-inverse-shortest"),
        &[
            ("2026-01-02", "D", "2026-01-02  x: D 0.1428571429"),
            ("2026-01-02", "F", "2026-01-02  x: F 0.0069930070"),
            ("2026-01-02", "C", "2026-01-02  x: C 0.1666666667"),
        ],
    );
}

#[test]
fn forward_neighbours_are_tried_before_inverse_ones_whatever_the_names() {
    assert_lines(
        probe!("value-path-mixed"),
        &[("2026-01-02", "D", "2026-01-02  x: D 0.4")],
    );
    assert_lines(
        probe!("value-path-mixed-names"),
        &[("2026-01-02", "M", "2026-01-02  x: M 0.6666666667")],
    );
    assert_lines(
        probe!("value-path-mixed-ties"),
        &[("2026-01-02", "D", "2026-01-02  x: D 2.5")],
    );
    assert_lines(
        probe!("value-path-mixed-both-directions"),
        &[("2026-01-02", "C", "2026-01-02  x: C 0.6666666667")],
    );
}

#[test]
fn cycles_do_not_matter_and_an_unreachable_amount_keeps_its_cost_and_lot() {
    assert_lines(
        probe!("value-path-cycle-and-none"),
        &[
            (
                "2026-01-02",
                "C",
                "2026-01-02  x: C 6 | x: C 1 | x: K 4 @ L 3 | x: K 2 {L 4} | x: Q 1",
            ),
            (
                "2026-01-02",
                "L",
                "2026-01-02  x: A 1 | x: C 1 | x: L 8 | x: L 4 | x: Q 1",
            ),
        ],
    );
}

// ───────────── inverse rates ─────────────

#[test]
fn an_inverse_rate_has_255_places_and_is_not_shortened() {
    let p = probe!("value-inverse");
    assert_lines(
        p,
        &[
            (
                "2026-01-02",
                "X",
                "2026-01-02  a: X 0.3333333333 | b: X 1.0000000000 | c: X 3.0000000000 | d: X 0.6666666667 | e: X 0.3 | f: X 333333.3333333333",
            ),
            (
                "2026-01-02",
                "Y",
                "2026-01-02  a: Y 0.125 | b: Y 0.375 | c: Y 1.125 | d: Y 0.25 | e: Y 0.1125 | f: Y 125000",
            ),
            (
                "2026-01-02",
                "W",
                "2026-01-02  a: W 0 | b: W 0 | c: W 0 | d: W 0 | e: W 0 | f: W 0",
            ),
            (
                "2026-01-02",
                "V",
                "2026-01-02  a: V 4.8309178744 | b: V 14.4927536232 | c: V 43.4782608696 | d: V 9.6618357488 | e: V 4.3478260870 | f: V 4830917.8743961353",
            ),
            (
                "2026-01-02",
                "U",
                "2026-01-02  a: U 0.1428571429 | b: U 0.4285714286 | c: U 1.2857142857 | d: U 0.2857142857 | e: U 0.1285714286 | f: U 142857.1428571429",
            ),
        ],
    );
}

#[test]
fn two_inverse_edges_multiply_their_rounded_rates() {
    assert_lines(
        probe!("value-inverse-two-hops"),
        &[(
            "2026-01-02",
            "Y",
            "2026-01-02  a: Y 0.1111111111 | b: Y 0.1 | c: Y 1.0000000000 | d: Y 3.0000000000",
        )],
    );
}

// ───────────── arithmetic ─────────────

#[test]
fn a_result_is_rounded_half_to_even_at_ten_places() {
    assert_lines(
        probe!("value-round-half-even"),
        &[(
            "2026-01-02",
            "EUR",
            "2026-01-02  a: EUR 0.0000000000 | b: EUR 0.0000000002 | c: EUR 0.0000000002 | d: EUR 0.0000000004 | e: EUR -0.0000000002 | f: EUR -0.0000000002 | g: EUR 0.0000000003 | h: EUR 0.1234567890 | i: EUR 0.1234567892",
        )],
    );
}

#[test]
fn trailing_zeros_of_a_converted_amount_go_and_an_unconverted_one_keeps_its_places() {
    assert_lines(
        probe!("value-zero-and-places"),
        &[(
            "2026-01-02",
            "EUR",
            "2026-01-02  a: EUR 3 | b: EUR 0.625 | c: EUR 0 | d: EUR 0 | e: EUR 0.0000000000 | f: EUR 2.500 | g: EUR 0.00 | h: EUR 50 | i: EUR 0.0000000000",
        )],
    );
}

#[test]
fn the_rate_of_a_path_is_folded_from_the_amount_side_and_the_amount_comes_last() {
    // 247 digits before the point: 41 repeats of 142857 and the 1 that starts the next.
    let got = value(probe!("value-fold-order"), "2026-01-02", "D");
    assert_eq!(got.len(), 1);
    let digits = got[0].strip_prefix("2026-01-02  x: D ").unwrap();
    let (whole, places) = digits.split_once('.').unwrap();
    assert_eq!(whole, format!("{}1", "142857".repeat(41)));
    assert_eq!(places, "4285714286");
}

#[test]
fn a_cost_and_a_lot_are_no_prices_and_go_with_a_conversion() {
    let p = probe!("value-amount-forms");
    assert_lines(
        p,
        &[
            (
                "2026-01-02",
                "EUR",
                "2026-01-02  a: EUR 6 | b: EUR 6 | c: EUR 6 | d: EUR 6 | e: EUR 6 | f: EUR 3 @ USD 4 | g: Y 3 @ EUR 4 | h:  3 | i: EUR 3 {USD 4}",
            ),
            (
                "2026-01-02",
                "USD",
                "2026-01-02  a: USD 6.6 | b: USD 6.6 | c: USD 6.6 | d: USD 6.6 | e: USD 6.6 | f: USD 3.3 | g: Y 3 @ EUR 4 | h:  3 | i: USD 3.3",
            ),
            (
                "2026-01-02",
                "X",
                "2026-01-02  a: X 3 @ EUR 4 | b: X 3 @@ EUR 12 | c: X 3 {EUR 4} | d: X 3 @ EUR 5 {EUR 4} | e: X 3 {EUR 4} | f: X 1.5 | g: Y 3 @ EUR 4 | h:  3 | i: X 1.5",
            ),
        ],
    );
}

// ───────────── the amounts of a posting ─────────────

#[test]
fn the_amounts_of_a_posting_are_added_at_full_precision_then_rounded_once() {
    let p = probe!("value-merge-sum");
    assert_eq!(
        short_lines(load(&[file(p.0, p.1)]).transactions()),
        ["2026-01-02  a: T 1 | a: U 1 | a: V 1 | b: T -1, U -1, V -1"]
    );
    assert_lines(
        p,
        &[(
            "2026-01-02",
            "X",
            "2026-01-02  a: X 0.3333333333 | a: X 0.3333333333 | a: X 0.3333333333 | b: X -1.0000000000",
        )],
    );
}

#[test]
fn merged_amounts_are_ordered_by_commodity_with_the_empty_one_first() {
    let got = value(probe!("value-merge-order"), "2026-01-02", "EUR");
    assert_eq!(
        got,
        [
            "2026-01-02  x: EUR 1 | x: EUR 2 | x: C 1 | x: EUR 1 | x: EUR 3 | x: Z 1 | x: a 1 | y: C -1, EUR -7, Z -1, a -1",
            "2026-01-02  x: EUR 2 | x:  4 | y:  -4, EUR -2",
        ]
    );
}

#[test]
fn a_sum_keeps_the_places_of_its_more_precise_term_and_a_zero_gives_the_other_back() {
    assert_eq!(
        value(probe!("value-merge-zero-places"), "2026-01-02", "M"),
        [
            "2026-01-02  x: M -1.25 | x: M 1.25 | x: M -3 | y: M 3",
            "2026-01-02  x: M -3 | x: M -1.25 | x: M 1.25 | y: M 3.00",
            "2026-01-02  x: M 0 | x: M 0.00 | y: M 0",
            "2026-01-02  x: M 0.00 | x: M 0 | y: M 0.00",
        ]
    );
}

#[test]
fn a_lot_stays_only_on_an_amount_alone_in_its_commodity_and_not_converted() {
    assert_eq!(
        value(probe!("value-lots-merge"), "2026-01-02", "EUR"),
        [
            "2026-01-02  a: EUR 5 {USD 1} | b: EUR -5 {USD 1}",
            "2026-01-02  a: EUR 5 {USD 1} | a: EUR 6 | b: EUR -11",
            "2026-01-02  a: EUR 5 {USD 1} | a: EUR 6 | a: EUR -6 | b: EUR -5",
            "2026-01-02  a: Q 5 {USD 1} | a: EUR 6 | b: EUR -6, Q -5 {USD 1}",
            "2026-01-02  a: EUR 10 | b: EUR -10",
            "2026-01-02  a: EUR 5 {USD 1} | a: EUR 2 | b: EUR -7",
        ]
    );
}

#[test]
fn an_amount_left_as_it_is_keeps_its_cost_and_is_not_added_to_a_converted_one() {
    let p = probe!("value-merge-costs");
    assert_eq!(
        value(p, "2026-01-05", "X"),
        [
            "2026-01-02  a:d: X 4 @@ USD 9 | a: X 6, X -4 @@ USD -9 | z: USD 9, X -6",
            "2026-01-03  a:d: X 4 @ USD 2 | a: X -4 @ USD 2 | z: USD 8",
            "2026-01-04  a:d: X 4 | a: X -4 | z: X 4",
        ]
    );
    assert_eq!(
        value(p, "2026-01-05", "EUR"),
        [
            "2026-01-02  a:d: EUR 2 | a: EUR 1 | z: EUR -3, USD 9",
            "2026-01-03  a:d: EUR 2 | a: EUR -2 | z: USD 8",
            "2026-01-04  a:d: EUR 2 | a: EUR -2 | z: EUR 2",
        ]
    );
}

// ───────────── what is valued ─────────────

#[test]
fn an_elided_amount_is_worked_out_before_valuation_and_assertions_run_on_original_amounts() {
    let p = probe!("value-elided-and-assertions");
    assert_eq!(
        value(p, "2026-01-07", "EUR"),
        [
            "2026-01-02  a: EUR 6 | b: EUR -12",
            "2026-01-02  c: EUR 10 | d: EUR -10",
            "2026-01-03  c: EUR 6 | d: EUR -6",
            "2026-01-04  c: EUR 2 | e: EUR 18 | d: EUR -20",
            "2026-01-05  c: EUR 0 | d: EUR 0",
            "2026-01-06  c:s: EUR 8 | c: EUR 16 | d: EUR -24",
        ]
    );
    assert_eq!(
        value(p, "2026-01-07", "X"),
        [
            "2026-01-02  a: X 3 @ EUR 4 | b: X -6",
            "2026-01-02  c: X 5 | d: X -5",
            "2026-01-03  c: X 3 | d: X -3",
            "2026-01-04  c: X 1 | e: X 9 | d: X -10",
            "2026-01-05  c: X 0 | d: X 0",
            "2026-01-06  c:s: X 4 | c: X 8 | d: X -12",
        ]
    );
}

#[test]
fn virtual_and_balanced_virtual_postings_are_valued_too() {
    assert_lines(
        probe!("value-virtual"),
        &[(
            "2026-01-07",
            "EUR",
            "2026-01-02  a: EUR 2 | v: EUR 4 | bv: EUR 6 | bw: EUR -6 | b: EUR -2",
        )],
    );
}

#[test]
fn a_posting_with_no_amount_stays_without() {
    let journal = "P 2026-01-01 X 2 EUR\n\n2026-01-02 t\n    a\n";
    assert_eq!(
        valued(&[("t.journal", journal)], "2026-01-02", "EUR"),
        ["2026-01-02  a: -"]
    );
}

// ───────────── arguments and refusals ─────────────

#[test]
fn a_date_that_is_not_yyyy_mm_dd_is_an_invalid_argument() {
    let ledger = load(&[file("t.journal", "2026-01-02 t\n    a  1\n    b\n")]);
    for bad in [
        "2026-1-9",
        "2026/01/09",
        "20260109",
        "2026-02-30",
        "2026-13-01",
        "",
        "today",
        "2026-01-09 ",
    ] {
        let finding = ledger.value(bad, "EUR").unwrap_err();
        assert_eq!(finding.code, "invalid-argument", "{bad:?}");
        assert_eq!(finding.details, json!({ "argument": "date", "value": bad }));
    }
    assert!(ledger.value("2026-02-28", "EUR").is_ok());
}

#[test]
fn valuing_in_no_commodity_is_refused() {
    let ledger = load(&[file("t.journal", "2026-01-02 t\n    a  1\n    b\n")]);
    let finding = ledger.value("2026-01-02", "").unwrap_err();
    assert_eq!(finding.code, "unsupported-syntax");
    assert_eq!(
        finding.details,
        json!({ "construct": "value-without-commodity" })
    );
}

#[test]
fn a_beancount_ledger_is_refused() {
    let ledger = load(&[file(
        "t.beancount",
        "2026-01-01 open Assets:A\n2026-01-01 open Equity:E\n2026-01-02 * \"t\"\n  Assets:A  1 EUR\n  Equity:E\n",
    )]);
    let finding = ledger.value("2026-01-02", "EUR").unwrap_err();
    assert_eq!(finding.code, "unsupported-syntax");
    assert_eq!(
        finding.details,
        json!({ "construct": "beancount-valuation" })
    );
    let finding = ledger.at_cost().unwrap_err();
    assert_eq!(
        finding.details,
        json!({ "construct": "beancount-valuation" })
    );
}

#[test]
fn a_journal_with_an_error_gives_what_transactions_gives_valued() {
    // The second transaction does not balance: hledger rejects the journal, and `transactions()`
    // still lists it; so does `value`.
    let journal = "P 2026-01-01 X 2 EUR\n\n2026-01-02 ok\n    a  1 X\n    b  -1 X\n\n2026-01-03 bad\n    a  1 X\n    b  -2 X\n";
    let ledger = load(&[file("t.journal", journal)]);
    assert!(
        ledger
            .check()
            .iter()
            .any(|f| f.code == "unbalanced-transaction")
    );
    let valued = ledger.value("2026-01-03", "EUR").unwrap();
    assert_eq!(valued.len(), ledger.transactions().len());
    assert_eq!(
        short_lines(&valued),
        [
            "2026-01-02  a: EUR 2 | b: EUR -2",
            "2026-01-03  a: EUR 2 | b: EUR -4"
        ]
    );
}

#[test]
fn the_shape_and_order_of_transactions_is_kept() {
    let journal = "P 2026-01-01 X 2 EUR\n\n2026-01-05 * (c) later  ; note\n    a  1 X  ; p\n    b\n\n2026-01-02 ! earlier\n    a  1 X\n    b\n";
    let ledger = load(&[file("t.journal", journal)]);
    let valued = ledger.value("2026-01-05", "EUR").unwrap();
    let raw = ledger.transactions();
    assert_eq!(valued.len(), raw.len());
    for (v, r) in valued.iter().zip(raw) {
        assert_eq!(
            (&v.date, v.status, &v.description),
            (&r.date, r.status, &r.description)
        );
        assert_eq!((&v.comment, &v.source), (&r.comment, &r.source));
        let comments: Vec<_> = v
            .postings
            .iter()
            .map(|p| (&p.account, &p.comment))
            .collect();
        let raw_comments: Vec<_> = r
            .postings
            .iter()
            .map(|p| (&p.account, &p.comment))
            .collect();
        assert_eq!(comments, raw_comments);
    }
}
