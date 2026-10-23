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
mod check;
mod lima;
mod plain;
mod pydec;
mod text;
mod value;

pub(crate) use value::{at_cost, holdings, value};

use crate::model::{
    Amount, Cost, Finding, FlatPosting, Lot, Posting, PostingKind, PriceDirective, Quantity,
    Severity, Source, SourceFile, Status, Transaction,
};
use plain::{BookFailure, Day, IssueKind, Item, Method, ParseIssue};
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
    /// In the order the reference tool prints them (spec/beancount/check-order.md).
    pub findings: Vec<Finding>,
    /// The `price` directives, by date and then line (spec/beancount/price-table.md).
    pub prices: Vec<PriceDirective>,
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

/// A parser error as a finding: syntax errors stay `parse-error`, the others have a code of
/// their own (spec/README.md).
fn parse_finding(file: &str, issue: ParseIssue) -> Finding {
    let (code, details, message) = match issue.kind {
        IssueKind::Syntax => (
            "parse-error",
            json!({ "reason": issue.reason }),
            "Beancount does not accept this.",
        ),
        IssueKind::InvalidAccount(account) => (
            "invalid-account-name",
            json!({ "account": account }),
            "The first part of an account name must be Assets, Liabilities, Equity, Income or Expenses.",
        ),
        IssueKind::PopWithoutPush { metadata, name } => (
            "unbalanced-push",
            json!({ "kind": push_kind(metadata), "name": name, "reason": "pop-without-push" }),
            "This pops a tag or metadata key that was not pushed.",
        ),
        IssueKind::PushNotPopped { metadata, name } => (
            "unbalanced-push",
            json!({ "kind": push_kind(metadata), "name": name, "reason": "push-not-popped" }),
            "This tag or metadata key is pushed and never popped.",
        ),
        IssueKind::DuplicateMetadata { key, posting } => (
            "duplicate-metadata",
            json!({ "key": key, "scope": if posting { "posting" } else { "entry" } }),
            "The same metadata key is given twice.",
        ),
    };
    finding(file, issue.line, code, details, message)
}

fn push_kind(metadata: bool) -> &'static str {
    if metadata { "metadata" } else { "tag" }
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

/// The transaction a `pad` adds: on the pad's line, with its postings on the balance's line
/// (spec/beancount/pad.md).
fn padding_transaction(file: &str, padding: &check::Padding) -> Transaction {
    let currency = &padding.currency;
    let posting = |(account, units): (&str, pydec::Num)| Posting {
        account: account.to_string(),
        kind: PostingKind::Regular,
        comment: String::new(),
        amounts: vec![Amount {
            commodity: currency.clone(),
            quantity: units.to_string(),
            cost: None,
            lot: None,
        }],
        flag: None,
        line: Some(padding.balance_line),
    };
    Transaction {
        date: padding.date.to_string(),
        status: status_of('P'),
        flag: Some("P".into()),
        payee: None,
        description: format!(
            "(Padding inserted for Balance of {} {currency} for difference {} {currency})",
            padding.expected.plain(),
            padding.difference.plain()
        ),
        comment: String::new(),
        source: Source {
            file: file.to_string(),
            line: padding.line,
        },
        postings: padding.legs().into_iter().map(posting).collect(),
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
                    kind: PostingKind::Regular,
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
            prices: Vec::new(),
        };
    }

    let read = match lima::read(&file.content) {
        Ok(read) => read,
        Err(issues) => {
            let findings = issues
                .into_iter()
                .map(|i| parse_finding(&file.name, i))
                .collect();
            return Loaded {
                transactions: Vec::new(),
                findings,
                prices: Vec::new(),
            };
        }
    };

    let mut methods: HashMap<String, Method> = HashMap::new();
    for item in &read.items {
        if let Item::Open {
            account,
            method: Some(method),
            ..
        } = item
        {
            methods.insert(account.clone(), *method);
        }
    }
    let mut books = lima::Books::new(methods, read.default_method);
    let mut transactions = Vec::new();
    // What the checks need of each transaction that booking keeps, and the one error that is
    // printed last, after every other check.
    let mut kept: Vec<check::Kept> = Vec::new();
    let mut unbalanced: Vec<Finding> = Vec::new();
    let mut average_accounts: HashSet<String> = HashSet::new();
    let mut refused = false;

    for item in &read.items {
        match item {
            Item::Open { .. }
            | Item::Close { .. }
            | Item::Commodity { .. }
            | Item::Price { .. }
            | Item::Pad { .. }
            | Item::Balance { .. }
            | Item::Note { .. }
            | Item::Document { .. } => {}
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
                        let (reason, line) = match failure {
                            BookFailure::Booking(reason) => (reason, tx.line),
                            BookFailure::OnPosting(reason, at) => {
                                (reason, tx.postings.get(at).map_or(tx.line, |p| p.line))
                            }
                            _ => ("auto-posting-with-several-currencies", tx.line),
                        };
                        findings.push(finding(
                            &file.name,
                            line,
                            "booking-error",
                            json!({ "reason": reason }),
                            "This transaction cannot be booked.",
                        ));
                    }
                    Ok(booked) => {
                        for line in book::zero_units_at_cost(tx) {
                            // The transaction stays: bean-query lists it, unlike other booking errors.
                            findings.push(finding(
                                &file.name,
                                line,
                                "booking-error",
                                json!({ "reason": "zero-amount-at-cost" }),
                                "A posting of zero units cannot have a cost.",
                            ));
                        }
                        if !booked.residuals.is_empty() {
                            let residual: Vec<Value> = booked
                                .residuals
                                .iter()
                                .map(|(c, q)| json!({ "commodity": c, "quantity": q.to_string() }))
                                .collect();
                            unbalanced.push(finding(
                                &file.name,
                                tx.line,
                                "unbalanced-transaction",
                                json!({ "residual": residual }),
                                "The postings do not add up to zero.",
                            ));
                        }
                        kept.push(check::Kept {
                            date: tx.date,
                            line: tx.line,
                            postings: booked
                                .postings
                                .iter()
                                .map(|p| check::Posted {
                                    account: p.account.clone(),
                                    currency: p.currency.clone(),
                                    units: p.units.clone(),
                                    lot: p.lot.clone(),
                                })
                                .collect(),
                        });
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
        // A partial answer would look complete, and the later checks would run on it.
        transactions.clear();
        findings.sort_by_key(|f| f.line);
        return Loaded {
            transactions,
            findings,
            prices: Vec::new(),
        };
    }
    // The checks come in the order the reference prints them, not in line order.
    let checked = check::run(&read.items, &kept);
    findings.extend(checked.documents.iter().map(|&line| Finding {
        code: "ignored-syntax".into(),
        severity: Severity::Warning,
        file: file.name.clone(),
        line,
        column: None,
        details: json!({ "construct": "document" }),
        message: "The file of a document is not looked for; a missing one is not reported.".into(),
    }));
    findings.extend(
        checked
            .problems
            .into_iter()
            .map(|p| finding(&file.name, p.line, p.code, p.details, p.message)),
    );
    findings.extend(unbalanced);
    transactions.extend(
        checked
            .paddings
            .iter()
            .map(|p| padding_transaction(&file.name, p)),
    );
    // The items arrive in date order already; the sort pins the order the reference tool uses.
    transactions
        .sort_by(|a, b| (a.date.as_str(), a.source.line).cmp(&(b.date.as_str(), b.source.line)));
    Loaded {
        transactions,
        findings,
        prices: listed_prices(&file.name, &read.items),
    }
}

/// The `price` directives as the `prices` table lists them: by date, the same date in the order
/// written (spec/beancount/price-table.md).
fn listed_prices(file: &str, items: &[Item]) -> Vec<PriceDirective> {
    let mut prices: Vec<(Day, u32, PriceDirective)> = items
        .iter()
        .filter_map(|item| match item {
            Item::Price {
                date,
                line,
                currency,
                number,
                quote,
            } => Some((
                *date,
                *line,
                PriceDirective {
                    date: date.to_string(),
                    commodity: currency.clone(),
                    rate: quantity(quote, number),
                    file: file.to_string(),
                    line: *line,
                },
            )),
            _ => None,
        })
        .collect();
    prices.sort_by_key(|(date, line, _)| (*date, *line));
    prices.into_iter().map(|(_, _, price)| price).collect()
}
