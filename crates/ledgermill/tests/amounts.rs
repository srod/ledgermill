//! Amount forms and decimal places. Expected values: hledger 1.52.4 on the named probes
//! (mantissa and places from `show.sh`, written here as exact decimal strings).

mod support;

use serde_json::json;
use support::*;

#[test]
fn amount_forms_give_commodity_and_quantity() {
    let probe = include_str!("../../../spec/hledger/probes/amount-forms.journal");
    assert_eq!(
        amounts("amount-forms.journal", probe),
        json!([
            [1, [["a", "", "3"], ["b", "", "-3"]]],
            [5, [["a", "$", "3.50"], ["b", "$", "-3.50"]]],
            [9, [["a", "$", "-3.50"], ["b", "$", "3.50"]]],
            [13, [["a", "EUR", "3.50"], ["b", "EUR", "-3.50"]]],
            [17, [["a", "EUR", "3.50"], ["b", "EUR", "-3.50"]]],
            [21, [["a", "EUR", "3.50"], ["b", "EUR", "-3.50"]]],
            [25, [["a", "EUR", "-3.50"], ["b", "EUR", "3.50"]]],
            [29, [["a", "€", "3.50"], ["b", "€", "-3.50"]]],
            [33, [["a", "руб", "3"], ["b", "руб", "-3"]]],
            [37, [["a", "元", "1"], ["b", "元", "-1"]]],
            [41, [["a", "eur", "3"], ["b", "eur", "-3"]]],
            [45, [["a", "E", "2"], ["b", "E", "-2"]]]
        ])
    );
    assert_eq!(findings("amount-forms.journal", probe), json!([]));
}

#[test]
fn quantities_keep_the_places_as_written() {
    let probe = include_str!("../../../spec/hledger/probes/decimal-places.journal");
    assert_eq!(
        amounts("decimal-places.journal", probe),
        json!([
            [1, [["a", "", "0.30"], ["b", "", "0.7"], ["c", "", "-1"]]],
            [6, [["a", "", "0.00"], ["b", "", "0"]]],
            [10, [["a", "EUR", "0.000"], ["b", "EUR", "0"]]],
            [
                14,
                [
                    ["a", "EUR", "1234567.8901234"],
                    ["b", "EUR", "-1234567.8901234"]
                ]
            ]
        ])
    );
    assert_eq!(findings("decimal-places.journal", probe), json!([]));
}
