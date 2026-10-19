//! Source lines, file layout and output order. Expected values: hledger 1.52.4 on the named probes.

mod support;

use ledgermill::load;
use serde_json::json;
use support::*;

#[test]
fn tab_and_mixed_indentation_are_fine() {
    let probe = include_str!("../../../spec/hledger/probes/tab-indent.journal");
    assert_eq!(
        amounts("tab-indent.journal", probe),
        json!([
            [1, [["a", "", "1"], ["b", "", "-1"]]],
            [5, [["a", "", "1"], ["b", "", "-1"]]]
        ])
    );
    assert_eq!(findings("tab-indent.journal", probe), json!([]));
}

#[test]
fn crlf_bom_and_a_missing_final_newline_change_nothing() {
    let crlf = include_str!("../../../spec/hledger/probes/crlf.journal");
    assert!(
        crlf.contains("\r\n"),
        "the probe must keep its CRLF line endings"
    );
    assert_eq!(
        headers("crlf.journal", crlf),
        json!([[1, "", "crlf", "", ["c", ""]]])
    );
    assert_eq!(findings("crlf.journal", crlf), json!([]));

    let bom = include_str!("../../../spec/hledger/probes/bom.journal");
    assert!(bom.starts_with('\u{feff}'));
    assert_eq!(
        headers("bom.journal", bom),
        json!([[1, "", "bom", "", ["", ""]]])
    );
    assert_eq!(findings("bom.journal", bom), json!([]));

    let bare = include_str!("../../../spec/hledger/probes/no-final-newline.journal");
    assert_eq!(
        headers("bare.journal", bare),
        json!([[1, "", "no final newline", "", ["", ""]]])
    );
    assert_eq!(findings("bare.journal", bare), json!([]));
}

#[test]
fn an_empty_journal_has_nothing_and_no_finding() {
    assert_eq!(transactions("empty.journal", ""), json!([]));
    assert_eq!(findings("empty.journal", ""), json!([]));
    assert_eq!(
        findings("comments.journal", "; just a comment\n\n# another\n"),
        json!([])
    );
}

#[test]
fn transactions_come_out_by_date_then_file_order() {
    let probe = include_str!("../../../spec/hledger/probes/order-by-date.journal");
    let rows = headers("order-by-date.journal", probe);
    let order: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r[0].clone(), r[2].clone()))
        .collect();
    assert_eq!(
        order,
        vec![
            (json!(5), json!("a")),
            (json!(13), json!("a2 same date, later in the file")),
            (json!(9), json!("b")),
            (json!(1), json!("c")),
        ]
    );
}

#[test]
fn several_files_are_read_in_the_order_given_with_their_own_line_numbers() {
    let a = file(
        "a.journal",
        include_str!("../../../spec/hledger/probes/order-file-a.journal"),
    );
    let b = file(
        "b.journal",
        include_str!("../../../spec/hledger/probes/order-file-b.journal"),
    );
    let seen = |files: &[ledgermill::SourceFile]| -> Vec<(String, String, u32)> {
        load(files)
            .transactions()
            .into_iter()
            .map(|t| (t.date, t.source.file, t.source.line))
            .collect()
    };
    let s = |d: &str, f: &str, l| (d.to_string(), f.to_string(), l);
    assert_eq!(
        seen(&[a.clone(), b.clone()]),
        vec![
            s("2026-01-01", "b.journal", 1),
            s("2026-01-02", "a.journal", 1),
            s("2026-01-02", "b.journal", 5)
        ]
    );
    // Equal dates follow the order the files were given in.
    assert_eq!(
        seen(&[b, a]),
        vec![
            s("2026-01-01", "b.journal", 1),
            s("2026-01-02", "b.journal", 5),
            s("2026-01-02", "a.journal", 1)
        ]
    );
}

#[test]
fn the_same_file_given_twice_is_read_twice() {
    let a = file(
        "a.journal",
        include_str!("../../../spec/hledger/probes/order-file-a.journal"),
    );
    assert_eq!(load(&[a.clone(), a]).transactions().len(), 2);
}

#[test]
fn a_last_line_of_only_spaces_without_a_newline_is_refused() {
    let bad = include_str!("../../../spec/hledger/probes/spaces-only-last-line-no-newline.journal");
    let f = findings("s.journal", bad);
    assert_eq!(f[0]["details"]["construct"], "spaces-at-end-of-file");
    assert_eq!(f[0]["line"], 4);
    // With the newline it is an ordinary blank line, and hledger reads the transaction.
    let fine =
        include_str!("../../../spec/hledger/probes/spaces-only-last-line-with-newline.journal");
    assert_eq!(findings("s.journal", fine), json!([]));
    assert_eq!(transactions("s.journal", fine).as_array().unwrap().len(), 1);
}
