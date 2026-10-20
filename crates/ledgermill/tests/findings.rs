//! What ledgermill reports instead of figures, and how. Expected verdicts: hledger 1.52.4 on the
//! named probes (see the notes under spec/hledger/): it rejects every `parse-error` and
//! `unbalanced-transaction` journal here and accepts every `unsupported-syntax` one.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::*;

/// `[line, code, construct]` of each finding.
fn short(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = findings(name, content)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"], f["details"]["construct"]]))
        .collect();
    Value::Array(rows)
}

fn refused(line: u32, construct: &str) -> Value {
    json!([line, "unsupported-syntax", construct])
}

fn invalid(line: u32) -> Value {
    json!([line, "parse-error", null])
}

#[test]
fn a_finding_has_the_documented_shape() {
    let f = findings("x.journal", "2026-01-01 t\n  a  1 EUR = 1 EUR\n  b  -1\n");
    assert_eq!(
        f,
        json!([{
            "code": "unsupported-syntax", "severity": "error", "file": "x.journal", "line": 2,
            "details": { "construct": "balance-assertion", "text": "a  1 EUR = 1 EUR" },
            "message": "Balance assertions and assignments are not supported yet."
        }])
    );
}

#[test]
fn a_transaction_with_a_refused_line_is_not_returned_but_its_neighbours_are() {
    let journal = "2026-01-01 before\n  a  1\n  b  -1\n\n2026-01-02 refused\n  a  1 = 1\n  b\n\n2026-01-03 after\n  a  2\n  b  -2\n";
    assert_eq!(
        short("x.journal", journal),
        json!([refused(6, "balance-assertion")])
    );
    let lines: Vec<Value> = transactions("x.journal", journal)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["source"]["line"].clone())
        .collect();
    assert_eq!(lines, vec![json!(1), json!(9)]);
}

#[test]
fn lines_hledger_rejects_are_parse_errors() {
    let cases: [(&str, &str, Value); 12] = [
        (
            "numbers-invalid-separators",
            include_str!("../../../spec/hledger/probes/numbers-invalid-separators.journal"),
            invalid(2),
        ),
        (
            "numbers-groups-and-exponent",
            include_str!("../../../spec/hledger/probes/numbers-groups-and-exponent.journal"),
            invalid(2),
        ),
        (
            "directive-commodity-needs-mark",
            include_str!("../../../spec/hledger/probes/directive-commodity-needs-mark.journal"),
            invalid(1),
        ),
        (
            "directive-price-invalid",
            include_str!("../../../spec/hledger/probes/directive-price-invalid.journal"),
            invalid(1),
        ),
        (
            "directive-price-no-amount",
            include_str!("../../../spec/hledger/probes/directive-price-no-amount.journal"),
            invalid(1),
        ),
        (
            "header-date-mixed-separators",
            include_str!("../../../spec/hledger/probes/header-date-mixed-separators.journal"),
            invalid(1),
        ),
        (
            "header-status-glued-to-date",
            include_str!("../../../spec/hledger/probes/header-status-glued-to-date.journal"),
            invalid(1),
        ),
        (
            "date-invalid",
            include_str!("../../../spec/hledger/probes/date-invalid.journal"),
            invalid(1),
        ),
        (
            "conversion-type-invalid",
            include_str!("../../../spec/hledger/probes/conversion-type-invalid.journal"),
            invalid(1),
        ),
        (
            "comment-date-invalid",
            include_str!("../../../spec/hledger/probes/comment-date-invalid.journal"),
            invalid(2),
        ),
        (
            "posting-junk-after-amount",
            include_str!("../../../spec/hledger/probes/posting-junk-after-amount.journal"),
            invalid(3),
        ),
        (
            "an unknown word in column 0",
            "x\n2026-01-01 t\n  a  1\n  b\n",
            invalid(1),
        ),
    ];
    for (name, content, expected) in cases {
        assert_eq!(short(name, content), json!([expected]), "{name}");
    }
    // The posting after the comment is out of place; what came before is then incomplete.
    let ended = include_str!("../../../spec/hledger/probes/comments-col0-ends-transaction.journal");
    assert_eq!(
        short("c.journal", ended),
        json!([[1, "unbalanced-transaction", null], invalid(4)])
    );
}

#[test]
fn what_ledgermill_does_not_read_yet_is_refused_at_its_line() {
    let cases: [(&str, Value); 6] = [
        (
            "2026-01-01 t\n  a  1 = 1\n  b  -1\n",
            refused(2, "balance-assertion"),
        ),
        ("include other.journal\n", refused(1, "include")),
        ("~ monthly\n  a  1\n  b\n", refused(1, "directive")),
        ("= expenses\n  (budget)  *1\n", refused(1, "directive")),
        (
            include_str!("../../../spec/hledger/probes/directive-date-without-year.journal"),
            refused(1, "date-without-year"),
        ),
        (
            "2026-01-01 t\n  a  1\r  b  -1\n",
            refused(2, "control-character"),
        ),
    ];
    for (content, expected) in cases {
        assert_eq!(short("u.journal", content), json!([expected]), "{content}");
        assert_eq!(transactions("u.journal", content), json!([]), "{content}");
    }
}

#[test]
fn conversion_shapes_whose_rule_is_not_pinned_down_are_refused() {
    for journal in [
        // several priced postings, one right after an odd run of conversion postings that
        // opens the transaction
        "2026-01-01 t\n  equity:conversion  -3 X\n  a  3 X @ 2 EUR\n  b  4 Y @ 1 EUR\n  equity:conversion  1 EUR\n  equity:conversion  1 EUR\n  c  -6 EUR\n",
        // the same after a priced posting
        "2026-01-01 t\n  b  4 Y @ 1 EUR\n  equity:conversion  -3 X\n  a  3 X @ 2 EUR\n  equity:conversion  1 EUR\n  equity:conversion  1 EUR\n  c  -6 EUR\n",
        // a priced posting on a conversion account next to a pair
        "2026-01-01 t\n  equity:conversion  -3 X\n  equity:conversion  6 EUR\n  equity:conversion  3 X @ 2 EUR\n  c  -12 EUR\n  c  3 X\n",
    ] {
        assert_eq!(
            short("c.journal", journal),
            json!([refused(1, "conversion-postings")]),
            "{journal}"
        );
        assert_eq!(transactions("c.journal", journal), json!([]), "{journal}");
    }
}

#[test]
fn conversion_pairs_that_hledger_does_not_match_leave_the_transaction_unbalanced() {
    for probe in [
        include_str!("../../../spec/hledger/probes/conversion-pair-places-off.journal"),
        include_str!("../../../spec/hledger/probes/conversion-several-same-numbers.journal"),
    ] {
        assert_eq!(
            short("c.journal", probe),
            json!([[1, "unbalanced-transaction", null]]),
            "{probe}"
        );
    }
}

#[test]
fn an_unbalanced_transaction_is_reported_on_its_header_and_still_returned() {
    let probe = include_str!("../../../spec/hledger/probes/balance-unbalanced.journal");
    assert_eq!(
        findings("u.journal", probe),
        json!([{
            "code": "unbalanced-transaction", "severity": "error", "file": "u.journal", "line": 1,
            "details": { "group": "real", "residual": [{ "commodity": "EUR", "quantity": "0.004" }] },
            "message": "The postings do not add up to zero."
        }])
    );
    assert_eq!(
        transactions("u.journal", probe).as_array().unwrap().len(),
        1
    );
}

#[test]
fn hledger_rejections_after_parsing_are_unbalanced_transactions() {
    let cases: [(&str, &str); 6] = [
        (
            include_str!("../../../spec/hledger/probes/balance-single-posting.journal"),
            "group",
        ),
        (
            include_str!("../../../spec/hledger/probes/tolerance-fails.journal"),
            "group",
        ),
        (
            include_str!("../../../spec/hledger/probes/inferred-prices-same-sign.journal"),
            "group",
        ),
        (
            include_str!("../../../spec/hledger/probes/conversion-names.journal"),
            "group",
        ),
        (
            include_str!("../../../spec/hledger/probes/elided-two.journal"),
            "reason",
        ),
        (
            include_str!("../../../spec/hledger/probes/conversion-with-cost.journal"),
            "reason",
        ),
    ];
    for (content, key) in cases {
        let f = findings("u.journal", content);
        assert_eq!(f[0]["code"], "unbalanced-transaction", "{content}");
        assert!(f[0]["details"].get(key).is_some(), "{content}");
    }
    let elided = include_str!("../../../spec/hledger/probes/conversion-elided-with-pair.journal");
    assert_eq!(
        findings("u.journal", elided)[0]["code"],
        "unbalanced-transaction"
    );
}

#[test]
fn balanced_forms_hledger_accepts_raise_no_finding() {
    for probe in [
        include_str!("../../../spec/hledger/probes/balance-ok-mixed-places.journal"),
        include_str!("../../../spec/hledger/probes/balance-single-zero-posting.journal"),
        include_str!("../../../spec/hledger/probes/tolerance.journal"),
        include_str!("../../../spec/hledger/probes/conversion.journal"),
        include_str!("../../../spec/hledger/probes/directive-ignored.journal"),
    ] {
        assert_eq!(findings("b.journal", probe), json!([]), "{probe}");
    }
}

#[test]
fn findings_are_ordered_by_file_then_line_and_name_their_file() {
    let a = file("a.journal", "2026-01-01 t\n  a  1 = 1\n  b\n\ninclude x\n");
    let b = file("b.journal", "2026-01-01 t\n  a  1 EUR\n  b  -2 EUR\n");
    let all: Vec<(String, u32, String)> = load(&[a, b])
        .check()
        .iter()
        .map(|f| (f.file.clone(), f.line, f.code.clone()))
        .collect();
    let s = |f: &str, l, c: &str| (f.to_string(), l, c.to_string());
    assert_eq!(
        all,
        vec![
            s("a.journal", 2, "unsupported-syntax"),
            s("a.journal", 5, "unsupported-syntax"),
            s("b.journal", 1, "unbalanced-transaction")
        ]
    );
}
