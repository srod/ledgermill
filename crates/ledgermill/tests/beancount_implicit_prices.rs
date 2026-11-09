//! `plugin "beancount.plugins.implicit_prices"`: a `price` entry for every price annotation and every
//! lot a posting opens (spec/beancount/implicit-prices.md). It changes `prices()`, `value()` and
//! `holdings()`.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{HoldingsQuery, load};
use support::{file, price_lines, probe, valued};

/// `DATE COMMODITY RATE QUOTE@LINE`, one per entry of the prices table.
fn table(journal: &str) -> Vec<String> {
    let ledger = load(&[file("p.beancount", journal)]);
    ledger
        .prices()
        .iter()
        .map(|p| {
            format!(
                "{} {} {} {}@{}",
                p.date, p.commodity, p.rate.quantity, p.rate.commodity, p.line
            )
        })
        .collect()
}

#[test]
fn annotations_and_added_lots_make_prices_on_the_line_of_their_transaction() {
    assert_eq!(
        table(&probe("implicit-prices")),
        [
            "2026-01-02 BTC 100.00 EUR@6",
            "2026-01-03 BTC 2.5 EUR@10",
            "2026-01-04 BTC 101.50 EUR@14",
            "2026-01-05 BTC 12.5 EUR@18",
        ]
    );
    // Without the plugin line the table is empty.
    let without =
        probe("implicit-prices").replace("plugin \"beancount.plugins.implicit_prices\"\n", "");
    assert!(table(&without).is_empty());
}

#[test]
fn a_price_annotation_wins_over_a_cost_and_a_reduction_makes_no_price() {
    assert_eq!(
        table(&probe("implicit-prices-cost-and-price")),
        [
            "2026-01-02 BTC 12.50 EUR@7",
            "2026-01-04 BTC 10 EUR@11",
            "2026-01-05 BTC 25 EUR@15",
            "2026-01-06 BTC 10 EUR@19",
            "2026-01-07 BTC 12 EUR@23",
        ]
    );
    assert_eq!(
        table(&probe("implicit-prices-reductions")),
        [
            "2026-01-02 BTC 10 EUR@5",
            "2026-01-05 BTC 14 EUR@17",
            "2026-01-06 BTC 15 EUR@22",
            "2026-01-07 BTC 10 EUR@27",
        ]
    );
}

#[test]
fn entries_are_equal_by_value_not_by_how_the_rate_is_written() {
    // Each day has two rates of the same value written differently (one has an exponent, from a
    // booked total price); the first one written stays, as written, whichever form it has.
    assert_eq!(
        table(&probe("implicit-prices-dedupe-values")),
        [
            "2026-01-02 BTC 200 EUR@6",
            "2026-01-03 BTC 2.0E+2 EUR@14",
            "2026-01-04 BTC 1.000E+6 EUR@22",
        ]
    );
}

#[test]
fn equal_entries_of_one_run_are_dropped_but_not_those_of_a_price_directive() {
    assert_eq!(
        table(&probe("implicit-prices-dedupe")),
        [
            "2026-01-02 BTC 100.00 EUR@7",
            "2026-01-03 BTC 100.0 EUR@15",
            "2026-01-04 BTC 100 EUR@23",
            "2026-01-04 BTC 100 USD@27",
            "2026-01-05 BTC 100 EUR@31",
            "2026-01-05 ETH 100 EUR@35",
            "2026-01-06 BTC 100 EUR@39",
            "2026-01-07 BTC 100 EUR@43",
            "2026-01-08 BTC 100 EUR@47",
            "2026-01-08 BTC 101 EUR@51",
        ]
    );
    let lines = price_lines(&[("p.beancount", &probe("implicit-prices-duplicates"))]);
    assert_eq!(
        lines,
        [
            "2026-01-02 BTC 90.00 EUR",
            "2026-01-02 BTC 100.00 EUR",
            "2026-01-02 BTC 110.00 EUR",
            "2026-01-03 BTC 100.00 EUR",
            "2026-01-03 BTC 90.00 EUR",
            "2026-01-04 BTC 100.00 EUR",
            "2026-01-04 BTC 100.00 EUR",
            "2026-01-05 BTC 100.00 EUR",
            "2026-01-06 BTC 100.00 EUR",
            "2026-01-06 BTC 120.00 EUR",
            "2026-01-06 BTC 130.00 EUR",
        ]
    );
}

#[test]
fn each_plugin_line_runs_the_plugin_again() {
    assert_eq!(table(&probe("implicit-prices-twice")).len(), 3);
    assert_eq!(
        table(&probe("implicit-prices-order")),
        [
            "2026-01-01 AAA 3 EUR@20",
            "2026-01-01 AAA 3 EUR@20",
            "2026-01-02 AAA 1 EUR@9",
            "2026-01-02 AAA 5 EUR@11",
            "2026-01-02 AAA 7 EUR@11",
            "2026-01-02 BBB 6 USD@11",
            "2026-01-02 AAA 5 EUR@11",
            "2026-01-02 AAA 7 EUR@11",
            "2026-01-02 BBB 6 USD@11",
            "2026-01-02 AAA 2 EUR@18",
        ]
    );
}

#[test]
fn transactions_booking_rejects_make_no_price_and_the_others_do() {
    assert_eq!(
        table(&probe("implicit-prices-rejected")),
        [
            "2026-01-02 BTC 100 EUR@6",
            "2026-01-04 BTC 102 EUR@15",
            "2026-01-06 BTC 103 EUR@19",
            "2026-01-07 BTC 104 EUR@23",
            "2026-01-09 BTC 106 EUR@33",
        ]
    );
}

#[test]
fn the_rate_has_the_figures_of_the_booked_price() {
    assert_eq!(
        table(&probe("implicit-prices-numbers")),
        [
            "2026-01-02 BTC 15 EUR@4",
            "2026-01-03 BTC 1.25 EUR@8",
            "2026-01-04 BTC 12 EUR@12",
            "2026-01-05 BTC 3.333333333333333333333333333 EUR@16",
            "2026-01-06 BTC 3.333333333333333333333333333 EUR@20",
            "2026-01-07 BTC 2.0E+2 EUR@24",
            "2026-01-08 BTC 1E-7 EUR@28",
            "2026-01-09 BTC 1.000E+6 EUR@32",
            "2026-01-10 BTC 1234.500 EUR@36",
            "2026-01-11 BTC 12.5 EUR@40",
            "2026-01-12 BTC 3.333333333333333333333333333 EUR@44",
        ]
    );
    assert_eq!(
        table(&probe("implicit-prices-inferred")).len(),
        8,
        "inferred costs and prices are prices too"
    );
}

#[test]
fn value_uses_the_entries_in_the_order_of_the_table() {
    let journal = probe("implicit-prices-convert");
    let at = |date: &str| valued(&[("p.beancount", &journal)], date, "EUR").join("\n");
    // 2 BTC at 100.00 on 01-02, then 4 BTC @@ 520 makes 130 on 01-04.
    assert!(
        at("2026-01-02").contains("Assets:Btc: EUR 200.00"),
        "{}",
        at("2026-01-02")
    );
    assert!(
        at("2026-01-04").contains("Assets:Btc: EUR 260 |"),
        "{}",
        at("2026-01-04")
    );
    // ETH on 01-03: the entry of the posting (30) is written above the directive (31), which wins;
    // on 01-05 the directive (40) is written above the posting (41), which wins.
    assert!(
        at("2026-01-03").contains("Assets:Eth: EUR 31 |"),
        "{}",
        at("2026-01-03")
    );
    assert!(
        at("2026-01-05").contains("Assets:Eth: EUR 41 |"),
        "{}",
        at("2026-01-05")
    );
}

#[test]
fn holdings_take_the_latest_implicit_entry() {
    let ledger = load(&[file("p.beancount", &probe("implicit-prices-convert"))]);
    let rows = ledger
        .holdings(&HoldingsQuery {
            commodity: "EUR".into(),
            end: None,
            today: "2026-11-09".into(),
        })
        .unwrap();
    let btc = rows
        .iter()
        .find(|h| h.account == "Assets:Btc")
        .expect("a BTC holding");
    // 2 BTC @ 100.00 and 4 BTC @@ 520 hold 6 BTC at the latest entry, 130 EUR.
    assert_eq!(btc.units, "6");
    assert_eq!(btc.value.first().map(|q| q.quantity.as_str()), Some("780"));
}
