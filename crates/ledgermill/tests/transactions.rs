//! Expected values are what `hledger 1.52.4 print -O json` reports for the probe journals
//! (see spec/hledger/*.md); they are written out by hand, not computed.

mod support;

use serde_json::json;
use support::*;

#[test]
fn basic_journal_gives_dates_statuses_comments_lines_and_amounts() {
    let probe = include_str!("../../../spec/hledger/probes/basic.journal");
    assert_eq!(
        transactions("basic.journal", probe),
        json!([
            {
                "date": "2026-01-05", "status": "*", "description": "Coffee shop",
                "comment": "morning\nsecond line of the transaction comment",
                "source": { "file": "basic.journal", "line": 2 },
                "postings": [
                    { "account": "expenses:food", "kind": "regular", "comment": "latte",
                      "amounts": [{ "commodity": "$", "quantity": "3.50" }] },
                    { "account": "assets:cash", "kind": "regular", "comment": "",
                      "amounts": [{ "commodity": "$", "quantity": "-3.50" }] }
                ]
            },
            {
                "date": "2026-01-06", "status": "!", "description": "Lunch", "comment": "",
                "source": { "file": "basic.journal", "line": 7 },
                "postings": [
                    { "account": "expenses:food", "kind": "regular", "comment": "",
                      "amounts": [{ "commodity": "EUR", "quantity": "12.5" }] },
                    { "account": "assets:cash", "kind": "regular", "comment": "",
                      "amounts": [{ "commodity": "EUR", "quantity": "-12.50" }] }
                ]
            }
        ])
    );
    assert_eq!(findings("basic.journal", probe), json!([]));
}
