//! What `check()` says about the file itself: names, tags, metadata, syntax, and the `document`
//! directive that ledgermill reads and skips (spec/beancount/check-errors.md, check-order.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::*;

fn probe(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../spec/beancount/probes/{name}.beancount",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// `[line, code]` of each finding, in order.
fn lines_and_codes(name: &str) -> Value {
    Value::Array(
        findings("f.beancount", &probe(name))
            .as_array()
            .unwrap()
            .iter()
            .map(|f| json!([f["line"], f["code"]]))
            .collect(),
    )
}

#[test]
fn tags_and_metadata_errors_come_in_file_order_and_the_pushes_left_open_last_on_line_0() {
    let found = findings("f.beancount", &probe("check-tags-and-meta"));
    let summary: Vec<Value> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"]]))
        .collect();
    assert_eq!(
        summary,
        vec![
            json!([1, "unbalanced-push"]),
            json!([4, "unbalanced-push"]),
            json!([8, "duplicate-metadata"]),
            json!([8, "duplicate-metadata"]),
            json!([0, "unbalanced-push"]),
            json!([0, "unbalanced-push"]),
        ]
    );
    assert_eq!(
        found[0]["details"],
        json!({ "kind": "tag", "name": "never-pushed", "reason": "pop-without-push" })
    );
    assert_eq!(
        found[1]["details"],
        json!({ "kind": "metadata", "name": "other", "reason": "pop-without-push" })
    );
    assert_eq!(
        found[2]["details"],
        json!({ "key": "note", "scope": "entry" })
    );
    assert_eq!(
        found[3]["details"],
        json!({ "key": "memo", "scope": "posting" })
    );
    assert_eq!(
        found[4]["details"],
        json!({ "kind": "tag", "name": "left-open", "reason": "push-not-popped" })
    );
    assert_eq!(
        found[5]["details"],
        json!({ "kind": "metadata", "name": "trip", "reason": "push-not-popped" })
    );
}

#[test]
fn a_bad_account_root_is_an_invalid_name_on_the_line_of_the_name() {
    let found = findings("f.beancount", &probe("check-account-names"));
    assert_eq!(found[0]["line"], json!(5));
    assert_eq!(found[0]["code"], json!("invalid-account-name"));
    assert_eq!(found[0]["details"], json!({ "account": "Asset:Typo" }));
}

#[test]
fn a_syntax_error_is_a_parse_error_and_the_first_one_is_in_file_order() {
    let found = lines_and_codes("check-parse-errors");
    assert_eq!(found[0], json!([7, "parse-error"]));
}

/// A date or time written with non-ASCII digits is a parse error on its line, as in bean-check,
/// and `format()` returns the file as it was (PLA-406: the parser used to panic).
#[test]
fn a_date_or_time_in_non_ascii_digits_is_a_parse_error_on_its_line() {
    for (name, line) in [("check-digits-date", 3), ("check-digits-time", 5)] {
        let journal = probe(name);
        let found = findings("f.beancount", &journal);
        assert_eq!(found[0]["code"], json!("parse-error"), "{name}");
        assert_eq!(found[0]["file"], json!("f.beancount"), "{name}");
        assert_eq!(found[0]["line"], json!(line), "{name}");
        let formatted = ledgermill::format(&file("f.beancount", &journal));
        assert_eq!(formatted, journal, "{name}");
    }
}

#[test]
fn a_document_is_read_and_skipped_with_a_warning() {
    let journal = probe("check-document-existing");
    let found = findings("f.beancount", &journal);
    assert_eq!(
        found,
        json!([
            {
                "code": "ignored-syntax", "severity": "warning", "file": "f.beancount",
                "line": 3, "details": { "construct": "document" },
                "message": found[0]["message"]
            },
            {
                "code": "ignored-syntax", "severity": "warning", "file": "f.beancount",
                "line": 4, "details": { "construct": "document" },
                "message": found[1]["message"]
            }
        ])
    );
    // The transaction after them is read as usual.
    assert_eq!(
        load(&[file("f.beancount", &journal)]).transactions().len(),
        1
    );
}

#[test]
fn a_document_on_an_unknown_account_is_an_error_besides_the_warning() {
    let journal = "2026-01-01 open Assets:A\n2026-01-02 document Assets:Never \"x.pdf\"\n";
    let found = findings("f.beancount", journal);
    let codes: Vec<(Value, Value)> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["code"].clone(), f["severity"].clone()))
        .collect();
    assert_eq!(
        codes,
        vec![
            (json!("ignored-syntax"), json!("warning")),
            (json!("account-not-open"), json!("error"))
        ]
    );
}

/// A booking error is on the line of the posting the reference blames for what it cannot
/// categorize or work out, and on the transaction's header for matching a reduction to lots
/// (spec/beancount/check-errors.md, "Which line a booking error is on").
#[test]
fn a_booking_error_is_on_the_posting_the_reference_blames_or_on_the_header_for_lots() {
    for (name, line, reason) in [
        ("booking-line-no-units-currency", 7, "cannot-infer-currency"),
        (
            "booking-line-price-without-units",
            7,
            "cannot-infer-currency",
        ),
        (
            "booking-line-units-missing-number",
            7,
            "too-many-missing-numbers",
        ),
        ("booking-line-negative-price", 7, "negative-price"),
        ("booking-line-second-posting", 8, "cannot-infer-currency"),
        (
            "booking-line-missing-number-first",
            6,
            "cannot-infer-currency",
        ),
        ("booking-line-three-elided", 9, "ambiguous-auto-posting"),
        (
            "booking-line-elided-after-amounts",
            8,
            "ambiguous-auto-posting",
        ),
        ("booking-line-not-enough-lots", 10, "not-enough-lots"),
        ("booking-line-no-matching-lot", 10, "no-matching-lot"),
        ("lots-inferred-cost-negative-result", 7, "negative-cost"),
    ] {
        let found = findings("b.beancount", &probe(name));
        let first = &found[0];
        assert_eq!(first["code"], json!("booking-error"), "{name}");
        assert_eq!(first["line"], json!(line), "{name}");
        assert_eq!(first["details"]["reason"], json!(reason), "{name}");
    }
}

/// Zero units with a written cost per unit is a booking error on that posting, and the
/// transaction is still listed; zero units plain or with a price are fine
/// (spec/beancount/check-errors.md, "Zero units at a cost").
#[test]
fn zero_units_at_a_written_cost_is_a_booking_error_on_the_posting_and_the_transaction_is_kept() {
    for (name, line) in [
        ("booking-zero-amount-at-cost", 5),
        ("booking-zero-amount-at-cost-places", 5),
        ("booking-zero-amount-at-cost-negative", 5),
        ("booking-zero-amount-at-cost-second", 6),
    ] {
        let found = findings("b.beancount", &probe(name));
        assert_eq!(found.as_array().unwrap().len(), 1, "{name}");
        assert_eq!(found[0]["code"], json!("booking-error"), "{name}");
        assert_eq!(found[0]["line"], json!(line), "{name}");
        assert_eq!(
            found[0]["details"]["reason"],
            json!("zero-amount-at-cost"),
            "{name}"
        );
        let listed = transactions("b.beancount", &probe(name));
        assert_eq!(listed.as_array().unwrap().len(), 1, "{name}");
    }
    for name in [
        "booking-zero-amount-plain",
        "booking-zero-amount-with-price",
    ] {
        assert_eq!(findings("b.beancount", &probe(name)), json!([]), "{name}");
    }
}
