//! hledger's verdict on 200 lines of the margin (spec/hledger/directive-forms.txt): ledgermill
//! must accept the same lines and reject the same ones, on the line hledger blames.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Severity, load};
use support::file;

const FORMS: &str = include_str!("../../../spec/hledger/directive-forms.txt");

/// The journal `directive-verdicts.sh` writes for a line: the line (with `\n` and `\t` spelled
/// out), then a small valid transaction.
fn journal(line: &str) -> String {
    let text = line.replace("\\n", "\n").replace("\\t", "\t");
    format!("{text}\n\n2026-01-02 t\n    a  1\n    b\n")
}

#[test]
fn every_line_of_the_margin_gets_hledgers_verdict() {
    let mut wrong = Vec::new();
    for row in FORMS.lines() {
        let (verdict, line) = row.split_once('\t').unwrap();
        let ledger = load(&[file("d.journal", &journal(line))]);
        let first = ledger
            .check()
            .iter()
            .find(|f| f.severity == Severity::Error);
        let ours = first.map(|f| (f.code.as_str(), f.line));
        let expected = verdict
            .strip_prefix("err:")
            .map(|p| p.split(':').next().unwrap().parse::<u32>().ok());
        let agrees = match (expected, ours) {
            (None, None) => true,
            // A date with no year and no `Y` directive is read with the current year by
            // hledger; ledgermill will not guess (spec/hledger/syntax.md).
            (None, Some(("unsupported-syntax", _))) => line.starts_with('P'),
            // `include` of a file that is not there is an error in hledger, a refusal here
            // (spec/hledger/syntax.md): same line.
            (Some(Some(l)), Some(("unsupported-syntax", got))) => {
                line.starts_with("include") && got == l
            }
            (Some(Some(l)), Some(("parse-error", got))) => got == l,
            _ => false,
        };
        if !agrees {
            wrong.push(format!("{verdict}\t{line}: ours {ours:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
