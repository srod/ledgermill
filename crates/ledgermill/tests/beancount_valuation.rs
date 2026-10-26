//! `Ledger::prices()`, `value()`, `at_cost()` and `holdings()` for Beancount. Expected values are what
//! `bean-query` 0.2.0 (Beancount 3.2.3) reports for the probe journals, with Plainledger's queries
//! (see spec/beancount/price-table.md, convert-rates.md, convert-positions.md, cost-of-position.md and
//! holdings.md); they were read from the reference tool, never from ledgermill.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Holding, HoldingsQuery, Ledger, Quantity, Transaction, load};
use serde_json::json;
use support::file;

macro_rules! probe {
    ($name:literal) => {
        load(&[file(
            concat!($name, ".beancount"),
            include_str!(concat!(
                "../../../spec/beancount/probes/",
                $name,
                ".beancount"
            )),
        )])
    };
}

/// `(account, units, commodity)` for the postings of the accounts that start with `prefix`, in the
/// order of `transactions()`.
fn rows(transactions: &[Transaction], prefix: &str) -> Vec<(String, String, String)> {
    transactions
        .iter()
        .flat_map(|t| &t.postings)
        .filter(|p| p.account.starts_with(prefix))
        .map(|p| {
            let a = p.amounts.first().unwrap();
            (p.account.clone(), a.quantity.clone(), a.commodity.clone())
        })
        .collect()
}

/// `account units commodity -> amount commodity`, from the postings `before` and `after` a valuation.
fn pairs(before: &[Transaction], after: &[Transaction], prefix: &str) -> Vec<String> {
    let (before, after) = (rows(before, prefix), rows(after, prefix));
    assert_eq!(before.len(), after.len());
    before
        .iter()
        .zip(&after)
        .map(|((account, units, commodity), (_, amount, kind))| {
            format!("{account} {units} {commodity} -> {amount} {kind}")
        })
        .collect()
}

fn valued(ledger: &Ledger, commodity: &str, date: &str, prefix: &str) -> Vec<String> {
    let after = ledger.value(date, commodity).unwrap();
    pairs(ledger.transactions(), &after, prefix)
}

fn amounts(list: &[Quantity]) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|q| format!("{} {}", q.quantity, q.commodity))
        .collect();
    format!("[{}]", parts.join(", "))
}

fn held(holdings: &[Holding], prefix: &str) -> Vec<String> {
    holdings
        .iter()
        .filter(|h| h.account.starts_with(prefix))
        .map(|h| {
            format!(
                "{} {} units={} cost={} value={} cost_value={}",
                h.account,
                h.commodity,
                h.units,
                amounts(&h.cost),
                amounts(&h.value),
                amounts(&h.cost_value)
            )
        })
        .collect()
}

fn query(commodity: &str, end: Option<&str>) -> HoldingsQuery {
    HoldingsQuery {
        commodity: commodity.into(),
        end: end.map(Into::into),
        today: "2026-10-23".into(),
    }
}

#[test]
fn value_takes_the_latest_price_dated_on_or_before_the_date() {
    let ledger = probe!("convert-rate-dates");
    let cases: [(&str, [&str; 5]); 8] = [
        (
            "2026-01-09",
            [
                "Assets:Dated 10 AAA -> 10 AAA",
                "Assets:SameDay 10 BBB -> 10 BBB",
                "Assets:DirectFirst 10 CCC -> 10 CCC",
                "Assets:InverseFirst 10 DDD -> 10 DDD",
                "Assets:Mixed 10 EEE -> 10 EEE",
            ],
        ),
        (
            "2026-02-01",
            [
                "Assets:Dated 10 AAA -> 10 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 10 EEE",
            ],
        ),
        (
            "2026-02-02",
            [
                "Assets:Dated 10 AAA -> 10 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 70 EUR",
            ],
        ),
        (
            "2026-02-03",
            [
                "Assets:Dated 10 AAA -> 10 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 30 EUR",
            ],
        ),
        (
            "2026-02-04",
            [
                "Assets:Dated 10 AAA -> 10 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 1.0E+2 EUR",
            ],
        ),
        (
            "2026-02-05",
            [
                "Assets:Dated 10 AAA -> 10 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 50 EUR",
            ],
        ),
        (
            "2026-03-09",
            [
                "Assets:Dated 10 AAA -> 20 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 50 EUR",
            ],
        ),
        (
            "2099-01-01",
            [
                "Assets:Dated 10 AAA -> 90 EUR",
                "Assets:SameDay 10 BBB -> 30 EUR",
                "Assets:DirectFirst 10 CCC -> 2.50 EUR",
                "Assets:InverseFirst 10 DDD -> 20 EUR",
                "Assets:Mixed 10 EEE -> 50 EUR",
            ],
        ),
    ];
    for (date, expected) in cases {
        assert_eq!(valued(&ledger, "EUR", date, "Assets:"), expected, "{date}");
    }
}

#[test]
fn value_inverts_prices_and_keeps_a_zero_price_a_rate_of_zero() {
    let ledger = probe!("convert-rate-inverse");
    let cases: [(&str, &str, Vec<&str>); 6] = [
        (
            "EUR",
            "2026-02-01",
            vec![
                "Assets:Quarter 10 QTR -> 2.50 EUR",
                "Assets:Tenth 10 TNT -> 1.0E+2 EUR",
                "Assets:Zero 10 ZER -> 20 EUR",
                "Assets:Short -2.50 ZER -> -5.00 EUR",
                "Assets:Negative -4 NEG -> 2.00 EUR",
                "Assets:Chain 3 CAD -> 3 CAD",
                "Assets:Cash 10 EUR -> 10 EUR",
            ],
        ),
        (
            "EUR",
            "2026-02-02",
            vec![
                "Assets:Quarter 10 QTR -> 2.50 EUR",
                "Assets:Tenth 10 TNT -> 1.0E+2 EUR",
                "Assets:Zero 10 ZER -> 0 EUR",
                "Assets:Short -2.50 ZER -> -0.00 EUR",
                "Assets:Negative -4 NEG -> 2.00 EUR",
                "Assets:Chain 3 CAD -> 3 CAD",
                "Assets:Cash 10 EUR -> 10 EUR",
            ],
        ),
        (
            "USD",
            "2026-02-02",
            vec![
                "Assets:Quarter 10 QTR -> 10 QTR",
                "Assets:Tenth 10 TNT -> 10 TNT",
                "Assets:Zero 10 ZER -> 10 ZER",
                "Assets:Short -2.50 ZER -> -2.50 ZER",
                "Assets:Negative -4 NEG -> -4 NEG",
                "Assets:Chain 3 CAD -> 6 USD",
                "Assets:Cash 10 EUR -> 25.0 USD",
            ],
        ),
        (
            "QTR",
            "2026-02-02",
            vec![
                "Assets:Quarter 10 QTR -> 10 QTR",
                "Assets:Tenth 10 TNT -> 10 TNT",
                "Assets:Zero 10 ZER -> 10 ZER",
                "Assets:Short -2.50 ZER -> -2.50 ZER",
                "Assets:Negative -4 NEG -> -4 NEG",
                "Assets:Chain 3 CAD -> 3 CAD",
                "Assets:Cash 10 EUR -> 40 QTR",
            ],
        ),
        (
            "ZER",
            "2026-02-02",
            vec![
                "Assets:Quarter 10 QTR -> 10 QTR",
                "Assets:Tenth 10 TNT -> 10 TNT",
                "Assets:Zero 10 ZER -> 10 ZER",
                "Assets:Short -2.50 ZER -> -2.50 ZER",
                "Assets:Negative -4 NEG -> -4 NEG",
                "Assets:Chain 3 CAD -> 3 CAD",
                "Assets:Cash 10 EUR -> 5.0 ZER",
            ],
        ),
        (
            "NIL",
            "2026-02-02",
            vec![
                "Assets:Quarter 10 QTR -> 10 QTR",
                "Assets:Tenth 10 TNT -> 10 TNT",
                "Assets:Zero 10 ZER -> 10 ZER",
                "Assets:Short -2.50 ZER -> -2.50 ZER",
                "Assets:Negative -4 NEG -> -4 NEG",
                "Assets:Chain 3 CAD -> 3 CAD",
                "Assets:Cash 10 EUR -> 10 EUR",
            ],
        ),
    ];
    for (commodity, date, expected) in cases {
        assert_eq!(
            valued(&ledger, commodity, date, "Assets:"),
            expected,
            "{commodity} {date}"
        );
    }
}

#[test]
fn at_cost_is_the_units_times_the_booked_cost() {
    let ledger = probe!("cost-of-position");
    let after = ledger.at_cost().unwrap();
    assert_eq!(
        pairs(ledger.transactions(), &after, ""),
        [
            "Assets:Lots 3 AAA -> 99.99 EUR",
            "Assets:Lots 2.50 AAA -> 10.00 EUR",
            "Assets:Lots 1 AAA -> 1E-7 EUR",
            "Assets:Lots 10000000 AAA -> 10000000000000000 EUR",
            "Assets:Lots 3 AAA -> 9.999999999999999999999999999 EUR",
            "Assets:Lots 0.5 AAA -> 100.0 EUR",
            "Assets:Lots 2 AAA -> 205.0 EUR",
            "Equity:Other -10000000000000424.99000010000 EUR -> -10000000000000424.99000010000 EUR",
            "Assets:Plain 5 BBB -> 5 BBB",
            "Assets:Plain 4 CCC -> 4 CCC",
            "Assets:Plain 7.50 EUR -> 7.50 EUR",
            "Equity:Other -29.50 EUR -> -29.50 EUR",
            "Assets:Free 2.50 FFF -> 0.00 EUR",
            "Assets:Free -1.5 GGG -> -0.000 EUR",
            "Assets:Sold 2 HHH -> 20 EUR",
            "Assets:Sold 3 HHH -> 37.50 EUR",
            "Equity:Other -57.50 EUR -> -57.50 EUR",
            "Assets:Sold -2 HHH -> -20 EUR",
            "Assets:Sold -3 HHH -> -37.50 EUR",
            "Equity:Other 57.50 EUR -> 57.50 EUR",
        ]
    );
}

#[test]
fn holdings_go_through_the_cost_currency_only_when_both_rates_exist() {
    let ledger = probe!("convert-cost-bridge");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Bridge T2 units=10 cost=[30 K2] value=[200 EUR] cost_value=[60 EUR]",
            "Assets:CostIsTarget T10 units=10 cost=[30 EUR] value=[10 T10] cost_value=[30 EUR]",
            "Assets:CostIsTarget T11 units=10 cost=[30 EUR] value=[50 EUR] cost_value=[30 EUR]",
            "Assets:Direct T1 units=10 cost=[30 K1] value=[50 EUR] cost_value=[60 EUR]",
            "Assets:InverseFirstLeg T3 units=10 cost=[30 K3] value=[200 EUR] cost_value=[60 EUR]",
            "Assets:InverseSecondLeg T4 units=10 cost=[30 K4] value=[200 EUR] cost_value=[60 EUR]",
            "Assets:NoCost T11 units=10 cost=[10 T11] value=[50 EUR] cost_value=[50 EUR]",
            "Assets:NoFirstLeg T6 units=10 cost=[30 K6] value=[10 T6] cost_value=[60 EUR]",
            "Assets:NoPrice T7 units=10 cost=[30 K8] value=[10 T7] cost_value=[60 EUR]",
            "Assets:NoSecondLeg T5 units=10 cost=[30 K5] value=[10 T5] cost_value=[30 K5]",
            "Assets:UnitsAreTarget EUR units=10 cost=[11.0 K1] value=[10 EUR] cost_value=[22.0 EUR]",
            "Assets:ZeroSecondLeg T9 units=10 cost=[30 K9] value=[] cost_value=[]",
        ]
    );
}

#[test]
fn holdings_ignore_annotations_as_rates() {
    let ledger = probe!("convert-annotations");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Cost CCC units=2 cost=[160 EUR] value=[2 CCC] cost_value=[160 EUR]",
            "Assets:CostAndPrice BBB units=2 cost=[160 EUR] value=[2 BBB] cost_value=[160 EUR]",
            "Assets:Native EUR units=10 cost=[10 EUR] value=[10 EUR] cost_value=[10 EUR]",
            "Assets:PriceAndDirective PPP units=2 cost=[2 PPP] value=[10 EUR] cost_value=[10 EUR]",
            "Assets:TotalPrice TTT units=-2 cost=[-2 TTT] value=[-2 TTT] cost_value=[-2 TTT]",
            "Assets:UnitPrice UUU units=2 cost=[2 UUU] value=[2 UUU] cost_value=[2 UUU]",
        ]
    );
}

#[test]
fn holdings_round_each_product_of_the_bridge() {
    let ledger = probe!("convert-bridge-arith");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Lot QQQ units=12889439741600 cost=[12889439741600 USD] value=[7436171342102426326.559485529 EUR] cost_value=[207225719318.3279742765273312 EUR]",
        ]
    );
}

#[test]
fn holdings_drop_a_zero_sum_and_keep_the_finer_places() {
    let ledger = probe!("holdings-sums");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Finer EUR units=3.75 cost=[3.75 EUR] value=[3.75 EUR] cost_value=[3.75 EUR]",
            "Assets:Mixed MMM units=21 cost=[3 CHF, 20 EUR, 2 MMM, 35 USD] value=[154 EUR, 10 MMM] cost_value=[102 EUR, 2 MMM]",
            "Assets:Padded EUR units=100 cost=[100 EUR] value=[100 EUR] cost_value=[100 EUR]",
            "Assets:Pending EUR units=4 cost=[4 EUR] value=[4 EUR] cost_value=[4 EUR]",
            "Assets:Reset EUR units=3 cost=[3 EUR] value=[3 EUR] cost_value=[3 EUR]",
            "Assets:Short XXX units=0 cost=[-10 EUR] value=[] cost_value=[-10 EUR]",
            "Assets:Zeros EUR units=5.00 cost=[5.00 EUR] value=[5.00 EUR] cost_value=[5.00 EUR]",
            "Assets:Zeros USD units=7 cost=[7 USD] value=[14 EUR] cost_value=[14 EUR]",
        ]
    );
}

#[test]
fn holdings_end_is_exclusive_the_day_before_counts() {
    let ledger = probe!("holdings-sums");
    let holdings = ledger.holdings(&query("EUR", Some("2026-01-04"))).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        ["Assets:Padded EUR units=100 cost=[100 EUR] value=[100 EUR] cost_value=[100 EUR]",]
    );
}

#[test]
fn holdings_take_the_latest_prices_there_are_with_no_date() {
    let ledger = probe!("convert-rate-dates");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Dated AAA units=10 cost=[10 AAA] value=[90 EUR] cost_value=[90 EUR]",
            "Assets:DirectFirst CCC units=10 cost=[10 CCC] value=[2.50 EUR] cost_value=[2.50 EUR]",
            "Assets:InverseFirst DDD units=10 cost=[10 DDD] value=[20 EUR] cost_value=[20 EUR]",
            "Assets:Mixed EEE units=10 cost=[10 EEE] value=[50 EUR] cost_value=[50 EUR]",
            "Assets:SameDay BBB units=10 cost=[10 BBB] value=[30 EUR] cost_value=[30 EUR]",
        ]
    );
}

#[test]
fn holdings_value_the_latest_price_whatever_the_end() {
    let ledger = probe!("convert-rate-dates");
    let holdings = ledger.holdings(&query("EUR", Some("2026-01-10"))).unwrap();
    assert_eq!(
        held(&holdings, "Assets:"),
        [
            "Assets:Dated AAA units=10 cost=[10 AAA] value=[90 EUR] cost_value=[90 EUR]",
            "Assets:DirectFirst CCC units=10 cost=[10 CCC] value=[2.50 EUR] cost_value=[2.50 EUR]",
            "Assets:InverseFirst DDD units=10 cost=[10 DDD] value=[20 EUR] cost_value=[20 EUR]",
            "Assets:Mixed EEE units=10 cost=[10 EEE] value=[50 EUR] cost_value=[50 EUR]",
            "Assets:SameDay BBB units=10 cost=[10 BBB] value=[30 EUR] cost_value=[30 EUR]",
        ]
    );
}

// ───────────── what is refused, and the arguments ─────────────

fn only(content: &str) -> Ledger {
    load(&[file("t.beancount", content)])
}

#[test]
fn a_price_written_as_an_expression_is_refused_by_every_read() {
    let ledger = load(&[file(
        "price-expression.beancount",
        include_str!("../../../spec/beancount/probes/refused/price-expression.beancount"),
    )]);
    let construct = json!({ "construct": "number-expression" });
    assert_eq!(ledger.check()[0].details, construct);
    assert!(ledger.prices().is_empty());
    for finding in [
        ledger.value("2026-02-01", "EUR").unwrap_err(),
        ledger.at_cost().unwrap_err(),
        ledger.holdings(&query("EUR", None)).unwrap_err(),
    ] {
        assert_eq!(finding.code, "unsupported-syntax");
        assert_eq!(finding.details, construct);
        assert_eq!(finding.line, 3);
    }
}

#[test]
fn a_set_of_files_that_cannot_be_read_together_is_refused_by_every_read() {
    let beancount = file("a.beancount", "2026-01-01 open Assets:A\n");
    let ledger = load(&[
        beancount.clone(),
        file("b.beancount", "2026-01-01 open Assets:B\n"),
    ]);
    let finding = ledger.value("2026-02-01", "EUR").unwrap_err();
    assert_eq!(
        finding.details,
        json!({ "construct": "several-beancount-files" })
    );
    let ledger = load(&[beancount, file("c.journal", "2026-01-01 x\n  a  1\n  b\n")]);
    let finding = ledger.holdings(&query("EUR", None)).unwrap_err();
    assert_eq!(finding.details, json!({ "construct": "mixed-formats" }));
}

#[test]
fn a_journal_with_a_booking_error_answers_for_the_transactions_it_lists() {
    // Two elided postings: bean-query leaves the transaction out, so there is nothing to value.
    let ledger = only(
        "2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n2026-01-02 * \"t\"\n  Assets:A\n  Assets:B\n",
    );
    assert_eq!(ledger.check()[0].code, "booking-error");
    assert!(ledger.holdings(&query("EUR", None)).unwrap().is_empty());
    assert!(ledger.value("2026-02-01", "EUR").unwrap().is_empty());
}

#[test]
fn a_date_that_is_not_a_date_is_an_invalid_argument() {
    let ledger = probe!("holdings-sums");
    for date in ["2026-02-30", "2026-1-1", "tomorrow", ""] {
        let finding = ledger.value(date, "EUR").unwrap_err();
        assert_eq!(finding.code, "invalid-argument", "{date}");
        let finding = ledger.holdings(&query("EUR", Some(date))).unwrap_err();
        assert_eq!(finding.details, json!({ "argument": "end", "value": date }));
    }
    let mut asked = query("EUR", None);
    asked.today = "later".into();
    assert_eq!(
        ledger.holdings(&asked).unwrap_err().details,
        json!({ "argument": "today", "value": "later" })
    );
}

#[test]
fn valuing_in_no_commodity_is_refused() {
    let ledger = probe!("holdings-sums");
    for finding in [
        ledger.value("2026-02-01", "").unwrap_err(),
        ledger.holdings(&query("", None)).unwrap_err(),
    ] {
        assert_eq!(finding.code, "unsupported-syntax");
        assert_eq!(
            finding.details,
            json!({ "construct": "value-without-commodity" })
        );
    }
}

#[test]
fn today_changes_nothing_in_a_beancount_ledger() {
    let ledger = probe!("convert-rate-dates");
    let mut later = query("EUR", None);
    later.today = "2099-12-31".into();
    assert_eq!(
        ledger.holdings(&query("EUR", None)).unwrap(),
        ledger.holdings(&later).unwrap()
    );
}

#[test]
fn nothing_is_held_before_the_first_day() {
    let ledger = probe!("holdings-sums");
    assert!(
        ledger
            .holdings(&query("EUR", Some("0000-01-01")))
            .unwrap()
            .is_empty()
    );
    // The first transaction is the padding of 2026-01-02: the day before it holds nothing.
    assert!(
        ledger
            .holdings(&query("EUR", Some("2026-01-02")))
            .unwrap()
            .is_empty()
    );
}

// ───────────── shapes of the answers ─────────────

#[test]
fn a_converted_amount_has_no_cost_or_lot_and_a_stay_keeps_them() {
    let ledger = probe!("convert-cost-bridge");
    let after = ledger.value("2026-02-01", "EUR").unwrap();
    let amount = |account: &str| {
        after
            .iter()
            .flat_map(|t| &t.postings)
            .find(|p| p.account == account)
            .and_then(|p| p.amounts.first())
            .unwrap()
            .clone()
    };
    // Converted through the cost's currency: an amount in EUR, nothing else.
    let bridged = amount("Assets:Bridge");
    assert_eq!(
        (bridged.commodity.as_str(), bridged.quantity.as_str()),
        ("EUR", "200")
    );
    assert!(bridged.cost.is_none() && bridged.lot.is_none());
    // No rate: the posting is left whole, lot included.
    let stays = amount("Assets:NoSecondLeg");
    assert_eq!(
        (stays.commodity.as_str(), stays.quantity.as_str()),
        ("T5", "10")
    );
    assert_eq!(stays.lot.unwrap().cost.unwrap().commodity, "K5");
}

#[test]
fn the_sign_of_a_zero_product_is_kept() {
    let ledger = only(
        "2026-01-01 open Assets:A\n2026-01-01 open Equity:E\n2026-01-01 price Z 0 EUR\n\
         2026-01-02 * \"t\"\n  Assets:A  -2.50 Z\n  Equity:E  2.50 Z\n  Assets:A  2 Z\n  Equity:E  -2 Z\n",
    );
    let after = ledger.value("2026-01-02", "EUR").unwrap();
    let numbers: Vec<_> = after[0]
        .postings
        .iter()
        .map(|p| p.amounts[0].quantity.as_str())
        .collect();
    assert_eq!(numbers, ["-0.00", "0.00", "0", "-0"]);
}

#[test]
fn a_holding_with_no_units_left_is_listed_while_a_cost_remains() {
    // NONE booking: a sale at another cost is another lot, so the units cancel and the costs do not.
    let ledger = probe!("holdings-sums");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    let short = holdings
        .iter()
        .find(|h| h.account == "Assets:Short")
        .unwrap();
    assert_eq!(short.units, "0");
    assert_eq!(
        short.cost,
        [Quantity {
            commodity: "EUR".into(),
            quantity: "-10".into()
        }]
    );
    // A group that sums to nothing everywhere is not listed.
    let before_the_sale = ledger.holdings(&query("EUR", Some("2026-01-04"))).unwrap();
    assert!(before_the_sale.iter().all(|h| h.account != "Assets:Reset"));
}

#[test]
fn holdings_are_ordered_by_account_names_then_commodity() {
    let ledger = probe!("holdings-sums");
    let holdings = ledger.holdings(&query("EUR", None)).unwrap();
    let keys: Vec<(&str, &str)> = holdings
        .iter()
        .filter(|h| h.account.starts_with("Assets:"))
        .map(|h| (h.account.as_str(), h.commodity.as_str()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn a_pair_priced_both_ways_gets_its_own_prices_back_as_a_double_inverse() {
    // The rate of one unit of FROM in TO on 2026-01-01 to 06, as bean-query gives it ("-": the unit stays).
    let ledger = probe!("convert-rate-both-ways");
    assert_eq!(ledger.check().len(), 0);
    let cases: [(&str, &str, [&str; 6]); 10] = [
        (
            "A1",
            "B1",
            [
                "3",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
            ],
        ),
        (
            "B1",
            "A1",
            [
                "0.3333333333333333333333333333",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
            ],
        ),
        (
            "A2",
            "B2",
            [
                "0.1428571428571428571428571429",
                "3.000000000000000000000000000",
                "3.000000000000000000000000000",
                "3.000000000000000000000000000",
                "3.000000000000000000000000000",
                "3.000000000000000000000000000",
            ],
        ),
        (
            "B2",
            "A2",
            [
                "7",
                "0.3333333333333333333333333333",
                "0.3333333333333333333333333333",
                "0.3333333333333333333333333333",
                "0.3333333333333333333333333333",
                "0.3333333333333333333333333333",
            ],
        ),
        (
            "A3",
            "B3",
            [
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
                "0.1428571428571428571428571429",
            ],
        ),
        (
            "B3",
            "A3",
            [
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
                "6.999999999999999999999999998",
            ],
        ),
        (
            "A4",
            "B4",
            [
                "3",
                "0.1428571428571428571428571429",
                "11",
                "0.07692307692307692307692307692",
                "0.07692307692307692307692307692",
                "0.07692307692307692307692307692",
            ],
        ),
        (
            "B4",
            "A4",
            [
                "0.3333333333333333333333333333",
                "6.999999999999999999999999998",
                "0.09090909090909090909090909091",
                "13.00000000000000000000000000",
                "13.00000000000000000000000000",
                "13.00000000000000000000000000",
            ],
        ),
        (
            "B5",
            "C5",
            [
                "1000.500000000000000000000000",
                "1000.500000000000000000000000",
                "1000.500000000000000000000000",
                "3.333333333333333333333333333",
                "2",
                "2",
            ],
        ),
        (
            "C5",
            "B5",
            [
                "0.0009995002498750624687656171914",
                "0.0009995002498750624687656171914",
                "0.0009995002498750624687656171914",
                "0.3",
                "0.5",
                "0.5",
            ],
        ),
    ];
    for (from, to, expected) in cases {
        let rates: Vec<String> = (1..=6)
            .map(|day| {
                let after = ledger.value(&format!("2026-01-0{day}"), to).unwrap();
                let posting = after
                    .iter()
                    .flat_map(|t| &t.postings)
                    .find(|p| p.account == format!("Assets:{from}"))
                    .unwrap();
                let amount = &posting.amounts[0];
                if amount.commodity == to {
                    amount.quantity.clone()
                } else {
                    "-".into()
                }
            })
            .collect();
        assert_eq!(rates, expected, "{from} in {to}");
    }
}

#[test]
fn a_zero_price_in_a_pair_priced_both_ways_is_refused_for_value_and_holdings_only() {
    let ledger = load(&[file(
        "z.beancount",
        include_str!("../../../spec/beancount/probes/refused/price-both-ways-zero.beancount"),
    )]);
    assert_eq!(ledger.check().len(), 0);
    for finding in [
        ledger.value("2026-01-03", "CCC").unwrap_err(),
        ledger.holdings(&query("CCC", None)).unwrap_err(),
    ] {
        assert_eq!(finding.code, "unsupported-syntax");
        assert_eq!(finding.details, json!({ "construct": "price-both-ways" }));
        assert_eq!((finding.file.as_str(), finding.line), ("z.beancount", 7));
    }
    // Nothing else of the ledger is in doubt.
    assert_eq!(ledger.prices().len(), 2);
    assert!(ledger.at_cost().is_ok());
    // A zero price in a pair priced one way only is the rate of zero.
    let one_way = probe!("convert-rate-inverse");
    assert!(one_way.value("2026-02-02", "EUR").is_ok());
}

#[test]
fn a_kept_negative_zero_keeps_its_sign_when_valued() {
    // interpolation-rounds-to-zero, 2026-01-08: the elided posting is `-0.00 EUR`.
    let ledger = probe!("interpolation-rounds-to-zero");
    let last = |commodity: &str| {
        valued(&ledger, commodity, "2026-01-09", "Assets:C")
            .last()
            .unwrap()
            .clone()
    };
    assert_eq!(last("USD"), "Assets:C -0.00 EUR -> -0.00 USD");
    assert_eq!(last("GBP"), "Assets:C -0.00 EUR -> 0.00 GBP");
}
