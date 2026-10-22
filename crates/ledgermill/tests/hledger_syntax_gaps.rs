//! The directive and amount forms that ledgermill used to get wrong (spec/hledger/syntax.md):
//! hledger reads them, or rejects them on a line, and ledgermill now says the same. Probes are
//! under spec/hledger/probes; the rest of the margin is in `hledger_directive_forms.rs`.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::json;
use std::path::PathBuf;
use support::*;

fn probe(name: &str) -> String {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "../../spec/hledger/probes",
        &format!("{name}.journal"),
    ]
    .iter()
    .collect();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The probes hledger reads, with the number of transactions it prints for each.
const READ: [(&str, usize); 10] = [
    ("directive-commodity-lone-mark", 1),
    ("directive-default-commodity-text", 1),
    ("directive-end-apply-fixed-text", 1),
    ("directive-end-apply-tag-text", 1),
    ("directive-end-apply-year-text", 1),
    ("directive-flag-line", 1),
    ("directive-python-block", 1),
    ("directive-year-glued", 1),
    ("syntax-valuation-expression", 2),
    ("syntax-virtual-cost", 2),
];

/// The probes hledger rejects, with the line of its first error.
const REJECTED: [(&str, u32); 17] = [
    ("directive-account-balanced-virtual-name", 2),
    ("directive-account-virtual-name", 2),
    ("directive-alias-empty-pattern", 2),
    ("directive-alias-no-equals", 7),
    ("directive-apply-bare", 2),
    ("directive-apply-unknown", 2),
    ("directive-bare-a", 2),
    ("directive-bare-bucket", 2),
    ("directive-bare-c", 2),
    ("directive-bare-n", 2),
    ("directive-c-no-number", 2),
    ("directive-c-one-amount", 2),
    ("directive-comment-end-text", 4),
    ("directive-n-number", 2),
    ("directive-tag-two-words", 2),
    ("numbers-too-many-places-exponent", 3),
    ("numbers-too-many-places", 3),
];

#[test]
fn what_hledger_reads_is_read() {
    for (name, count) in READ {
        let journal = probe(name);
        assert_eq!(findings(name, &journal), json!([]), "{name}");
        assert_eq!(
            transactions(name, &journal).as_array().unwrap().len(),
            count,
            "{name}"
        );
    }
}

#[test]
fn what_hledger_rejects_is_a_parse_error_on_its_line() {
    for (name, line) in REJECTED {
        assert_eq!(
            short_findings(name, &probe(name)),
            json!([[line, "parse-error", null]]),
            "{name}"
        );
    }
}

#[test]
fn a_virtual_cost_prices_the_amount_like_a_cost() {
    let journal = "2026-01-02 t\n  a  1 EUR (@) 1.1 USD\n  b  -1.1 USD\n\n2026-01-03 u\n  a  2 EUR (@@) 2.2 USD\n  b  -2.2 USD\n";
    let found = transactions("v.journal", journal);
    let cost = |tx: usize| found[tx]["postings"][0]["amounts"][0]["cost"].clone();
    assert_eq!(cost(0)["kind"], "unit");
    assert_eq!(cost(0)["amount"]["quantity"], "1.1");
    assert_eq!(cost(1)["kind"], "total");
    assert_eq!(cost(1)["amount"]["quantity"], "2.2");
}

#[test]
fn a_parenthesis_that_is_not_a_virtual_cost_or_valuation_is_a_label() {
    // No amount after `(@)`: a label. A second `((...))`: the label `(...`, and a stray `)`.
    for (line, count) in [
        ("1 EUR (@) @ 1 USD", 0),
        ("1 EUR ((x)) ((y)", 0),
        ("1 EUR ((x)) ((y))", 1),
        ("1 EUR ((x)", 1),
        ("1 EUR ((a)b))", 1),
        ("1 EUR = 1 EUR ((x))", 1),
        ("1 EUR (@) (lab)", 1),
    ] {
        let journal = format!("2026-01-02 t\n  a  {line}\n  b\n");
        assert_eq!(
            findings("l.journal", &journal).as_array().unwrap().len(),
            count,
            "{line}"
        );
    }
}

#[test]
fn an_alias_without_equals_reads_to_the_next_one_in_the_file() {
    let journal = "alias a\n\n2026-01-01 t\n  x  1\n  y\n\nalias b = c\n";
    // `alias a` runs on to the `=` of the second line, taking the transaction with it.
    assert_eq!(findings("a.journal", journal), json!([]));
    assert_eq!(transactions("a.journal", journal), json!([]));
    let aliased = "alias a\n=b\n\n2026-01-01 t\n  a  1\n  c\n";
    assert_eq!(
        amounts("a.journal", aliased),
        json!([[4, [["b", "", "1"], ["c", "", "-1"]]]])
    );
}

#[test]
fn a_status_mark_alone_is_not_a_posting() {
    let journal = "2026-01-01 t\n  a  1\n  !\n";
    assert_eq!(
        short_findings("s.journal", journal),
        json!([[3, "parse-error", null]])
    );
}
