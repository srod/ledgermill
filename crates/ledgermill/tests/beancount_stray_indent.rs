//! An indented line after a blank line, or after a comment or an option at the margin, belongs to
//! no entry, and `check()` says so (spec/beancount/format.md, "What ends an entry"). Every
//! expected line below was confirmed with `bean-check` on the same text.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::{Value, json};
use support::*;

const OPEN: &str = "2026-01-01 open Assets:Bank EUR\n";

/// The lines of the `parse-error` findings, in order.
fn parse_error_lines(content: &str) -> Vec<u64> {
    findings("f.beancount", content)
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "parse-error")
        .map(|f| f["line"].as_u64().unwrap())
        .collect()
}

fn refused_probe(name: &str) -> String {
    probe(&format!("refused/{name}"))
}

#[test]
fn metadata_after_a_blank_line_is_a_parse_error_on_its_line() {
    let journal = refused_probe("metadata-after-blank");
    assert_eq!(
        short_findings("f.beancount", &journal),
        json!([[
            4,
            "parse-error",
            "an indented line after a blank line or a comment"
        ]])
    );
}

#[test]
fn a_posting_after_a_blank_line_a_line_of_blanks_or_a_comment_is_a_parse_error_on_its_line() {
    let journal = refused_probe("format-transaction-ends");
    let found: Vec<Value> = findings("f.beancount", &journal)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"]]))
        .collect();
    // 20 is an account at the margin, which the parser reads as such.
    assert_eq!(
        found,
        vec![
            json!([7, "parse-error"]),
            json!([12, "parse-error"]),
            json!([17, "parse-error"]),
            json!([20, "parse-error"]),
        ]
    );
}

#[test]
fn an_indented_line_after_a_blank_is_stray_whatever_it_holds() {
    let cases = [
        ("a comment", format!("{OPEN}\n  ; note\n"), vec![3]),
        (
            "a tab-indented posting",
            format!("{OPEN}\n\tAssets:Bank 1 EUR\n"),
            vec![3],
        ),
        ("a colon", format!("{OPEN}\n  :foo\n"), vec![3]),
        (
            "a line of blanks first",
            format!("{OPEN}   \n  ; c\n"),
            vec![3],
        ),
        (
            "a line of tabs first",
            format!("{OPEN}\n\t\n\t; c\n"),
            vec![4],
        ),
        (
            "no final newline",
            format!("{OPEN}\n  name: \"x\""),
            vec![3],
        ),
        (
            "a comment after a posting",
            format!("{OPEN}2026-01-02 * \"x\"\n  Assets:Bank 1 EUR\n\n  ; c\n"),
            vec![5],
        ),
    ];
    for (name, journal, lines) in cases {
        assert_eq!(parse_error_lines(&journal), lines, "{name}");
    }
}

#[test]
fn an_indented_line_at_the_start_of_the_file_is_stray() {
    for (name, journal, lines) in [
        ("content", format!("  name: \"x\"\n{OPEN}"), vec![1]),
        ("a comment", format!("  ; c\n{OPEN}"), vec![1]),
        (
            "a plugin",
            format!("  plugin \"beancount.plugins.auto_accounts\"\n{OPEN}"),
            vec![1],
        ),
        (
            "an option",
            format!("  option \"title\" \"x\"\n{OPEN}"),
            vec![1],
        ),
        ("after a blank", format!("\n  name: \"x\"\n{OPEN}"), vec![2]),
        (
            "a comment after blanks",
            format!("\n\n  ; c\n{OPEN}"),
            vec![3],
        ),
        (
            "after a comment",
            format!("; top\n  name: \"x\"\n{OPEN}"),
            vec![2],
        ),
    ] {
        assert_eq!(parse_error_lines(&journal), lines, "{name}");
    }
}

/// A quote in a comment at the margin opens no string, whatever the comment starts with: the
/// stray line after the next entry is still an error.
#[test]
fn a_quote_in_a_margin_comment_opens_no_string() {
    for start in [";", "*", "!", "&", "#", "?", "%", ":"] {
        let journal = format!(
            "{OPEN}{start} \"odd quote\n2026-01-02 open Assets:Cash EUR\n\n  name: \"x\"\n"
        );
        assert_eq!(parse_error_lines(&journal), vec![5], "{start}");
    }
}

/// bean-check also reports a lexical error (`Invalid token`) on a later line of a run, and
/// check() does not: the parser's error there is the same as the one on a stray posting, so it is
/// dropped with it (spec/beancount/format.md).
#[test]
fn a_lexical_error_on_a_later_line_of_a_run_is_not_reported() {
    let journal = "2026-01-02 * \"t\"\n  Assets:Bank 1 EUR\n  Expenses:Food 1 EUR\n\n  Assets:Cash 2 EUR\n  Assets:Bank garbage!\n";
    assert_eq!(parse_error_lines(journal), vec![5]);
}

#[test]
fn a_comment_at_the_margin_ends_an_entry() {
    for start in [";", "*", "!", "&", "#", "?", "%", ":"] {
        for line in [format!("{start} x"), format!("{start}x")] {
            let journal = format!("{OPEN}{line}\n  name: \"x\"\n");
            // `#x` is a tag: bean-check's error is on that line (`unexpected TAG`), and it
            // says nothing about the indented line after it.
            let expected = if line == "#x" { vec![2] } else { vec![3] };
            assert_eq!(parse_error_lines(&journal), expected, "{line}");
        }
    }
    // Alone on its line a `;` is a comment too.
    assert_eq!(
        parse_error_lines(&format!("{OPEN};\n  name: \"x\"\n")),
        vec![3]
    );
}

/// An indented `plugin` is no directive: bean-check rejects it right after an entry (as a key
/// it cannot read) and after a blank line (as a stray line).
#[test]
fn an_indented_plugin_line_is_a_parse_error() {
    assert_eq!(
        parse_error_lines(&refused_probe("plugin-indented")),
        vec![2, 6]
    );
}

#[test]
fn an_option_or_a_push_or_pop_at_the_margin_ends_an_entry() {
    for line in [
        "option \"title\" \"t\"",
        "pushtag #t",
        "poptag #t",
        "pushmeta key1: \"v\"",
        "popmeta key1:",
    ] {
        let journal = format!("{OPEN}{line}\n  name: \"x\"\n");
        assert_eq!(parse_error_lines(&journal), vec![3], "{line}");
    }
}

#[test]
fn an_indented_line_stays_in_the_entry_above_it() {
    for (name, journal) in [
        (
            "comments between postings",
            format!("{OPEN}2026-01-02 note Assets:Bank \"x\"\n  ; c\n  name: \"y\"\n  ; d\n"),
        ),
        (
            "blanks before the next entry",
            format!("{OPEN}\n   \n  \n2026-01-02 open Assets:Cash EUR\n"),
        ),
        (
            "metadata on a price",
            format!("{OPEN}2026-01-02 price EUR 1 USD\n  name: \"x\"\n"),
        ),
        (
            "a string that spans a blank line and an indented one",
            format!("{OPEN}2026-01-02 note Assets:Bank \"a\n\n  b\"\n  name: \"x\"\n"),
        ),
        (
            "a comment that holds a quote, and a margin comment before an entry",
            format!("{OPEN}; \"quote\n2026-01-02 note Assets:Bank \"x\"\n  name: \"x\"\n"),
        ),
    ] {
        assert_eq!(parse_error_lines(&journal), Vec::<u64>::new(), "{name}");
    }
}

#[test]
fn a_blank_inside_a_string_does_not_end_an_entry_but_one_after_it_does() {
    let journal = format!("{OPEN}2026-01-02 note Assets:Bank \"a ; \n\n  b\"\n\n  name: \"x\"\n");
    assert_eq!(parse_error_lines(&journal), vec![6]);
}

#[test]
fn crlf_line_endings_end_entries_too() {
    for (name, journal, lines) in [
        (
            "an empty line",
            "2026-01-01 open Assets:Bank EUR\r\n\r\n  name: \"C\"\r\n",
            vec![3],
        ),
        (
            "a line of blanks",
            "2026-01-01 open Assets:Bank EUR\r\n  \r\n  name: \"C\"\r\n",
            vec![3],
        ),
        (
            "a comment at the margin",
            "2026-01-01 open Assets:Bank EUR\r\n; c\r\n  name: \"C\"\r\n",
            vec![3],
        ),
    ] {
        assert_eq!(parse_error_lines(journal), lines, "{name}");
    }
}

/// bean-check says nothing more about stray lines until it has read a line of an entry after the
/// last one it reported, or two blank lines or margin comments: a run of them, with at most one
/// of those between, is one error.
#[test]
fn a_run_of_stray_lines_is_one_error_on_its_first_line() {
    for (name, journal, lines) in [
        (
            "consecutive",
            format!("{OPEN}\n  name: \"x\"\n  other: \"y\"\n"),
            vec![3],
        ),
        (
            "blanks and an indented comment between",
            format!("{OPEN}\n\n  name: \"x\"\n  ; c\n\n  other: \"y\"\n"),
            vec![4],
        ),
        (
            "a margin comment between",
            format!("{OPEN}; c\n\n  name: \"x\"\n; d\n  other: \"y\"\n"),
            vec![4],
        ),
        (
            "a stray posting run",
            format!("{OPEN}\n  Assets:Bank 1 EUR\n  Assets:Bank 2 EUR\n\n  Assets:Bank 3 EUR\n"),
            vec![3],
        ),
    ] {
        assert_eq!(parse_error_lines(&journal), lines, "{name}");
    }
}

/// ...and two blank lines or margin comments (of any mix) between two runs make the second one
/// count.
#[test]
fn two_blank_or_comment_lines_between_two_runs_make_the_second_an_error_too() {
    for (name, journal, lines) in [
        (
            "two blanks",
            format!("{OPEN}\n  name: \"x\"\n\n\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "two margin comments",
            format!("{OPEN}\n  name: \"x\"\n; c\n; d\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "a blank and a comment",
            format!("{OPEN}\n  name: \"x\"\n\n; d\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "two lines of blanks",
            format!("{OPEN}\n  name: \"x\"\n   \n\t\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "at the start of the file",
            format!("  name: \"x\"\n\n\n  name: \"y\"\n{OPEN}"),
            vec![1, 4],
        ),
        (
            "each run after two",
            format!("{OPEN}\n  name: \"x\"\n\n\n\n  name: \"y\"\n\n\n  name: \"z\"\n"),
            vec![3, 7, 10],
        ),
    ] {
        assert_eq!(parse_error_lines(&journal), lines, "{name}");
    }
}

/// The parser's own error on a stray posting is the same error, not a second one.
#[test]
fn a_stray_posting_after_an_indented_comment_is_not_reported_twice() {
    assert_eq!(
        parse_error_lines(&format!("{OPEN}\n  ; c\n  Assets:Bank 1 EUR\n")),
        vec![3]
    );
    assert_eq!(
        parse_error_lines(&format!("  ; c\n  name: \"x\"\n{OPEN}")),
        vec![1]
    );
}

/// `bean-check` drops a stray line whole, so it says nothing else about it: an account with an
/// unknown root there is no `invalid-account-name`, on the first line of a run or a later one.
#[test]
fn a_stray_line_has_no_other_finding_of_its_own() {
    let head = "2026-01-01 open Assets:Bank EUR\n2026-01-01 open Expenses:Food EUR\n2026-01-02 * \"x\"\n  Assets:Bank -1 EUR\n  Expenses:Food 1 EUR\n";
    for (name, tail) in [
        ("first line of a run", "\n  Foo:Account 1 EUR\n"),
        (
            "later line of a run",
            "\n  Assets:Bank 1 EUR\n  Foo:Account 1 EUR\n",
        ),
    ] {
        let found: Vec<Value> = findings("f.beancount", &format!("{head}{tail}"))
            .as_array()
            .unwrap()
            .iter()
            .map(|f| json!([f["line"], f["code"]]))
            .collect();
        assert_eq!(found, vec![json!([7, "parse-error"])], "{name}");
    }
}

/// ...and a line of an entry, or an option, between two runs makes the second one count.
#[test]
fn a_directive_between_two_runs_makes_the_second_an_error_too() {
    for (name, journal, lines) in [
        (
            "an open",
            format!("{OPEN}\n  name: \"x\"\n2026-01-02 open Assets:Cash EUR\n\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "an open with metadata",
            format!(
                "{OPEN}\n  name: \"x\"\n2026-01-02 open Assets:Cash EUR\n  name: \"z\"\n\n  name: \"y\"\n"
            ),
            vec![3, 7],
        ),
        (
            "an option",
            format!("{OPEN}\n  name: \"x\"\noption \"title\" \"t\"\n\n  name: \"y\"\n"),
            vec![3, 6],
        ),
        (
            "a pushtag and a poptag",
            format!("{OPEN}\n  name: \"x\"\npushtag #t\npoptag #t\n\n  name: \"y\"\n"),
            vec![3, 7],
        ),
        (
            "a close",
            format!("{OPEN}\n  name: \"x\"\n2026-01-02 close Assets:Bank\n\n  name: \"y\"\n"),
            vec![3, 6],
        ),
    ] {
        assert_eq!(parse_error_lines(&journal), lines, "{name}");
    }
}

/// A byte order mark makes the first line an error for bean-check (format.md), which then keeps
/// quiet about a stray line until it has read two blank lines or comments, or an entry.
#[test]
fn after_a_byte_order_mark_stray_lines_are_reported_from_the_next_entry_on() {
    let bom = "\u{feff}";
    assert_eq!(
        parse_error_lines(&format!("{bom}{OPEN}\n  name: \"C\"\n")),
        vec![1]
    );
    assert_eq!(
        parse_error_lines(&format!(
            "{bom}{OPEN}2026-01-02 open Assets:Cash EUR\n\n  name: \"C\"\n"
        )),
        vec![1, 4]
    );
    assert_eq!(
        parse_error_lines(&format!("{bom}{OPEN}  name: \"C\"\n")),
        vec![1]
    );
    assert_eq!(
        parse_error_lines(&format!("{bom}{OPEN}\n\n\n  name: \"C\"\n")),
        vec![1, 5]
    );
}
