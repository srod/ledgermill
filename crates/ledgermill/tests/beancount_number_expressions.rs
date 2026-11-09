//! Arithmetic in a number (spec/beancount/number-expressions.md): each probe's figures are what
//! `bean-check` and `bean-query` printed for it, copied from the note.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use std::collections::BTreeMap;
use support::*;

const OPERATORS_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-operators.beancount");
const DIVISION_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-division.beancount");
const PLACES_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-places.beancount");
const ZERO_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-zero.beancount");
const TOLERANCE_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-tolerance.beancount");
const TOLERANCE_FAILS_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-tolerance-fails.beancount");
const POSITIONS_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-positions.beancount");
const BALANCE_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-balance.beancount");
const PRICE_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-price-directive.beancount");
const METADATA_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-metadata.beancount");
const DISPLAY_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-display.beancount");
const ERRORS_PROBE: &str =
    include_str!("../../../spec/beancount/probes/number-expression-errors.beancount");
const ZERO_DIVISION_PROBE: &str = include_str!(
    "../../../spec/beancount/probes/refused/number-expression-division-by-zero.beancount"
);
const ZERO_DIVISION_FIRST_PROBE: &str = include_str!(
    "../../../spec/beancount/probes/refused/number-expression-division-by-zero-first-operand.beancount"
);

const OPERATORS: &[(u32, &str, &str)] = &[
    (5, "(7 + 2)", "9"),
    (6, "(7 - 2)", "5"),
    (7, "(7 * 2)", "14"),
    (8, "(7 / 2)", "3.5"),
    (12, "(1 + 2 * 3)", "7"),
    (16, "(2 - 3 - 4)", "-5"),
    (20, "(8 / 4 / 2)", "1"),
    (24, "(8 / 2 * 4)", "16"),
    (28, "(10 - 2 + 3)", "11"),
    (32, "(10 / 2 + 3)", "8"),
    (36, "((1 + 2) * 3)", "9"),
    (37, "(2 * (3 + 4))", "14"),
    (38, "(((5)))", "5"),
    (42, "(-2 * 3)", "-6"),
    (43, "(2 * -3)", "-6"),
    (44, "(2 - -3)", "5"),
    (45, "(--4)", "4"),
    (46, "(+5)", "5"),
    (47, "(+-6)", "-6"),
    (48, "(- 7)", "-7"),
    (52, "-(1 + 2)", "-3"),
    (53, "+(1 + 2)", "3"),
    (54, "- (4 - 5)", "1"),
    (58, "(1+2)", "3"),
    (59, "( 1 + 2 )", "3"),
    (60, "(1 +2)", "3"),
    (61, "(1+ 2)", "3"),
    (62, "(2*-3)", "-6"),
    (63, "(2 --3)", "5"),
    (64, "(1\\t+\\t2)", "3"),
    (68, "1 + 2", "3"),
    (69, "1+2", "3"),
    (70, "2 * 3", "6"),
    (71, "3 -1", "2"),
    (72, "-1 + 3", "2"),
    (73, "1 + 2 * 3", "7"),
    (74, "(1 + 2) * 3", "9"),
    (75, "(1 + 2)", "3"),
    (76, "1+2", "3"),
    (80, "(1,000 + 1)", "1001"),
    (81, "(1,000.50 * 2)", "2001.00"),
    (82, "(5. + 1)", "6"),
];

const DIVISION: &[(u32, &str, &str)] = &[
    (5, "(10 / 3)", "3.333333333333333333333333333"),
    (9, "(2 / 3)", "0.6666666666666666666666666667"),
    (13, "(1 / 7)", "0.1428571428571428571428571429"),
    (17, "(100 / 3.0)", "33.33333333333333333333333333"),
    (21, "(1 / 3.00)", "0.3333333333333333333333333333"),
    (25, "(1 / 3 / 1000000)", "3.333333333333333333333333333E-7"),
    (29, "(90.00 / 3)", "30.00"),
    (33, "(6.0 / 3)", "2.0"),
    (37, "(6 / 3.0)", "2"),
    (41, "(100 / 8)", "12.5"),
    (45, "(1.0 / 4)", "0.25"),
    (49, "(1 / 8)", "0.125"),
    (53, "(1 / 1000)", "0.001"),
    (57, "(7 / 2)", "3.5"),
    (61, "(15 / 6)", "2.5"),
    (65, "(3.00 / 6)", "0.50"),
    (69, "(0.5 / 5)", "0.1"),
    (73, "(1 / 400000)", "0.0000025"),
    (77, "(1 / 10000000)", "1E-7"),
    (81, "(1.00 / 100000000)", "1E-8"),
    (
        85,
        "(1 / 3 / 1000000 / 1000)",
        "3.333333333333333333333333333E-10",
    ),
    (89, "(50 / 0.5)", "1.0E+2"),
    (93, "(500 / 0.5)", "1.00E+3"),
    (97, "(500 / 0.05)", "1.00E+4"),
    (101, "(5 / 0.25)", "2E+1"),
    (105, "(2 / 0.5)", "4"),
    (109, "(1 / 0.5)", "2"),
    (113, "(2 / 3 + 1000000)", "1000000.666666666666666666667"),
    (117, "(1 / 3 * 3)", "0.9999999999999999999999999999"),
    (121, "(10 / 4 * 4)", "10.0"),
];

const PLACES: &[(u32, &str, &str)] = &[
    (6, "(1.50 * 2)", "3.00"),
    (10, "(1.5 * 2.25)", "3.375"),
    (14, "(2.0 * 3)", "6.0"),
    (18, "(0.10 * 0.10)", "0.0100"),
    (22, "(0.001 * 0.001 * 0.001)", "1E-9"),
    (26, "(1000000.0 * 1000000)", "1000000000000.0"),
    (30, "(100 * 1000000)", "100000000"),
    (34, "(0.1 * 3)", "0.3"),
    (38, "(0.1 + 0.20)", "0.30"),
    (42, "(1.00 + 2)", "3.00"),
    (46, "(1 + 2.00)", "3.00"),
    (50, "(1.10 - 0.1)", "1.00"),
    (54, "(2.50 - 2.5)", "0.00"),
    (58, "(1,000.50 * 2)", "2001.00"),
    (
        62,
        "(0.1234567890123456789 * 0.1234567890123456789)",
        "0.01524157875323883675019051999",
    ),
    (
        66,
        "(100000000000000000000 * 100)",
        "10000000000000000000000",
    ),
    (70, "(10 / 3)", "3.333333333333333333333333333"),
    (71, "1.5", "1.5"),
    (75, "(10 / 4)", "2.5"),
    (76, "(4 / 2)", "2"),
    (80, "(1.5 * 2)", "3.0"),
    (81, "1.25", "1.25"),
];

const ZERO: &[(u32, &str, &str)] = &[
    (4, "(0.00 * -1)", "-0.00"),
    (7, "(-(0.00 * -1))", "0.00"),
    (10, "((0.00 * -1) * -1)", "0.00"),
    (13, "((0.00 * -1) * 1)", "-0.00"),
    (16, "(0.00 / -5)", "-0.00"),
    (19, "((0.00 * -1) / 5)", "-0.00"),
    (22, "((0.00 * -1) + (0.00 * -1))", "-0.00"),
    (25, "((0.00 * -1) + 0.00)", "0.00"),
    (28, "((0.00 * -1) - 0.00)", "-0.00"),
    (31, "(0.00 - (0.00 * -1))", "0.00"),
    (34, "(-0.00)", "0.00"),
    (37, "(-0.00 * 1)", "0.00"),
    (40, "(0 * -1)", "-0"),
    (43, "(0.0 - 0.0)", "0.0"),
    (46, "(-1.0 + 1.0)", "0.0"),
    (49, "(0 / 5)", "0"),
    (52, "(0.00 / 5)", "0.00"),
    (55, "(0.00 * -0.00)", "0.0000"),
];

/// The number of each posting, by the posting's line.
fn numbers(content: &str) -> BTreeMap<u32, String> {
    load(&[file("x.beancount", content)])
        .transactions()
        .iter()
        .flat_map(|t| &t.postings)
        .map(|p| (p.line.unwrap(), p.amounts[0].quantity.clone()))
        .collect()
}

/// Every row of a table of the note: the number `bean-query` printed for the posting on that line.
fn matches_the_reference(content: &str, table: &[(u32, &str, &str)]) {
    assert_eq!(findings("x.beancount", content), json!([]));
    let got = numbers(content);
    for (line, written, want) in table {
        assert_eq!(
            got.get(line).map(String::as_str),
            Some(*want),
            "line {line}: {written}"
        );
    }
}

fn transaction_count(content: &str) -> usize {
    load(&[file("x.beancount", content)]).transactions().len()
}

#[test]
fn the_four_operators_signs_parentheses_and_blanks_give_the_reference_numbers() {
    matches_the_reference(OPERATORS_PROBE, OPERATORS);
    // The elided posting of each transaction balances against the evaluated weights.
    assert_eq!(transaction_count(OPERATORS_PROBE), 13);
}

#[test]
fn a_division_has_the_exponent_the_reference_gives() {
    matches_the_reference(DIVISION_PROBE, DIVISION);
    // The elided posting is the negated sum, which keeps a positive exponent (bean-query).
    let got = numbers(DIVISION_PROBE);
    for (line, want) in [(90, "-1.0E+2"), (94, "-1.00E+3"), (98, "-1.00E+4")] {
        assert_eq!(
            got.get(&line).map(String::as_str),
            Some(want),
            "line {line}"
        );
    }
}

/// The probe without the transactions on these lines (kept as empty lines, so the others keep
/// their line numbers).
#[test]
fn the_places_of_a_result_and_its_rounding_to_28_digits_follow_the_reference() {
    matches_the_reference(PLACES_PROBE, PLACES);
}

#[test]
fn the_sign_of_a_zero_follows_the_decimal_rules() {
    matches_the_reference(ZERO_PROBE, ZERO);
}

#[test]
fn the_tolerance_is_that_of_the_places_of_the_result() {
    assert_eq!(findings("x.beancount", TOLERANCE_PROBE), json!([]));
    assert_eq!(transaction_count(TOLERANCE_PROBE), 8);
}

#[test]
fn an_inexact_quotient_leaves_a_residual_the_reference_rejects() {
    let found = short_findings("x.beancount", TOLERANCE_FAILS_PROBE);
    let lines: Vec<u64> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f[0].as_u64().unwrap())
        .collect();
    assert_eq!(lines, [4, 10, 16, 20, 24, 28, 32, 36, 40]);
    assert!(
        found
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f[1] == json!("unbalanced-transaction"))
    );
    // bean-query still lists the transactions.
    assert_eq!(transaction_count(TOLERANCE_FAILS_PROBE), 9);
    // The first residual: three thirds of 100 against 100, 1E-26.
    let first = &findings("x.beancount", TOLERANCE_FAILS_PROBE)[0];
    assert_eq!(
        first["details"]["residual"],
        json!([{ "commodity": "EUR", "quantity": "-1E-26" }])
    );
}

#[test]
fn a_price_a_cost_and_both_parts_of_a_cost_can_be_expressions() {
    assert_eq!(findings("x.beancount", POSITIONS_PROBE), json!([]));
    // Per posting: [account, quantity, commodity, line, price, lot].
    let rows = beancount_rows("x.beancount", POSITIONS_PROBE);
    let postings: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|t| t[5].as_array().unwrap().clone())
        .collect();
    let at = |line: u64| {
        postings
            .iter()
            .find(|p| p[3] == json!(line))
            .unwrap_or_else(|| panic!("no posting on line {line}"))
    };
    for (line, price) in [
        (6, "2.5 EUR"),
        (10, "1.25 EUR"),
        (14, "2 EUR"),
        (18, "2.5 EUR"),
    ] {
        assert_eq!(at(line)[4], json!(price), "price on line {line}");
    }
    assert_eq!(at(18)[1], json!("2"));
    for (line, lot) in [
        (22, "2.5 EUR 2026-01-06"),
        (26, "1.25 EUR 2026-01-07"),
        (30, "2.75 EUR 2026-01-08"),
        (34, "2.75 EUR 2026-01-09"),
        (38, "2 EUR 2026-01-10"),
        (42, "2 EUR 2026-01-10 lot"),
        (46, "1.5 EUR 2026-01-12"),
    ] {
        assert_eq!(at(line)[5], json!(lot), "lot on line {line}");
    }
    // An elided posting takes the evaluated weights: 2 x 1.5 = 3.0, kept with its place.
    assert_eq!(at(47)[1], json!("-3.0"));
    assert_eq!(at(31)[1], json!("-11"));
}

#[test]
fn a_balance_amount_and_its_tolerance_can_be_expressions() {
    let found = findings("x.beancount", BALANCE_PROBE);
    let failed: Vec<_> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["line"].as_u64().unwrap(), f["code"].as_str().unwrap()))
        .collect();
    assert_eq!(
        failed,
        [
            (27, "assertion-failed"),
            (30, "assertion-failed"),
            (32, "assertion-failed"),
            (35, "assertion-failed"),
        ]
    );
    assert_eq!(
        found[2]["details"]["expected"],
        json!("333.3333333333333333333333333")
    );
    assert_eq!(found[3]["details"]["expected"], json!("500.0"));
}

#[test]
fn a_price_directive_lists_the_value_of_its_expression() {
    let ledger = load(&[file("x.beancount", PRICE_PROBE)]);
    assert!(ledger.check().is_empty());
    let listed: Vec<_> = ledger
        .prices()
        .iter()
        .map(|p| (p.line, p.commodity.as_str(), p.rate.quantity.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            (4, "AAA", "2.5"),
            (5, "BBB", "2"),
            (6, "AAA", "3.333333333333333333333333333"),
            (7, "BBB", "3.00"),
            (8, "AAA", "0.125"),
            (9, "BBB", "-2"),
            (10, "AAA", "1E-7"),
            (11, "BBB", "1.0E+2"),
        ]
    );
}

#[test]
fn metadata_and_custom_values_can_be_expressions_and_change_nothing() {
    assert_eq!(findings("x.beancount", METADATA_PROBE), json!([]));
    assert_eq!(transaction_count(METADATA_PROBE), 1);
}

#[test]
fn bean_query_prints_the_number_unrounded() {
    let got = numbers(DISPLAY_PROBE);
    assert_eq!(
        got.get(&5).map(String::as_str),
        Some("33.33333333333333333333333333")
    );
    assert_eq!(findings("x.beancount", DISPLAY_PROBE), json!([]));
}

#[test]
fn a_malformed_expression_is_a_syntax_error_on_the_line_the_reference_names() {
    let found = short_findings("x.beancount", ERRORS_PROBE);
    let lines: Vec<u64> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f[0].as_u64().unwrap())
        .collect();
    assert_eq!(
        lines,
        [
            9, 17, 25, 33, 41, 49, 57, 65, 73, 81, 89, 97, 105, 113, 121, 129, 137, 145, 154, 163
        ]
    );
    assert!(
        found
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f[1] == json!("parse-error"))
    );
}

#[test]
fn a_division_by_zero_is_an_error_and_loses_only_its_entry() {
    let found = short_findings("x.beancount", ZERO_DIVISION_PROBE);
    assert_eq!(found, json!([[5, "division-by-zero", null]]));
    assert_eq!(transaction_count(ZERO_DIVISION_PROBE), 0);

    // The reference reports the line of the posting and keeps the entries around it.
    let found = short_findings("x.beancount", ZERO_DIVISION_FIRST_PROBE);
    assert_eq!(found, json!([[9, "division-by-zero", null]]));
    let kept: Vec<_> = load(&[file("x.beancount", ZERO_DIVISION_FIRST_PROBE)])
        .transactions()
        .iter()
        .map(|t| t.source.line)
        .collect();
    assert_eq!(kept, [4, 12]);
}

#[test]
fn a_division_by_zero_in_a_balance_or_a_price_loses_that_directive_only() {
    let journal = "2026-01-01 open Assets:A\n2026-01-01 commodity AAA\n2026-01-02 balance Assets:A  (1 / 0) EUR\n2026-01-02 price AAA (1 / (2 - 2)) EUR\n2026-01-03 price AAA 2 EUR\n";
    let ledger = load(&[file("x.beancount", journal)]);
    let lines: Vec<_> = ledger
        .check()
        .iter()
        .map(|f| (f.line, f.code.as_str()))
        .collect();
    assert_eq!(lines, [(3, "division-by-zero"), (4, "division-by-zero")]);
    assert_eq!(ledger.prices().len(), 1);
}

#[test]
fn two_divisions_by_zero_on_one_line_are_one_finding() {
    let journal = "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n\n2026-01-02 * \"t\"\n  Assets:A  (1 / 0) AAA {(2 / 0) EUR}\n  Assets:B\n";
    let found = short_findings("x.beancount", journal);
    assert_eq!(found, json!([[5, "division-by-zero", null]]));
}

#[test]
fn an_expression_deeper_than_the_limit_or_larger_than_the_decimal_type_does_not_panic() {
    // The parser crate would stop on these; they never reach it.
    let nested = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
    let big = "(99999999999999999999 * 99999999999999999999 * 99999999999999999999)";
    for amount in [nested.as_str(), big] {
        let journal = format!(
            "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n2026-01-02 * \"t\"\n  Assets:A  {amount} EUR\n  Assets:B\n"
        );
        let _ = load(&[file("x.beancount", &journal)]);
    }
}

#[test]
fn stacked_signs_without_parentheses_and_a_tolerance_sum_are_refused() {
    for line in ["  Assets:A  --4 EUR", "  Assets:A  +-6 EUR"] {
        let journal = format!(
            "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n2026-01-02 * \"t\"\n{line}\n  Assets:B\n"
        );
        assert_eq!(
            short_findings("x.beancount", &journal),
            json!([[4, "unsupported-syntax", "number-expression"]]),
            "{line}"
        );
    }
    let journal = "2026-01-01 open Assets:A\n2026-01-02 balance Assets:A  0 ~ 1 + 2 EUR\n";
    assert_eq!(
        short_findings("x.beancount", journal),
        json!([[2, "unsupported-syntax", "number-expression"]])
    );
}

#[test]
fn a_posting_flag_a_merge_mark_and_a_negative_amount_are_not_arithmetic() {
    let journal = "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n2026-01-02 * \"t (1/0) * 2\"\n  * Assets:A  -5 EUR ; (1 / 0)\n  Assets:B  5 EUR\n";
    assert_eq!(findings("x.beancount", journal), json!([]));
    assert_eq!(transaction_count(journal), 1);
}

#[test]
fn a_star_flag_straight_before_the_account_is_a_flag() {
    let journal = include_str!("../../../spec/beancount/probes/format-flag-gap-star.beancount");
    assert_eq!(findings("x.beancount", journal), json!([]));
    let flags: Vec<_> = load(&[file("x.beancount", journal)])
        .transactions()
        .iter()
        .flat_map(|t| &t.postings)
        .map(|p| p.flag.clone())
        .collect();
    assert_eq!(flags, [Some("*".to_string()), Some("*".to_string())]);
}

#[test]
fn format_aligns_the_postings_of_an_entry_left_out_for_a_division_by_zero() {
    // bean-format works on text, so the postings of the lost transaction are aligned all the same.
    let got = ledgermill::format(&ledgermill::SourceFile {
        name: "x.beancount".into(),
        content: ZERO_DIVISION_PROBE.into(),
    });
    assert!(got.contains("  Assets:B       -1 EUR\n"), "{got}");
}
