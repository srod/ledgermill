//! What ledgermill reports for Beancount instead of figures. The reference tool (bean-check
//! 3.2.3) accepts every `unsupported-syntax` journal here unless the note says otherwise
//! (spec/beancount/refusals.md) and rejects every `parse-error`, `booking-error` and
//! `unbalanced-transaction` one.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

fn refused(name: &str, content: &str, line: u32, construct: &str) {
    let l = load(&[file(name, content)]);
    assert!(
        l.transactions().is_empty(),
        "{name}: no transaction when a line is refused"
    );
    let found = short_findings(name, content);
    assert!(
        found
            .as_array()
            .unwrap()
            .contains(&json!([line, "unsupported-syntax", construct])),
        "{name}: {found}"
    );
}

#[test]
fn plugin_include_and_unchecked_options_are_refused() {
    refused(
        "p.beancount",
        include_str!("../../../spec/beancount/probes/refused/plugin.beancount"),
        1,
        "plugin",
    );
    refused(
        "i.beancount",
        include_str!("../../../spec/beancount/probes/refused/include.beancount"),
        1,
        "include",
    );
    let tolerance =
        include_str!("../../../spec/beancount/probes/refused/option-tolerance.beancount");
    refused("t.beancount", tolerance, 1, "option");
    refused("t.beancount", tolerance, 2, "option");
    let names = include_str!("../../../spec/beancount/probes/refused/option-unknown.beancount");
    refused("n.beancount", names, 1, "option");
    let finding = &findings("n.beancount", names)[0];
    assert_eq!(finding["details"]["name"], json!("name_assets"));
}

#[test]
fn options_that_change_nothing_are_accepted() {
    let journal = include_str!("../../../spec/beancount/probes/options-harmless.beancount");
    assert_eq!(findings("o.beancount", journal), json!([]));
    assert_eq!(
        load(&[file("o.beancount", journal)]).transactions().len(),
        1
    );
    let ignored = include_str!("../../../spec/beancount/probes/directives-ignored.beancount");
    assert_eq!(findings("d.beancount", ignored), json!([]));
    assert_eq!(
        load(&[file("d.beancount", ignored)]).transactions().len(),
        2
    );
}

#[test]
fn arithmetic_in_amounts_and_lot_merging_are_refused() {
    refused(
        "x.beancount",
        include_str!("../../../spec/beancount/probes/refused/number-expression.beancount"),
        5,
        "number-expression",
    );
    refused(
        "m.beancount",
        include_str!("../../../spec/beancount/probes/refused/cost-merge.beancount"),
        4,
        "cost-merge",
    );
}

#[test]
fn the_bean_extension_is_beancount_too_and_other_names_go_to_hledger() {
    let journal = "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n\n2026-01-02 * \"t\"\n  Assets:A  1 EUR\n  Assets:B\n";
    for name in ["a.bean", "A.BEANCOUNT"] {
        assert_eq!(
            load(&[file(name, journal)]).transactions().len(),
            1,
            "{name}"
        );
    }
    // Not Beancount: the hledger reader gets it and reads every dated line as a transaction (`hledger
    // print` lists the same three).
    let other = load(&[file("a.journal", journal)]);
    assert_eq!(other.transactions().len(), 3);
}

#[test]
fn a_syntax_error_is_a_parse_error_at_its_line() {
    let journal = "2026-01-01 open Assets:A\n\n2026-01-02 * \"t\" \"u\" \"v\"\n  Assets:A  1 EUR\n  Assets:A  -1 EUR\n";
    let found = short_findings("e.beancount", journal);
    assert_eq!(found[0][1], json!("parse-error"));
    assert_eq!(found[0][0], json!(3));
    assert!(
        load(&[file("e.beancount", journal)])
            .transactions()
            .is_empty()
    );
}

#[test]
fn unbalanced_and_unbookable_transactions_become_findings_and_nothing_panics() {
    let tolerance = include_str!("../../../spec/beancount/probes/balance-fails-integers.beancount");
    assert_eq!(
        short_findings("t.beancount", tolerance),
        json!([[4, "unbalanced-transaction", null]])
    );
    let found = findings("t.beancount", tolerance);
    assert_eq!(
        found[0]["details"]["residual"],
        json!([{ "commodity": "EUR", "quantity": "-0.4" }])
    );
    // The transaction is still reported, like hledger's unbalanced ones.
    assert_eq!(
        load(&[file("t.beancount", tolerance)]).transactions().len(),
        1
    );

    for (probe, reason) in [
        (
            include_str!("../../../spec/beancount/probes/balance-fails-tolerance.beancount"),
            None,
        ),
        (
            include_str!("../../../spec/beancount/probes/balance-fails-price.beancount"),
            None,
        ),
        (
            include_str!("../../../spec/beancount/probes/balance-fails-two-elided.beancount"),
            Some("ambiguous-auto-posting"),
        ),
    ] {
        let found = findings("t.beancount", probe);
        assert_eq!(found.as_array().unwrap().len(), 1, "{probe}");
        if let Some(reason) = reason {
            assert_eq!(found[0]["code"], json!("booking-error"));
            assert_eq!(found[0]["details"]["reason"], json!(reason));
        } else {
            assert_eq!(found[0]["code"], json!("unbalanced-transaction"));
        }
    }
}

#[test]
fn balance_tolerance_follows_the_places_written() {
    let journal = include_str!("../../../spec/beancount/probes/balance-tolerance.beancount");
    assert_eq!(findings("t.beancount", journal), json!([]));
}
