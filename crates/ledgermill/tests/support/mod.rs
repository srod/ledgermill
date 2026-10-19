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
