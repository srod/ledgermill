//! `load` never panics: the core runs in browsers and apps, where a panic is a crash. Lines are
//! built from awkward pieces (multi-byte blanks, lone marks, brackets) in every position.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use support::file;

const PIECES: [&str; 24] = [
    "\u{a0}",
    "\u{a0}\u{a0}",
    "\u{2003}",
    "\t",
    "  ",
    " ",
    ";",
    "=",
    "@",
    "@@",
    "{",
    "}",
    "{{",
    "[",
    "]",
    "(",
    ")",
    "\"",
    "1,000.5",
    "-",
    "é",
    "2026-01-02",
    "type:",
    "e",
];

#[test]
fn awkward_lines_never_panic() {
    let prefixes = [
        "account a",
        "commodity ",
        "D ",
        "P 2026-01-01 X ",
        "alias ",
        "alias /a/ = ",
        "Y ",
        "apply account ",
        "decimal-mark ",
        "2026-01-01 ",
        "  a",
        "  a  1 X",
        "  a  ",
    ];
    for prefix in prefixes {
        for a in PIECES {
            for b in PIECES {
                let line = format!("{prefix}{a}{b}");
                let journal = format!("{line}\n2026-01-01 t\n  x  1\n{line}\n  y\n");
                // Only the absence of a panic is checked here.
                let _ = load(&[file("r.journal", &journal)]);
            }
        }
    }
}

#[test]
fn two_no_break_spaces_end_an_account_name_like_hledger() {
    let journal = "account a\u{a0}\u{a0}b\n";
    let ledger = load(&[file("r.journal", journal)]);
    assert_eq!(ledger.check()[0].code, "parse-error");
}
