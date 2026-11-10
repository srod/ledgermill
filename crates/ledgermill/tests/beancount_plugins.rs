//! Beancount `plugin` lines: `beancount.plugins.auto_accounts` opens every account that is used
//! (spec/beancount/auto-accounts.md); every other plugin is refused by name
//! (spec/beancount/refused.md).
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::load;
use serde_json::{Value, json};
use support::*;

/// `[line, code]` of each error, in order.
fn errors(journal: &str) -> Value {
    Value::Array(
        findings("p.beancount", journal)
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["severity"] == json!("error"))
            .map(|f| json!([f["line"], f["code"]]))
            .collect(),
    )
}

#[test]
fn a_journal_with_no_open_is_valid_with_the_plugin() {
    let journal = probe("auto-accounts");
    assert_eq!(findings("p.beancount", &journal), json!([]));
    assert_eq!(
        load(&[file("p.beancount", &journal)]).transactions().len(),
        1
    );
    // The same journal without the plugin line: both accounts are unknown.
    let without = journal.replace("plugin \"beancount.plugins.auto_accounts\"\n", "");
    assert_eq!(
        errors(&without),
        json!([[2, "account-not-open"], [2, "account-not-open"]]),
        "one finding per account"
    );
}

#[test]
fn every_directive_that_names_an_account_opens_it() {
    let journal = probe("auto-accounts-directives");
    assert_eq!(errors(&journal), json!([]));
    // The padding transaction is made as usual.
    let loaded = load(&[file("p.beancount", &journal)]);
    let flags: Vec<_> = loaded
        .transactions()
        .iter()
        .map(|t| t.flag.clone().unwrap_or_default())
        .collect();
    assert_eq!(flags, ["P", "*"]);
}

#[test]
fn the_plugin_line_may_stand_anywhere_and_be_given_twice() {
    for name in [
        "auto-accounts-mixed",
        "auto-accounts-twice",
        "auto-accounts-first-use",
        "auto-accounts-close-ok",
    ] {
        assert_eq!(errors(&probe(name)), json!([]), "{name}");
    }
}

#[test]
fn an_open_dated_after_the_use_is_not_replaced() {
    assert_eq!(
        errors(&probe("auto-accounts-open-after-use")),
        json!([[5, "account-not-open"]])
    );
    assert_eq!(
        findings("p.beancount", &probe("auto-accounts-open-after-use"))[0]["details"],
        json!({ "account": "Assets:B", "reason": "not-yet-open" })
    );
}

#[test]
fn an_account_used_after_its_close_is_still_an_error() {
    let found = findings("p.beancount", &probe("auto-accounts-use-after-close"));
    let short: Vec<Value> = found
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!([f["line"], f["code"], f["details"].get("reason")]))
        .collect();
    assert_eq!(
        short,
        [
            json!([8, "duplicate-close", null]),
            json!([10, "account-not-open", "closed"]),
            json!([15, "account-not-open", "closed"]),
        ]
    );
}

#[test]
fn an_explicit_open_keeps_its_currencies_and_an_auto_one_has_none() {
    assert_eq!(
        errors(&probe("auto-accounts-currencies")),
        json!([
            [11, "currency-not-allowed"],
            [5, "duplicate-open"],
            [7, "currency-not-allowed"]
        ])
    );
}

#[test]
fn the_other_errors_keep_their_order() {
    assert_eq!(
        errors(&probe("auto-accounts-errors-order")),
        json!([
            [31, "booking-error"],
            [26, "unused-pad"],
            [23, "assertion-failed"],
            [3, "duplicate-open"],
            [19, "account-not-open"],
            [15, "currency-not-allowed"],
            [9, "duplicate-commodity"],
            [11, "unbalanced-transaction"],
        ])
    );
}

#[test]
fn a_transaction_booking_rejects_gives_its_booking_error_and_no_other() {
    // The `open` dates the plugin picks are not output, so which accounts the rejected
    // transaction opens cannot be told from here (auto-accounts.md).
    assert_eq!(
        errors(&probe("auto-accounts-booking-error")),
        json!([[5, "booking-error"]])
    );
}

#[test]
fn the_plugin_is_for_its_own_file_only() {
    let with = probe("auto-accounts");
    let without = "2026-01-02 * \"no open\"\n  Assets:C  1 EUR\n  Assets:D\n";
    let loaded = load(&[file("a.beancount", &with), file("b.beancount", without)]);
    let found: Vec<(&str, &str)> = loaded
        .check()
        .iter()
        .map(|f| (f.file.as_str(), f.code.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("b.beancount", "account-not-open"),
            ("b.beancount", "account-not-open")
        ]
    );
}

/// The refusals of a journal: `[line, construct, name]`.
fn refusals(journal: &str) -> Value {
    Value::Array(
        findings("p.beancount", journal)
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["code"] == json!("unsupported-syntax"))
            .map(|f| json!([f["line"], f["details"]["construct"], f["details"]["name"]]))
            .collect(),
    )
}

#[test]
fn any_other_plugin_is_refused_by_its_module_name() {
    for (name, line) in [
        ("plugin-check-commodity", 1),
        ("plugin-auto-accounts-config", 1),
        ("plugin-implicit-prices-config", 1),
        ("plugin-unknown-module", 1),
        ("plugin-escaped-name", 1),
        ("plugin-escaped-control", 1),
    ] {
        let journal = probe(&format!("refused/{name}"));
        let name_written = journal.lines().next().unwrap().split('"').nth(1).unwrap();
        assert_eq!(
            refusals(&journal),
            json!([[line, "plugin", name_written]]),
            "{name}"
        );
        assert!(
            load(&[file("p.beancount", &journal)])
                .transactions()
                .is_empty(),
            "{name}: no transaction when a plugin is refused"
        );
    }
}

#[test]
fn a_refused_plugin_next_to_auto_accounts_still_refuses_the_file() {
    let journal = "plugin \"beancount.plugins.auto_accounts\"\nplugin \"beancount.plugins.check_commodity\" ; why not\n\n2026-01-02 * \"x\"\n  Assets:A  1 EUR\n  Assets:B\n";
    assert_eq!(
        refusals(journal),
        json!([[2, "plugin", "beancount.plugins.check_commodity"]])
    );
    assert!(
        load(&[file("p.beancount", journal)])
            .transactions()
            .is_empty()
    );
}

#[test]
fn forms_of_the_plugin_line_the_reference_reads() {
    for line in [
        "plugin\t\"beancount.plugins.leafonly\"",
        "plugin\"beancount.plugins.leafonly\"",
        "plugin \"beancount.plugins.leafonly\"   ; a comment",
        "plugin \"beancount.plugins.leafonly\" \"config\"",
    ] {
        assert_eq!(
            refusals(line),
            json!([[1, "plugin", "beancount.plugins.leafonly"]]),
            "{line}"
        );
    }
    // A keyword with no name is refused too, without one.
    assert_eq!(refusals("plugin\n"), json!([[1, "plugin", null]]));
}

#[test]
fn a_plugin_line_the_reference_rejects_is_a_parse_error() {
    for line in [
        "plugin beancount.plugins.auto_accounts",
        "plugin \"beancount.plugins.auto_accounts\" 3",
        "plugin \"a\" \"b\" \"c\"",
        "plugin \"beancount.plugins.auto_accounts\"x",
    ] {
        let found = findings("p.beancount", &format!("{line}\n"));
        assert_eq!(found[0]["code"], json!("parse-error"), "{line}");
        assert_eq!(found[0]["line"], json!(1), "{line}");
    }
}

#[test]
fn a_module_name_over_two_lines_is_refused_as_written() {
    // The reference fails to import a module with a line feed in its name.
    let journal = "plugin \"beancount.plugins.auto_accounts\n\"\n";
    assert_eq!(
        refusals(journal),
        json!([[1, "plugin", "beancount.plugins.auto_accounts\n"]])
    );
}

#[test]
fn a_configuration_string_may_follow_the_module_with_no_blank() {
    assert_eq!(
        refusals("plugin \"beancount.plugins.auto_accounts\"\"x\"\n"),
        json!([[1, "plugin", "beancount.plugins.auto_accounts"]])
    );
}
