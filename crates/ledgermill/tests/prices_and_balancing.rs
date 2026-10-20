//! Prices, lots, virtual postings, elided amounts, inferred prices and conversion postings.
//! Expected values: hledger 1.52.4 on the named probes (spec/hledger/prices-and-lots.md,
//! postings.md, elided-amounts.md, balancing.md, inferred-prices.md, conversion.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use serde_json::{Value, json};
use support::*;

/// Per transaction: its line and, per posting, `[account, kind, [amount...]]` with each amount
/// in its JSON form (commodity, quantity, cost, lot).
fn postings(name: &str, content: &str) -> Value {
    assert_eq!(findings(name, content), json!([]), "{name} has findings");
    let rows: Vec<Value> = transactions(name, content)
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let ps: Vec<Value> = t["postings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| json!([p["account"], p["kind"], p["amounts"]]))
                .collect();
            json!([t["source"]["line"], ps])
        })
        .collect();
    Value::Array(rows)
}

fn q(commodity: &str, quantity: &str) -> Value {
    json!({ "commodity": commodity, "quantity": quantity })
}

fn unit(commodity: &str, quantity: &str, price: Value) -> Value {
    json!({ "commodity": commodity, "quantity": quantity, "cost": { "kind": "unit", "amount": price } })
}

fn total(commodity: &str, quantity: &str, price: Value) -> Value {
    json!({ "commodity": commodity, "quantity": quantity, "cost": { "kind": "total", "amount": price } })
}

fn regular(account: &str, amounts: Value) -> Value {
    json!([account, "regular", amounts])
}

#[test]
fn prices_are_reported_as_hledger_reports_them() {
    let probe = include_str!("../../../spec/hledger/probes/prices.journal");
    assert_eq!(
        postings("prices", probe),
        json!([
            [
                1,
                [
                    regular("a", json!([unit("X", "2", q("EUR", "3.50"))])),
                    regular("b", json!([q("EUR", "-7")]))
                ]
            ],
            [
                5,
                [
                    regular("a", json!([total("X", "-2", q("EUR", "-100"))])),
                    regular("b", json!([q("EUR", "100")]))
                ]
            ],
            [
                9,
                [
                    regular("a", json!([total("X", "2", q("EUR", "3"))])),
                    regular("b", json!([q("EUR", "-3")]))
                ]
            ],
            [
                13,
                [
                    regular("a", json!([unit("X", "2", q("", "3"))])),
                    regular("b", json!([q("", "-6")]))
                ]
            ]
        ])
    );
}

#[test]
fn lots_keep_their_cost_date_and_label() {
    let probe = include_str!("../../../spec/hledger/probes/lots.journal");
    let lot = |cost: Value, date: Value, label: Value| json!({ "cost": cost, "date": date, "label": label });
    let with_lot =
        |quantity: &str, l: Value| json!({ "commodity": "X", "quantity": quantity, "lot": l });
    assert_eq!(
        postings("lots", probe),
        json!([
            [
                1,
                [
                    regular(
                        "a",
                        json!([with_lot(
                            "2",
                            lot(q("EUR", "3"), json!("2026-01-02"), json!("label"))
                        )])
                    ),
                    regular("b", json!([q("X", "-2")]))
                ]
            ],
            [
                5,
                [
                    regular(
                        "a",
                        json!([with_lot(
                            "3",
                            lot(q("EUR", "3.3333333333"), Value::Null, Value::Null)
                        )])
                    ),
                    regular("b", json!([q("X", "-3")]))
                ]
            ],
            [
                9,
                [
                    regular(
                        "a",
                        json!([with_lot(
                            "-4",
                            lot(q("EUR", "-2.5"), Value::Null, Value::Null)
                        )])
                    ),
                    regular("b", json!([q("X", "4")]))
                ]
            ],
            [
                13,
                [
                    regular(
                        "a",
                        json!([with_lot("1", lot(q("EUR", "1"), Value::Null, Value::Null))])
                    ),
                    regular(
                        "b",
                        json!([with_lot("-1", lot(q("EUR", "1"), Value::Null, Value::Null))])
                    )
                ]
            ],
            [
                17,
                [
                    regular(
                        "a",
                        json!([{ "commodity": "X", "quantity": "1",
                    "cost": { "kind": "unit", "amount": q("EUR", "3") },
                    "lot": lot(q("EUR", "2"), Value::Null, Value::Null) }])
                    ),
                    regular("b", json!([q("EUR", "-3")]))
                ]
            ],
            [
                21,
                [
                    regular(
                        "a",
                        json!([with_lot("1", lot(Value::Null, Value::Null, Value::Null))])
                    ),
                    regular(
                        "b",
                        json!([with_lot("1", lot(q("EUR", "2"), Value::Null, Value::Null))])
                    ),
                    regular("c", json!([q("X", "-2")]))
                ]
            ]
        ])
    );
}

#[test]
fn virtual_postings_and_statuses() {
    let probe = include_str!("../../../spec/hledger/probes/posting-status-and-virtual.journal");
    assert_eq!(
        postings("posting-status-and-virtual", probe),
        json!([[
            1,
            [
                regular("a", json!([q("", "1")])),
                regular("b", json!([q("", "1")])),
                ["v", "virtual", [q("", "5")]],
                ["bv", "balanced-virtual", [q("", "2")]],
                ["bw", "balanced-virtual", [q("", "-2")]],
                regular("c", json!([q("", "-2")]))
            ]
        ]])
    );
    let names = include_str!("../../../spec/hledger/probes/posting-account-separators.journal");
    let accounts: Vec<Value> = postings("posting-account-separators", names)[0][1]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p[0].clone())
        .collect();
    assert_eq!(accounts, vec![json!("a b"), json!("c d"), json!("e")]);
}

#[test]
fn elided_amounts_get_the_negated_sum_with_hledgers_places() {
    let probe = include_str!("../../../spec/hledger/probes/elided.journal");
    let last = |t: &Value| t[1].as_array().unwrap().last().unwrap()[2].clone();
    let rows = postings("elided", probe);
    let got: Vec<Value> = rows.as_array().unwrap().iter().map(last).collect();
    assert_eq!(
        got,
        vec![
            json!([q("EUR", "0.00")]),
            json!([q("AA", "-1"), q("ZZ", "-1"), q("bb", "-1")]),
            json!([q("EUR", "0"), q("USD", "-1.0")]),
            json!([q("C", "-3.00")]),
            json!([]),
            json!([q("", "0")])
        ]
    );
    let grouping = include_str!("../../../spec/hledger/probes/elided-grouping.journal");
    let got: Vec<Value> = postings("elided-grouping", grouping)
        .as_array()
        .unwrap()
        .iter()
        .map(last)
        .collect();
    assert_eq!(got, vec![json!([q("X", "-1")]), json!([q("Y", "-3.0")])]);
}

#[test]
fn balancing_checks_each_commodity_to_its_written_places() {
    let probe = include_str!("../../../spec/hledger/probes/tolerance.journal");
    assert_eq!(postings("tolerance", probe).as_array().unwrap().len(), 3);
}

#[test]
fn inferred_prices_follow_hledger() {
    let probe = include_str!("../../../spec/hledger/probes/inferred-prices.journal");
    let first = |t: &Value| -> Value {
        t[1].as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p[2].as_array().unwrap().clone())
            .find(|a| a.get("cost").is_some())
            .unwrap_or(Value::Null)
    };
    let got: Vec<Value> = postings("inferred-prices", probe)
        .as_array()
        .unwrap()
        .iter()
        .map(first)
        .collect();
    assert_eq!(
        got,
        vec![
            total("X", "1", q("Y", "2.50")),
            unit("X", "1", q("Y", "0.3333333333")),
            unit("X", "1", q("Y", "1.5")),
            total("Y", "-2", q("X", "-1")),
            total("X", "1", q("Y", "2"))
        ]
    );
}

#[test]
fn a_matching_conversion_pair_balances_the_priced_posting_by_quantity() {
    for probe in [
        include_str!("../../../spec/hledger/probes/conversion.journal"),
        include_str!("../../../spec/hledger/probes/conversion-declared.journal"),
        include_str!("../../../spec/hledger/probes/conversion-type-tags.journal"),
        include_str!("../../../spec/hledger/probes/conversion-pair-places.journal"),
        include_str!("../../../spec/hledger/probes/conversion-several.journal"),
    ] {
        assert_eq!(findings("conversion", probe), json!([]), "{probe}");
    }
    // In conversion.journal the last elided amount is filled in by cost: the pair has an elided
    // amount, so it does not match.
    let rows = postings(
        "conversion",
        include_str!("../../../spec/hledger/probes/conversion.journal"),
    );
    assert_eq!(
        rows[3][1][2],
        regular("equity:trade", json!([q("EUR", "-6")]))
    );
}

#[test]
fn an_odd_run_of_conversion_postings_opening_the_transaction_balances_by_quantity() {
    // Each transaction of the probe balances only the way hledger reads it: by quantity after a
    // leading odd run, by cost when a posting repeats the quantity or the run is not leading.
    let probe = include_str!("../../../spec/hledger/probes/conversion-leading-run.journal");
    assert_eq!(findings("leading", probe), json!([]));
    assert_eq!(transactions("leading", probe).as_array().unwrap().len(), 6);
}
