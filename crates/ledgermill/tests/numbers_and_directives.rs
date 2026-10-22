//! Numbers, decimal marks and the directives that change how amounts and accounts are read.
//! Expected values: hledger 1.52.4 on the named probes (spec/hledger/numbers.md,
//! directives-numbers.md, aliases-and-parents.md, dates.md, directives-other.md), mantissa and
//! places written out as exact decimal strings.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::*;

fn probe_amounts(name: &str, content: &str) -> Value {
    assert_eq!(findings(name, content), json!([]), "{name} has findings");
    amounts(name, content)
}

#[test]
fn the_decimal_mark_is_decided_per_number() {
    let probe = include_str!("../../../spec/hledger/probes/numbers-marks.journal");
    assert_eq!(
        probe_amounts("numbers-marks", probe),
        json!([
            [1, [["a", "", "1000.5"], ["b", "", "-1000.5"]]],
            [5, [["a", "", "1000000"], ["b", "", "-1000000"]]],
            [9, [["a", "", "1000.5"], ["b", "", "-1000.5"]]],
            [13, [["a", "", "1.5"], ["b", "", "-1.5"]]],
            [17, [["a", "", "1.000"], ["b", "", "-1.000"]]]
        ])
    );
}

#[test]
fn other_number_forms_are_read() {
    let probe = include_str!("../../../spec/hledger/probes/numbers-forms.journal");
    assert_eq!(
        probe_amounts("numbers-forms", probe),
        json!([
            [1, [["a", "", "3"], ["b", "", "-3"]]],
            [5, [["a", "", "0.5"], ["b", "", "-0.5"]]],
            [9, [["a", "", "5"], ["b", "", "-5"]]],
            [13, [["a", "", "7.50"], ["b", "", "-7.5"]]],
            [17, [["a", "EUR", "-3"], ["b", "EUR", "3"]]],
            [21, [["a", "", "150"], ["b", "", "-150"]]],
            [25, [["a", "", "0.15"], ["b", "", "-0.15"]]],
            [29, [["a", "e", "2"], ["b", "e", "-2"]]]
        ])
    );
    let spaces = include_str!("../../../spec/hledger/probes/numbers-group-spaces.journal");
    assert_eq!(
        probe_amounts("numbers-group-spaces", spaces),
        json!([
            [1, [["a", "", "1000"], ["b", "", "-1000"]]],
            [5, [["a", "", "1000"], ["b", "", "-1000"]]]
        ])
    );
}

#[test]
fn values_are_exact_whatever_their_size_and_reported_to_ten_places() {
    let big = include_str!("../../../spec/hledger/probes/numbers-big.journal");
    assert_eq!(
        probe_amounts("numbers-big", big),
        json!([
            [
                1,
                [
                    ["a", "EUR", "9007199254740993"],
                    ["b", "EUR", "-9007199254740993"]
                ]
            ],
            [
                5,
                [
                    ["a", "EUR", "123456789012345678901234567890.1234567890"],
                    ["b", "EUR", "-123456789012345678901234567890.1234567890"]
                ]
            ]
        ])
    );
    let places = include_str!("../../../spec/hledger/probes/numbers-many-places.journal");
    assert_eq!(
        probe_amounts("numbers-many-places", places),
        json!([
            [
                1,
                [["a", "EUR", "0.0000000000"], ["b", "EUR", "0.0000000000"]]
            ],
            [
                5,
                [["a", "EUR", "1.1234567890"], ["b", "EUR", "-1.1234567890"]]
            ]
        ])
    );
}

#[test]
fn directives_decide_ambiguous_separators() {
    let mark = include_str!("../../../spec/hledger/probes/directive-decimal-mark.journal");
    assert_eq!(
        probe_amounts("directive-decimal-mark", mark),
        json!([[
            3,
            [["a", "", "1.000"], ["b", "", "-1000"], ["c", "", "999.000"]]
        ]])
    );
    let commodity = include_str!("../../../spec/hledger/probes/directive-commodity-format.journal");
    assert_eq!(
        probe_amounts("directive-commodity-format", commodity),
        json!([
            [
                5,
                [
                    ["a", "EUR", "1000"],
                    ["b", "USD", "-1000"],
                    ["c", "X", "1.000"],
                    ["d", "EUR", "-1000"],
                    ["d", "USD", "1000"],
                    ["d", "X", "-1.000"]
                ]
            ],
            [
                11,
                [
                    ["a", "EUR", "1.5"],
                    ["b", "EUR", "-15.0"],
                    ["c", "EUR", "13.5"]
                ]
            ],
            [16, [["a", "EUR", "1000.50"], ["b", "EUR", "-1000.50"]]]
        ])
    );
    let default = include_str!("../../../spec/hledger/probes/directive-default-commodity.journal");
    assert_eq!(
        probe_amounts("directive-default-commodity", default),
        json!([
            [3, [["a", "EUR", "5"], ["b", "EUR", "-5"]]],
            [
                7,
                [
                    ["a", "USD", "1000"],
                    ["b", "USD", "-1000"],
                    ["c", "USD", "1.000"],
                    ["d", "USD", "-1.000"]
                ]
            ]
        ])
    );
    let precedence = include_str!("../../../spec/hledger/probes/directive-precedence.journal");
    assert_eq!(
        probe_amounts("directive-precedence", precedence),
        json!([[
            5,
            [
                ["a", "EUR", "1000"],
                ["b", "USD", "-1000"],
                ["c", "EUR", "1.000"],
                ["d", "USD", "-1.000"]
            ]
        ]])
    );
}

#[test]
fn directives_reach_only_their_own_file() {
    let a = file(
        "a.journal",
        include_str!("../../../spec/hledger/probes/directive-scope-a.journal"),
    );
    let b = file(
        "b.journal",
        include_str!("../../../spec/hledger/probes/directive-scope-b.journal"),
    );
    let ledger = load(&[a, b]);
    assert!(ledger.check().is_empty());
    let posting = &ledger.transactions()[0].postings[0];
    assert_eq!(posting.account, "a");
    assert_eq!(posting.amounts[0].commodity, "");
    assert_eq!(posting.amounts[0].quantity, "1.000");
}

/// `[line, [account, ...]]` per transaction.
fn accounts(name: &str, content: &str) -> Value {
    assert_eq!(findings(name, content), json!([]), "{name} has findings");
    let rows: Vec<Value> = load(&[file(name, content)])
        .transactions()
        .iter()
        .map(|t| {
            let names: Vec<&str> = t.postings.iter().map(|p| p.account.as_str()).collect();
            json!([t.source.line, names])
        })
        .collect();
    Value::Array(rows)
}

#[test]
fn aliases_rename_accounts() {
    let probe = include_str!("../../../spec/hledger/probes/directive-alias.journal");
    assert_eq!(
        accounts("directive-alias", probe),
        json!([
            [
                5,
                [
                    "expenses:grub",
                    "expenses:x",
                    "expenses",
                    "exposed",
                    "bank:a"
                ]
            ],
            [14, ["exp:food", "assets:bank"]]
        ])
    );
    let order = include_str!("../../../spec/hledger/probes/directive-alias-order.journal");
    assert_eq!(
        accounts("directive-alias-order", order),
        json!([[4, ["c", "c"]]])
    );
}

#[test]
fn apply_account_prefixes_every_posting_until_its_end() {
    let probe = include_str!("../../../spec/hledger/probes/directive-apply-account.journal");
    assert_eq!(
        accounts("directive-apply-account", probe),
        json!([
            [
                4,
                [
                    "top:mid:a",
                    "top:mid:v",
                    "top:mid:bv",
                    "top:mid:bw",
                    "top:mid:b"
                ]
            ],
            [13, ["top:a", "top:b"]],
            [19, ["a", "b"]]
        ])
    );
}

#[test]
fn dates_take_their_form_and_year_from_the_journal() {
    let year = include_str!("../../../spec/hledger/probes/directive-year.journal");
    assert_eq!(findings("directive-year", year), json!([]));
    let dates: Vec<(Value, Value)> = transactions("directive-year", year)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["source"]["line"].clone(), t["date"].clone()))
        .collect();
    assert_eq!(
        dates,
        vec![
            (json!(13), json!("2024-02-29")),
            (json!(3), json!("2025-01-02")),
            (json!(7), json!("2026-01-03"))
        ]
    );
    let forms = include_str!("../../../spec/hledger/probes/header-code-and-dates.journal");
    assert_eq!(
        headers("header-code-and-dates", forms),
        json!([
            [17, "", "a four-digit year with zeros", "", ["", ""]],
            [1, "*", "slashes", "", ["", ""]],
            [5, "!", "dots, unpadded", "", ["", ""]],
            [9, "", "secondary date", "", ["", ""]],
            [13, "*", "y  spacing", "", ["", ""]]
        ])
    );
    let first: Vec<Value> = transactions("header-code-and-dates", forms)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["date"].clone())
        .collect();
    assert_eq!(
        first,
        vec![
            json!("0026-01-10"),
            json!("2026-01-05"),
            json!("2026-01-06"),
            json!("2026-01-07"),
            json!("2026-01-08")
        ]
    );
}

#[test]
fn directives_that_change_nothing_and_comment_blocks_are_read() {
    let ignored = include_str!("../../../spec/hledger/probes/directive-ignored.journal");
    assert_eq!(
        probe_amounts("directive-ignored", ignored),
        json!([[11, [["a", "X", "1"], ["b", "X", "-1"]]]])
    );
    let block = include_str!("../../../spec/hledger/probes/directive-comment-block.journal");
    assert_eq!(
        probe_amounts("directive-comment-block", block),
        json!([[7, [["a", "", "1"], ["b", "", "-1"]]]])
    );
}

#[test]
fn more_than_255_decimal_places_is_an_error_even_through_an_exponent() {
    for probe in [
        include_str!("../../../spec/hledger/probes/numbers-too-many-places.journal"),
        include_str!("../../../spec/hledger/probes/numbers-too-many-places-exponent.journal"),
    ] {
        let found = short_findings("n.journal", probe);
        assert_eq!(found, json!([[3, "parse-error", null]]), "{probe}");
    }
    // The exponent moves the point: 255 places remain in each of these.
    let zeros = "0".repeat(255);
    for number in [
        format!("1.{zeros}"),
        format!("1.{zeros}e0"),
        format!("1.0{zeros}e1"),
        "1e-255".to_string(),
    ] {
        let journal = format!("2026-01-01 t\n  a  {number}\n  b\n");
        assert_eq!(short_findings("n.journal", &journal), json!([]), "{number}");
    }
}
