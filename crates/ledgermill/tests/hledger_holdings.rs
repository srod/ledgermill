//! `Ledger::holdings()`: units, cost, value and cost value per account and commodity. Expected values:
//! hledger 1.52.4 `balance --layout=tidy -O json --real` on the named `hold-*` probes, as recorded
//! in `spec/hledger/holdings.md`.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{Holding, HoldingsQuery, Quantity, load};
use serde_json::json;
use support::file;

macro_rules! probe {
    ($name:literal) => {
        (
            concat!($name, ".journal"),
            include_str!(concat!("../../../spec/hledger/probes/", $name, ".journal")),
        )
    };
}

/// The day the probes of the note were made.
const TODAY: &str = "2026-10-23";

fn query(commodity: &str, end: Option<&str>, today: &str) -> HoldingsQuery {
    HoldingsQuery {
        commodity: commodity.into(),
        end: end.map(Into::into),
        today: today.into(),
    }
}

fn run(
    files: &[(&str, &str)],
    commodity: &str,
    end: Option<&str>,
    today: &str,
) -> Result<Vec<Holding>, ledgermill::Finding> {
    let files: Vec<_> = files.iter().map(|(n, c)| file(n, c)).collect();
    load(&files).holdings(&query(commodity, end, today))
}

fn amounts(list: &[Quantity]) -> String {
    if list.is_empty() {
        return "-".into();
    }
    let parts: Vec<String> = list
        .iter()
        .map(|q| format!("{} {}", q.commodity, q.quantity))
        .collect();
    parts.join(", ")
}

/// `ACCOUNT [COMMODITY] | units | cost | value | cost value`, one line per holding; `-` for an
/// empty list.
fn lines(probe: (&str, &str), commodity: &str, end: Option<&str>) -> Vec<String> {
    lines_at(probe, commodity, end, TODAY)
}

fn lines_at(probe: (&str, &str), commodity: &str, end: Option<&str>, today: &str) -> Vec<String> {
    run(&[probe], commodity, end, today)
        .unwrap()
        .iter()
        .map(|h| {
            format!(
                "{} [{}] | {} | {} | {} | {}",
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

fn refusal(files: &[(&str, &str)], commodity: &str, end: Option<&str>) -> ledgermill::Finding {
    run(files, commodity, end, TODAY).unwrap_err()
}

/// The lines of the accounts whose name starts with `account`.
fn of(account: &str, all: Vec<String>) -> Vec<String> {
    all.into_iter().filter(|l| l.starts_with(account)).collect()
}

// ───────────── which postings count ─────────────

#[test]
fn a_posting_counts_when_it_is_dated_before_the_end() {
    let p = probe!("hold-e-boundary");
    assert_eq!(lines(p, "EUR", Some("2026-01-10")), Vec::<String>::new());
    assert_eq!(
        lines(p, "EUR", Some("2026-01-11")),
        [
            "a [X] | 10 | X 10 | EUR 10 | EUR 10",
            "b [X] | -10 | X -10 | EUR -10 | EUR -10"
        ]
    );
    assert_eq!(
        of("a ", lines(p, "EUR", Some("2026-02-10"))),
        ["a [X] | 10 | X 10 | EUR 20 | EUR 20"]
    );
    assert_eq!(
        of("a ", lines(p, "EUR", Some("2026-02-11"))),
        ["a [X] | 20 | X 20 | EUR 40 | EUR 40"]
    );
}

#[test]
fn a_posting_counts_on_its_own_date_tag_not_the_transactions_or_the_secondary_date() {
    let p = probe!("hold-e-tags");
    assert_eq!(
        lines(p, "EUR", Some("2026-01-06")),
        ["a [X] | 10 | X 10 | EUR 10 | EUR 10"]
    );
    assert_eq!(
        lines(p, "EUR", Some("2026-03-02")),
        [
            "a [X] | 10 | X 10 | EUR 20 | EUR 20",
            "c [X] | 1 | X 1 | EUR 2 | EUR 2",
            "d [X] | -1 | X -1 | EUR -2 | EUR -2"
        ]
    );
    assert_eq!(
        lines(p, "EUR", Some("2026-05-06")),
        [
            "a [X] | 10 | X 10 | EUR 40 | EUR 40",
            "b [X] | -10 | X -10 | EUR -40 | EUR -40",
            "c [X] | 1 | X 1 | EUR 4 | EUR 4",
            "d [X] | -1 | X -1 | EUR -4 | EUR -4"
        ]
    );
}

#[test]
fn no_posting_before_the_end_gives_no_holding() {
    assert_eq!(
        lines(probe!("hold-e-boundary"), "EUR", Some("2025-01-01")),
        Vec::<String>::new()
    );
    assert_eq!(
        lines(probe!("hold-end-only-p"), "EUR", None),
        Vec::<String>::new()
    );
}

#[test]
fn only_real_postings_count() {
    assert_eq!(
        lines(probe!("hold-real"), "EUR", None),
        [
            "a [X] | 10 | X 10 | EUR 20 | EUR 20",
            "b [X] | -10 | X -10 | EUR -20 | EUR -20"
        ]
    );
}

// ───────────── the date values are taken at ─────────────

#[test]
fn with_an_end_values_are_taken_the_day_before_whatever_the_prices_say() {
    // The last price of X (4 EUR) is dated 04-01; -e 05-01 values at 04-30.
    let p = probe!("hold-e-boundary");
    assert_eq!(
        of("a ", lines(p, "EUR", Some("2026-05-01"))),
        ["a [X] | 20 | X 20 | EUR 80 | EUR 80"]
    );
}

#[test]
fn without_an_end_a_price_after_the_last_transaction_moves_the_date() {
    let p = probe!("hold-end-p-after");
    assert_eq!(
        of("a ", lines(p, "EUR", None)),
        ["a [X] | 20 | X 20 | EUR 100 | EUR 100"]
    );
    assert_eq!(
        of("a ", lines(p, "EUR", Some("2026-03-11"))),
        ["a [X] | 20 | X 20 | EUR 40 | EUR 40"]
    );
}

#[test]
fn a_price_after_today_does_not_move_the_date_and_one_dated_today_does() {
    let p = probe!("hold-end-p-future");
    let value = |today: &str| of("a ", lines_at(p, "EUR", None, today));
    assert_eq!(value(TODAY), ["a [X] | 10 | X 10 | EUR 10 | EUR 10"]);
    assert_eq!(value("2098-12-31"), ["a [X] | 10 | X 10 | EUR 10 | EUR 10"]);
    assert_eq!(value("2099-01-01"), ["a [X] | 10 | X 10 | EUR 90 | EUR 90"]);
}

#[test]
fn the_whole_ledger_decides_the_date_not_the_account() {
    // The last price is for Y (08-01), the last transaction is in another account.
    assert_eq!(
        of("a ", lines(probe!("hold-end-unrelated"), "EUR", None)),
        ["a [X] | 10 | X 10 | EUR 20 | EUR 20"]
    );
}

#[test]
fn a_virtual_postings_date_moves_the_valuation_date() {
    assert_eq!(
        lines(probe!("hold-end-virtual"), "EUR", None),
        [
            "a [X] | 10 | X 10 | EUR 40 | EUR 40",
            "b [X] | -10 | X -10 | EUR -40 | EUR -40"
        ]
    );
}

// ───────────── how a position is made ─────────────

#[test]
fn values_are_summed_before_they_are_rounded_to_ten_places() {
    assert_eq!(
        lines(probe!("hold-sum"), "Y", None),
        [
            "a [EUR] | 3 | EUR 3 | Y 1.0000000000 | Y 1.0000000000",
            "b [EUR] | -3 | EUR -3 | Y -1.0000000000 | Y -1.0000000000"
        ]
    );
}

#[test]
fn cost_is_the_postings_converted_and_summed_per_commodity() {
    let p = probe!("hold-cost");
    assert_eq!(
        of("a [X]", lines(p, "EUR", None)),
        ["a [X] | 17 | EUR 20, USD 5, X 5 | EUR 51 | EUR 39.5454545455"]
    );
    assert_eq!(
        of("a [X]", lines(p, "USD", None)),
        ["a [X] | 17 | EUR 20, USD 5, X 5 | USD 56.1 | USD 43.5"]
    );
}

#[test]
fn units_in_the_valuation_commodity_stay_and_the_cost_is_valued_into_it() {
    assert_eq!(
        of("a [X]", lines(probe!("hold-cost"), "X", None)),
        ["a [X] | 17 | EUR 20, USD 5, X 5 | X 17 | X 13.1818181818"]
    );
}

#[test]
fn a_cost_in_the_valuation_commodity_leaves_it_and_comes_back_at_the_price() {
    assert_eq!(
        of("a ", lines(probe!("hold-cost-self"), "EUR", None)),
        ["a [EUR] | 10 | USD 12 | EUR 10 | EUR 10.8"]
    );
}

#[test]
fn what_has_no_path_of_prices_stays_where_it_is() {
    assert_eq!(
        of("a ", lines(probe!("hold-noprice"), "EUR", None)),
        [
            "a [W] | 4 | W 4 | W 4 | W 4",
            "a [X] | 12 | USD 20, X 2 | EUR 36 | EUR 6, USD 20"
        ]
    );
}

#[test]
fn conversion_postings_are_dropped_from_the_cost_but_not_from_units_or_value() {
    assert_eq!(
        lines(probe!("hold-cost-conversion"), "EUR", None),
        [
            "assets:eur [EUR] | -100 | EUR -100 | EUR -100 | EUR -100",
            "assets:usd [USD] | 110 | USD 110 | EUR 99 | EUR 99",
            "equity:conversion [EUR] | 100 | - | EUR 100 | -",
            "equity:conversion [USD] | -110 | - | EUR -99 | -"
        ]
    );
}

// ───────────── which rows exist ─────────────

#[test]
fn holdings_are_ordered_by_account_names_then_commodity_in_code_point_order() {
    let accounts = lines(probe!("hold-tidy-accounts"), "EUR", None);
    let names: Vec<&str> = accounts
        .iter()
        .map(|l| l.split(" [").next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "10:x",
            "9:x",
            "Alpha",
            "Zeta",
            "a:B",
            "a:b",
            "a b",
            "a-b",
            "a_b",
            "alpha",
            "parent",
            "parent:child",
            "zz",
            "Émile",
            "é"
        ]
    );
    let commodities: Vec<String> = of("a [", lines(probe!("hold-tidy-commodities"), "EUR", None))
        .iter()
        .map(|l| l["a [".len()..l.find(']').unwrap()].to_string())
        .collect();
    assert_eq!(
        commodities,
        [
            "", "$", "10X", "2X", "9X", "A", "A B", "A-B", "A.B", "A_B", "E F", "EUR", "X", "Z",
            "a", "x", "é", "€"
        ]
    );
}

#[test]
fn units_that_cancel_leave_no_holding_unless_a_cost_remains() {
    let p = probe!("hold-tidy-zero");
    let all = || lines(p, "EUR", None);
    assert_eq!(of("zeromix", all()), ["zeromix [Y] | 3 | Y 3 | Y 3 | Y 3"]);
    assert_eq!(of("unitzero", all()), Vec::<String>::new());
    // 10 X at 2 EUR and -10 X at 3 EUR: no units, but the cost is -10 EUR.
    assert_eq!(
        of("twocosts", all()),
        ["twocosts [X] | 0 | EUR -10 | - | EUR -10"]
    );
}

#[test]
fn a_cost_that_cancels_a_plain_amount_is_per_commodity() {
    let p = probe!("hold-tidy-zero");
    assert_eq!(
        of("costzero", lines(p, "EUR", None)),
        [
            "costzero [EUR] | -20 | EUR -20 | EUR -20 | EUR -20",
            "costzero [X] | 10 | EUR 20 | EUR 20 | EUR 20"
        ]
    );
    assert_eq!(
        of("costhalf", lines(p, "EUR", None)),
        [
            "costhalf [EUR] | -15 | EUR -15 | EUR -15 | EUR -15",
            "costhalf [X] | 10 | EUR 20 | EUR 20 | EUR 20"
        ]
    );
}

#[test]
fn a_price_of_zero_values_to_nothing() {
    assert_eq!(
        of("zeroprice", lines(probe!("hold-tidy-zero"), "EUR", None)),
        ["zeroprice [Z] | 5 | Z 5 | - | -"]
    );
}

#[test]
fn a_sum_that_rounds_to_zero_keeps_its_zero_with_ten_places() {
    let p = probe!("hold-tidy-tiny");
    assert_eq!(
        of("tiny", lines(p, "EUR", None)),
        ["tiny [X] | 0.0000000001 | X 0.0000000001 | EUR 0.0000000000 | EUR 0.0000000000"]
    );
    assert_eq!(
        of("big", lines(p, "EUR", None)),
        ["big [X] | 1000.00 | X 1000.00 | EUR 500 | EUR 500"]
    );
    assert_eq!(
        of("sink", lines(p, "EUR", None)),
        [
            "sink [X] | -1000.0000000001 | X -1000.0000000001 | EUR -500.0000000000 | EUR -500.0000000000"
        ]
    );
}

#[test]
fn amounts_without_a_commodity_are_held_in_the_empty_commodity() {
    assert_eq!(
        lines(probe!("hold-cur-bare"), "EUR", None),
        [
            "a [] | 5 |  5 |  5 |  5",
            "b [X] | 3 | X 3 | X 3 | X 3",
            "c [X] | -3 | X -3 | X -3 | X -3",
            "d [] | -5 |  -5 |  -5 |  -5"
        ]
    );
}

// ───────────── what is refused ─────────────

fn code_and_construct(finding: &ledgermill::Finding) -> (String, serde_json::Value) {
    (finding.code.clone(), finding.details.clone())
}

#[test]
fn a_transaction_cost_cannot_convert_is_refused() {
    let finding = refusal(&[probe!("conversion-several")], "EUR", None);
    assert_eq!(
        code_and_construct(&finding),
        (
            "unsupported-syntax".into(),
            json!({ "construct": "cost-conversion-postings" })
        )
    );
    assert_eq!(finding.file, "conversion-several.journal");
}

#[test]
fn valuing_in_no_commodity_is_refused() {
    let finding = refusal(&[probe!("hold-cost")], "", None);
    assert_eq!(
        code_and_construct(&finding),
        (
            "unsupported-syntax".into(),
            json!({ "construct": "value-without-commodity" })
        )
    );
}

#[test]
fn a_beancount_ledger_is_refused() {
    let finding = refusal(
        &[(
            "t.beancount",
            "2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n",
        )],
        "EUR",
        None,
    );
    assert_eq!(
        code_and_construct(&finding),
        (
            "unsupported-syntax".into(),
            json!({ "construct": "beancount-valuation" })
        )
    );
}

#[test]
fn a_date_that_is_not_a_date_is_an_invalid_argument() {
    let p = probe!("hold-cost");
    for end in ["2026-02-30", "2026-1-1", "tomorrow", ""] {
        let finding = refusal(&[p], "EUR", Some(end));
        assert_eq!(
            code_and_construct(&finding),
            (
                "invalid-argument".into(),
                json!({ "argument": "end", "value": end })
            )
        );
    }
    let finding = run(&[p], "EUR", None, "2026/10/23").unwrap_err();
    assert_eq!(
        code_and_construct(&finding),
        (
            "invalid-argument".into(),
            json!({ "argument": "today", "value": "2026/10/23" })
        )
    );
}
