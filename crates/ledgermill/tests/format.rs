//! `format()` for hledger journals (`spec/hledger/format.md`): only the whitespace of posting
//! lines changes.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ledgermill::{SourceFile, format};

fn fmt(name: &str, content: &str) -> String {
    format(&SourceFile {
        name: name.into(),
        content: content.into(),
    })
}

fn journal(content: &str) -> String {
    fmt("a.journal", content)
}

#[test]
fn amounts_of_a_transaction_end_in_one_column() {
    let input = "2026-03-01 Lunch\n expenses:food  12.5 EUR\n   assets:cash:checking   -12.5 EUR\n";
    let want = "2026-03-01 Lunch\n    expenses:food          12.5 EUR\n    assets:cash:checking  -12.5 EUR\n";
    assert_eq!(journal(input), want);
}
