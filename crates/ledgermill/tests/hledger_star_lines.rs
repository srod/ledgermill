//! A posting line that holds only `*` (spec/hledger/posting-star-line.md): it ends the
//! transaction, or the rule, and reads as a comment line. Probes are under spec/hledger/probes.
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
    ("comment-line-indented-after-alias", 0),
    ("comment-line-indented-hash-star", 0),
    ("posting-star-alone", 1),
    ("posting-star-alone-blanks", 1),
    ("posting-star-alone-comment-detached", 1),
    ("posting-star-alone-comments", 1),
    ("posting-star-alone-next-transaction", 2),
    ("rule-periodic-star-alone", 0),
    ("rule-star-alone", 0),
    ("rule-star-alone-after-posting", 0),
];

/// The probes hledger rejects at a line of its own, with that line.
const REJECTED: [(&str, u32); 5] = [
    ("posting-bang-alone", 4),
    ("posting-star-alone-in-middle", 4),
    ("posting-star-alone-then-posting", 5),
    ("rule-periodic-star-alone-then-posting", 3),
    ("rule-star-alone-then-posting", 3),
];

#[test]
fn a_lone_star_is_read_as_a_comment_line() {
    for (name, count) in READ {
        let journal = probe(name);
        // A rule gives its warning and nothing else.
        let errors: Vec<_> = short_findings(name, &journal)
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f[1] != json!("ignored-syntax"))
            .cloned()
            .collect();
        assert_eq!(errors, Vec::<serde_json::Value>::new(), "{name}");
        assert_eq!(
            transactions(name, &journal).as_array().unwrap().len(),
            count,
            "{name}"
        );
    }
}

#[test]
fn a_posting_after_a_lone_star_is_a_parse_error_on_its_line() {
    for (name, line) in REJECTED {
        // Later findings (the transaction cut short is unbalanced) come after hledger's first.
        let found = short_findings(name, &probe(name));
        let first = found
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f[1] != json!("ignored-syntax"));
        assert_eq!(first, Some(&json!([line, "parse-error", null])), "{name}");
    }
}

#[test]
fn comments_after_a_lone_star_do_not_belong_to_the_posting_above() {
    let journal = probe("posting-star-alone-comment-detached");
    assert_eq!(
        headers("d.journal", &journal),
        json!([[1, "", "t", "", ["k:v", ""]]])
    );
}

#[test]
fn a_star_with_something_after_it_is_a_posting() {
    for name in [
        "posting-star-account",
        "posting-star-comment-text",
        "posting-star-space-account",
        "posting-star-star",
    ] {
        let found = short_findings(name, &probe(name));
        assert_eq!(found.as_array().unwrap().len(), 1, "{name}");
        assert_eq!(found[0][0], json!(1), "{name}: the transaction is blamed");
        assert_ne!(found[0][1], json!("parse-error"), "{name}");
    }
}
