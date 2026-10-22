//! Lines the reference reads as comments (a margin line starting with `*` or `#`), and options it
//! accepts twice (spec/beancount/syntax.md, options-and-directives.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

fn probe(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../../spec/beancount/probes/{name}.beancount",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

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
