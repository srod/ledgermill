//! Balance assignments: the amounts they get, how they take part in balancing, and how a
//! transaction that holds one is processed. Expected values: hledger 1.52.4 on the named probes
//! (spec/hledger/assignments.md, assignment-lots-and-costs.md, assignment-processing.md and
//! assertion-failures.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Cost, load};
use serde_json::{Value, json};
use support::*;

fn amount_text(a: &ledgermill::Amount) -> String {
    let mut text = a.quantity.clone();
    if !a.commodity.is_empty() {
        text.push(' ');
        text.push_str(&a.commodity);
    }
    let quantity = |q: &ledgermill::Quantity| {
        if q.commodity.is_empty() {
            q.quantity.clone()
        } else {
            format!("{} {}", q.quantity, q.commodity)
        }
    };
    match &a.cost {
        Some(Cost::Unit { amount }) => text.push_str(&format!(" @ {}", quantity(amount))),
        Some(Cost::Total { amount }) => text.push_str(&format!(" @@ {}", quantity(amount))),
        None => {}
    }
    if let Some(lot) = &a.lot {
        text.push(' ');
        match &lot.cost {
            Some(cost) => text.push_str(&format!("{{{}}}", quantity(cost))),
            None => text.push_str("{}"),
        }
        if let Some(date) = &lot.date {
            text.push_str(&format!("[{date}]"));
        }
        if let Some(label) = &lot.label {
            text.push_str(&format!("({label})"));
        }
    }
    text
}

/// One line per transaction: `LINE: account => amounts | account => amounts`; `-` for a
/// posting without an amount.
fn rows(name: &str, content: &str) -> Vec<String> {
    rows_of(&[file(name, content)])
}

fn rows_of(files: &[ledgermill::SourceFile]) -> Vec<String> {
    load(files)
        .transactions()
        .iter()
        .map(|t| {
            let postings: Vec<String> = t
                .postings
                .iter()
                .map(|p| {
                    let amounts: Vec<String> = p.amounts.iter().map(amount_text).collect();
                    let amounts = if amounts.is_empty() {
                        "-".to_string()
                    } else {
                        amounts.join(", ")
                    };
                    format!("{} => {amounts}", p.account)
                })
                .collect();
            format!("{}: {}", t.source.line, postings.join(" | "))
        })
        .collect()
}

/// `[line, code]` of each finding, in the order `check()` gives them.
fn order(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = findings(name, content)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"]]))
        .collect();
    Value::Array(rows)
}

#[test]
fn an_assignment_gets_the_asserted_amount_less_the_balance_with_the_places_of_both() {
    // A zero difference is no amount, the partner's too; an unused account has the balance 0; the balance afterwards is the asserted amount as written.
    let probe = include_str!("../../../spec/hledger/probes/assign-basic.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 1.5 EUR | z => -1.5 EUR",
            "5: a => 0.75 EUR | z => -0.75 EUR",
            "9: a => - | z => -",
            "13: a => 0.750 EUR | z => -0.750 EUR",
            "17: a => - | z => -",
            "21: a => -2 EUR | z => 2 EUR",
            "25: c => 5 EUR | z => -5 EUR",
            "29: d => - | z => -",
            "33: e => -5.5 EUR | z => 5.5 EUR",
            "37: c => 7 | c => 2 USD | z => -7, -2 USD",
            "42: c => - | c => -2 USD | z => 2 USD"
        ]
    );
}

#[test]
fn the_places_of_an_assigned_amount_follow_the_sum_rules() {
    // A zero side gives back the other one with its places; two non-zero sides give the larger number of places.
    let probe = include_str!("../../../spec/hledger/probes/assign-diff-places.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: p1 => 3 EUR | z => -3 EUR",
            "9: p2 => 3.00 EUR | z => -3.00 EUR",
            "17: p3 => 3.00 EUR | z => -3.00 EUR",
            "25: p4 => 3 EUR | z => -3 EUR",
            "33: p5 => 1.5 EUR | z => -1.5 EUR",
            "41: p6 => 1.250 EUR | z => -1.250 EUR",
            "49: p7 => 0.00 EUR | z => 0.00 EUR",
            "57: p8 => 0.00 EUR | z => 0.00 EUR",
            "65: p9 => 3 EUR | z => -3 EUR",
            "73: p10 => 3.00 EUR | z => -3.00 EUR",
            "81: p11 => -3.00 EUR | z => 3.00 EUR",
            "89: p12 => -3.00 EUR | z => 3.00 EUR",
            "97: p13 => 2.5 EUR | z => -2.5 EUR",
            "105: p14 => 0.5 EUR | z => -0.5 EUR",
            "113: p15 => 1 EUR | z => -1 EUR",
            "121: p16 => 0.00 EUR | z => 0.00 EUR",
            "5: p1 => -3 EUR | z => 3 EUR",
            "13: p2 => -3.00 EUR | z => 3.00 EUR",
            "21: p3 => -1.50 EUR | z => 1.50 EUR",
            "29: p4 => -1.50 EUR | z => 1.50 EUR",
            "37: p5 => 1.5 EUR | z => -1.5 EUR",
            "45: p6 => 2.250 EUR | z => -2.250 EUR",
            "53: p7 => 3 EUR | z => -3 EUR",
            "61: p8 => 3.00 EUR | z => -3.00 EUR",
            "69: p9 => - | z => -",
            "77: p10 => - | z => -",
            "85: p11 => - | z => -",
            "93: p12 => 6.00 EUR | z => -6.00 EUR",
            "101: p13 => -5.0 EUR | z => 5.0 EUR",
            "109: p14 => 0.01 EUR | z => -0.01 EUR",
            "117: p15 => 0.001 EUR | z => -0.001 EUR",
            "125: p16 => - | z => -"
        ]
    );
}

#[test]
fn the_running_balance_keeps_the_places_of_its_sums() {
    // A sum that went through zero loses the places of the terms before it; a plain assertion changes no balance.
    let probe = include_str!("../../../spec/hledger/probes/assign-sum-places.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: x1 => 1.25 EUR | x1 => 3 EUR | x1 => -1.25 EUR | z => -3.00 EUR",
            "7: x2 => 1.25 EUR | x2 => -1.25 EUR | x2 => 3 EUR | z => -3 EUR",
            "13: x3 => 1.5 EUR | z => -1.5 EUR",
            "21: x4 => 0.00 EUR | x4 => 3 EUR | z => -3 EUR",
            "26: x5 => 3 EUR | x5 => 0.00 EUR | z => -3 EUR",
            "31: x6 => 1.25 EUR | x6 => -1.25 EUR | z => 0.00 EUR",
            "36: x7 => 1 EUR | z => -1 EUR",
            "40: x8 => 1 EUR | x8 => 0 EUR | z => -1 EUR",
            "17: x3 => 0.50 EUR | z => -0.50 EUR",
            "45: x1 => 1.00 EUR | x2 => 1 EUR | x3 => 2.00 EUR | x4 => 1 EUR | x5 => 1 EUR | x6 => 4 EUR | x7 => 3 EUR | x8 => 3 EUR | z => -16.00 EUR"
        ]
    );
}

#[test]
fn a_double_equals_assignment_zeroes_every_other_commodity_in_code_point_order() {
    // Zero results are not listed; afterwards the account holds the asserted amount only.
    let probe = include_str!("../../../spec/hledger/probes/assign-total.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 5 EUR | a => 2 ZZ | a => 3 AA | a => 4 mm | b => 10 EUR | b => 3 USD | b => -3 USD | c => 10 EUR | d => 3 USD | d => 4 EUR | e => 3 USD | e => 4 | f => 4 EUR | f => 3 USD | g => 3 USD | g => 4 EUR | h => 4 EUR | h => 3 USD | z => -4, -3 AA, -41 EUR, -15 USD, -2 ZZ, -4 mm",
            "22: a => -3 AA, 2 EUR, -2 ZZ, -4 mm | z => 3 AA, -2 EUR, 2 ZZ, 4 mm",
            "26: b => - | z => -",
            "30: b => 2 EUR | z => -2 EUR",
            "34: c => - | z => -",
            "38: g => -4 EUR, 10 JPY, -3 USD | z => 4 EUR, -10 JPY, 3 USD",
            "42: d => -4 EUR, -3 USD | z => 4 EUR, 3 USD",
            "46: f => -4 EUR, -3 USD | z => 4 EUR, 3 USD",
            "50: e => -4, -3 USD | z => 4, 3 USD",
            "54: a => 4 EUR | a => - | a => - | a => - | z => -4 EUR",
            "61: h => 5 EUR | h => -3 USD | z => -5 EUR, 3 USD"
        ]
    );
}

#[test]
fn a_star_assignment_counts_the_sub_accounts_and_puts_the_amount_on_the_account_itself() {
    // The own balance afterwards is the asserted amount less the sub-accounts' sum, also on a zero difference.
    let probe = include_str!("../../../spec/hledger/probes/assign-inclusive.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 1 EUR | a:b => 2 EUR | a:c:d => 3 EUR | ab => 100 EUR | h:b => 2 EUR | i => 1 EUR | i:b => 2.5 EUR | j:b => 2 EUR | k:b => 2 EUR | k:c => 3 USD | k => 4 GBP | m => 2 EUR | z => -117.5 EUR, -4 GBP, -3 USD",
            "16: a => 4 EUR | z => -4 EUR",
            "20: a => - | z => -",
            "24: h => 3 EUR | z => -3 EUR",
            "28: i => - | i => 1.0 EUR | z => -1.0 EUR",
            "33: k => 8 EUR, -4 GBP, -3 USD | z => -8 EUR, 4 GBP, 3 USD",
            "37: k:b => -2 EUR | k => 2 EUR | z => 0 EUR",
            "42: m => 3 EUR | z => -3 EUR"
        ]
    );
}

#[test]
fn an_assigned_amount_takes_part_in_balancing_and_is_not_an_elided_amount() {
    // One posting without an amount may stay next to assignments; prices are inferred for assigned amounts too.
    let probe = include_str!("../../../spec/hledger/probes/assign-balancing.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 5 EUR | b => -4 EUR | c => -1 EUR",
            "6: d => 5 EUR | e => -5 EUR",
            "10: f => 5 EUR | f => 2 EUR | g => -7 EUR",
            "15: h => 1 EUR | h => 6 EUR | h => 2 EUR | h => 2 EUR | g => -11 EUR",
            "22: i => 5 EUR | j => 1 EUR | k => -1 EUR",
            "27: l => 5 EUR | v => 0 | m => -5 EUR",
            "32: l => - | n => 3 EUR | o => -3 EUR | m => -",
            "38: p => - | q => 5 EUR | r => -5 EUR",
            "43: s => 5 USD @@ 3 EUR | t => -3 EUR",
            "47: u => -3 EUR @@ -5 USD | v => 5 USD",
            "51: w => 1 USD @ 0.8 EUR | w => 4 USD @ 0.8 EUR | t => -3 EUR | t => -1 EUR"
        ]
    );
}

#[test]
fn the_places_of_an_assigned_amount_count_for_the_tolerance_but_those_of_a_plain_assertion_do_not()
{
    let probe = include_str!("../../../spec/hledger/probes/assign-precision.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 1.234 EUR | z => -1.234 EUR",
            "5: a => 0.766 EUR | b => -0.766 EUR",
            "9: c => 1 X @ 0.44 EUR | d => -0.4 EUR",
            "13: e => 1 X @ 0.44 EUR | f => -0.4 EUR"
        ]
    );
}

#[test]
fn a_transaction_with_an_assignment_is_processed_as_a_block_at_its_date() {
    // Its elided amount joins the balances only after the block; a plain transaction's is known to its assertions.
    let probe = include_str!("../../../spec/hledger/probes/assign-processing.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 5 EUR | b => -5 EUR | b => 0 EUR",
            "6: c => -5 EUR | d => 5 EUR | c => 0 EUR",
            "11: c => 0 EUR | y => 0 EUR",
            "15: m => 2 EUR | f => 5 EUR | m => -7 EUR",
            "20: m => 0 EUR | y => 0 EUR",
            "24: g => 5 EUR | h => -5 EUR | v => - | h => 0 EUR",
            "30: k => 5 EUR | k => 1 EUR | z => -6 EUR",
            "35: k => 0 EUR | y => 0 EUR",
            "39: l => 5 EUR | l => 1 EUR | z => -6 EUR",
            "44: l => 0 EUR | y => 0 EUR"
        ]
    );
}

#[test]
fn assignments_are_worked_out_in_date_order_and_then_in_file_order() {
    let probe = include_str!("../../../spec/hledger/probes/assign-order.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "5: a => 3 EUR | z => -3 EUR",
            "9: b => 5 EUR | z => -5 EUR",
            "13: b => 3 EUR | z => -3 EUR",
            "17: c => 1 EUR | z => -1 EUR",
            "29: e => 1 EUR | z => -1 EUR",
            "21: c => 2 EUR | z => -2 EUR",
            "25: e => 3 EUR | z => -3 EUR",
            "33: c => 0 EUR | e => 0 EUR | y => 0 EUR",
            "1: a => 2 EUR | z => -2 EUR"
        ]
    );
}

#[test]
fn an_assignment_with_a_date2_tag_is_fine() {
    let probe = include_str!("../../../spec/hledger/probes/assign-posting-date2.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec!["1: a => 5 EUR | z => -5 EUR"]
    );
}

#[test]
fn an_assignment_with_a_posting_date_is_an_error_on_its_posting() {
    for (name, probe) in [
        (
            "a date: tag",
            include_str!("../../../spec/hledger/probes/assign-fail-posting-date.journal"),
        ),
        (
            "the transaction's own date",
            include_str!("../../../spec/hledger/probes/assign-fail-posting-date-same.journal"),
        ),
        (
            "a bracketed date",
            include_str!("../../../spec/hledger/probes/assign-fail-posting-date-bracket.journal"),
        ),
    ] {
        let f = findings("x.journal", probe);
        assert_eq!(
            f,
            json!([{
                "code": "assignment-with-posting-date", "severity": "error", "file": "x.journal",
                "line": 2, "details": { "account": "a" },
                "message": "A balance assignment cannot have a posting date."
            }]),
            "{name}"
        );
    }
}

#[test]
fn an_assertion_after_an_assignment_does_not_see_the_elided_amount_of_its_own_transaction() {
    for (name, probe, line) in [
        (
            "the elided posting comes first",
            include_str!("../../../spec/hledger/probes/assign-fail-elided-not-yet.journal"),
            4,
        ),
        (
            "a virtual assignment",
            include_str!("../../../spec/hledger/probes/assign-fail-elided-not-yet-virtual.journal"),
            5,
        ),
        (
            "a zero difference",
            include_str!("../../../spec/hledger/probes/assign-fail-elided-not-yet-zero.journal"),
            5,
        ),
        (
            "the postings of a block are all in, whatever their dates",
            include_str!(
                "../../../spec/hledger/probes/assign-fail-posting-dates-in-transaction.journal"
            ),
            7,
        ),
    ] {
        assert_eq!(
            order("x.journal", probe),
            json!([[line, "assertion-failed"]]),
            "{name}"
        );
    }
}

#[test]
fn what_an_assignment_leaves_unbalanced_is_reported_on_the_header() {
    for (name, probe, line) in [
        (
            "the places of the assigned amount count",
            include_str!("../../../spec/hledger/probes/assign-fail-precision.journal"),
            5,
        ),
        (
            "the places of a plain assertion count for nothing, the assignment's do",
            include_str!("../../../spec/hledger/probes/assign-fail-precision-assertion.journal"),
            1,
        ),
        (
            "an assignment with nothing against it",
            include_str!("../../../spec/hledger/probes/assign-fail-single.journal"),
            1,
        ),
        (
            "== also zeroes a commodity that nothing else can take",
            include_str!("../../../spec/hledger/probes/assign-fail-total-unbalanced.journal"),
            5,
        ),
        (
            "two assignments of one sign",
            include_str!("../../../spec/hledger/probes/assign-fail-unbalanced.journal"),
            1,
        ),
    ] {
        assert_eq!(
            order("x.journal", probe),
            json!([[line, "unbalanced-transaction"]]),
            "{name}"
        );
    }
}

#[test]
fn an_assignment_is_not_an_elided_amount_but_two_amount_less_postings_still_are() {
    let probe = include_str!("../../../spec/hledger/probes/assign-fail-two-elided.journal");
    let f = findings("x.journal", probe);
    assert_eq!(f[0]["line"], json!(1));
    assert_eq!(f[0]["code"], json!("unbalanced-transaction"));
    assert_eq!(f[0]["details"]["reason"], json!("two-elided-amounts"));
    assert_eq!(f.as_array().unwrap().len(), 1);
}

#[test]
fn a_transaction_with_an_assignment_reports_in_its_place_in_the_walk() {
    // [probe, findings as [line, code]]
    let cases: [(&str, &str, Value); 9] = [
        (
            "an earlier failing assertion, then the assignment that does not balance",
            include_str!("../../../spec/hledger/probes/assign-prec-assertion-first.journal"),
            json!([[2, "assertion-failed"], [5, "unbalanced-transaction"]]),
        ),
        (
            "an assertion fails before its own transaction is balanced",
            include_str!(
                "../../../spec/hledger/probes/assign-prec-assertion-in-transaction.journal"
            ),
            json!([[3, "assertion-failed"], [1, "unbalanced-transaction"]]),
        ),
        (
            "the unbalanced assignment is dated first",
            include_str!("../../../spec/hledger/probes/assign-prec-assignment-first.journal"),
            json!([[1, "unbalanced-transaction"], [6, "assertion-failed"]]),
        ),
        (
            "two unbalanced assignments: date order",
            include_str!("../../../spec/hledger/probes/assign-prec-date-order.journal"),
            json!([[5, "unbalanced-transaction"], [1, "unbalanced-transaction"]]),
        ),
        (
            "the assertion after the assignment fails",
            include_str!("../../../spec/hledger/probes/assign-prec-in-transaction.journal"),
            json!([[2, "assertion-failed"], [7, "assertion-failed"]]),
        ),
        (
            "a plain unbalanced transaction is balanced before any assignment",
            include_str!("../../../spec/hledger/probes/assign-prec-plain-first.journal"),
            json!([[5, "unbalanced-transaction"], [1, "unbalanced-transaction"]]),
        ),
        (
            "two plain unbalanced transactions: file order",
            include_str!("../../../spec/hledger/probes/assign-prec-plain-parse-order.journal"),
            json!([[1, "unbalanced-transaction"], [5, "unbalanced-transaction"]]),
        ),
        (
            "an earlier failing assertion, then a posting date on an assignment",
            include_str!("../../../spec/hledger/probes/assign-prec-posting-date.journal"),
            json!([[2, "assertion-failed"], [6, "assignment-with-posting-date"]]),
        ),
        (
            "an earlier failing assertion, then two postings without an amount",
            include_str!("../../../spec/hledger/probes/assign-prec-two-elided.journal"),
            json!([[2, "assertion-failed"], [5, "unbalanced-transaction"]]),
        ),
    ];
    for (name, probe, expected) in cases {
        assert_eq!(order("x.journal", probe), expected, "{name}");
    }
}

#[test]
fn a_price_is_inferred_for_the_first_posting_unless_it_holds_several_amounts() {
    // `a == 3 USD` also gives `a` the amount -1 X. When `d -6 Y` comes first it gets the price;
    // when `a` comes first it is the one that would, and holds two amounts: no inference.
    let inferred = include_str!("../../../harness/journals/prototype/pi72-infer.journal");
    assert_eq!(findings("x.journal", inferred), json!([]));
    assert_eq!(
        rows("x.journal", inferred),
        vec![
            "1: a => 1 X | b => -1 X",
            "4: d => -6 Y @@ -3 USD | c => 1 X | a => 3 USD, -1 X"
        ]
    );
    let refused = include_str!("../../../harness/journals/prototype/pi71-infer.journal");
    assert_eq!(
        order("x.journal", refused),
        json!([[4, "unbalanced-transaction"]])
    );
}

#[test]
fn the_amounts_that_zero_other_commodities_keep_the_lot_of_one_posting_and_never_more() {
    // A lot survives only when exactly one posting made the balance in that commodity; the asserted commodity never has one.
    let probe = include_str!("../../../spec/hledger/probes/assign-lots.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: l1 => 1 X {10 EUR} | l1 => 1 X {10 EUR} | l2 => 1 X {10 EUR} | l2 => 1 X | l4 => 1 X {10 EUR} | l4 => 0 X | l5 => 0 X | l5 => 1 X {10 EUR} | l6 => 2 X {10 EUR}[2026-01-01](lbl) | l7 => 2 X {15 EUR} | l8 => 2 X {10 EUR} | l9 => 2 X @ 12 EUR {10 EUR} | l10 => 2 X {10 EUR} | l10 => 3 EUR | l12 => 2 Y {10 EUR} | l12 => 1 EUR | l13 => 1 EUR | l13:s => 2 Y {10 EUR} | z => -29 EUR, -14 X, -4 Y",
            "22: l1 => -2 X | l2 => -2 X | z => 4 X",
            "27: l4 => -1 X | l5 => -1 X | z => 2 X",
            "32: l6 => -2 X {10 EUR}[2026-01-01](lbl) | l7 => -2 X {15 EUR} | z => 4 X",
            "37: l8 => -1 X | z => 1 X",
            "41: l9 => -2 X {10 EUR} | z => 2 X {10 EUR}",
            "45: l10 => -2 X {10 EUR} | z => 2 X {10 EUR}",
            "49: l11 => 2 X {10 EUR} | l11 => 3 X {11 EUR} | z => -5 X",
            "54: l11 => -5 X | z => 5 X",
            "58: l12 => 1 Y | z => -1 Y",
            "62: l12 => -3 Y | z => 3 Y",
            "66: l13 => -2 Y {10 EUR} | z => 2 Y {10 EUR}"
        ]
    );
}

#[test]
fn a_price_in_the_asserted_amount_is_subtracted_from_the_entries_as_they_are() {
    // Entries are grouped by price: costless first, then by price commodity, total before unit, then by value.
    let probe = include_str!("../../../spec/hledger/probes/assign-cost.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => 3 X | b => 3 X | c => 1 X | c => -1 X | d => 3 X @ 2 EUR | d => 2 X @ 3 EUR | d => 4 X | e => 3 X | f => 2 Y | z => -12 EUR, -13 X, -2 Y",
            "13: d => 1 X | z => -1 X",
            "17: a => -3 X, 3 X @ 2 EUR | z => -6 EUR, 3 X",
            "21: a => 5 X, -3 X @ 2 EUR | z => 6 EUR, -5 X",
            "25: b => -3 X, 3 X @@ 6 EUR | z => -6 EUR, 3 X",
            "29: c => 3 X @ 2 EUR | z => -6 EUR",
            "33: g => 3 X @ 2 EUR | z => -6 EUR",
            "37: g => 1 X | g => 3 X, -3 X @ 2 EUR | z => 6 EUR, -4 X",
            "42: f => 3 X @ 2 EUR, -2 Y | z => -6 EUR, 2 Y",
            "46: h => 1 X @ 1 EUR | h => 1 X @ 1 EUR | z => -2 EUR",
            "51: i => 1 X @@ 3 EUR | i => 1 X @@ -2 EUR | z => -1 EUR",
            "56: j => 1 X @ 2 EUR | j => 1 X @ 1 EUR, -1 X @ 2 EUR | j => 1 X @@ 9 EUR, -1 X @ 1 EUR | j => -1 X @@ -9 EUR, 1 X @ 1 USD | j => 1 X, -1 X @ 1 USD | j => -1 X | z => 0 EUR, 0 USD, 0 X",
            "65: k => 3 X @ 2 EUR | k => 1 X | k => 0 X | z => -6 EUR, -1 X"
        ]
    );
}

#[test]
fn sub_accounts_are_added_in_name_order_so_places_can_differ_from_hledger_when_a_partial_sum_is_zero()
 {
    // 1.25, 3 and -1.25 over three sub-accounts, `a =* 4`: hledger adds them in an order that
    // follows no rule we know (the notes call it unsettled), so it gives 1.00 for the first
    // journal and 1 for the second. ledgermill adds in name order and gives 1.00 for both. The
    // value is the same; the harness compares these two ignoring places.
    for probe in [
        include_str!("../../../spec/hledger/probes/assign-inclusive-order-names-bcd.journal"),
        include_str!("../../../spec/hledger/probes/assign-inclusive-order-names-x123.journal"),
    ] {
        assert_eq!(
            rows("x.journal", probe).last().unwrap(),
            "7: a => 1.00 EUR | z => -1.00 EUR"
        );
    }
}

#[test]
fn an_assignment_sees_only_the_postings_of_its_own_file() {
    let a = file(
        "a.journal",
        include_str!("../../../spec/hledger/probes/assert-file-a.journal"),
    );
    let b = file(
        "b.journal",
        include_str!("../../../spec/hledger/probes/assign-file-b.journal"),
    );
    // File a posts 10 EUR to `a` on 1 January; file b assigns 20 EUR on 2 January from nothing.
    assert_eq!(
        rows_of(&[a, b]).last().unwrap(),
        "1: a => 20 EUR | b => -20 EUR"
    );
}

#[test]
fn a_star_assignment_also_zeroes_the_commodities_that_sub_accounts_hold() {
    // The amount is the sub-accounts' share (the whole less the account's own); the account
    // itself then holds its own plus that amount.
    let probe = include_str!("../../../spec/hledger/probes/assign-inclusive-other.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => -2.00 X | a:b => -1 X | d:b:c => 2 Y | f => 1 Y | f:g => 1.00 Y | z => 3.00 X, -4.00 Y",
            "9: a => 4 EUR, 1.00 X | d => -2 Y | f => -1 Y | z => -4 EUR, -1.00 X, 3 Y",
            "15: a => 1.00 X | z => -1.00 X"
        ]
    );
}

#[test]
fn a_posting_keeps_its_price_as_an_entry_when_its_transaction_has_an_assignment() {
    let probe = include_str!("../../../spec/hledger/probes/assign-entry-prices.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: x => 1 X @ 2 EUR | w => - | y => -2 EUR",
            "6: x => -1 X @ 2 EUR | z => 2 EUR",
            "10: p => 1 X @ 2 EUR | y => -2 EUR",
            "14: p => -1 X | z => 1 X",
            "18: q => 3 X @@ 6 EUR | q => -3 X @@ -6 EUR | y => 0 EUR",
            "23: r => 1 X @ 2 EUR | r => 2 X | r => -2 X, -1 X @ 2 EUR | y => 0 EUR, 0 X"
        ]
    );
}

#[test]
fn a_posting_dated_after_its_transaction_joins_the_balance_at_the_end_of_the_block() {
    let probe = include_str!("../../../spec/hledger/probes/assign-later-posting-date.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: x => 5 EUR | x => 7 EUR | x => 0 EUR | z => -12 EUR",
            "7: x => 0 EUR | y => 0 EUR",
            "11: w => 5 EUR | v => 5 EUR | w => 2 EUR | v => 3 EUR | z => -15 EUR"
        ]
    );
    let failing =
        include_str!("../../../spec/hledger/probes/assign-fail-later-posting-date.journal");
    assert_eq!(
        order("x.journal", failing),
        json!([[4, "assertion-failed"]])
    );
}

#[test]
fn a_star_amount_is_asserted_less_the_sub_accounts_less_the_own_balance() {
    // The whole is zero (-3.00 and 3.00): one step would give `1`, two steps give `1.00`.
    let probe = include_str!("../../../spec/hledger/probes/assign-inclusive-two-steps.journal");
    assert_eq!(
        rows("x.journal", probe),
        vec![
            "1: a => -3.00 EUR | a:d => 3.00 EUR | z => 0.00 EUR",
            "6: a => 1.00 EUR | z => -1.00 EUR"
        ]
    );
}

#[test]
fn a_star_amount_keeps_its_two_steps_beside_a_priced_entry() {
    // A priced amount of the asserted commodity in the same transaction no longer turns the
    // price-less part into one step (fuzz seed 100, journal 168).
    let probe =
        include_str!("../../../spec/hledger/probes/assign-inclusive-two-steps-priced.journal");
    let rows = rows("x.journal", probe);
    let assigned: Vec<&str> = rows
        .iter()
        .filter(|r| {
            [" 8:", "18:", "28:", "38:"]
                .iter()
                .any(|l| r.starts_with(l.trim()))
        })
        .map(String::as_str)
        .collect();
    assert_eq!(
        assigned,
        vec![
            "8: a:d => -4.0 X @@ -9 USD | a => 1.00 X, 4.0 X @@ 9 USD | z => 0 USD, -1.00 X",
            "18: b => -4.0 X @ 2 USD | b => 1.00 X, 4.0 X @ 2 USD | z => 0 USD, -1.00 X",
            "28: c:b => -4.0 X @@ -9 USD | c => -4.00 X, 4.0 X @@ 9 USD | z => 0 USD, 4.00 X",
            "38: e:d => -4.0 X @@ -9 USD | e => 3 X, 4.0 X @@ 9 USD | z => 0 USD, -3 X",
        ]
    );
}
