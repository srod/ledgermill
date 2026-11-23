//! Beancount: reads the files of one ledger, books their transactions and reports them in our
//! model.
//!
//! The parser and the lot matching come from two forks (see `lima.rs`, the only module that
//! names their types). The figures are derived here, following the spec notes in
//! `spec/beancount/`.
//!
//! Anything that could change which transactions or postings come out and is not built yet is
//! refused with `unsupported-syntax`, and then no transaction is returned: a partial answer would
//! look complete.

mod align;
mod auto_accounts;
mod book;
mod check;
mod expr;
mod implicit_prices;
mod lima;
mod plain;
mod pydec;
mod text;
mod value;

pub(crate) use align::realign;
pub(crate) use value::{at_cost, both_ways_refusal, holdings, value};

use crate::model::{
    Amount, Cost, Finding, FlatPosting, Lot, Posting, PostingKind, PriceDirective, Quantity,
    Severity, Source, SourceFile, Status, Transaction,
};
use foldhash::{HashMap, HashSet};
use plain::{BookFailure, Day, IssueKind, Item, Method, ParseIssue};
use serde_json::{Value, json};

/// Files ending in `.beancount` or `.bean` are Beancount.
pub(crate) fn is_beancount(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".beancount") || lower.ends_with(".bean")
}

pub(crate) struct Loaded {
    /// The lines `format` may rewrite (see [`amount_lines`]); only filled when asked for.
    pub amount_lines: Option<HashSet<u32>>,
    /// By date, then the position of the file in the set, then source line.
    pub transactions: Vec<Transaction>,
    /// The rows of `transactions`, one per posting and commodity, in the same order of keys.
    pub postings: Vec<FlatPosting>,
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
        "plugin" => {
            "This plugin is not supported: only beancount.plugins.auto_accounts and beancount.plugins.implicit_prices are, without a configuration string."
        }
        "option" => "This option is not supported yet.",
        "number-expression" => {
            "This arithmetic is not supported yet: put a sign in front of a sign, or a sum after `~`, in parentheses."
        }
        "number-too-large" => "This number is too large to handle.",
        "inferred-price" => "A price to work out next to a cost is not supported.",
        "cost-partial" => "A cost with one part of `#` left out is not supported yet.",
        "cost-merge" => "Merging lots with {*} is not supported.",
        "inferred-cost" => "A lot cost to work out is not supported in this form.",
        "average-booking" => "AVERAGE booking is not supported.",
        "number-too-precise" => "This number needs more digits than are handled.",
        "implicit-prices-limit" => {
            "Repeating implicit_prices produces too many price entries to handle."
        }
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
        IssueKind::DivisionByZero => (
            "division-by-zero",
            json!({}),
            "A number is divided by zero. The entry is left out.",
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
            quantity: if p.negative_zero {
                format!("-{}", p.units)
            } else {
                p.units.to_string()
            },
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
    let balance_line = padding.balance_at.line;
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
        line: Some(balance_line),
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
            line: padding.at.line,
        },
        postings: padding.legs().into_iter().map(posting).collect(),
    }
}

/// The position of each file name in the set (the first, for a name given twice).
fn ranks<'a>(files: &[&'a SourceFile]) -> HashMap<&'a str, usize> {
    let mut ranks = HashMap::default();
    for (at, file) in files.iter().enumerate() {
        ranks.entry(file.name.as_str()).or_insert(at);
    }
    ranks
}

/// The flat posting list of booked transactions, each with the file its postings stand in (a
/// padding's postings stand in the file of its `balance`): sorted by date, the position of the
/// file in the set and line, like the reference's `ORDER BY date, filename, lineno` with the files
/// in the order given. A transaction lists its postings grouped by currency, so the line sort puts
/// them back in the order written; rows tied on a line (a posting split over lots) keep booking
/// order because the sort is stable.
fn flatten<'a>(
    ranks: &HashMap<&str, usize>,
    transactions: impl Iterator<Item = (&'a Transaction, &'a str)>,
) -> Vec<FlatPosting> {
    let mut rows: Vec<FlatPosting> = transactions
        .flat_map(|(t, file)| {
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
                    file: file.to_string(),
                    line: p.line.unwrap_or(t.source.line),
                })
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let (a_rank, b_rank) = (ranks.get(&*a.file), ranks.get(&*b.file));
        (&a.date, a_rank, a.line).cmp(&(&b.date, b_rank, b.line))
    });
    rows
}

/// A ledger that lists nothing and only says what was found.
fn nothing_but(findings: Vec<Finding>) -> Loaded {
    Loaded {
        amount_lines: None,
        transactions: Vec::new(),
        postings: Vec::new(),
        findings,
        prices: Vec::new(),
    }
}

/// Read, book and report Beancount files as one ledger, in the order given. The first file is the
/// root: its `booking_method` and `plugin` lines apply to all, those of the others are ignored, as
/// the reference does for an included file (spec/beancount/several-files.md).
pub(crate) fn load(files: &[&SourceFile]) -> Loaded {
    load_for(files, false)
}

/// [`load`] of one file, and its `amount_lines`, which `format` reads from the same parse.
pub(crate) fn load_with_amount_lines(file: &SourceFile) -> Loaded {
    load_for(&[file], true)
}

fn load_for(files: &[&SourceFile], with_amount_lines: bool) -> Loaded {
    let mut findings = Vec::new();

    // Lines refused before the parser runs: it would look for included files on disk, and a
    // few numbers would stop it. A refusal in any file refuses the ledger: what the other files
    // would give next to it is not the answer.
    let mut root_plugins = (false, 0);
    for (at, file) in files.iter().enumerate() {
        let prescan = text::prescan(&file.content);
        if at == 0 {
            root_plugins = (prescan.auto_accounts, prescan.implicit_prices);
        }
        for refusal in prescan.refusals {
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
    }
    if !findings.is_empty() {
        return nothing_but(findings);
    }
    let (auto_accounts_asked, implicit_runs) = root_plugins;

    let mut reads = Vec::with_capacity(files.len());
    for file in files {
        match lima::read(&file.content) {
            Ok(read) => reads.push(read),
            Err(issues) => {
                findings.extend(issues.into_iter().map(|i| parse_finding(&file.name, i)))
            }
        }
    }
    if !findings.is_empty() {
        return nothing_but(findings);
    }

    // Entries left out for a division by zero are reported first, as the reference reports
    // what it cannot read before it checks anything.
    for (file, read) in files.iter().zip(&reads) {
        findings.extend(
            read.errors
                .iter()
                .cloned()
                .map(|issue| parse_finding(&file.name, issue)),
        );
    }

    let mut methods: HashMap<String, Method> = HashMap::default();
    for item in reads.iter().flat_map(|read| &read.items) {
        if let Item::Open {
            account,
            method: Some(method),
            ..
        } = item
        {
            methods.insert(account.clone(), *method);
        }
    }
    let default_method = reads.first().map_or(Method::Strict, |r| r.default_method);
    let mut books = lima::Books::new(methods, default_method);
    let mut transactions = Vec::new();
    // What the checks need of each transaction that booking keeps, and the one error that is
    // printed last, after every other check.
    let mut kept: Vec<check::Kept> = Vec::new();
    let mut unbalanced: Vec<Finding> = Vec::new();
    let mut average_accounts: HashSet<String> = HashSet::default();
    let mut refused = false;
    // The entries the `implicit_prices` plugin adds, when a `plugin` line asks for it.
    let mut implicit = (implicit_runs > 0).then(implicit_prices::Run::default);

    // Booking sees the entries of all the files as one list, by date and then line (a line shared
    // by two files in file order, as the sort is stable), which is the order of one file as read.
    let mut ordered: Vec<(usize, &SourceFile, &Item)> = files
        .iter()
        .zip(&reads)
        .enumerate()
        .flat_map(|(index, (file, read))| read.items.iter().map(move |item| (index, *file, item)))
        .filter(|(.., item)| matches!(item, Item::Transaction(_) | Item::Refused { .. }))
        .collect();
    ordered.sort_by_key(|(.., item)| match item {
        Item::Transaction(tx) => Some((tx.date, tx.line)),
        _ => None,
    });
    for (index, file, item) in ordered {
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
                        if let Some(run) = implicit.as_mut() {
                            run.transaction(index, tx.date, tx.line, &booked.postings);
                        }
                        let postings = booked.postings.iter().map(posting).collect();
                        // The checks take what the postings hold; nothing is copied for them.
                        kept.push(check::Kept {
                            date: tx.date,
                            at: check::At {
                                file: index,
                                line: tx.line,
                            },
                            postings: booked
                                .postings
                                .into_iter()
                                .map(|p| check::Posted {
                                    account: p.account,
                                    currency: p.currency,
                                    units: p.units,
                                    lot: p.lot,
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
                            postings,
                        });
                    }
                }
            }
        }
    }
    if refused {
        // A partial answer would look complete, and the later checks would run on it.
        // This finding order is ledgermill's own: the reference is not compared on refused sets.
        let ranks = ranks(files);
        findings.sort_by_key(|f| (ranks.get(f.file.as_str()).copied(), f.line));
        return nothing_but(findings);
    }
    let items: Vec<&[Item]> = reads.iter().map(|read| read.items.as_slice()).collect();
    // The checks must see the `open` directives the plugin adds.
    let added = if auto_accounts_asked {
        auto_accounts::opens(&items, &kept)
    } else {
        Vec::new()
    };
    // The checks come in the order the reference prints them, not in line order.
    let checked = check::run(&items, &added, &kept);
    let name_of = |at: check::At| files.get(at.file).map_or("", |file| file.name.as_str());
    findings.extend(checked.documents.iter().map(|&at| Finding {
        code: "ignored-syntax".into(),
        severity: Severity::Warning,
        file: name_of(at).into(),
        line: at.line,
        column: None,
        details: json!({ "construct": "document" }),
        message: "The file of a document is not looked for; a missing one is not reported.".into(),
    }));
    findings.extend(
        checked
            .problems
            .into_iter()
            .map(|p| finding(name_of(p.at), p.at.line, p.code, p.details, p.message)),
    );
    findings.extend(unbalanced);
    let paddings: Vec<Transaction> = checked
        .paddings
        .iter()
        .map(|p| padding_transaction(name_of(p.at), p))
        .collect();
    let ranks = ranks(files);
    let implied = match implicit {
        None => Vec::new(),
        Some(run) => match run.repeated(implicit_runs) {
            Some(implied) => implied,
            None => {
                // Like a refused file: what was found so far stays, with the refusal, in the
                // order of a refused set.
                let root_name = files.first().map_or("", |f| f.name.as_str());
                findings.push(unsupported(
                    root_name,
                    0,
                    "implicit-prices-limit",
                    json!({}),
                    refusal_message("implicit-prices-limit"),
                ));
                findings.sort_by_key(|f| (ranks.get(f.file.as_str()).copied(), f.line));
                return nothing_but(findings);
            }
        },
    };
    // A refused file never gets here, so this is the same answer as "no `unsupported-syntax`
    // finding" without looking through them.
    let amount_lines = reads
        .first()
        .filter(|_| with_amount_lines)
        .map(amount_lines);
    let prices = listed_prices(files, &reads, &implied);
    // What reading and booking held is not needed any more. The flat rows built next stay in the
    // ledger; built while those are alive, they raise the peak and wasm memory never shrinks
    // (3.2 MB more for 10,000 transactions).
    drop(items);
    drop((reads, kept, books, added, implied));
    // The rows of a padding stand in the file of its `balance`, not of its `pad`.
    let postings = flatten(
        &ranks,
        transactions
            .iter()
            .map(|t| (t, t.source.file.as_str()))
            .chain(
                paddings
                    .iter()
                    .zip(&checked.paddings)
                    .map(|(t, p)| (t, name_of(p.balance_at))),
            ),
    );
    transactions.extend(paddings);
    // The sort pins the order the reference tool uses, the files in the order given.
    transactions.sort_by(|a, b| {
        let (a_rank, b_rank) = (ranks.get(&*a.source.file), ranks.get(&*b.source.file));
        (&a.date, a_rank, a.source.line).cmp(&(&b.date, b_rank, b.source.line))
    });
    Loaded {
        amount_lines,
        transactions,
        postings,
        findings,
        prices,
    }
}

/// The lines (1-based) that ledgermill reads as a posting of a transaction, a `balance` or a
/// `price`: the only ones `format` may rewrite, so a line inside a multi-line string or a
/// metadata line never is. A file that is refused or does not parse has none (`Loaded` holds
/// `None`): then ledgermill reads nothing of it and a rewrite could not be checked.
fn amount_lines(read: &plain::Read) -> HashSet<u32> {
    let mut lines: HashSet<u32> = read.left_out.iter().copied().collect();
    for item in &read.items {
        match item {
            Item::Transaction(tx) => lines.extend(tx.postings.iter().map(|p| p.line)),
            Item::Balance { line, .. } | Item::Price { line, .. } => {
                lines.insert(*line);
            }
            _ => {}
        }
    }
    lines
}

/// The `price` directives as the `prices` table lists them: by date, the same date in the order
/// written (spec/beancount/price-table.md), a line shared by two files in file order. The entries
/// `implicit_prices` added stand on the line and in the file of their transaction
/// (spec/beancount/implicit-prices.md).
fn listed_prices(
    files: &[&SourceFile],
    reads: &[plain::Read],
    implied: &[(usize, Item)],
) -> Vec<PriceDirective> {
    let written = reads
        .iter()
        .enumerate()
        .flat_map(|(file, read)| read.items.iter().map(move |item| (file, item)));
    let mut prices: Vec<(Day, usize, u32, PriceDirective)> = written
        .chain(implied.iter().map(|(file, item)| (*file, item)))
        .filter_map(|(file, item)| match item {
            Item::Price {
                date,
                line,
                currency,
                number,
                quote,
            } => Some((
                *date,
                file,
                *line,
                PriceDirective {
                    date: date.to_string(),
                    commodity: currency.clone(),
                    rate: quantity(quote, number),
                    file: files.get(file).map_or(String::new(), |f| f.name.clone()),
                    line: *line,
                },
            )),
            _ => None,
        })
        .collect();
    // By line before file, as the table does: with two prices of a pair on one day the later one
    // in this order is the one a valuation takes.
    prices.sort_by_key(|(date, file, line, _)| (*date, *line, *file));
    prices.into_iter().map(|(.., price)| price).collect()
}
