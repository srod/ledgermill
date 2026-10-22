//! Balance assertions (`=`, `==`, `=*`, `==*`) as `load` and `check()` report them. Expected
//! values: hledger 1.52.4 on the named probes (spec/hledger/assertion-syntax.md, assertions.md,
//! assertion-order.md, assertion-failures.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::{Value, json};
use support::*;

/// `[line, column, account, commodity, expected, actual, total, inclusive]` of each
/// `assertion-failed` finding, in the order `check()` gives them.
fn failures(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = findings(name, content)
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "assertion-failed")
        .map(|f| {
            let d = &f["details"];
            json!([
                f["line"],
                f["column"],
                d["account"],
                d["commodity"],
                d["expected"],
                d["actual"],
                d["total"],
                d["inclusive"]
            ])
        })
        .collect();
    Value::Array(rows)
}

/// The one failure of a journal: `[line, column, commodity, expected, actual]`.
fn only_failure(name: &str, content: &str) -> Value {
    let all = failures(name, content);
    let all = all.as_array().unwrap();
    assert_eq!(all.len(), 1, "{name}: {all:?}");
    let f = all[0].as_array().unwrap();
    json!([f[0], f[1], f[3], f[4], f[5]])
}

#[test]
fn a_holding_assertion_gives_no_finding_and_changes_no_amount() {
    let journal = "2026-01-01 t\n  a  10 EUR\n  b  -10 EUR\n\n2026-01-02 u\n  a  5 EUR = 15 EUR\n  b  -5 EUR = -15 EUR\n";
    assert_eq!(findings("x.journal", journal), json!([]));
    assert_eq!(
        amounts("x.journal", journal),
        json!([
            [1, [["a", "EUR", "10"], ["b", "EUR", "-10"]]],
            [5, [["a", "EUR", "5"], ["b", "EUR", "-5"]]]
        ])
    );
}

#[test]
fn blanks_around_the_equals_sign_are_free() {
    let journal = "2026-01-01 t\n  a  1 EUR=1 EUR\n  b  -1 EUR =\t-1 EUR  ; trailing\n\n2026-01-02 u\n  a  0 EUR=      1 EUR;c\n  b  0 EUR\n";
    assert_eq!(findings("x.journal", journal), json!([]));
}

#[test]
fn a_single_space_before_the_equals_sign_is_part_of_the_account_name() {
    let probe = include_str!("../../../spec/hledger/probes/assert-single-space.journal");
    assert_eq!(findings("x.journal", probe), json!([]));
    assert_eq!(
        amounts("x.journal", probe),
        json!([[1, [["a", "EUR", "1"], ["b = 1 EUR", "EUR", "-1"]]]])
    );
}

#[test]
fn a_failing_assertion_is_reported_at_the_posting_and_the_first_equals_sign() {
    let probe = include_str!("../../../spec/hledger/probes/assert-fail-basic.journal");
    assert_eq!(
        findings("x.journal", probe),
        json!([{
            "code": "assertion-failed", "severity": "error", "file": "x.journal",
            "line": 6, "column": 12,
            "details": {
                "account": "a", "commodity": "EUR", "expected": "99", "actual": "15",
                "total": false, "inclusive": false
            },
            "message": "The balance of a in EUR is 15, not 99."
        }])
    );
}

#[test]
fn a_failed_assertion_does_not_stop_load() {
    let probe = include_str!("../../../spec/hledger/probes/assert-fail-basic.journal");
    assert_eq!(
        amounts("x.journal", probe),
        json!([
            [1, [["a", "EUR", "10"], ["b", "EUR", "-10"]]],
            [5, [["a", "EUR", "5"], ["b", "EUR", "-5"]]]
        ])
    );
}

#[test]
fn what_an_assertion_compares_is_the_exact_balance_in_its_commodity() {
    // [probe, line, column, commodity, expected, actual]
    let cases: [(&str, &str, Value); 8] = [
        (
            "assert-fail-cost: a price is not a balance",
            include_str!("../../../spec/hledger/probes/assert-fail-cost.journal"),
            json!([6, 10, "EUR", "6", "0"]),
        ),
        (
            "assert-fail-absent-commodity",
            include_str!("../../../spec/hledger/probes/assert-fail-absent-commodity.journal"),
            json!([6, 12, "USD", "5", "0"]),
        ),
        (
            "assert-fail-commodity-less: no commodity is a commodity",
            include_str!("../../../spec/hledger/probes/assert-fail-commodity-less.journal"),
            json!([6, 12, "", "10", "0"]),
        ),
        (
            "assert-fail-unused",
            include_str!("../../../spec/hledger/probes/assert-fail-unused.journal"),
            json!([6, 12, "EUR", "3", "0"]),
        ),
        (
            "assert-fail-exact: no tolerance",
            include_str!("../../../spec/hledger/probes/assert-fail-exact.journal"),
            json!([6, 12, "EUR", "1.004", "1.00"]),
        ),
        (
            "assert-fail-exact-places: forty places count",
            include_str!("../../../spec/hledger/probes/assert-fail-exact-places.journal"),
            json!([
                6,
                12,
                "EUR",
                "0.1234567890123456789012345678901234567891",
                "0.1234567890123456789012345678901234567890"
            ]),
        ),
        (
            "assert-fail-case: commodity names are case sensitive",
            include_str!("../../../spec/hledger/probes/assert-fail-case.journal"),
            json!([6, 12, "eur", "1", "0"]),
        ),
        (
            "assert-fail-order-in-transaction: later postings are not in yet",
            include_str!("../../../spec/hledger/probes/assert-fail-order-in-transaction.journal"),
            json!([2, 12, "EUR", "10", "5"]),
        ),
    ];
    for (name, probe, expected) in cases {
        assert_eq!(only_failure("x.journal", probe), expected, "{name}");
    }
}

#[test]
fn the_line_is_the_posting_s_own_whatever_surrounds_it() {
    let cases: [(&str, &str, Value); 4] = [
        (
            "after a block comment",
            include_str!("../../../spec/hledger/probes/assert-fail-line-block-comment.journal"),
            json!([7, 17, "EUR", "5", "-1"]),
        ),
        (
            "with a BOM and CRLF line ends",
            include_str!("../../../spec/hledger/probes/assert-fail-line-bom-crlf.journal"),
            json!([7, 15, "EUR", "5", "-1"]),
        ),
        (
            "no final newline",
            include_str!("../../../spec/hledger/probes/assert-fail-line-no-final-newline.journal"),
            json!([3, 17, "EUR", "5", "-1"]),
        ),
        (
            "tabs move the column to the next multiple of eight",
            include_str!("../../../spec/hledger/probes/assert-fail-line-tabs.journal"),
            json!([3, 33, "EUR", "5", "-1"]),
        ),
    ];
    for (name, probe, expected) in cases {
        assert_eq!(only_failure("x.journal", probe), expected, "{name}");
    }
}

#[test]
fn postings_are_in_the_balance_by_date_not_by_position_in_the_file() {
    // The later-dated transaction is written first: its assertion sees only itself.
    let journal =
        "2026-02-01 later\n  a  5 EUR = 5 EUR\n  b\n\n2026-01-01 earlier\n  a  10 EUR\n  b\n";
    assert_eq!(
        failures("x.journal", journal),
        json!([[2, 12, "a", "EUR", "5", "15", false, false]])
    );
    // Same date: the transaction written first is first.
    let probe = include_str!("../../../spec/hledger/probes/assert-fail-order-same-date.journal");
    assert_eq!(
        only_failure("x.journal", probe),
        json!([2, 12, "EUR", "15", "5"])
    );
}

#[test]
fn a_malformed_assertion_is_a_parse_error_on_its_posting() {
    let cases: [(&str, &str, u32); 6] = [
        (
            "nothing after the equals sign",
            include_str!("../../../spec/hledger/probes/assert-syntax-empty.journal"),
            3,
        ),
        (
            "text after the amount",
            include_str!("../../../spec/hledger/probes/assert-syntax-junk.journal"),
            2,
        ),
        (
            "a lot in the amount",
            include_str!("../../../spec/hledger/probes/assert-syntax-lot.journal"),
            4,
        ),
        (
            "a star apart from the equals sign",
            include_str!("../../../spec/hledger/probes/assert-syntax-star-space.journal"),
            3,
        ),
        (
            "three equals signs",
            include_str!("../../../spec/hledger/probes/assert-syntax-three-equals.journal"),
            2,
        ),
        (
            "two assertions",
            include_str!("../../../spec/hledger/probes/assert-syntax-two.journal"),
            2,
        ),
    ];
    for (name, probe, line) in cases {
        assert_eq!(
            short_findings("x.journal", probe),
            json!([[line, "parse-error", null]]),
            "{name}"
        );
    }
}

#[test]
fn journals_whose_assertions_all_hold_raise_no_finding() {
    let probes: [(&str, &str); 5] = [
        (
            "every form, commodity and exactness rule",
            include_str!("../../../spec/hledger/probes/assert-compare.journal"),
        ),
        (
            "the four kinds, spacing, marks and amount forms",
            include_str!("../../../spec/hledger/probes/assert-forms.journal"),
        ),
        (
            "later-dated transactions written first, elided amounts, virtual postings",
            include_str!("../../../spec/hledger/probes/assert-order-date.journal"),
        ),
        (
            "posting dates before and after the transaction's",
            include_str!("../../../spec/hledger/probes/assert-order-posting-date.journal"),
        ),
        (
            "the other ways to write a posting date",
            include_str!("../../../spec/hledger/probes/assert-order-posting-date-forms.journal"),
        ),
    ];
    for (name, probe) in probes {
        assert_eq!(findings("x.journal", probe), json!([]), "{name}");
    }
}

#[test]
fn double_equals_wants_every_other_commodity_at_zero_and_names_the_first_that_is_not() {
    // [probe, line, column, commodity, expected, actual]
    let cases: [(&str, &str, Value); 4] = [
        (
            "another commodity holds something",
            include_str!("../../../spec/hledger/probes/assert-fail-total-other.journal"),
            json!([8, 12, "USD", "0", "5"]),
        ),
        (
            "the asserted commodity is absent",
            include_str!("../../../spec/hledger/probes/assert-fail-total-absent.journal"),
            json!([6, 12, "USD", "10", "0"]),
        ),
        (
            "of several, the first in code point order",
            include_str!("../../../spec/hledger/probes/assert-fail-total-several.journal"),
            json!([8, 12, "GBP", "0", "1"]),
        ),
        (
            "the asserted commodity first, though another is wrong too",
            include_str!("../../../spec/hledger/probes/assert-fail-total-asserted-first.journal"),
            json!([7, 12, "USD", "10", "5"]),
        ),
    ];
    for (name, probe, expected) in cases {
        assert_eq!(only_failure("x.journal", probe), expected, "{name}");
        let all = failures("x.journal", probe);
        assert_eq!(all[0][6], json!(true), "{name}: total");
        assert_eq!(all[0][7], json!(false), "{name}: inclusive");
    }
}

#[test]
fn a_star_adds_the_sub_accounts_and_only_the_sub_accounts() {
    let cases: [(&str, &str, Value, bool); 3] = [
        (
            "= leaves the sub-accounts out",
            include_str!("../../../spec/hledger/probes/assert-fail-exclusive.journal"),
            json!([7, 12, "EUR", "9", "7"]),
            false,
        ),
        (
            "=* counts them",
            include_str!("../../../spec/hledger/probes/assert-fail-inclusive.journal"),
            json!([7, 12, "EUR", "7", "9"]),
            true,
        ),
        (
            "ab is not below a",
            include_str!("../../../spec/hledger/probes/assert-fail-sibling.journal"),
            json!([7, 12, "EUR", "9", "7"]),
            true,
        ),
    ];
    for (name, probe, expected, inclusive) in cases {
        assert_eq!(only_failure("x.journal", probe), expected, "{name}");
        assert_eq!(
            failures("x.journal", probe)[0][7],
            json!(inclusive),
            "{name}"
        );
    }
}

#[test]
fn a_posting_is_in_the_balance_from_its_own_date() {
    let cases: [(&str, &str, Value); 5] = [
        (
            "a posting dated after the assertion is not in yet",
            include_str!("../../../spec/hledger/probes/assert-fail-order-posting-date.journal"),
            json!([6, 12, "EUR", "6", "1"]),
        ),
        (
            "in one transaction, the earlier-dated posting is looked at first",
            include_str!(
                "../../../spec/hledger/probes/assert-fail-order-posting-date-in-transaction.journal"
            ),
            json!([3, 12, "EUR", "8", "3"]),
        ),
        (
            "of two failures, the earlier-dated one is first",
            include_str!("../../../spec/hledger/probes/assert-fail-first-by-date.journal"),
            json!([6, 12, "EUR", "77", "1"]),
        ),
        (
            "whichever account it is on",
            include_str!("../../../spec/hledger/probes/assert-fail-first-by-date-accounts.journal"),
            json!([6, 12, "EUR", "77", "1"]),
        ),
        (
            "in one transaction, posting order",
            include_str!("../../../spec/hledger/probes/assert-fail-first-in-transaction.journal"),
            json!([2, 12, "EUR", "99", "1"]),
        ),
    ];
    for (name, probe, first) in cases {
        let all = failures("x.journal", probe);
        let all = all.as_array().unwrap();
        let f = all[0].as_array().unwrap();
        assert_eq!(json!([f[0], f[1], f[3], f[4], f[5]]), first, "{name}");
    }
}

#[test]
fn the_first_date_in_a_posting_s_comment_is_its_date() {
    // 15 January is before the tag's February date, 15 March after the bracket's.
    let journal = |comment: &str, day: &str| {
        format!(
            "2026-01-01 t\n  x  5 EUR  ; {comment}\n  z\n\n{day} check\n  x  0 EUR = 5 EUR\n  y\n"
        )
    };
    let cases: [(&str, &str); 5] = [
        ("date:2026-02-01 [2026-03-01]", "2026-02-15"),
        ("[2026-03-01] date:2026-02-01", "2026-03-15"),
        ("[=2026-05-05] [2026-03-01]", "2026-03-15"),
        ("date2:2026-03-01 date:2026-02-01", "2026-01-15"),
        ("[2026-02-01 ]", "2026-01-15"),
    ];
    for (comment, day) in cases {
        assert_eq!(
            findings("x.journal", &journal(comment, day)),
            json!([]),
            "{comment}: in by {day}"
        );
    }
    assert_ne!(
        findings(
            "x.journal",
            &journal("date:2026-02-01 [2026-03-01]", "2026-01-15")
        ),
        json!([]),
        "not in before February"
    );
}

/// `[line, code]` of each finding, in the order `check()` gives them.
fn order(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = findings(name, content)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"]]))
        .collect();
    Value::Array(rows)
}

#[test]
fn an_unbalanced_transaction_comes_before_a_failing_assertion_written_earlier() {
    // hledger balances every plain transaction before it looks at any assertion.
    let cases: [(&str, &str, Value); 3] = [
        (
            "unbalanced, later in the file",
            include_str!("../../../spec/hledger/probes/assert-prec-unbalanced.journal"),
            json!([[5, "unbalanced-transaction"], [2, "assertion-failed"]]),
        ),
        (
            "two postings without an amount",
            include_str!("../../../spec/hledger/probes/assert-prec-two-elided.journal"),
            json!([[5, "unbalanced-transaction"], [2, "assertion-failed"]]),
        ),
        (
            "both in one transaction: it is reported as unbalanced first",
            include_str!("../../../spec/hledger/probes/assert-prec-both.journal"),
            json!([[1, "unbalanced-transaction"], [2, "assertion-failed"]]),
        ),
    ];
    for (name, probe, expected) in cases {
        assert_eq!(order("x.journal", probe), expected, "{name}");
    }
}

#[test]
fn a_parse_error_is_first_and_the_walk_of_an_unread_file_reports_nothing() {
    // The balances of a file with a line that was not read cannot be trusted.
    let probe = include_str!("../../../spec/hledger/probes/assert-prec-parse.journal");
    assert_eq!(order("x.journal", probe), json!([[8, "parse-error"]]));
}

#[test]
fn plain_transactions_are_balanced_in_file_order_and_assertions_are_walked_in_date_order() {
    let journal = "\
2026-03-01 unbalanced, written first
  a  1 EUR = 7 EUR
  b  -2 EUR

2026-01-01 unbalanced, written second
  a  1 EUR = 8 EUR
  b  -2 EUR
";
    assert_eq!(
        order("x.journal", journal),
        json!([
            [1, "unbalanced-transaction"],
            [5, "unbalanced-transaction"],
            [6, "assertion-failed"],
            [2, "assertion-failed"]
        ])
    );
    let probe = include_str!("../../../spec/hledger/probes/assert-fail-first-by-date.journal");
    assert_eq!(
        order("x.journal", probe),
        json!([[6, "assertion-failed"], [2, "assertion-failed"]])
    );
}

#[test]
fn a_journal_with_failed_assertions_still_returns_its_transactions() {
    let probe = include_str!("../../../spec/hledger/probes/assert-fail-first-by-date.journal");
    let lines: Vec<Value> = transactions("x.journal", probe)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["source"]["line"].clone())
        .collect();
    // Printed by date: the transaction written second is first.
    assert_eq!(lines, vec![json!(5), json!(1)]);
}
