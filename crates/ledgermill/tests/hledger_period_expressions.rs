//! hledger's verdict on 474 period expressions (spec/hledger/period-forms.txt), as the header of
//! a periodic rule: ledgermill must accept the ones it accepts, reject the others on the line of
//! the rule, and never guess about an open range sign.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Severity, load};
use support::file;

const FORMS: &str = include_str!("../../../spec/hledger/period-forms.txt");

/// What ledgermill says about `~ EXPRESSION` as the first line of a journal: `(code, construct)`
/// of its first error, if any.
fn verdict(expression: &str) -> Option<(String, Option<String>)> {
    let journal = format!("~ {expression}\n    (x)  1\n");
    let ledger = load(&[file("p.journal", &journal)]);
    ledger
        .check()
        .iter()
        .find(|f| f.severity == Severity::Error)
        .map(|f| {
            assert_eq!(f.line, 1, "{expression}");
            (
                f.code.clone(),
                f.details["construct"].as_str().map(str::to_string),
            )
        })
}

/// An open range sign after a date: the reference tool takes some of these and not others, so
/// ledgermill refuses them all.
fn ends_with_a_range_sign(expression: &str) -> bool {
    let text = expression.trim_end();
    text.ends_with('-') || text.ends_with("..")
}

#[test]
fn every_period_expression_gets_hledgers_verdict() {
    let mut wrong = Vec::new();
    let (mut refused, mut accepted, mut rejected) = (0, 0, 0);
    for row in FORMS.lines() {
        let (expected, expression) = row.split_once('\t').unwrap();
        let ours = verdict(expression);
        let agrees = match (expected, &ours) {
            ("ok", None) => {
                accepted += 1;
                true
            }
            (_, Some((code, Some(construct))))
                if code == "unsupported-syntax" && construct == "period-expression" =>
            {
                refused += 1;
                ends_with_a_range_sign(expression)
            }
            (e, Some((code, None))) if e.starts_with("err:") => {
                rejected += 1;
                code == "parse-error"
            }
            _ => false,
        };
        if !agrees {
            wrong.push(format!("{expected}\t{expression}: ours {ours:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    // Most of the corpus is decided, one way or the other.
    assert!(accepted > 250 && rejected > 150 && refused <= 12);
    eprintln!("{accepted} accepted, {rejected} rejected, {refused} refused");
}
