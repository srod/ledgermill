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

#[test]
fn tab_indent_and_tab_separator_become_spaces() {
    let input = "2026-03-01 Lunch\n\texpenses:food\t\t12.5 EUR\n\tassets:cash\n";
    let want = "2026-03-01 Lunch\n    expenses:food  12.5 EUR\n    assets:cash\n";
    assert_eq!(journal(input), want);
}

#[test]
fn crlf_bom_and_a_missing_final_newline_are_kept() {
    let input = "\u{feff}2026-03-01 Lunch\r\n expenses:food  12.5 EUR  \r\n assets:cash   ";
    let want = "\u{feff}2026-03-01 Lunch\r\n    expenses:food  12.5 EUR\r\n    assets:cash";
    assert_eq!(journal(input), want);
}

#[test]
fn comments_and_directives_are_untouched() {
    let input = "; a comment  \n  ; indented comment   \naccount expenses:food  \n  ; under a directive\ncommodity 1.000,00 EUR  \n  format 1.000,00 EUR\nP 2026-03-01 EUR 1.10 USD   \n\n2026-03-01 Lunch  ; note   \n  ; about the transaction\n expenses:food  12.5 EUR\n  ; about the posting   \n assets:cash\n\n";
    let want = "; a comment  \n  ; indented comment   \naccount expenses:food  \n  ; under a directive\ncommodity 1.000,00 EUR  \n  format 1.000,00 EUR\nP 2026-03-01 EUR 1.10 USD   \n\n2026-03-01 Lunch  ; note   \n  ; about the transaction\n    expenses:food  12.5 EUR\n  ; about the posting   \n    assets:cash\n\n";
    assert_eq!(journal(input), want);
}

#[test]
fn number_notation_is_kept() {
    let input = "2026-03-01 Rent\n  expenses:rent   1 003,11 €\n  assets:bank   EUR -1 003,11\n  assets:other  \"AB 1\" 3\n  assets:other  -12,50 €\n";
    let want = "2026-03-01 Rent\n    expenses:rent   1 003,11 €\n    assets:bank  EUR -1 003,11\n    assets:other      \"AB 1\" 3\n    assets:other      -12,50 €\n";
    assert_eq!(journal(input), want);
}

#[test]
fn cost_lot_and_assertion_follow_the_amount_as_written() {
    let input = "2026-03-01 Buy\n  assets:stock  10 AAA {2 EUR} [2026-03-01]  =  10 AAA ; lot\n  assets:cash  -20 EUR   @@   3 USD\n  assets:bank  -1 EUR = 5 EUR\n  assets:other  1 EUR (@) 2 USD\n  assets:more  1 EUR == -3 EUR\n";
    let want = "2026-03-01 Buy\n    assets:stock  10 AAA {2 EUR} [2026-03-01]  =  10 AAA ; lot\n    assets:cash  -20 EUR   @@   3 USD\n    assets:bank   -1 EUR = 5 EUR\n    assets:other   1 EUR (@) 2 USD\n    assets:more    1 EUR == -3 EUR\n";
    assert_eq!(journal(input), want);
}

#[test]
fn status_and_virtual_postings_keep_their_marks_and_spacing() {
    let input = "2026-03-01 t\n  *   expenses:food  12.5 EUR\n  ! expenses:tea  3 EUR\n  (virtual)  1 EUR\n  [balanced b]  5 EUR\n  [other]  -5 EUR\n  assets:cash\n";
    let want = "2026-03-01 t\n    *   expenses:food  12.5 EUR\n    ! expenses:tea        3 EUR\n    (virtual)             1 EUR\n    [balanced b]          5 EUR\n    [other]              -5 EUR\n    assets:cash\n";
    assert_eq!(journal(input), want);
}

#[test]
fn a_posting_without_amount_only_gets_the_indent() {
    let input = "2026-03-01 t\n  expenses:food  12.5 EUR\n  assets:cash   ; where from  \n";
    let want = "2026-03-01 t\n    expenses:food  12.5 EUR\n    assets:cash   ; where from\n";
    assert_eq!(journal(input), want);
}

#[test]
fn an_assignment_only_posting_is_not_aligned() {
    let input = "2026-03-01 t\n  assets:cash  = 5 EUR\n  expenses:food  12.5 EUR\n";
    let want = "2026-03-01 t\n    assets:cash  = 5 EUR\n    expenses:food  12.5 EUR\n";
    assert_eq!(journal(input), want);
}

#[test]
fn rule_and_comment_blocks_are_untouched() {
    let input = "= expenses:food\n    (budget)  *-1  ; rule   \n\n~ monthly\n  expenses:rent   600 EUR\n  assets:bank\n\ncomment\n  2026-03-01 not a transaction\n   a    1\nend comment\n\n2026-03-02 t\n a  1\n b\n";
    let want = "= expenses:food\n    (budget)  *-1  ; rule   \n\n~ monthly\n  expenses:rent   600 EUR\n  assets:bank\n\ncomment\n  2026-03-01 not a transaction\n   a    1\nend comment\n\n2026-03-02 t\n    a  1\n    b\n";
    assert_eq!(journal(input), want);
}

#[test]
fn each_transaction_has_its_own_column() {
    let input = "2026-03-01 a\n x  1\n y  -1\n\n2026-03-02 b\n long:account:name  100.00 EUR\n y  -100.00 EUR\n";
    let want = "2026-03-01 a\n    x   1\n    y  -1\n\n2026-03-02 b\n    long:account:name  100.00 EUR\n    y                 -100.00 EUR\n";
    assert_eq!(journal(input), want);
}

#[test]
fn a_transaction_ledgermill_refuses_is_left_as_it_is() {
    let input = "2026-03-01 bad\n a   1 EUR extra\n b\n\n2026-03-02 ok\n  a  1\n  b\n";
    let want = "2026-03-01 bad\n a   1 EUR extra\n b\n\n2026-03-02 ok\n    a  1\n    b\n";
    assert_eq!(journal(input), want);
}

#[test]
fn formatting_twice_gives_the_same_text() {
    let input = "2026-03-01 Lunch\n\texpenses:food\t\t12.5 EUR  \n   * assets:cash:checking   -12.5 EUR @ 1 USD  ; x\n";
    let once = journal(input);
    assert_eq!(journal(&once), once);
}

/// Every `.journal` under `dir`, recursively.
fn journals(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            journals(&path, found);
        } else if path.extension().is_some_and(|e| e == "journal") {
            found.push(path);
        }
    }
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn over_the_probe_corpus_format_is_idempotent_and_changes_only_whitespace() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    journals(&root.join("spec/hledger/probes"), &mut files);
    journals(&root.join("harness/journals"), &mut files);
    files.sort();
    let mut changed = 0;
    for path in &files {
        // A journal that is not UTF-8 cannot be a SourceFile.
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let name = path.to_string_lossy();
        let once = fmt(&name, &content);
        assert_eq!(fmt(&name, &once), once, "not idempotent: {name}");
        assert_eq!(
            without_whitespace(&once),
            without_whitespace(&content),
            "more than whitespace changed: {name}"
        );
        changed += usize::from(once != content);
    }
    assert!(files.len() > 1000, "found only {} journals", files.len());
    assert!(changed > 100, "only {changed} journals changed");
}
