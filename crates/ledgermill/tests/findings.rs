//! What ledgermill must refuse, and how it says so. hledger 1.52.4 accepts most of these journals
//! (see the notes under spec/hledger/); the point is that ledgermill never answers for them.
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

#[test]
fn a_finding_has_the_documented_shape() {
    let f = findings("x.journal", "2026-01-01 t\n  a  1 EUR extra\n  b  -1\n");
    assert_eq!(
        f,
        json!([{
            "code": "unsupported-syntax", "severity": "error", "file": "x.journal", "line": 2,
            "column": 6, "details": { "construct": "amount", "text": "1 EUR extra" },
            "message": "This amount form is not supported yet."
        }])
    );
}

#[test]
fn a_transaction_with_a_refused_line_is_not_returned_but_its_neighbours_are() {
    let journal = "2026-01-01 before\n  a  1\n  b  -1\n\n2026-01-02 refused\n  a  1\n  b\n\n2026-01-03 after\n  a  2\n  b  -2\n";
    assert_eq!(
        short("x.journal", journal),
        json!([refused(7, "elided-amount")])
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
fn postings_in_unsupported_forms_are_refused() {
    let cases = [
        ("posting-elided-amount", refused(4, "elided-amount")),
        (
            "posting-single-space-before-amount",
            refused(3, "elided-amount"),
        ),
        ("posting-indented-header-line", refused(4, "elided-amount")),
        ("posting-status", refused(2, "account-name")),
        ("posting-virtual", refused(2, "account-name")),
        ("posting-price", refused(2, "amount")),
        ("posting-assertion", refused(2, "amount")),
        ("posting-junk-after-amount", refused(3, "amount")),
    ];
    let probes: [(&str, &str); 8] = [
        (
            "posting-elided-amount",
            include_str!("../../../spec/hledger/probes/posting-elided-amount.journal"),
        ),
        (
            "posting-single-space-before-amount",
            include_str!("../../../spec/hledger/probes/posting-single-space-before-amount.journal"),
        ),
        (
            "posting-indented-header-line",
            include_str!("../../../spec/hledger/probes/posting-indented-header-line.journal"),
        ),
        (
            "posting-status",
            include_str!("../../../spec/hledger/probes/posting-status.journal"),
        ),
        (
            "posting-virtual",
            include_str!("../../../spec/hledger/probes/posting-virtual.journal"),
        ),
        (
            "posting-price",
            include_str!("../../../spec/hledger/probes/posting-price.journal"),
        ),
        (
            "posting-assertion",
            include_str!("../../../spec/hledger/probes/posting-assertion.journal"),
        ),
        (
            "posting-junk-after-amount",
            include_str!("../../../spec/hledger/probes/posting-junk-after-amount.journal"),
        ),
    ];
    for ((name, expected), (probe_name, content)) in cases.iter().zip(probes) {
        assert_eq!(*name, probe_name);
        assert_eq!(short(name, content), json!([expected]), "{name}");
        assert_eq!(
            transactions(name, content),
            json!([]),
            "{name}: transaction must be dropped"
        );
    }
}

#[test]
fn number_forms_hledger_accepts_are_refused() {
    let probe =
        include_str!("../../../spec/hledger/probes/number-forms-accepted-by-hledger.journal");
    assert_eq!(
        short("n.journal", probe),
        json!([2, 6, 7, 10, 11, 14, 18].map(|line| refused(line, "amount")))
    );
    assert_eq!(transactions("n.journal", probe), json!([]));
}

#[test]
fn other_number_forms_are_refused() {
    for amount in [
        "1,000.00",
        "1 000",
        "1,5",
        "1.5.3",
        "1e2",
        "1E-2",
        "1.",
        "2.5%",
        "٣",
        "1_000",
        "--1",
        "- 1",
        "$ -$1",
        "EUR 1 EUR",
        "1  EUR",
        "\"AB C\" 1",
        "1.12345678901 EUR",
    ] {
        let journal = format!("2026-01-01 t\n  a  {amount}\n  b  -1\n");
        assert_eq!(
            short("n.journal", &journal),
            json!([refused(2, "amount")]),
            "{amount}"
        );
        assert_eq!(transactions("n.journal", &journal), json!([]), "{amount}");
    }
    let too_large = findings(
        "n.journal",
        "2026-01-01 t\n  a  1234567890123456\n  b  -1\n",
    );
    assert_eq!(too_large[0]["details"]["construct"], "number-too-large");
}

#[test]
fn headers_hledger_accepts_in_other_forms_are_refused() {
    let probe =
        include_str!("../../../spec/hledger/probes/header-unusual-accepted-by-hledger.journal");
    assert_eq!(
        short("h.journal", probe),
        json!([
            refused(1, "transaction-code"),
            refused(5, "header-syntax"),
            refused(9, "header-syntax"),
            refused(13, "date")
        ])
    );
    assert_eq!(transactions("h.journal", probe), json!([]));

    let glued = include_str!("../../../spec/hledger/probes/header-status-glued-to-date.journal");
    assert_eq!(
        short("h.journal", glued),
        json!([refused(1, "header-syntax")])
    );
    let invalid = include_str!("../../../spec/hledger/probes/date-invalid.journal");
    assert_eq!(short("h.journal", invalid), json!([refused(1, "date")]));
    let years = include_str!("../../../spec/hledger/probes/date-year-zero-and-five-digits.journal");
    assert_eq!(
        short("h.journal", years),
        json!([refused(1, "date"), refused(5, "date")])
    );
    let leap = include_str!("../../../spec/hledger/probes/date-leap.journal");
    assert_eq!(short("h.journal", leap), json!([]));

    for date in [
        "2026/01/05",
        "26-01-05",
        "20260105",
        "2026-13-01",
        "2026-00-10",
        "2026-04-31",
        "2025-02-29",
    ] {
        let journal = format!("{date} t\n  a  1\n  b  -1\n");
        assert_eq!(
            short("h.journal", &journal),
            json!([refused(1, "date")]),
            "{date}"
        );
    }
}

#[test]
fn directives_and_rules_are_refused_with_their_indented_lines() {
    let journal = "account assets:cash\ncommodity 1,000.00 EUR\n  format 1,000.00 EUR\n\ninclude other.journal\nP 2026-01-01 EUR 1.1 USD\nD 1,000.00 EUR\nalias a = b\nY 2026\ncomment\nend comment\n~ monthly\n  a  1\n  b  -1\n= expenses\n  a  1\n";
    assert_eq!(
        short("d.journal", journal),
        json!([
            refused(1, "directive"),
            refused(2, "directive"),
            refused(5, "directive"),
            refused(6, "directive"),
            refused(7, "directive"),
            refused(8, "directive"),
            refused(9, "directive"),
            refused(10, "directive"),
            refused(11, "directive"),
            refused(12, "periodic-or-auto-transaction"),
            refused(15, "periodic-or-auto-transaction"),
        ])
    );
    assert_eq!(transactions("d.journal", journal), json!([]));
}

#[test]
fn indented_lines_outside_a_transaction_are_refused_but_indented_comments_are_not() {
    let journal = "; top\n  ; fine\n  a  1\n\n2026-01-01 t\n  a  1\n  b  -1\n# ends it\n  b  1\n  ; ignored\n";
    assert_eq!(
        short("i.journal", journal),
        json!([refused(3, "indented-line"), refused(9, "indented-line")])
    );
    assert_eq!(
        transactions("i.journal", journal).as_array().unwrap().len(),
        1
    );
}

#[test]
fn tabs_and_unusual_whitespace_outside_the_indentation_are_refused() {
    for (what, journal) in [
        ("tab in description", "2026-01-01 a\tb\n  a  1\n  b  -1\n"),
        ("tab as separator", "2026-01-01 t\n  a\t1\n  b  -1\n"),
        ("tab in comment", "2026-01-01 t\n  a  1 ; x\ty\n  b  -1\n"),
        (
            "carriage return inside a line",
            "2026-01-01 t\n  a  1\r  b  -1\n",
        ),
        ("no-break space", "2026-01-01 t\u{a0}x\n  a  1\n  b  -1\n"),
        ("tab-only line", "2026-01-01 t\n  a  1\n\t\n  b  -1\n"),
    ] {
        let f = findings("w.journal", journal);
        assert!(!f.as_array().unwrap().is_empty(), "{what}");
        assert_eq!(f[0]["details"]["construct"], "control-character", "{what}");
        assert_eq!(transactions("w.journal", journal), json!([]), "{what}");
    }
}

#[test]
fn account_names_outside_the_supported_form_are_refused() {
    for account in [
        "a;x",
        "a(b)",
        "a=b",
        "*a",
        "!a",
        "(a)",
        "[a]",
        "-a",
        "a\u{2003}b",
    ] {
        let journal = format!("2026-01-01 t\n  {account}  1\n  b  -1\n");
        assert!(
            !findings("a.journal", &journal)
                .as_array()
                .unwrap()
                .is_empty(),
            "{account}"
        );
        assert_eq!(transactions("a.journal", &journal), json!([]), "{account}");
    }
    let ok =
        "2026-01-01 t\n  Assets:Bank Account  1\n  Expenses:Food & Drink, Café_2-b.c/d'e+f  -1\n";
    assert_eq!(findings("a.journal", ok), json!([]));
}

#[test]
fn several_commodities_in_a_transaction_are_refused_at_the_posting_that_differs() {
    for probe in [
        include_str!("../../../spec/hledger/probes/balance-two-commodities.journal"),
        include_str!("../../../spec/hledger/probes/balance-commodity-less-and-symbol.journal"),
        include_str!("../../../spec/hledger/probes/balance-case-differs.journal"),
    ] {
        assert_eq!(
            short("m.journal", probe),
            json!([refused(3, "multiple-commodities")])
        );
        assert_eq!(transactions("m.journal", probe), json!([]));
    }
}

#[test]
fn an_unbalanced_transaction_is_reported_on_its_header_and_still_returned() {
    let probe = include_str!("../../../spec/hledger/probes/balance-unbalanced.journal");
    assert_eq!(
        findings("u.journal", probe),
        json!([{
            "code": "unbalanced-transaction", "severity": "error", "file": "u.journal", "line": 1,
            "details": { "commodity": "EUR", "residual": "0.004" },
            "message": "The postings do not add up to zero."
        }])
    );
    assert_eq!(
        transactions("u.journal", probe).as_array().unwrap().len(),
        1
    );

    let single = include_str!("../../../spec/hledger/probes/balance-single-posting.journal");
    assert_eq!(
        short("u.journal", single),
        json!([[1, "unbalanced-transaction", null]])
    );
}

#[test]
fn balanced_forms_hledger_accepts_raise_no_finding() {
    let mixed = include_str!("../../../spec/hledger/probes/balance-ok-mixed-places.journal");
    assert_eq!(findings("b.journal", mixed), json!([]));
    let zero = include_str!("../../../spec/hledger/probes/balance-single-zero-posting.journal");
    assert_eq!(findings("b.journal", zero), json!([]));
    assert_eq!(transactions("b.journal", zero).as_array().unwrap().len(), 1);
}

#[test]
fn findings_are_ordered_by_file_then_line_and_name_their_file() {
    let a = file("a.journal", "2026-01-01 t\n  a  1\n  b\n\ninclude x\n");
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
            s("a.journal", 3, "unsupported-syntax"),
            s("a.journal", 5, "unsupported-syntax"),
            s("b.journal", 1, "unbalanced-transaction")
        ]
    );
}
