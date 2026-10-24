//! `format()` for Beancount ledgers (`spec/beancount/format.md`): only the whitespace of postings,
//! `balance` and `price` lines changes, and the amounts of the whole file end in one column.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ledgermill::{SourceFile, format};

fn fmt(name: &str, content: &str) -> String {
    format(&SourceFile {
        name: name.into(),
        content: content.into(),
    })
}

fn bean(content: &str) -> String {
    fmt("a.beancount", content)
}

#[test]
fn amounts_of_the_whole_file_end_in_one_column() {
    let input = "2026-03-01 * \"a\"\n Expenses:Food  12.5 EUR\n Assets:Bank  -12.5 EUR\n\n2026-03-02 * \"b\"\n Expenses:Food:Groceries  100.00 EUR\n Assets:Bank  -100.00 EUR\n";
    let want = "2026-03-01 * \"a\"\n Expenses:Food               12.5 EUR\n Assets:Bank                -12.5 EUR\n\n2026-03-02 * \"b\"\n Expenses:Food:Groceries   100.00 EUR\n Assets:Bank              -100.00 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn both_extensions_are_formatted() {
    let input = "2026-03-01 * \"a\"\n Expenses:Food  12.5 EUR\n Assets:Bank  -12.5 EUR\n";
    let want = "2026-03-01 * \"a\"\n Expenses:Food   12.5 EUR\n Assets:Bank    -12.5 EUR\n";
    assert_eq!(fmt("a.bean", input), want);
    assert_eq!(fmt("A.BEANCOUNT", input), want);
}

#[test]
fn balance_and_price_lines_are_aligned_with_the_postings() {
    let input = "2026-03-01 price USD 0.9 EUR\n2026-03-02  balance   Assets:Bank\t  12.50 EUR\n2026-03-03 * \"x\"\n  Expenses:Food  1 EUR\n  Assets:Bank   -1 EUR\n";
    let want = "2026-03-01 price USD                 0.9 EUR\n2026-03-02  balance   Assets:Bank  12.50 EUR\n2026-03-03 * \"x\"\n  Expenses:Food                        1 EUR\n  Assets:Bank                         -1 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_tolerance_is_the_number_that_is_aligned() {
    let input = "2026-03-01 * \"x\"\n  Expenses:Food  1 EUR\n  Assets:Bank   -1 EUR\n\n2026-03-02 balance Assets:Bank 0 ~ 0.01 EUR\n";
    let want = "2026-03-01 * \"x\"\n  Expenses:Food                        1 EUR\n  Assets:Bank                         -1 EUR\n\n2026-03-02 balance Assets:Bank 0 ~  0.01 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_flagged_posting_is_kept_as_written_and_not_counted() {
    let input = "2026-03-01 * \"x\"\n  Expenses:Tea   5 USD\n  Equity:Tea  -5 USD\n  ! Expenses:Food:Groceries:Very:Long    1000000.00 EUR\n  Assets:Bank\n";
    let want = "2026-03-01 * \"x\"\n  Expenses:Tea   5 USD\n  Equity:Tea    -5 USD\n  ! Expenses:Food:Groceries:Very:Long    1000000.00 EUR\n  Assets:Bank\n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_posting_without_an_amount_only_gets_the_indent() {
    let input =
        "2026-03-01 * \"x\"\n  Expenses:Food  1 EUR\n  Assets:Cash\n\tAssets:Bank  ; from  \n";
    let want =
        "2026-03-01 * \"x\"\n  Expenses:Food  1 EUR\n  Assets:Cash\n  Assets:Bank  ; from  \n";
    assert_eq!(bean(input), want);
}

#[test]
fn the_commonest_indent_wins_and_a_tie_goes_to_the_larger() {
    let tie = "2026-03-01 * \"x\"\n  Expenses:Food 1 EUR\n  Assets:Bank -1 EUR\n\n2026-03-02 * \"y\"\n    Expenses:Tea 2 EUR\n    Assets:Cash -2 EUR\n";
    let want = "2026-03-01 * \"x\"\n    Expenses:Food   1 EUR\n    Assets:Bank    -1 EUR\n\n2026-03-02 * \"y\"\n    Expenses:Tea    2 EUR\n    Assets:Cash    -2 EUR\n";
    assert_eq!(bean(tie), want);

    let majority = "2026-03-01 * \"x\"\n  Expenses:Food 1 EUR\n  Assets:Bank -1 EUR\n\n2026-03-02 * \"y\"\n    Expenses:Tea 2 EUR\n";
    let want = "2026-03-01 * \"x\"\n  Expenses:Food   1 EUR\n  Assets:Bank    -1 EUR\n\n2026-03-02 * \"y\"\n  Expenses:Tea    2 EUR\n";
    assert_eq!(bean(majority), want);
}

#[test]
fn a_tab_indent_becomes_one_space() {
    let input = "2026-03-01 * \"x\"\n\tExpenses:Food\t\t12.50 EUR\n\tAssets:Bank\t-12.50 EUR\n";
    let want = "2026-03-01 * \"x\"\n Expenses:Food   12.50 EUR\n Assets:Bank    -12.50 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn metadata_comments_and_other_directives_are_untouched() {
    let input = "2026-01-01 open Assets:Bank EUR  \n2026-03-01 * \"x\"  ; note  \n  id: \"a\"\n  Expenses:Food  1 EUR\n      note:   \"deeper\"  \n    ; a comment   \n  Assets:Bank   -1 EUR\n; margin comment   \n2026-03-02 custom \"x\"   5 EUR\n";
    let want = "2026-01-01 open Assets:Bank EUR  \n2026-03-01 * \"x\"  ; note  \n  id: \"a\"\n  Expenses:Food   1 EUR\n      note:   \"deeper\"  \n    ; a comment   \n  Assets:Bank    -1 EUR\n; margin comment   \n2026-03-02 custom \"x\"   5 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_line_inside_a_multi_line_string_is_untouched() {
    let input = "2026-03-01 * \"first line\n  Assets:Bank  5 EUR\nsecond\"\n  Expenses:Food  5 EUR\n  Assets:Bank  -5 EUR\n";
    let want = "2026-03-01 * \"first line\n  Assets:Bank  5 EUR\nsecond\"\n  Expenses:Food   5 EUR\n  Assets:Bank    -5 EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn what_follows_the_currency_is_kept() {
    let input = "2026-03-01 * \"x\"\n  Assets:Broker  10 VT  {100.5 EUR}  @  101.25 EUR  ; c  \n  Assets:Bank   -1005 EUR   \n";
    let want = "2026-03-01 * \"x\"\n  Assets:Broker     10 VT  {100.5 EUR}  @  101.25 EUR  ; c  \n  Assets:Bank    -1005 EUR   \n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_number_without_a_blank_before_its_currency_is_not_aligned() {
    let input = "2026-03-01 * \"x\"\n  Expenses:Food   12.50 EUR\n  Assets:Bank   -12.50 EUR\n  Expenses:Tea  3.50EUR\n  Assets:Cash  -3.50EUR\n";
    let want = "2026-03-01 * \"x\"\n  Expenses:Food   12.50 EUR\n  Assets:Bank    -12.50 EUR\n  Expenses:Tea  3.50EUR\n  Assets:Cash  -3.50EUR\n";
    assert_eq!(bean(input), want);
}

#[test]
fn a_file_with_a_number_expression_is_returned_unchanged() {
    // ledgermill refuses the expression, so it reads no posting of the file (rule 8).
    let input = "2026-03-01 * \"x\"\n  Expenses:Food   (4 + 2.5) EUR\n  Assets:Bank  -6.5 EUR\n";
    assert_eq!(bean(input), input);
}

#[test]
fn a_file_with_an_include_is_returned_unchanged() {
    let input = "include \"other.beancount\"\n2026-03-01 * \"x\"\n  Expenses:Food   1 EUR\n  Assets:Bank  -1 EUR\n";
    assert_eq!(bean(input), input);
}

#[test]
fn a_file_with_a_syntax_error_is_returned_unchanged() {
    let input =
        "2026-03-01 * \"x\"\n  Expenses:Food   1 EUR\n  Assets:Bank  -1 EUR\n\nnot beancount\n";
    assert_eq!(bean(input), input);
}

#[test]
fn crlf_and_a_missing_final_newline_are_kept() {
    let input = "2026-03-01 * \"x\"\r\n Expenses:Food  12.5 EUR  \r\n Assets:Bank  -12.5 EUR";
    let want = "2026-03-01 * \"x\"\r\n Expenses:Food   12.5 EUR  \r\n Assets:Bank    -12.5 EUR";
    assert_eq!(bean(input), want);
}

#[test]
fn a_byte_order_mark_is_kept() {
    // Beancount rejects the mark (`Invalid token`), so ledgermill reads nothing of the file and
    // returns it as it was, mark included.
    let input =
        "\u{feff}; ledger\n2026-03-01 * \"x\"\n Expenses:Food  12.5 EUR\n Assets:Bank  -12.5 EUR\n";
    assert_eq!(bean(input), input);
}

#[test]
fn blank_and_whitespace_only_files_are_unchanged() {
    for input in ["", "\n", "  \n\t\n", "   "] {
        assert_eq!(bean(input), input);
    }
}

#[test]
fn formatting_twice_gives_the_same_text() {
    // Two indents, a tolerance and tabs: the case where `bean-format` is not idempotent.
    let input = "2026-03-01 * \"x\"\n\tExpenses:Tea\t2 EUR\n\tAssets:Cash\t-2 EUR\n\n2026-03-02 * \"y\"\n    Expenses:Food:Groceries  3 EUR\n    Assets:Cash  -3 EUR\n\n2026-03-03 balance Assets:Cash 1 ~ 0.01 EUR\n";
    let once = bean(input);
    assert_ne!(once, input);
    assert_eq!(bean(&once), once);
}

/// Every `.beancount` directly in `dir`.
fn ledgers(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "beancount") {
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
    // `refused/` holds files ledgermill does not read: they come back unchanged, which the
    // properties below allow, so they stay in.
    ledgers(&root.join("spec/beancount/probes"), &mut files);
    ledgers(&root.join("spec/beancount/probes/refused"), &mut files);
    ledgers(&root.join("harness/journals/app"), &mut files);
    files.sort();
    let mut changed = 0;
    for path in &files {
        // A file that is not UTF-8 cannot be a SourceFile.
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
    assert!(files.len() > 150, "found only {} ledgers", files.len());
    assert!(changed > 50, "only {changed} ledgers changed");
}
