//! `Ledger::prices()` for Beancount. Expected values are what `bean-query` 0.2.0 (Beancount 3.2.3)
//! reports for the probe journal with Plainledger's query and the entries' lines (see
//! spec/beancount/price-table.md); they were read from the reference tool, never from ledgermill.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Ledger, load};
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

fn price_lines(ledger: &Ledger) -> Vec<String> {
    ledger
        .prices()
        .iter()
        .map(|p| {
            format!(
                "{} {} {} {} line {}",
                p.date, p.commodity, p.rate.quantity, p.rate.commodity, p.line
            )
        })
        .collect()
}

#[test]
fn prices_lists_the_directives_by_date_and_keeps_every_one() {
    let ledger = probe!("price-table");
    assert_eq!(ledger.check().len(), 0);
    assert_eq!(
        price_lines(&ledger),
        [
            "2026-01-05 BBB 2 EUR line 8",
            "2026-02-01 CCC 1E-7 EUR line 11",
            "2026-02-01 DDD 1234.50 EUR line 12",
            "2026-02-01 EEE 3 EUR line 13",
            "2026-02-01 FFF 0 EUR line 14",
            "2026-02-01 GGG -2 EUR line 15",
            "2026-02-01 EUR 1 EUR line 16",
            "2026-02-01 HHH 1.123456789012345678 EUR line 17",
            "2026-02-01 JJJ 0 EUR line 18",
            "2026-02-01 KKK 1.000 EUR line 19",
            "2026-02-01 LLL 12345678901234567890 EUR line 20",
            "2026-03-01 AAA 1.50 EUR line 7",
            "2026-03-01 AAA 1.60 EUR line 9",
            "2026-03-01 AAA 1.60 EUR line 10",
        ]
    );
    assert!(
        ledger
            .prices()
            .iter()
            .all(|p| p.file == "price-table.beancount")
    );
}

#[test]
fn an_annotation_on_a_posting_or_a_cost_is_not_a_price() {
    // MMM, NNN and OOO are bought with `@`, `@@` and a cost in price-table.beancount.
    let ledger = probe!("price-table");
    assert!(
        ledger
            .prices()
            .iter()
            .all(|p| !["MMM", "NNN", "OOO"].contains(&p.commodity.as_str()))
    );
}

#[test]
fn a_number_may_end_in_a_dot_or_start_with_a_plus_on_a_price_line() {
    // `bean-check` accepts all five; a dot after a digit of a currency (`X1.Y`) is no number's.
    let ledger = load(&[file(
        "t.beancount",
        "2026-02-01 price A +3. EUR\n2026-02-01 price B +3 EUR\n2026-02-01 price C 3. EUR\n\
         2026-02-01 price X1.Y 3 EUR\n2026-02-01 price D 3. EUR ; 4.\n",
    )]);
    assert_eq!(ledger.check().len(), 0);
    let listed: Vec<_> = ledger
        .prices()
        .iter()
        .map(|p| (p.commodity.as_str(), p.rate.quantity.as_str()))
        .collect();
    assert_eq!(
        listed,
        [
            ("A", "3"),
            ("B", "3"),
            ("C", "3"),
            ("X1.Y", "3"),
            ("D", "3")
        ]
    );
}

#[test]
fn a_price_written_as_an_expression_is_refused_and_lists_nothing() {
    let ledger = load(&[file(
        "price-expression.beancount",
        include_str!("../../../spec/beancount/probes/refused/price-expression.beancount"),
    )]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(
        ledger.check()[0].details,
        json!({ "construct": "number-expression" })
    );
    assert_eq!(ledger.check()[0].line, 3);
    assert!(ledger.prices().is_empty());
}
