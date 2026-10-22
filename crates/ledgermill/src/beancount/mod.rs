//! Beancount: reads one file, books its transactions and reports them in our model.
//!
//! The parser and the lot matching come from two forks (see `lima.rs`, the only module that
//! names their types). The figures are derived here, following the spec notes in
//! `spec/beancount/`.
//!
//! Anything that could change which transactions or postings come out and is not built yet is
//! refused with `unsupported-syntax`, and then no transaction is returned: a partial answer would
//! look complete.

mod book;
mod lima;
mod plain;
mod pydec;
mod text;

use crate::model::{
    Amount, Cost, Finding, FlatPosting, Lot, Posting, PostingKind, Quantity, Severity, Source,
    SourceFile, Status, Transaction,
};
use plain::{BookFailure, Item, Method};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// Files ending in `.beancount` or `.bean` are Beancount.
pub(crate) fn is_beancount(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".beancount") || lower.ends_with(".bean")
}

pub(crate) struct Loaded {
    /// By date, then source line.
    pub transactions: Vec<Transaction>,
    /// By line.
    pub findings: Vec<Finding>,
}

fn finding(
    file: &str,
    line: u32,
    code: &str,
    details: Value,
    message: impl Into<String>,
) -> Finding {
    Finding {
        code: code.into(),
        severity: Severity::Error,
        file: file.into(),
        line,
        column: None,
        details,
        message: message.into(),
    }
}

fn unsupported(file: &str, line: u32, construct: &str, extra: Value, message: &str) -> Finding {
    let mut details = json!({ "construct": construct });
    if let (Some(map), Some(extra)) = (details.as_object_mut(), extra.as_object()) {
        for (k, v) in extra {
            map.insert(k.clone(), v.clone());
        }
    }
    finding(file, line, "unsupported-syntax", details, message)
}

fn refusal_message(construct: &str) -> &'static str {
    match construct {
        "include" => "Including other files is not supported.",
        "plugin" => "Plugins are not supported.",
        "option" => "This option is not supported yet.",
        "pad" => "Pad directives are not supported yet.",
        "balance" => "Balance assertions are not supported yet.",
        "number-expression" => "Arithmetic in a number is not supported yet.",
        "number-too-large" => "This number is too large to handle.",
        "inferred-price" => "A price to work out next to a cost is not supported.",
        "cost-partial" => "A cost with one part of `#` left out is not supported yet.",
        "cost-merge" => "Merging lots with {*} is not supported.",
        "inferred-cost" => "A lot cost to work out is not supported in this form.",
        "average-booking" => "AVERAGE booking is not supported.",
        "number-too-precise" => "This number needs more digits than are handled.",
        _ => "This is not supported yet.",
    }
}

fn status_of(flag: char) -> Status {
    match flag {
        '*' => Status::Cleared,
        '!' => Status::Pending,
        _ => Status::Unmarked,
    }
}

fn quantity(commodity: &str, value: &pydec::Num) -> Quantity {
    Quantity {
        commodity: commodity.to_string(),
        quantity: value.to_string(),
    }
}

fn posting(p: &book::BookedPosting) -> Posting {
    Posting {
        account: p.account.clone(),
        kind: PostingKind::Regular,
        comment: String::new(),
        amounts: vec![Amount {
            commodity: p.currency.clone(),
            quantity: p.units.to_string(),
            cost: p.price.as_ref().map(|(value, currency)| Cost::Unit {
                amount: quantity(currency, value),
            }),
            lot: p.lot.as_ref().map(|lot| Lot {
                cost: Some(quantity(&lot.currency, &lot.per_unit)),
                date: Some(lot.date.to_string()),
                label: lot.label.clone(),
            }),
        }],
        flag: p.flag.map(|c| c.to_string()),
        line: Some(p.line),
    }
}

/// The flat posting list of booked transactions: sorted by date and line like the reference's
/// `ORDER BY date, filename, lineno`. A transaction lists its postings grouped by currency, so the
/// line sort puts them back in the order written; rows tied on a line (a posting split over lots)
/// keep booking order because the sort is stable.
pub(crate) fn flatten(transactions: &[Transaction]) -> Vec<FlatPosting> {
    let mut rows: Vec<FlatPosting> = transactions
        .iter()
        .flat_map(|t| {
            t.postings.iter().flat_map(move |p| {
                p.amounts.iter().map(move |a| FlatPosting {
                    date: t.date.clone(),
                    flag: t.flag.clone().unwrap_or_default(),
                    payee: t.payee.clone(),
                    narration: t.description.clone(),
                    account: p.account.clone(),
                    number: a.quantity.clone(),
                    currency: a.commodity.clone(),
                    file: t.source.file.clone(),
                    line: p.line.unwrap_or(t.source.line),
                })
            })
        })
        .collect();
    rows.sort_by(|a, b| (&a.date, &a.file, a.line).cmp(&(&b.date, &b.file, b.line)));
    rows
}

/// Read, book and report one Beancount file.
pub(crate) fn load(file: &SourceFile) -> Loaded {
    let mut findings = Vec::new();

    // Lines refused before the parser runs: it would look for included files on disk, and a
    // few numbers would stop it.
    for refusal in text::prescan(&file.content) {
        let extra = match &refusal.name {
            Some(name) => json!({ "name": name }),
            None => json!({}),
        };
        findings.push(unsupported(
            &file.name,
            refusal.line,
            refusal.construct,
            extra,
            refusal_message(refusal.construct),
        ));
    }
    if !findings.is_empty() {
        return Loaded {
            transactions: Vec::new(),
            findings,
        };
    }

    let read = match lima::read(&file.content) {
        Ok(read) => read,
        Err(issues) => {
            let findings = issues
                .into_iter()
                .map(|i| {
                    finding(
                        &file.name,
                        i.line,
                        "parse-error",
                        json!({ "reason": i.reason }),
                        "Beancount does not accept this.",
                    )
                })
                .collect();
            return Loaded {
                transactions: Vec::new(),
                findings,
            };
        }
    };

    let mut methods: HashMap<String, Method> = HashMap::new();
    for item in &read.items {
        if let Item::Open {
            account,
            method: Some(method),
        } = item
        {
            methods.insert(account.clone(), *method);
        }
    }
    let mut books = lima::Books::new(methods, read.default_method);
    let mut transactions = Vec::new();
    let mut average_accounts: HashSet<String> = HashSet::new();
    let mut refused = false;

    for item in &read.items {
        match item {
            Item::Open { .. } => {}
            Item::Refused { line, construct } => {
                refused = true;
                findings.push(unsupported(
                    &file.name,
                    *line,
                    construct,
                    json!({}),
                    refusal_message(construct),
                ));
            }
            Item::Transaction(tx) => {
                if tx
                    .postings
                    .iter()
                    .any(|p| p.cost.as_ref().is_some_and(|c| c.merge))
                {
                    refused = true;
                    findings.push(unsupported(
                        &file.name,
                        tx.line,
                        "cost-merge",
                        json!({}),
                        refusal_message("cost-merge"),
                    ));
                    continue;
                }
                match book::book(&mut books, tx.date, tx) {
                    Err(BookFailure::Average(account)) => {
                        refused = true;
                        if average_accounts.insert(account.clone()) {
                            findings.push(unsupported(
                                &file.name,
                                tx.line,
                                "average-booking",
                                json!({ "account": account }),
                                refusal_message("average-booking"),
                            ));
                        }
                    }
                    Err(BookFailure::Unsupported(construct)) => {
                        refused = true;
                        findings.push(unsupported(
                            &file.name,
                            tx.line,
                            construct,
                            json!({}),
                            refusal_message(construct),
                        ));
                    }
                    Err(BookFailure::TooPrecise) => {
                        refused = true;
                        findings.push(unsupported(
                            &file.name,
                            tx.line,
                            "number-too-precise",
                            json!({}),
                            refusal_message("number-too-precise"),
                        ));
                    }
                    Err(failure) => {
                        let reason = match failure {
                            BookFailure::Booking(reason) => reason,
                            _ => "auto-posting-with-several-currencies",
                        };
                        findings.push(finding(
                            &file.name,
                            tx.line,
                            "booking-error",
                            json!({ "reason": reason }),
                            "This transaction cannot be booked.",
                        ));
                    }
                    Ok(booked) => {
                        if !booked.residuals.is_empty() {
                            let residual: Vec<Value> = booked
                                .residuals
                                .iter()
                                .map(|(c, q)| json!({ "commodity": c, "quantity": q.to_string() }))
                                .collect();
                            findings.push(finding(
                                &file.name,
                                tx.line,
                                "unbalanced-transaction",
                                json!({ "residual": residual }),
                                "The postings do not add up to zero.",
                            ));
                        }
                        transactions.push(Transaction {
                            date: tx.date.to_string(),
                            status: status_of(tx.flag),
                            flag: Some(tx.flag.to_string()),
                            payee: tx.payee.clone(),
                            description: tx.narration.clone().unwrap_or_default(),
                            comment: String::new(),
                            source: Source {
                                file: file.name.clone(),
                                line: tx.line,
                            },
                            postings: booked.postings.iter().map(posting).collect(),
                        });
                    }
                }
            }
        }
    }
    if refused {
        transactions.clear();
    }
    // The items arrive in date order already; the sort pins the order the reference tool uses.
    transactions
        .sort_by(|a, b| (a.date.as_str(), a.source.line).cmp(&(b.date.as_str(), b.source.line)));
    findings.sort_by_key(|f| f.line);
    Loaded {
        transactions,
        findings,
    }
}
