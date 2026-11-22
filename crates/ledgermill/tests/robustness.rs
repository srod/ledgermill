//! `load` never panics: the core runs in browsers and apps, where a panic is a crash. Lines are
//! built from awkward pieces (multi-byte blanks, lone marks, brackets) in every position.
// Integration tests are compiled outside `cfg(test)`; a panic here is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use ledgermill::{HoldingsQuery, format, load};
use support::file;

fn query(commodity: &str) -> HoldingsQuery {
    HoldingsQuery {
        commodity: commodity.into(),
        end: None,
        today: "2026-10-25".into(),
    }
}

const PIECES: [&str; 24] = [
    "\u{a0}",
    "\u{a0}\u{a0}",
    "\u{2003}",
    "\t",
    "  ",
    " ",
    ";",
    "=",
    "@",
    "@@",
    "{",
    "}",
    "{{",
    "[",
    "]",
    "(",
    ")",
    "\"",
    "1,000.5",
    "-",
    "é",
    "2026-01-02",
    "type:",
    "e",
];

#[test]
fn awkward_lines_never_panic() {
    let prefixes = [
        "account a",
        "commodity ",
        "D ",
        "P 2026-01-01 X ",
        "alias ",
        "alias /a/ = ",
        "Y ",
        "apply account ",
        "decimal-mark ",
        "2026-01-01 ",
        "  a",
        "  a  1 X",
        "  a  ",
    ];
    for prefix in prefixes {
        for a in PIECES {
            for b in PIECES {
                let line = format!("{prefix}{a}{b}");
                let journal = format!("{line}\n2026-01-01 t\n  x  1\n{line}\n  y\n");
                // Only the absence of a panic is checked here.
                let _ = load(&[file("r.journal", &journal)]);
            }
        }
    }
}

#[test]
fn two_no_break_spaces_end_an_account_name_like_hledger() {
    let journal = "account a\u{a0}\u{a0}b\n";
    let ledger = load(&[file("r.journal", journal)]);
    assert_eq!(ledger.check()[0].code, "parse-error");
}

#[test]
fn awkward_beancount_lines_and_downstream_calls_never_panic() {
    let prefixes = [
        "2026-01-01 open Assets:A ",
        "2026-01-01 * ",
        "2026-01-01 balance Assets:A ",
        "2026-01-01 price EUR ",
        "option ",
        "plugin ",
        "  Assets:A  ",
        "  Assets:A  1 EUR ",
    ];
    for prefix in prefixes {
        for a in PIECES {
            let line = format!("{prefix}{a}");
            let content = format!(
                "2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n{line}\n2026-01-02 * \"t\"\n  Assets:A  1 EUR\n  Equity:B\n"
            );
            let src = file("r.beancount", &content);
            let ledger = load(std::slice::from_ref(&src));
            let _ = ledger.transactions();
            let _ = ledger.postings();
            let _ = ledger.prices();
            let _ = ledger.check();
            let _ = ledger.at_cost();
            let _ = ledger.value("2026-01-02", "EUR");
            let _ = ledger.holdings(&query("EUR"));
            let _ = format(&src);
        }
    }
}

#[test]
fn chained_regex_aliases_and_large_nfas_are_refused_without_oom() {
    let mut doubling = String::new();
    for _ in 0..32 {
        doubling.push_str("alias /a/ = aa\n");
    }
    doubling.push_str("2026-01-01 t\n  a  1\n  b  -1\n");
    let ledger = load(&[file("bomb.journal", &doubling)]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(ledger.check()[0].details["construct"], "alias-too-large");

    let mut backrefs = String::new();
    for _ in 0..10 {
        backrefs.push_str("alias /.*/ = \\0\\0\\0\\0\\0\\0\\0\\0\\0\\0\n");
    }
    backrefs.push_str("2026-01-01 t\n  a  1\n  b  -1\n");
    let ledger = load(&[file("backref.journal", &backrefs)]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(ledger.check()[0].details["construct"], "alias-too-large");

    let huge_nfa = "alias /a{100000}/ = x\n2026-01-01 t\n  a  1\n  b  -1\n";
    let ledger = load(&[file("nfa.journal", huge_nfa)]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(ledger.check()[0].details["construct"], "alias-too-large");
}

#[test]
fn regex_aliases_that_do_too_much_work_for_a_small_journal_are_refused() {
    // Each limit above holds here: 255 regex aliases, one account name of at most 32 KiB, and
    // every name compiled small. Yet the 243 `x` aliases each scan the 32 KiB name the twelve
    // doublings build from 8 bytes, for every posting: about 8 MB of scanning per posting, from a
    // 3.7 KB journal.
    let mut journal = String::new();
    for _ in 0..243 {
        journal.push_str("alias /x/ = x\n");
    }
    for _ in 0..12 {
        journal.push_str("alias /^(.*)$/ = \\1\\1\n");
    }
    journal.push_str("2026-01-01 t\n    xxxxxxxx  1\n    b  -1\n");
    let ledger = load(&[file("work.journal", &journal)]);
    let findings = ledger.check();
    assert!(!findings.is_empty(), "the journal was loaded in full");
    assert_eq!(findings[0].code, "unsupported-syntax");
    assert_eq!(findings[0].details["construct"], "alias-too-large");
}

#[test]
fn plain_aliases_that_do_too_much_work_for_a_small_journal_are_refused() {
    // Plain aliases are no cheaper: each one that does not match still has to look at a 60,000-byte
    // name, once per alias and posting. 2,000 of them and five postings are 600 million bytes of
    // looking, from a journal of 80 KB.
    let mut journal = String::new();
    for i in 0..2_000 {
        journal.push_str(&format!("alias z{i} = y\n"));
    }
    journal.push_str(&format!("alias a = {}\n", "x".repeat(60_000)));
    journal.push_str("2026-01-01 t\n");
    for _ in 0..5 {
        journal.push_str("    a  1\n");
    }
    journal.push_str("    b\n");
    let ledger = load(&[file("plain.journal", &journal)]);
    let findings = ledger.check();
    assert!(!findings.is_empty(), "the journal was loaded in full");
    assert_eq!(findings[0].code, "unsupported-syntax");
    assert_eq!(findings[0].details["construct"], "alias-too-large");
}

#[test]
fn many_aliases_load_on_a_journal_that_names_few_accounts_often() {
    // What a real journal looks like: 150 aliases (100 regex, 50 plain) and 8,000 postings (300 KB)
    // that name only 500 accounts and the bank. The aliases need to run once per account, not once
    // per posting, so nothing here may be refused.
    let mut journal = String::new();
    for i in 0..99 {
        journal.push_str(&format!("alias /^wallet{i}:/ = assets:wallet{i}:\n"));
    }
    for i in 0..49 {
        journal.push_str(&format!("alias old{i} = archive:old{i}\n"));
    }
    journal.push_str("alias assets:bank = assets:checking\n");
    journal.push_str("alias /^expenses:/ = exp:\n");
    for n in 0..4_000 {
        let k = n % 500;
        journal.push_str(&format!(
            "2026-01-01 t\n    expenses:cat{}:item{k}  1\n    assets:bank\n",
            k % 20
        ));
    }
    let ledger = load(&[file("big.journal", &journal)]);
    assert!(ledger.check().is_empty(), "{:?}", ledger.check());
    let accounts: Vec<&str> = ledger
        .postings()
        .iter()
        .map(|p| p.account.as_str())
        .collect();
    assert!(accounts.contains(&"exp:cat3:item3"));
    assert!(accounts.contains(&"assets:checking"));
}

#[test]
fn format_alignment_bombs_return_unchanged_without_oom() {
    // 100k-char account + 500 short postings would expand to ~50 MB of spaces (over the 16 MB budget).
    let long = "a".repeat(100_000);
    let mut hledger_bomb = format!("2026-01-01 bomb\n  {long}  1\n");
    for _ in 0..500 {
        hledger_bomb.push_str("  b  1\n");
    }
    hledger_bomb.push_str("  c\n");
    assert_eq!(format(&file("bomb.journal", &hledger_bomb)), hledger_bomb);

    let long_bc = format!("Assets:{}", "A".repeat(100_000));
    let mut bc_bomb = format!(
        "2026-01-01 open {long_bc}\n2026-01-01 open Assets:B\n2026-01-02 * \"bomb\"\n  {long_bc}  1 EUR\n"
    );
    for _ in 0..500 {
        bc_bomb.push_str("  Assets:B  1 EUR\n");
    }
    bc_bomb.push_str("  Assets:B\n");
    assert_eq!(format(&file("bomb.beancount", &bc_bomb)), bc_bomb);
}

#[test]
fn extreme_price_chains_and_huge_integer_literals_are_refused() {
    // 20 chained `1e4096` prices would exceed the 65,536-bit cap (~19,700 decimal digits).
    let names: Vec<String> = (0u8..=20)
        .map(|i| format!("C{}", char::from(b'A' + i)))
        .collect();
    let mut chain = String::new();
    for i in 0..20 {
        chain.push_str(&format!(
            "P 2026-01-01 {} 1e4096 {}\n",
            names[i],
            names[i + 1]
        ));
    }
    chain.push_str(&format!("2026-01-01 t\n  a  1 {}\n  b\n", names[0]));
    let ledger = load(&[file("chain.journal", &chain)]);
    assert!(ledger.check().is_empty(), "{:?}", ledger.check());
    let err = ledger.value("2026-01-01", &names[20]).unwrap_err();
    assert_eq!(err.code, "unsupported-syntax");
    assert_eq!(err.details["construct"], "number-too-large");
    let err = ledger.holdings(&query(&names[20])).unwrap_err();
    assert_eq!(err.code, "unsupported-syntax");
    assert_eq!(err.details["construct"], "number-too-large");

    let digits = "9".repeat(5_000);
    let huge_int = format!("2026-01-01 t\n  a  {digits}\n  b\n");
    let ledger = load(&[file("big.journal", &huge_int)]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(ledger.check()[0].details["construct"], "number-too-large");
}

#[test]
fn beancount_extreme_exponents_do_not_oom() {
    // 2,000 chained tiny factors produce exponent -40,000; adding 1 and quantizing stays O(1).
    let mut expr = String::from("(1 + (0.00000000000000000001");
    for _ in 0..2_000 {
        expr.push_str(" * 0.00000000000000000001");
    }
    expr.push_str("))");
    let journal = format!(
        "2026-01-01 open Assets:A\n2026-01-01 open Equity:B\n2026-01-02 * \"tiny\"\n  Assets:A  {expr} EUR\n  Equity:B\n"
    );
    let ledger = load(&[file("tiny.beancount", &journal)]);
    assert!(ledger.check().is_empty(), "{:?}", ledger.check());
    assert_eq!(
        ledger.transactions()[0].postings[0].amounts[0].quantity,
        "1.000000000000000000000000000"
    );
}

#[test]
fn beancount_repeated_implicit_prices_do_not_oom() {
    // 5,000 implicit_prices plugin lines × 250 priced transactions = 1,250,000 entries -> refused.
    let mut repeated = String::new();
    for _ in 0..5_000 {
        repeated.push_str("plugin \"beancount.plugins.implicit_prices\"\n");
    }
    repeated.push_str("2026-01-01 open Assets:A\n2026-01-01 open Assets:B\n");
    for i in 0..250 {
        repeated.push_str(&format!(
            "2026-01-02 * \"t{i}\"\n  Assets:A  1 C{i} @ 2 EUR\n  Assets:B\n"
        ));
    }
    let ledger = load(&[file("rep.beancount", &repeated)]);
    assert_eq!(ledger.check()[0].code, "unsupported-syntax");
    assert_eq!(
        ledger.check()[0].details["construct"],
        "implicit-prices-limit"
    );
}

#[test]
fn implicit_prices_deduplicates_zero_rates_with_positive_exponent() {
    let journal = concat!(
        "plugin \"beancount.plugins.implicit_prices\"\n",
        "2026-01-01 open Assets:A\n",
        "2026-01-01 open Assets:B\n",
        "2026-01-02 * \"t1\"\n",
        "  Assets:A  0.1 BTC @@ 0 EUR\n",
        "  Assets:B  0 EUR\n",
        "2026-01-02 * \"t2\"\n",
        "  Assets:A  1 BTC @ 0 EUR\n",
        "  Assets:B  0 EUR\n",
    );
    let ledger = load(&[file("zero.beancount", journal)]);
    assert!(ledger.check().is_empty());
    assert_eq!(ledger.prices().len(), 1);
}
