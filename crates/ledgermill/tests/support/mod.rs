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
