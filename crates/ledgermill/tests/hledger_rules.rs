//! Automated (`=`) and periodic (`~`) rules (spec/hledger/rules-ignored.md): hledger applies them
//! only with `--auto` and `--forecast`, so ledgermill reads them in full, keeps nothing of them,
//! warns once per rule, and rejects what hledger rejects, on the line it names.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Severity, load};
use serde_json::{Value, json};
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
const READ: [(&str, usize); 18] = [
    ("rule-amount-style", 1),
    ("rule-auto-basic", 2),
    ("rule-auto-comment-dates", 0),
    ("rule-auto-headers", 1),
    ("rule-auto-multiplier", 1),
    ("rule-auto-postings", 1),
    ("rule-crlf", 2),
    ("rule-directives", 1),
    ("rule-ends-transaction", 3),
    ("rule-file-c", 1),
    ("rule-header-no-final-newline", 1),
    ("rule-in-comment-block", 1),
    ("rule-no-final-newline", 1),
    ("rule-periodic-basic", 2),
    ("rule-periodic-comment-dates", 0),
    ("rule-periodic-description", 1),
    ("rule-periodic-headers", 1),
    ("rule-periodic-postings", 1),
];

/// The probes hledger rejects, with the line of its first error.
const REJECTED: [(&str, u32); 72] = [
    ("rule-periodic-bad-blank", 6),
    ("rule-periodic-bad-comma", 6),
    ("rule-periodic-bad-comment", 6),
    ("rule-periodic-bad-comment-glued", 6),
    ("rule-periodic-bad-day", 6),
    ("rule-periodic-bad-double-sign", 6),
    ("rule-periodic-bad-empty", 6),
    ("rule-periodic-bad-every", 6),
    ("rule-periodic-bad-every-date", 6),
    ("rule-periodic-bad-every-negative", 6),
    ("rule-periodic-bad-every-one-day", 6),
    ("rule-periodic-bad-from", 6),
    ("rule-periodic-bad-in", 6),
    ("rule-periodic-bad-month", 6),
    ("rule-periodic-bad-one-space", 6),
    ("rule-periodic-bad-order", 6),
    ("rule-periodic-bad-partial", 6),
    ("rule-periodic-bad-quarter", 6),
    ("rule-periodic-bad-tab", 6),
    ("rule-periodic-bad-two-dates", 6),
    ("rule-periodic-bad-word", 6),
    ("rule-periodic-bad-year", 6),
    ("rule-auto-bad-account-amount", 7),
    ("rule-auto-bad-account-only", 7),
    ("rule-auto-bad-assertion", 7),
    ("rule-auto-bad-assertion-lot", 7),
    ("rule-auto-bad-comment-bracket-date", 7),
    ("rule-auto-bad-comment-bracket-no-year", 7),
    ("rule-auto-bad-comment-bracket-no-year-after-tag", 7),
    ("rule-auto-bad-comment-continued-date", 8),
    ("rule-auto-bad-comment-date", 7),
    ("rule-auto-bad-comment-date-calendar", 7),
    ("rule-auto-bad-comment-date-empty", 7),
    ("rule-auto-bad-comment-date-no-year", 7),
    ("rule-auto-bad-comment-date-no-year-first", 7),
    ("rule-auto-bad-comment-date-no-year-next-line", 8),
    ("rule-auto-bad-date-amount", 7),
    ("rule-auto-bad-expression", 7),
    ("rule-auto-bad-junk", 7),
    ("rule-auto-bad-lot-date", 7),
    ("rule-auto-bad-lot-open", 7),
    ("rule-auto-bad-number", 7),
    ("rule-auto-bad-posting-after-blank", 9),
    ("rule-auto-bad-posting-after-margin", 9),
    ("rule-auto-bad-price", 7),
    ("rule-auto-bad-price-twice", 7),
    ("rule-auto-bad-quote", 7),
    ("rule-auto-bad-star-after", 7),
    ("rule-auto-bad-star-commodity", 7),
    ("rule-auto-bad-star-end", 7),
    ("rule-auto-bad-star-sign", 7),
    ("rule-auto-bad-star-space", 7),
    ("rule-auto-bad-star-twice", 7),
    ("rule-auto-bad-status-alone", 7),
    ("rule-periodic-bad-assertion", 7),
    ("rule-periodic-bad-comment-continued-date", 8),
    ("rule-periodic-bad-comment-date", 7),
    ("rule-periodic-bad-comment-date-calendar", 7),
    ("rule-periodic-bad-comment-date-leap", 4),
    ("rule-periodic-bad-comment-date-leap-bracket", 4),
    ("rule-periodic-bad-junk", 7),
    ("rule-periodic-bad-number", 7),
    ("rule-periodic-bad-posting-after-blank", 9),
    ("rule-periodic-bad-star", 7),
    ("rule-periodic-bad-star-sign", 7),
    ("rule-auto-indented", 6),
    ("rule-periodic-indented", 6),
    ("rule-bad-before-unbalanced", 7),
    ("rule-bad-after-unbalanced", 7),
    ("rule-unbalanced-after-valid", 5),
    ("rule-file-a", 2),
    ("rule-file-b", 3),
];

#[test]
fn rules_hledger_reads_leave_no_trace_but_a_warning_per_rule() {
    for (name, transactions_count) in READ {
        let journal = probe(name);
        let ledger = load(&[file(&format!("{name}.journal"), &journal)]);
        let found = ledger.check();
        assert!(
            found.iter().all(|f| f.severity == Severity::Warning),
            "{name}: {found:?}"
        );
        assert_eq!(ledger.transactions().len(), transactions_count, "{name}");
        // One warning per rule header, on its line, naming its kind. A header inside a comment
        // block is only a comment.
        let in_comment_block = name == "rule-in-comment-block";
        let headers: Vec<(u32, &str)> = journal
            .lines()
            .enumerate()
            .filter(|_| !in_comment_block)
            .filter_map(|(i, l)| {
                let construct = match l.chars().next()? {
                    '=' => "automated-rule",
                    '~' => "periodic-rule",
                    _ => return None,
                };
                Some((u32::try_from(i + 1).unwrap(), construct))
            })
            .collect();
        let warned: Vec<(u32, &str)> = found
            .iter()
            .map(|f| {
                assert_eq!(f.code, "ignored-syntax", "{name}");
                (f.line, f.details["construct"].as_str().unwrap())
            })
            .collect();
        assert_eq!(warned, headers, "{name}");
    }
}

#[test]
fn a_rule_changes_nothing_in_the_transactions_around_it() {
    // The transactions of a journal with rules are those of the same journal without them, on the
    // same lines: the rule lines are replaced by blank lines so that line numbers stay.
    for name in [
        "rule-auto-basic",
        "rule-periodic-basic",
        "rule-amount-style",
        "rule-ends-transaction",
    ] {
        let journal = probe(name);
        let without = journal
            .lines()
            .scan(false, |in_rule, l| {
                *in_rule = match l.chars().next() {
                    Some('=' | '~') => true,
                    Some(' ' | '\t') => *in_rule,
                    _ => false,
                };
                Some(if *in_rule { "" } else { l })
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            transactions(name, &journal),
            transactions(name, &without),
            "{name}"
        );
    }
}

#[test]
fn a_rule_hledger_rejects_is_a_parse_error_on_the_line_it_names() {
    for (name, line) in REJECTED {
        let found = short_findings(&format!("{name}.journal"), &probe(name));
        let first_error = found
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f[1] != "ignored-syntax")
            .unwrap_or_else(|| panic!("{name}: no error in {found}"));
        assert_eq!(first_error[0], line, "{name}: {found}");
        assert!(
            ["parse-error", "unbalanced-transaction"].contains(&first_error[1].as_str().unwrap()),
            "{name}: {found}"
        );
    }
}

#[test]
fn a_file_is_finished_before_the_next_one_starts() {
    let [a, b, c] = ["rule-file-a", "rule-file-b", "rule-file-c"]
        .map(|n| file(&format!("{n}.journal"), &probe(n)));
    // [file, line, code] of the first error of each order.
    let first = |files: &[ledgermill::SourceFile]| -> Value {
        let ledger = load(files);
        let f = ledger
            .check()
            .iter()
            .find(|f| f.severity == Severity::Error)
            .unwrap();
        json!([f.file, f.line, f.code])
    };
    assert_eq!(
        first(&[a.clone(), b.clone()]),
        json!(["rule-file-a.journal", 2, "unbalanced-transaction"])
    );
    assert_eq!(
        first(&[b, a.clone()]),
        json!(["rule-file-b.journal", 3, "parse-error"])
    );
    assert_eq!(
        first(&[c, a]),
        json!(["rule-file-a.journal", 2, "unbalanced-transaction"])
    );
}

#[test]
fn a_date_without_a_year_in_a_periodic_rule_is_only_a_question_for_29_february() {
    // The rule takes the year of the `Y` directive above it, or else the current year, which
    // ledgermill does not read: only 29 February depends on it.
    let cases: [(&str, &str, Value); 8] = [
        (
            "date tag",
            "~ monthly\n    a  1 EUR  ; date:02-28\n",
            json!([]),
        ),
        (
            "bracket",
            "~ monthly\n    a  1 EUR  ; [02-03=03-04]\n",
            json!([]),
        ),
        (
            "29 February",
            "~ monthly\n    a  1 EUR  ; date:02-29\n",
            json!([[2, "unsupported-syntax", "date-without-year"]]),
        ),
        (
            "29 February in a lot date",
            "~ monthly\n    a  1 EUR [02-29]\n",
            json!([[2, "unsupported-syntax", "date-without-year"]]),
        ),
        (
            "29 February after a leap Y",
            "Y 2024\n~ monthly\n    a  1 EUR  ; date:02-29\n",
            json!([]),
        ),
        (
            "29 February after another Y",
            "Y 2025\n~ monthly\n    a  1 EUR  ; date:02-29\n",
            json!([[3, "parse-error", null]]),
        ),
        (
            "30 February",
            "~ monthly\n    a  1 EUR  ; date:02-30\n",
            json!([[2, "parse-error", null]]),
        ),
        (
            "second date of a bracket",
            "~ monthly\n    a  1 EUR  ; [2025-02-03=02-29]\n",
            json!([[2, "parse-error", null]]),
        ),
    ];
    for (what, journal, expected) in cases {
        let found = short_findings("d.journal", journal);
        // Drop the warning of the rule itself.
        let errors: Vec<&Value> = found
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f[1] != "ignored-syntax")
            .collect();
        assert_eq!(json!(errors), expected, "{what}");
    }
}

#[test]
fn an_automated_rule_has_no_year_for_the_dates_of_its_comments() {
    let cases: [(&str, &str, Value); 5] = [
        (
            "no year",
            "= x\n    a  1 EUR  ; date:02-03\n",
            json!([[2, "parse-error", null]]),
        ),
        (
            "not even with Y",
            "Y 2026\n= x\n    a  1 EUR  ; [02-03]\n",
            json!([[3, "parse-error", null]]),
        ),
        (
            "a full date, then another tag",
            "= x\n    a  1 EUR  ; date:2026-01-02, date:02-03\n",
            json!([[2, "parse-error", null]]),
        ),
        (
            "one tag holds both dates, and only the first is read",
            "= x\n    a  1 EUR  ; date:2026-01-02 date:02-03\n",
            json!([]),
        ),
        (
            "the second date of a bracket takes the first one's year",
            "= x\n    a  1 EUR  ; [2026-01-02=02-03]\n",
            json!([]),
        ),
    ];
    for (what, journal, expected) in cases {
        let found = short_findings("d.journal", journal);
        let errors: Vec<&Value> = found
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f[1] != "ignored-syntax")
            .collect();
        assert_eq!(json!(errors), expected, "{what}");
    }
}

#[test]
fn the_number_of_a_rule_is_never_read_but_its_shape_is_checked() {
    // 1e5000 is too large to hold, but nothing reads a rule's number.
    let big = "= x\n    a  1e5000\n~ monthly\n    a  1e5000 EUR\n";
    assert!(
        short_findings("b.journal", big)
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f[1] == "ignored-syntax")
    );
    // The same number in a transaction is refused.
    assert_eq!(
        short_findings("b.journal", "2026-01-01 t\n    a  1e5000\n    b\n"),
        json!([[2, "unsupported-syntax", "number-too-large"]])
    );
    // More than 255 places is an error in a rule too.
    assert_eq!(
        short_findings("b.journal", "= x\n    a  1e-256\n")
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f[1] != "ignored-syntax")
            .count(),
        1
    );
}

#[test]
fn a_period_expression_that_cannot_be_judged_is_refused_at_its_line() {
    let found = short_findings(
        "p.journal",
        "2026-01-01 t\n  a  1\n  b\n\n~ jan-\n    (x)  1\n",
    );
    assert_eq!(
        found,
        json!([[5, "unsupported-syntax", "period-expression"]])
    );
}

#[test]
fn an_indented_rule_header_is_not_a_rule() {
    for header in [" = x", " ~ monthly"] {
        let journal = format!("2026-01-01 t\n    a  1\n    b\n\n{header}\n    (x)  1\n");
        let found = short_findings("i.journal", &journal);
        assert_eq!(found[0], json!([5, "parse-error", null]), "{header}");
    }
}
