//! `Ledger::postings()` for hledger: one row per posting and commodity. Expected values: the rows
//! hledger 1.52.4 `print --location -O csv` gives on the named probes, recorded in
//! `spec/hledger/postings-flat.md`, in the form `DATE[ FLAG] | NARRATION | ACCOUNT | NUMBER[ COMMODITY]
//! | FILE:LINE`. The line is the posting's own (the CSV has none); a decimal comma in the note is a
//! point here. See `post_lines` in `support`.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{PostingKind, load};
use support::*;

macro_rules! probe {
    ($name:literal) => {
        (
            concat!($name, ".journal"),
            include_str!(concat!("../../../spec/hledger/probes/", $name, ".journal")),
        )
    };
}

#[test]
fn transactions_come_by_date_then_in_the_order_written_with_their_header_fields() {
    // post-basic: a margin comment ends a transaction; a header-only transaction gives no row; the status is the transaction's.
    assert_eq!(
        post_lines(&[probe!("post-basic")]),
        [
            "2026-01-03 | earlier date, written later | a | 1 X | post-basic.journal:12",
            "2026-01-03 | earlier date, written later | b | -1 X | post-basic.journal:13",
            "2026-01-03 | same date, written after | a | 2 X | post-basic.journal:16",
            "2026-01-03 | same date, written after | b | -2 X | post-basic.journal:17",
            "2026-01-04 ! | pending | a | 4 X | post-basic.journal:23",
            "2026-01-04 ! | pending | b | -4 X | post-basic.journal:24",
            "2026-01-05 * | Shop | bought things | expenses:food | 10.50 EUR | post-basic.journal:5",
            "2026-01-05 * | Shop | bought things | expenses:misc | 3 USD | post-basic.journal:6",
            "2026-01-05 * | Shop | bought things | assets:cash | -10.50 EUR | post-basic.journal:7",
            "2026-01-05 * | Shop | bought things | assets:cash | -3 USD | post-basic.journal:7",
        ]
    );
}

#[test]
fn the_secondary_date_and_posting_dates_change_neither_the_date_nor_the_order() {
    // post-dates
    assert_eq!(
        post_lines(&[probe!("post-dates")]),
        [
            "2026-01-05 | written second, sorts first | a | 1 X | post-dates.journal:6",
            "2026-01-05 | written second, sorts first | b | -1 X | post-dates.journal:7",
            "2026-01-10 | secondary date and posting dates | a | 1 X | post-dates.journal:3",
            "2026-01-10 | secondary date and posting dates | b | -1 X | post-dates.journal:4",
        ]
    );
}

#[test]
fn crlf_and_a_missing_final_newline_change_no_line() {
    // post-crlf: lines count \n only
    assert_eq!(
        post_lines(&[probe!("post-crlf")]),
        [
            "2026-01-01 | crlf | a | 1 X | post-crlf.journal:2",
            "2026-01-01 | crlf | b | -1 X | post-crlf.journal:4",
            "2026-01-02 | crlf, no trailing newline | a | 2 X | post-crlf.journal:7",
            "2026-01-02 | crlf, no trailing newline | b | -2 X | post-crlf.journal:8",
        ]
    );
}

#[test]
fn files_in_the_order_given_break_ties_between_equal_dates() {
    // post-files-a + post-files-b
    assert_eq!(
        post_lines(&[probe!("post-files-a"), probe!("post-files-b")]),
        [
            "2026-01-01 | a2 earlier | p | 2 X | post-files-a.journal:5",
            "2026-01-01 | a2 earlier | q | -2 X | post-files-a.journal:6",
            "2026-01-02 | a1 | p | 1 X | post-files-a.journal:2",
            "2026-01-02 | a1 | q | -1 X | post-files-a.journal:3",
            "2026-01-02 | b1 same date as a1 | p | 3 X | post-files-b.journal:2",
            "2026-01-02 | b1 same date as a1 | q | -3 X | post-files-b.journal:3",
            "2026-01-03 | b3 | p | 5 X | post-files-b.journal:8",
            "2026-01-03 | b3 | q | -5 X | post-files-b.journal:9",
        ]
    );
}

#[test]
fn a_file_given_twice_is_read_twice() {
    // post-files b a b (the note shows the p postings only)
    assert_eq!(
        post_lines(&[
            probe!("post-files-b"),
            probe!("post-files-a"),
            probe!("post-files-b")
        ])
        .into_iter()
        .filter(|r| r.contains("| p |"))
        .collect::<Vec<_>>(),
        [
            "2026-01-01 | a2 earlier | p | 2 X | post-files-a.journal:5",
            "2026-01-02 | b1 same date as a1 | p | 3 X | post-files-b.journal:2",
            "2026-01-02 | a1 | p | 1 X | post-files-a.journal:2",
            "2026-01-02 | b1 same date as a1 | p | 3 X | post-files-b.journal:2",
            "2026-01-03 | b3 | p | 5 X | post-files-b.journal:8",
            "2026-01-03 | b3 | p | 5 X | post-files-b.journal:8",
        ]
    );
}

#[test]
fn an_elided_posting_gives_a_row_per_commodity_in_code_point_order_and_zeros_follow_the_rules() {
    // post-elided
    assert_eq!(
        post_lines(&[probe!("post-elided")]),
        [
            "2026-01-01 | three commodities | a | 1 b | post-elided.journal:3",
            "2026-01-01 | three commodities | b | 2 Z | post-elided.journal:4",
            "2026-01-01 | three commodities | c | 3 a | post-elided.journal:5",
            "2026-01-01 | three commodities | d | 4 $ | post-elided.journal:6",
            "2026-01-01 | three commodities | e | 5 € | post-elided.journal:7",
            "2026-01-01 | three commodities | f | 6 E F | post-elided.journal:8",
            "2026-01-01 | three commodities | g | 7 É | post-elided.journal:9",
            "2026-01-01 | three commodities | h | -4 $ | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -6 E F | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -2 Z | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -3 a | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -1 b | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -7 É | post-elided.journal:10",
            "2026-01-01 | three commodities | h | -5 € | post-elided.journal:10",
            "2026-01-02 | elided posting in the middle | a | 4 X | post-elided.journal:12",
            "2026-01-02 | elided posting in the middle | b | -4 X | post-elided.journal:13",
            "2026-01-02 | elided posting in the middle | b | -7 Y | post-elided.journal:13",
            "2026-01-02 | elided posting in the middle | b | 9 Z | post-elided.journal:13",
            "2026-01-02 | elided posting in the middle | c | -9 Z | post-elided.journal:14",
            "2026-01-02 | elided posting in the middle | d | 7 Y | post-elided.journal:15",
            "2026-01-03 | every commodity nets to zero | a | 1 B | post-elided.journal:18",
            "2026-01-03 | every commodity nets to zero | b | -1 B | post-elided.journal:19",
            "2026-01-03 | every commodity nets to zero | c | 1 A | post-elided.journal:20",
            "2026-01-03 | every commodity nets to zero | d | -1 A | post-elided.journal:21",
            "2026-01-03 | every commodity nets to zero | e | 0 A | post-elided.journal:22",
            "2026-01-04 | zeros are left out when another amount is not zero | a | 1 B | post-elided.journal:24",
            "2026-01-04 | zeros are left out when another amount is not zero | b | -1 B | post-elided.journal:25",
            "2026-01-04 | zeros are left out when another amount is not zero | c | 2 A | post-elided.journal:26",
            "2026-01-04 | zeros are left out when another amount is not zero | d | 3 C | post-elided.journal:27",
            "2026-01-04 | zeros are left out when another amount is not zero | e | -3 C | post-elided.journal:28",
            "2026-01-04 | zeros are left out when another amount is not zero | f | -2 A | post-elided.journal:29",
            "2026-01-05 | a commodity-less zero | a | 1 | post-elided.journal:31",
            "2026-01-05 | a commodity-less zero | b | -1 | post-elided.journal:32",
            "2026-01-05 | a commodity-less zero | c | 1 Z | post-elided.journal:33",
            "2026-01-05 | a commodity-less zero | d | -1 Z | post-elided.journal:34",
            "2026-01-05 | a commodity-less zero | e | 0 Z | post-elided.journal:35",
            "2026-01-06 | only the commodity-less amount | a | 1 | post-elided.journal:37",
            "2026-01-06 | only the commodity-less amount | b | -1 | post-elided.journal:38",
            "2026-01-06 | only the commodity-less amount | c | 0 | post-elided.journal:39",
            "2026-01-07 | holdings to assign over | n | 5 X | post-elided.journal:42",
            "2026-01-07 | holdings to assign over | n | 7 Y | post-elided.journal:43",
            "2026-01-07 | holdings to assign over | q | -5 X | post-elided.journal:44",
            "2026-01-07 | holdings to assign over | q | -7 Y | post-elided.journal:44",
            "2026-01-08 | total assignment zeroes two commodities | n | -5 X | post-elided.journal:46",
            "2026-01-08 | total assignment zeroes two commodities | n | -7 Y | post-elided.journal:46",
            "2026-01-08 | total assignment zeroes two commodities | q | 5 X | post-elided.journal:47",
            "2026-01-08 | total assignment zeroes two commodities | q | 7 Y | post-elided.journal:47",
            "2026-01-09 | a difference of zero leaves an empty amount | n | 0 | post-elided.journal:49",
            "2026-01-09 | a difference of zero leaves an empty amount | m | 0 X | post-elided.journal:50",
            "2026-01-10 | and so does an elided posting after it | n | 0 | post-elided.journal:52",
            "2026-01-10 | and so does an elided posting after it | m | 0 | post-elided.journal:53",
            "2026-01-11 | a bare number and a named commodity | a | 1 | post-elided.journal:56",
            "2026-01-11 | a bare number and a named commodity | b | 3 B | post-elided.journal:57",
            "2026-01-11 | a bare number and a named commodity | c | -1 | post-elided.journal:58",
            "2026-01-11 | a bare number and a named commodity | c | -3 B | post-elided.journal:58",
            "2026-01-12 | zeros of a bare number and a named commodity | a | 1 | post-elided.journal:60",
            "2026-01-12 | zeros of a bare number and a named commodity | b | -1 | post-elided.journal:61",
            "2026-01-12 | zeros of a bare number and a named commodity | c | 1 B | post-elided.journal:62",
            "2026-01-12 | zeros of a bare number and a named commodity | d | -1 B | post-elided.journal:63",
            "2026-01-12 | zeros of a bare number and a named commodity | e | 0 B | post-elided.journal:64",
        ]
    );
}

#[test]
fn cost_lot_and_assertion_are_not_in_the_row() {
    // post-costs
    assert_eq!(
        post_lines(&[probe!("post-costs")]),
        [
            "2026-01-01 | unit cost | a | 10 X | post-costs.journal:3",
            "2026-01-01 | unit cost | b | -20 EUR | post-costs.journal:4",
            "2026-01-02 | total cost | a | 10 X | post-costs.journal:6",
            "2026-01-02 | total cost | b | -25 EUR | post-costs.journal:7",
            "2026-01-03 | lot cost | a | 10 X | post-costs.journal:9",
            "2026-01-03 | lot cost | b | -30 EUR | post-costs.journal:10",
            "2026-01-04 | total lot, date and label | a | 10 X | post-costs.journal:12",
            "2026-01-04 | total lot, date and label | b | -35 EUR | post-costs.journal:13",
            "2026-01-05 | lot and price | a | 10 X | post-costs.journal:15",
            "2026-01-05 | lot and price | b | -40 EUR | post-costs.journal:16",
            "2026-01-06 | the elided posting of a costed one holds the cost commodity | a | 10 X | post-costs.journal:18",
            "2026-01-06 | the elided posting of a costed one holds the cost commodity | b | -25.0 EUR | post-costs.journal:19",
            "2026-01-07 | an inferred price | a | 10 X | post-costs.journal:21",
            "2026-01-07 | an inferred price | b | -30 EUR | post-costs.journal:22",
        ]
    );
}

#[test]
fn virtual_and_balanced_virtual_postings_are_rows_with_their_brackets_back() {
    // post-virtual: the account is shown with the brackets of its kind
    assert_eq!(
        post_lines(&[probe!("post-virtual")]),
        [
            "2026-01-01 ! | pending | a | 1 X | post-virtual.journal:2",
            "2026-01-01 ! | pending | b | -1 X | post-virtual.journal:3",
            "2026-01-01 ! | pending | (v) | 5 X | post-virtual.journal:4",
            "2026-01-01 ! | pending | [bv1] | 2 Y | post-virtual.journal:5",
            "2026-01-01 ! | pending | [bv2] | -2 Y | post-virtual.journal:6",
            "2026-01-01 ! | pending | c | 3 Z | post-virtual.journal:7",
            "2026-01-01 ! | pending | d | -3 Z | post-virtual.journal:8",
            "2026-01-01 ! | pending | (nothing) | 0 | post-virtual.journal:9",
            "2026-01-01 ! | pending | [bnothing] | 0 Y | post-virtual.journal:10",
            "2026-01-02 * | cleared, virtual postings only | (v) | 5 X | post-virtual.journal:12",
            "2026-01-02 * | cleared, virtual postings only | (w) | 0 | post-virtual.journal:13",
            "2026-01-03 | names that look like brackets | (a) b | 1 X | post-virtual.journal:15",
            "2026-01-03 | names that look like brackets | a (b) | 1 X | post-virtual.journal:16",
            "2026-01-03 | names that look like brackets | (a) (b) | 1 X | post-virtual.journal:17",
            "2026-01-03 | names that look like brackets | [c] d | -1 X | post-virtual.journal:18",
            "2026-01-03 | names that look like brackets | (e) | -1 X | post-virtual.journal:19",
            "2026-01-03 | names that look like brackets | x[y] | -1 X | post-virtual.journal:20",
            "2026-01-03 | names that look like brackets | (f) | -1 X | post-virtual.journal:21",
            "2026-01-03 | names that look like brackets | () | 1 X | post-virtual.journal:22",
            "2026-01-04 | nested brackets are all stripped, the outer pair gives the kind | (g) | 1 X | post-virtual.journal:24",
            "2026-01-04 | nested brackets are all stripped, the outer pair gives the kind | (h) | 1 X | post-virtual.journal:25",
            "2026-01-04 | nested brackets are all stripped, the outer pair gives the kind | [i] | 2 X | post-virtual.journal:26",
            "2026-01-04 | nested brackets are all stripped, the outer pair gives the kind | [j] | -2 X | post-virtual.journal:27",
        ]
    );
}

#[test]
fn quantities_are_exact_and_keep_their_places() {
    // post-zero-and-bare
    assert_eq!(
        post_lines(&[probe!("post-zero-and-bare")]),
        [
            "2026-01-01 | zeros | a | 0 EUR | post-zero-and-bare.journal:3",
            "2026-01-01 | zeros | b | 0 EUR | post-zero-and-bare.journal:4",
            "2026-01-01 | zeros | c | 0 EUR | post-zero-and-bare.journal:5",
            "2026-01-01 | zeros | d | 0 | post-zero-and-bare.journal:6",
            "2026-01-01 | zeros | e | 0 | post-zero-and-bare.journal:7",
            "2026-01-01 | zeros | f | 0 EUR | post-zero-and-bare.journal:8",
            "2026-01-02 | no commodity | a | 5 | post-zero-and-bare.journal:10",
            "2026-01-02 | no commodity | b | -5 | post-zero-and-bare.journal:11",
            "2026-01-03 | number forms | (a) | 1000 X | post-zero-and-bare.journal:13",
            "2026-01-03 | number forms | (b) | 1234567.89 X | post-zero-and-bare.journal:14",
            "2026-01-03 | number forms | (c) | 0.0012 X | post-zero-and-bare.journal:15",
            "2026-01-03 | number forms | (d) | 123456789012345678901234567890.5 X | post-zero-and-bare.journal:16",
            "2026-01-03 | number forms | (e) | 0.000000000000000000000001 X | post-zero-and-bare.journal:17",
            "2026-01-03 | number forms | (f) | -12.50 X | post-zero-and-bare.journal:18",
            "2026-01-03 | number forms | (g) | 7 X | post-zero-and-bare.journal:19",
            "2026-01-03 | number forms | (h) | 1000 X | post-zero-and-bare.journal:20",
            "2026-01-04 | places of an elided posting | a | 1.5 X | post-zero-and-bare.journal:22",
            "2026-01-04 | places of an elided posting | b | -1.25 X | post-zero-and-bare.journal:23",
            "2026-01-04 | places of an elided posting | c | -0.25 X | post-zero-and-bare.journal:24",
            "2026-01-05 | an elided posting with more than ten places | a | 0.00000000001 X | post-zero-and-bare.journal:26",
            "2026-01-05 | an elided posting with more than ten places | b | -0.00000000001 X | post-zero-and-bare.journal:27",
        ]
    );
}

#[test]
fn a_decimal_comma_style_is_written_with_a_point() {
    // post-styles (the note has the comma of the style)
    assert_eq!(
        post_lines(&[probe!("post-styles")]),
        [
            "2026-01-01 | declared styles | (a) | 1234.5678 EUR | post-styles.journal:5",
            "2026-01-01 | declared styles | (b) | 1234.5678 USD | post-styles.journal:6",
            "2026-01-01 | declared styles | (c) | 1234.5678 CHF | post-styles.journal:7",
            "2026-01-01 | declared styles | (d) | 12345678.1 CHF | post-styles.journal:8",
            "2026-01-01 | declared styles | (e) | 3.5 EUR | post-styles.journal:9",
            "2026-01-01 | declared styles | (f) | 1000000 USD | post-styles.journal:10",
            "2026-01-02 | the style of a commodity without a directive comes from its first amount | (a) | 1.5 X | post-styles.journal:12",
            "2026-01-02 | the style of a commodity without a directive comes from its first amount | (b) | 2.25 X | post-styles.journal:13",
            "2026-01-02 | the style of a commodity without a directive comes from its first amount | (c) | 1234.50 X | post-styles.journal:14",
            "2026-01-02 | the style of a commodity without a directive comes from its first amount | (d) | 1.5 GBP | post-styles.journal:15",
            "2026-01-02 | the style of a commodity without a directive comes from its first amount | (e) | 2.5 GBP | post-styles.journal:16",
        ]
    );
}

#[test]
fn a_decimal_mark_directive_is_written_with_a_point() {
    // post-decimal-mark
    assert_eq!(
        post_lines(&[probe!("post-decimal-mark")]),
        [
            "2026-01-01 | comma everywhere | (a) | 1234.5 X | post-decimal-mark.journal:3",
            "2026-01-01 | comma everywhere | (b) | 2.5 | post-decimal-mark.journal:4",
            "2026-01-01 | comma everywhere | (c) | 3.25 EUR | post-decimal-mark.journal:5",
            "2026-01-01 | comma everywhere | (d) | 4 | post-decimal-mark.journal:6",
        ]
    );
}

#[test]
fn a_posting_is_on_its_own_line_and_all_its_commodity_rows_share_it() {
    // post-assertion-line: the postings are on lines 3, 5 (after an indented comment) and 6.
    let (name, text) = probe!("post-assertion-line");
    let ledger = load(&[file(name, text)]);
    let lines: Vec<(&str, u32)> = ledger
        .postings()
        .iter()
        .map(|p| (p.account.as_str(), p.line))
        .collect();
    assert_eq!(lines, [("a", 3), ("b", 5), ("c", 6)]);
    // post-basic: the elided `assets:cash` (line 7) gives a row per commodity, both on line 7.
    let (name, text) = probe!("post-basic");
    let ledger = load(&[file(name, text)]);
    let cash: Vec<u32> = ledger
        .postings()
        .iter()
        .filter(|p| p.account == "assets:cash")
        .map(|p| p.line)
        .collect();
    assert_eq!(cash, [7, 7]);
}

#[test]
fn a_row_has_the_status_of_the_transaction_and_no_payee() {
    let (name, text) = probe!("post-virtual");
    let ledger = load(&[file(name, text)]);
    let flags: Vec<&str> = ledger.postings().iter().map(|p| p.flag.as_str()).collect();
    // `!` for the pending transaction (the posting marks `*` and `!` do not count), then `*`, then none.
    assert_eq!(flags[0], "!");
    assert_eq!(flags[9], "*");
    assert_eq!(flags[11], "");
    assert!(ledger.postings().iter().all(|p| p.payee.is_none()));
    assert_eq!(
        serde_json::to_value(&ledger.postings()[2]).unwrap(),
        serde_json::json!({
            "date": "2026-01-01",
            "flag": "!",
            "narration": "pending",
            "account": "v",
            "kind": "virtual",
            "number": "5",
            "currency": "X",
            "file": "post-virtual.journal",
            "line": 4,
        })
    );
}

#[test]
fn the_name_inside_nested_brackets_and_the_kind_of_the_outer_pair() {
    use PostingKind::{BalancedVirtual as B, Regular as R, Virtual as V};
    let (name, text) = probe!("post-virtual-nested");
    let ledger = load(&[file(name, text)]);
    // The first posting of each of the 26 transactions (the others are `zr` and `[zb]`).
    let firsts: Vec<(PostingKind, &str)> = ledger
        .postings()
        .iter()
        .filter(|p| p.account != "zr" && p.account != "zb")
        .map(|p| (p.kind, p.account.as_str()))
        .collect();
    let expected = [
        (V, "a]"),
        (V, "[a"),
        (V, "(a"),
        (V, "a)"),
        (B, "a)"),
        (B, "[a"),
        (B, "a]"),
        (V, "a"),
        (V, "a"),
        (V, "[a)"),
        (V, "([a"),
        (V, "]a["),
        (V, "[a])x"),
        (R, "[(a])"),
        (R, "[(a)"),
        (R, "(a)]"),
        (B, "a"),
        (V, "[(])"),
        (V, " (a) "),
        (V, " a "),
        (V, "() "),
        (V, ""),
        (V, ""),
        (V, "("),
        (V, ")"),
        (B, ")("),
    ];
    assert_eq!(firsts, expected);
}

#[test]
fn a_worked_out_amount_is_listed_with_the_display_precision_of_its_terms() {
    // post-elided-places: the quantity of `-3` is listed `-3.00`, `-25` is `-25.00`, and so on.
    assert_eq!(
        post_lines(&[probe!("post-elided-places")]),
        [
            "2026-01-01 | zero in the middle of the sum | a | 1.25 C | post-elided-places.journal:4",
            "2026-01-01 | zero in the middle of the sum | b | -1.25 C | post-elided-places.journal:5",
            "2026-01-01 | zero in the middle of the sum | c | 3 C | post-elided-places.journal:6",
            "2026-01-01 | zero in the middle of the sum | d | -3.00 C | post-elided-places.journal:7",
            "2026-01-02 | whole term after | a | 1.25 C | post-elided-places.journal:10",
            "2026-01-02 | whole term after | b | 3 C | post-elided-places.journal:11",
            "2026-01-02 | whole term after | c | -1.25 C | post-elided-places.journal:12",
            "2026-01-02 | whole term after | d | -3.00 C | post-elided-places.journal:13",
            "2026-01-03 | unit prices | a | 10 Z | post-elided-places.journal:16",
            "2026-01-03 | unit prices | b | -25.00 W | post-elided-places.journal:17",
            "2026-01-04 | unit prices grouped, the first price gives the places | a | 3 Q | post-elided-places.journal:20",
            "2026-01-04 | unit prices grouped, the first price gives the places | b | 2 Q | post-elided-places.journal:21",
            "2026-01-04 | unit prices grouped, the first price gives the places | c | -7.5 R | post-elided-places.journal:22",
            "2026-01-05 | totals, the most places | a | 1 X | post-elided-places.journal:25",
            "2026-01-05 | totals, the most places | b | 1 X | post-elided-places.journal:26",
            "2026-01-05 | totals, the most places | c | -3.00 Y | post-elided-places.journal:27",
            "2026-01-06 | another place for R elsewhere changes nothing | f | 0.001 R | post-elided-places.journal:30",
            "2026-01-06 | another place for R elsewhere changes nothing | g | -0.001 R | post-elided-places.journal:31",
            "2026-01-07 | an assignment lists the places of the difference | n1 | 2.00 X | post-elided-places.journal:34",
            "2026-01-07 | an assignment lists the places of the difference | b | -2.00 X | post-elided-places.journal:35",
            "2026-01-08 | and so does a difference from a balance | n2 | 7.5 X | post-elided-places.journal:38",
            "2026-01-08 | and so does a difference from a balance | b | -7.5 X | post-elided-places.journal:39",
            "2026-01-09 | second | n2 | 1.5 X | post-elided-places.journal:42",
            "2026-01-09 | second | b | -1.5 X | post-elided-places.journal:43",
            "2026-01-10 | total assignment | n2 | -9 X | post-elided-places.journal:46",
            "2026-01-10 | total assignment | b | 9 X | post-elided-places.journal:47",
        ]
    );
}
