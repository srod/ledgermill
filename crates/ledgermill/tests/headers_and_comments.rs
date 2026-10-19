//! Header lines and comments. Expected values: hledger 1.52.4 on the named probes
//! (its comments end with a newline; ledgermill's are trimmed).

mod support;

use serde_json::json;
use support::*;

#[test]
fn header_forms_give_status_description_and_comment() {
    let probe = include_str!("../../../spec/hledger/probes/header-forms.journal");
    assert_eq!(
        headers("header-forms.journal", probe),
        json!([
            [1, "", "plain", "", ["", ""]],
            [5, "*", "cleared", "", ["", ""]],
            [9, "!", "pending", "", ["", ""]],
            [13, "*", "extra   spaces   inside", "", ["", ""]],
            [17, "", "Coffee", "memo glued", ["", ""]],
            [
                21,
                "",
                "Pay | note",
                "comment with trailing blanks",
                ["", ""]
            ],
            [25, "", "", "", ["", ""]],
            [29, "*", "", "", ["", ""]],
            [33, "*", "", "only a comment", ["", ""]],
            [37, "", "Café € déjà-vu", "", ["", ""]],
            [41, "", "a=b (not a code) and \"quotes\"", "", ["", ""]],
            [45, "", "no postings at all", "", []]
        ])
    );
    assert_eq!(findings("header-forms.journal", probe), json!([]));
}

#[test]
fn comments_attach_to_the_header_or_the_posting_above_and_are_trimmed() {
    let probe = include_str!("../../../spec/hledger/probes/comments.journal");
    assert_eq!(
        headers("comments.journal", probe),
        json!([
            [6, "", "t", "c1\nc2", ["p1\np1b", "p2"]],
            [
                13,
                "",
                "comments only after postings",
                "",
                ["", "last   one"]
            ],
            [
                19,
                "",
                "empty comment kinds",
                "",
                ["", "spaced   inner  text   ;  with semicolons"]
            ],
            [
                24,
                "",
                "glued to previous comment",
                "leading comment line, no space",
                ["glued", ""]
            ]
        ])
    );
    assert_eq!(findings("comments.journal", probe), json!([]));
}

#[test]
fn tags_in_comments_change_nothing() {
    let probe = include_str!("../../../spec/hledger/probes/comment-tags-do-nothing.journal");
    assert_eq!(
        headers("comment-tags-do-nothing.journal", probe),
        json!([[
            1,
            "",
            "t",
            "date:2026-02-01 x:y",
            ["date:2026-03-01", "tag: v, date: 2026-04-01"]
        ]])
    );
    assert_eq!(
        transactions("comment-tags-do-nothing.journal", probe)[0]["date"],
        "2026-01-01"
    );
}

#[test]
fn a_transaction_may_have_no_postings() {
    let t = transactions("t.journal", "2026-01-12 nothing to post\n");
    assert_eq!(t[0]["postings"], json!([]));
    assert_eq!(
        findings("t.journal", "2026-01-12 nothing to post\n"),
        json!([])
    );
}

#[test]
fn well_formed_date_tags_in_comments_are_fine_and_transaction_comments_are_not_checked() {
    let probe = include_str!(
        "../../../spec/hledger/probes/posting-comment-date-tags-ok-and-tx-unchecked.journal"
    );
    // `[2026-02-01]` and `[=2026-01-02]` are valid for hledger but ledgermill does not read brackets.
    assert_eq!(
        findings("p.journal", probe)[0]["details"]["construct"],
        "posting-date"
    );
    let ok = "2026-01-01 t ; date:2026-02-30 [2026-13-45]\n  a  1 ; date2:2026-03-01 x\n  b  -1 ; date:2026-04-01,k:v\n";
    assert_eq!(findings("p.journal", ok), json!([]));
    assert_eq!(
        headers("p.journal", ok),
        json!([[
            1,
            "",
            "t",
            "date:2026-02-30 [2026-13-45]",
            ["date2:2026-03-01 x", "date:2026-04-01,k:v"]
        ]])
    );
}

#[test]
fn a_posting_comment_hledger_cannot_read_is_refused() {
    for probe in [
        include_str!("../../../spec/hledger/probes/posting-comment-date-tag-invalid.journal"),
        include_str!("../../../spec/hledger/probes/posting-comment-bracket-date-invalid.journal"),
        "2026-01-01 t\n  a  1\n  ; date:oops\n  b  -1\n",
        "2026-01-01 t\n  a  1 ; x date:2026-02-01,date:bad\n  b  -1\n",
    ] {
        let found = findings("p.journal", probe);
        assert_eq!(found[0]["details"]["construct"], "posting-date", "{probe}");
        assert_eq!(transactions("p.journal", probe), json!([]), "{probe}");
    }
}
