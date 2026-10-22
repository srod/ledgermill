//! Test helpers: everything goes through `load` -> `transactions()` / `check()`.
#![allow(dead_code)]

use ledgermill::{SourceFile, load};
use serde_json::Value;

pub fn file(name: &str, content: &str) -> SourceFile {
    SourceFile {
        name: name.into(),
        content: content.into(),
    }
}

pub fn transactions(name: &str, content: &str) -> Value {
    serde_json::to_value(load(&[file(name, content)]).transactions()).unwrap()
}

pub fn findings(name: &str, content: &str) -> Value {
    serde_json::to_value(load(&[file(name, content)]).check()).unwrap()
}

/// Per transaction: its source line and `[account, commodity, quantity]` for each posting amount.
pub fn amounts(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = load(&[file(name, content)])
        .transactions()
        .iter()
        .map(|t| {
            let postings: Vec<Value> = t
                .postings
                .iter()
                .flat_map(|p| {
                    p.amounts
                        .iter()
                        .map(move |a| serde_json::json!([p.account, a.commodity, a.quantity]))
                })
                .collect();
            serde_json::json!([t.source.line, postings])
        })
        .collect();
    Value::Array(rows)
}

/// Per transaction: `[line, status, description, comment, [posting comments]]`.
pub fn headers(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = load(&[file(name, content)])
        .transactions()
        .iter()
        .map(|t| {
            let status = serde_json::to_value(t.status).unwrap();
            let posting_comments: Vec<&str> =
                t.postings.iter().map(|p| p.comment.as_str()).collect();
            serde_json::json!([
                t.source.line,
                status,
                t.description,
                t.comment,
                posting_comments
            ])
        })
        .collect();
    Value::Array(rows)
}

/// Beancount: per transaction `[date, flag, payee, narration, line, postings]`, each posting
/// `[account, quantity, commodity, line, price, lot]` where `price` is `"100.50 EUR"` or null and
/// `lot` is `"10 EUR 2026-01-02 label"` or null.
pub fn beancount_rows(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = load(&[file(name, content)])
        .transactions()
        .iter()
        .map(|t| {
            let postings: Vec<Value> = t
                .postings
                .iter()
                .map(|p| {
                    let a = &p.amounts[0];
                    let price = a.cost.as_ref().map(|c| match c {
                        ledgermill::Cost::Unit { amount } | ledgermill::Cost::Total { amount } => {
                            format!("{} {}", amount.quantity, amount.commodity)
                        }
                    });
                    let lot = a.lot.as_ref().map(|l| {
                        let cost = l.cost.as_ref().unwrap();
                        let mut text = format!(
                            "{} {} {}",
                            cost.quantity,
                            cost.commodity,
                            l.date.as_ref().unwrap()
                        );
                        if let Some(label) = &l.label {
                            text.push(' ');
                            text.push_str(label);
                        }
                        text
                    });
                    serde_json::json!([p.account, a.quantity, a.commodity, p.line, price, lot])
                })
                .collect();
            serde_json::json!([
                t.date,
                t.flag,
                t.payee,
                t.description,
                t.source.line,
                postings
            ])
        })
        .collect();
    Value::Array(rows)
}

/// `[line, code, construct-or-reason]` of each finding.
pub fn short_findings(name: &str, content: &str) -> Value {
    let rows: Vec<Value> = findings(name, content)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let what = f["details"]
                .get("construct")
                .or_else(|| f["details"].get("reason"));
            serde_json::json!([f["line"], f["code"], what])
        })
        .collect();
    Value::Array(rows)
}

fn quantity_text(commodity: &str, quantity: &str) -> String {
    format!("{commodity} {quantity}")
}

/// One amount in the short form of `spec/hledger/show-amounts.sh`: `COMMODITY QUANTITY`, then
/// ` @ AMOUNT` or ` @@ AMOUNT` for a cost and ` {AMOUNT}` for a lot (`{}` when it has no cost).
pub fn short_amount(a: &ledgermill::Amount) -> String {
    let mut text = quantity_text(&a.commodity, &a.quantity);
    match &a.cost {
        Some(ledgermill::Cost::Unit { amount }) => {
            text += &format!(" @ {}", quantity_text(&amount.commodity, &amount.quantity));
        }
        Some(ledgermill::Cost::Total { amount }) => {
            text += &format!(" @@ {}", quantity_text(&amount.commodity, &amount.quantity));
        }
        None => {}
    }
    if let Some(lot) = &a.lot {
        let cost = lot
            .cost
            .as_ref()
            .map(|c| quantity_text(&c.commodity, &c.quantity))
            .unwrap_or_default();
        text += &format!(" {{{cost}}}");
    }
    text
}

/// Transactions in the short form of `spec/hledger/show-amounts.sh`, one line each:
/// `DATE  ACCOUNT: AMOUNT, AMOUNT | ACCOUNT: -`.
pub fn short_lines(transactions: &[ledgermill::Transaction]) -> Vec<String> {
    transactions
        .iter()
        .map(|t| {
            let postings: Vec<String> = t
                .postings
                .iter()
                .map(|p| {
                    let amounts: Vec<String> = p.amounts.iter().map(short_amount).collect();
                    let amounts = if amounts.is_empty() {
                        "-".to_string()
                    } else {
                        amounts.join(", ")
                    };
                    format!("{}: {amounts}", p.account)
                })
                .collect();
            format!("{}  {}", t.date, postings.join(" | "))
        })
        .collect()
}

/// `load(files).value(date, commodity)` in the short form; panics on an `Err`.
pub fn valued(files: &[(&str, &str)], date: &str, commodity: &str) -> Vec<String> {
    let files: Vec<SourceFile> = files.iter().map(|(n, c)| file(n, c)).collect();
    short_lines(&load(&files).value(date, commodity).unwrap())
}

/// `load(files).at_cost()` in the short form; panics on an `Err`.
pub fn at_cost(files: &[(&str, &str)]) -> Vec<String> {
    let files: Vec<SourceFile> = files.iter().map(|(n, c)| file(n, c)).collect();
    short_lines(&load(&files).at_cost().unwrap())
}
