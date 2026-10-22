//! `Ledger::at_cost()`: costs converted, conversion postings dropped. Expected values: hledger
//! 1.52.4 `print -O json --cost` on the named probes (spec/hledger/cost.md and conversion.md; the
//! lines are the short form of `spec/hledger/show-amounts.sh`).
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
fn a_unit_cost_multiplies_and_a_total_cost_is_the_amount() {
    assert_eq!(
        at_cost(&[probe!("cost-unit-and-total")]),
        [
            "2026-01-01  a: EUR 7.5 | b: EUR -7.5 | c: EUR 7.5 | d: EUR -7.5 | e: EUR 5 | f: EUR 5 | g: EUR -4 | h: EUR 4 | i: EUR -8 | j: EUR 3 | k: EUR 3 | l: X 6 | m: EUR 0.1234567890",
        ]
    );
}

#[test]
fn zeros_and_rounding_follow_the_ten_place_rule() {
    assert_eq!(
        at_cost(&[probe!("cost-zero-and-rounding")]),
        [
            "2026-01-01  a: EUR 0 | b: EUR 0 | c: EUR 0 | d: EUR 0.0000000000 | e: EUR -0.0000000002 | f: EUR 0.0000000002 | g: EUR 0.0000000002 | h: EUR 0.0000000002 | i: EUR 0.0000000002 | j: EUR 50",
        ]
    );
}

#[test]
fn a_lot_alone_stays_and_a_lot_with_a_cost_goes_with_the_cost() {
    assert_eq!(
        at_cost(&[probe!("cost-lots")]),
        [
            "2026-01-01  a: Y 2 {EUR 4} | b: EUR 10 | c: EUR 20 | d: EUR 10 | e: Y 2 {EUR 4} | f: EUR 10 | g: Y 2 {EUR 4} | h: EUR 20 | i: EUR 20 | j: Y 3",
        ]
    );
}

#[test]
fn an_inferred_price_is_used_with_all_its_places() {
    assert_eq!(
        at_cost(&[probe!("cost-inferred-price")]),
        [
            "2026-01-01  a: Y 1.0000000000 | b: Y 0 | c: Y -1",
            "2026-01-02  a: Y 0.4285714286 | b: Y 0.5714285714 | c: Y -1",
            "2026-01-03  a: Y 2.0 | c: Y -2.0",
            "2026-01-04  a: Y 1.0 | c: Y -1.5 | d: Y 0.5",
            "2026-01-05  a: X 2 | c: Z -3.0 | d: X 1 | e: X -3.0 | e: Z 3.0",
        ]
    );
}

#[test]
fn a_tag_in_the_comment_of_a_posting_drops_it_unless_it_has_a_cost() {
    assert_eq!(
        at_cost(&[probe!("cost-drop-tags")]),
        [
            "2026-01-01  a: EUR 3",
            "2026-01-02  a: EUR 3",
            "2026-01-03  a: EUR 3",
            "2026-01-04  a: EUR 3 | b: EUR -3",
            "2026-01-05  a: EUR 3 | b: EUR -3",
            "2026-01-06  a: EUR 3 | b: EUR -2 | c: EUR -1",
            "2026-01-07  a: EUR 3 | b: EUR -3",
            "2026-01-08  a: EUR 3",
            "2026-01-09  ",
            "2026-01-10  a: EUR 3 | b: EUR -3 | c: EUR -5",
            "2026-01-11  a: EUR 3",
        ]
    );
}

#[test]
fn adjacent_conversion_postings_are_paired_left_to_right_and_the_pairs_go() {
    assert_eq!(
        at_cost(&[probe!("cost-drop-pairs")]),
        [
            "2026-01-01  a: EUR 3 | equity:conversion: EUR -3",
            "2026-01-02  a: EUR 3",
            "2026-01-03  a: EUR 6 | equity:conversion: EUR -3",
            "2026-01-04  a: EUR 10",
            "2026-01-05  a: EUR 15 | equity:conversion: EUR -5",
            "2026-01-06  equity:conversion: EUR -1 | a: EUR 10",
            "2026-01-07  equity:conversion: EUR 3 | a: EUR -6 | equity:conversion: EUR 3",
            "2026-01-08  equity:trade: EUR -3 | a: EUR 6",
            "2026-01-09  a: EUR 3 | b: EUR -3",
            "2026-01-10  a: EUR 1 | b: EUR -1",
            "2026-01-11  a: EUR 3 | b: EUR -1",
            "2026-01-12  equity:conversion: X 2 | equity:conversion: X -2",
            "2026-01-13  equity:conversion: EUR 1 | equity:conversion: EUR 2 | equity:conversion: EUR -3",
            "2026-01-14  v: EUR 5",
        ]
    );
}

#[test]
fn a_pair_stays_when_the_first_free_posting_is_repeated_by_another_number() {
    assert_eq!(
        at_cost(&[probe!("cost-drop-anchor")]),
        [
            "2026-02-01  a: EUR 3 | b: EUR -3",
            "2026-02-02  a: EUR 3 | b: EUR 3 | c: EUR -6 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-03  a: EUR 3 | b: USD 3 | c: USD -3 | d: EUR -3 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-04  a: EUR 3 | b: EUR 3.4 | c: EUR -6.4 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-05  a: EUR 3 | b: EUR 3.5 | c: EUR -6.5 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-06  a: EUR 3 | b: EUR 3.6 | c: EUR -6.6",
            "2026-02-07  a: EUR 3.0 | b: EUR 3.4 | c: EUR -6.4",
            "2026-02-08  a: EUR 3.0 | b: EUR 3.05 | c: EUR -6.05 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-09  a: EUR 3.0 | b: EUR 3.06 | c: EUR -6.06",
            "2026-02-10  b: EUR 3.05 | a: EUR 3.0 | c: EUR -6.05",
            "2026-02-11  a: EUR 3 | b: EUR 4 | c: EUR 4 | d: EUR -11",
            "2026-02-12  a: EUR 3 | b: EUR 5 | c: EUR 3 | d: EUR -11 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-13  a: EUR 3 | b: EUR -3 | c: EUR -3 | d: EUR 3 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-14  a: EUR 3 | v: EUR 3 | b: EUR -3 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-15  v: EUR 3 | a: EUR 4 | b: EUR -4",
            "2026-02-16  v: EUR 3 | w: EUR -3 | a: EUR 4 | b: EUR -4",
            "2026-02-17  a: EUR 3 | b: EUR 3 | c: EUR -6",
            "2026-02-18  p: EUR 6 | b: EUR -6",
            "2026-02-19  a: X 3 {EUR 2} | b: X 3 {EUR 3} | c: X -6 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-20  z: EUR 2 | a: EUR 2 | b: EUR -4",
            "2026-02-21  z: EUR 0 | a: EUR 0.4 | b: EUR -0.4 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-22  z: EUR 0 | a: EUR 0.6 | b: EUR -0.6",
            "2026-02-23  a: EUR 0.4 | b: EUR -0.4 | z: EUR 0",
            "2026-02-24  a: EUR 0 | b: EUR 5 | c: EUR -5 | z: EUR 0 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-25  a: EUR 0 | b: USD 0 | c: EUR 1 | d: EUR -1 | equity:conversion: EUR -1 | equity:conversion: EUR 1",
            "2026-02-26  a: EUR 0 | c: EUR 1 | d: EUR -1",
            "2026-02-27  a: EUR 3 | b: EUR -3",
        ]
    );
}

#[test]
fn a_pair_goes_with_a_priced_posting_balanced_by_quantity_and_stays_otherwise() {
    assert_eq!(
        at_cost(&[probe!("cost-drop-priced")]),
        [
            "2026-01-01  a: EUR 6 | b: EUR -6",
            "2026-01-02  a: EUR 6 | b: EUR -6",
            "2026-01-03  a: EUR 2 | equity:trading: EUR 4 | equity:trade: EUR -6",
            "2026-01-04  equity:conversion: X -3 | a: EUR 6 | b: EUR -2",
            "2026-01-05  equity:conversion: X -3 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3",
            "2026-01-06  equity:conversion: EUR 1 | equity:conversion: EUR 1 | equity:conversion: X -3 | a: EUR 6 | b: EUR -8 | c: X 3",
            "2026-01-07  b: EUR -12 | equity:conversion: X -3 | a: EUR 6 | c: X 6",
        ]
    );
}

#[test]
fn the_conversion_probes_of_the_conversion_note() {
    assert_eq!(
        at_cost(&[probe!("conversion")]),
        [
            "2026-01-01  a: EUR 6 | b: EUR -6",
            "2026-01-02  a: EUR 6 | c: Y -6 | d: Z 2 | e: X -2",
            "2026-01-03  a: EUR 6 | b: EUR -6",
            "2026-01-04  a: EUR 2 | equity:trading: EUR 4 | equity:trade: EUR -6",
        ]
    );
}

#[test]
fn an_odd_leading_run_of_conversion_postings_before_a_priced_posting() {
    assert_eq!(
        at_cost(&[probe!("conversion-leading-run")]),
        [
            "2026-01-01  equity:conversion: X -3 | a: EUR 6 | b: EUR -2",
            "2026-01-02  equity:conversion: X -3 | a: EUR 6 | b: EUR -2",
            "2026-01-03  equity:conversion: X -3 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3",
            "2026-01-04  equity:conversion: X -3.0 | a: EUR 6 | equity:conversion: EUR 1 | equity:conversion: EUR 1 | b: EUR -8 | c: X 3 | d: Y 3.05 | d: Y -3.05",
            "2026-01-05  equity:conversion: X -3.0 | a: EUR 6 | b: EUR -2 | d: Y 3.051 | d: Y -3.051",
            "2026-01-06  b: EUR -12 | equity:conversion: X -3 | a: EUR 6 | c: X 6",
        ]
    );
}

// ───────────── what is refused ─────────────

fn refusal(files: &[(&str, &str)]) -> ledgermill::Finding {
    let files: Vec<_> = files.iter().map(|(n, c)| file(n, c)).collect();
    load(&files).at_cost().unwrap_err()
}

#[test]
fn pairs_of_conversion_postings_with_several_priced_postings_are_refused() {
    let finding = refusal(&[probe!("conversion-several")]);
    assert_eq!(finding.code, "unsupported-syntax");
    assert_eq!(
        finding.details,
        json!({ "construct": "cost-conversion-postings" })
    );
    assert_eq!(
        (finding.file.as_str(), finding.line),
        ("conversion-several.journal", 1)
    );
}

#[test]
fn the_first_refused_transaction_is_the_one_reported() {
    let journal = "2026-01-01 fine\n    a  1\n    b\n\n\
        2026-01-02 refused\n    a  1 X @ 2 EUR\n    equity:conversion  -1 X\n    equity:conversion  2 EUR\n    b  2 Y @ 1 EUR\n    c  -4 EUR\n";
    let finding = refusal(&[("t.journal", journal)]);
    assert_eq!((finding.file.as_str(), finding.line), ("t.journal", 5));
}

#[test]
fn pairs_of_conversion_postings_with_a_balance_assignment_are_refused() {
    let journal = "2026-01-01 t\n    a  = 5 EUR\n    equity:conversion  -1 EUR\n    equity:conversion  1 EUR\n    b\n";
    let finding = refusal(&[("t.journal", journal)]);
    assert_eq!(
        finding.details,
        json!({ "construct": "cost-conversion-postings" })
    );
}

#[test]
fn several_priced_postings_are_converted_when_no_conversion_pair_is_there() {
    let journal = "2026-01-01 t\n    a  1 X @ 2 EUR\n    b  2 Y @@ 3 EUR\n    c  -5 EUR\n    equity:conversion  1 EUR\n";
    assert_eq!(
        at_cost(&[("t.journal", journal)]),
        ["2026-01-01  a: EUR 2 | b: EUR 3 | c: EUR -5 | equity:conversion: EUR 1"]
    );
}

#[test]
fn a_transaction_ledgermill_does_not_balance_is_left_out_as_transactions_leaves_it_out() {
    // A priced posting on a conversion account next to a pair: `check()` reports it, and the
    // transaction is in neither list.
    let journal = "2026-01-01 t\n    equity:conversion  1 X @ 2 EUR\n    equity:conversion  2 EUR\n    a  -2 EUR\n\n2026-01-02 u\n    a  1\n    b\n";
    let ledger = load(&[file("t.journal", journal)]);
    assert!(
        ledger
            .check()
            .iter()
            .any(|f| f.code == "unsupported-syntax" || f.code == "unbalanced-transaction")
    );
    assert_eq!(ledger.at_cost().unwrap().len(), ledger.transactions().len());
}

#[test]
fn a_transaction_with_every_posting_dropped_stays_without_postings() {
    let journal = "2026-01-01 t\n    a  3 EUR  ; _conversion-posting:\n    b  -3 EUR  ; _conversion-posting:\n";
    let ledger = load(&[file("t.journal", journal)]);
    let costed = ledger.at_cost().unwrap();
    assert_eq!(costed.len(), 1);
    assert!(costed[0].postings.is_empty());
    assert_eq!(
        ledger.transactions()[0].postings.len(),
        2,
        "transactions() is unchanged"
    );
}
