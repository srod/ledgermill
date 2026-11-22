//! Lines the reference reads as comments (a margin line starting with `*` or `#`), and options it
//! accepts twice (spec/beancount/syntax.md, options-and-directives.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

#[test]
fn org_headings_and_hash_lines_at_the_margin_are_comments() {
    for name in ["org-heading-lines", "hash-line", "margin-lines"] {
        let journal = probe(name);
        assert_eq!(findings("m.beancount", &journal), json!([]), "{name}");
        assert_eq!(
            load(&[file("m.beancount", &journal)]).transactions().len(),
            1,
            "{name}"
        );
    }
}

#[test]
fn a_margin_comment_does_not_shift_the_lines_after_it() {
    let journal = probe("org-heading-lines");
    let rows = beancount_rows("m.beancount", &journal);
    assert_eq!(rows[0][4], json!(7));
    assert_eq!(rows[0][5][0][3], json!(8));
}

#[test]
fn a_star_inside_a_string_that_spans_lines_stays_in_the_string() {
    let journal = probe("margin-lines");
    let rows = beancount_rows("m.beancount", &journal);
    assert_eq!(
        rows[0][3],
        json!(
            "a string that spans lines\n* does not start a comment inside it\n# nor does this one"
        )
    );
}

#[test]
fn a_tag_or_a_lone_hash_or_star_at_the_margin_is_still_an_error() {
    let found = short_findings("m.beancount", &probe("margin-lines-errors"));
    assert_eq!(found[0], json!([3, "parse-error", found[0][2]]));
    assert!(
        load(&[file("m.beancount", &probe("margin-lines-errors"))])
            .transactions()
            .is_empty()
    );
}

#[test]
fn a_harmless_option_given_twice_is_accepted_and_the_last_one_counts() {
    let journal = probe("option-title-twice");
    assert_eq!(findings("o.beancount", &journal), json!([]));
    assert_eq!(
        load(&[file("o.beancount", &journal)]).transactions().len(),
        1
    );

    let journal = probe("option-repeated-last-wins");
    assert_eq!(findings("o.beancount", &journal), json!([]));
    let rows = beancount_rows("o.beancount", &journal);
    // LIFO, given last: the newest lot (20 EUR) is sold.
    assert_eq!(rows[1][5][0][5], json!("20 EUR 2026-01-03"));
}

#[test]
fn a_quote_in_a_margin_comment_does_not_open_a_string_for_expressions_or_glued_currencies() {
    let valid = "* \"\n2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n2026-01-02 * \"tx\"\n  Assets:A  (10 / 2) EUR\n  Equity:B\n";
    assert_eq!(findings("m.beancount", valid), json!([]));
    let rows = beancount_rows("m.beancount", valid);
    assert_eq!(rows[0][5][0][1], json!("5"));

    let div_zero = "* \"\n2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n2026-01-02 * \"tx\"\n  Assets:A  (1 / 0) EUR\n  Equity:B\n";
    assert_eq!(
        short_findings("m.beancount", div_zero),
        json!([[5, "division-by-zero", null]])
    );

    let glued = "* \"\n2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n2026-01-02 * \"tx\"\n  Assets:A  1 X;\n  Equity:B\n";
    let found = short_findings("m.beancount", glued);
    assert_eq!(found[0][0], json!(5));
    assert_eq!(found[0][1], json!("parse-error"));
}

#[test]
fn a_multiline_string_does_not_trigger_prescan_options_or_rewrites() {
    let journal = concat!(
        "option \"booking_method\" \"LIFO\"\n",
        "2026-01-01 open Assets:A\n",
        "2026-01-01 open Equity:B\n",
        "2026-01-02 * \"note\n",
        "2026-01-02 P flag-like\n",
        "  {{braces}}\n",
        "  5.\n",
        "  #a #b\n",
        "include foo\n",
        "option \\\"booking_method\\\" \\\"FIFO\\\"\n",
        "plugin \\\"beancount.plugins.auto_accounts\\\"\n",
        "1234567890123456789012345\n",
        "\"\n",
        "  Assets:A  10 EUR\n",
        "  Equity:B\n",
    );
    assert_eq!(findings("m.beancount", journal), json!([]));
    let ledger = load(&[file("m.beancount", journal)]);
    assert_eq!(
        ledger.transactions()[0].description,
        concat!(
            "note\n",
            "2026-01-02 P flag-like\n",
            "  {{braces}}\n",
            "  5.\n",
            "  #a #b\n",
            "include foo\n",
            "option \"booking_method\" \"FIFO\"\n",
            "plugin \"beancount.plugins.auto_accounts\"\n",
            "1234567890123456789012345\n",
        )
    );
}

#[test]
fn a_long_number_in_a_margin_comment_is_not_refused() {
    let journal = "* 1234567890123456789012345\n2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n2026-01-02 * \"t\"\n  Assets:A  1 EUR\n  Equity:B\n";
    assert_eq!(findings("m.beancount", journal), json!([]));
}

#[test]
fn currencies_with_a_digit_before_a_dot_are_kept_on_posting_lines() {
    let journal = concat!(
        "2026-01-01 open Assets:A X1.Y, /1.A, X-1.Y\n",
        "2026-01-01 open Equity:B X1.Y, /1.A, X-1.Y\n",
        "2026-01-02 * \"buy\"\n",
        "  Assets:A  10. X1.Y\n",
        "  Assets:A  5. /1.A\n",
        "  Assets:A  2. X-1.Y\n",
        "  Equity:B  -10 X1.Y\n",
        "  Equity:B  -5 /1.A\n",
        "  Equity:B  -2 X-1.Y\n",
        "2026-01-03 balance Assets:A  10 X1.Y\n",
        "2026-01-03 balance Assets:A  5 /1.A\n",
        "2026-01-03 balance Assets:A  2 X-1.Y\n",
    );
    assert_eq!(findings("c.beancount", journal), json!([]));
    let ledger = load(&[file("c.beancount", journal)]);
    let currencies: Vec<&str> = ledger.transactions()[0]
        .postings
        .iter()
        .map(|p| p.amounts[0].commodity.as_str())
        .collect();
    assert!(currencies.contains(&"X1.Y"));
    assert!(currencies.contains(&"/1.A"));
    assert!(currencies.contains(&"X-1.Y"));
}
