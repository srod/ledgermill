//! ledgermill core: loads ledger sources from memory and answers through typed values.
//!
//! Portable by design: no file access, no JS types. `load` never fails; problems become findings.

mod alias;
mod amount;
mod balance;
mod balances;
mod beancount;
mod chars;
mod cost;
mod cursor;
mod date;
mod decimal;
mod exact;
mod flat;
mod format;
mod holdings;
mod model;
mod number;
mod parse;
mod period;
mod posting;
mod prices;
mod problem;
mod tags;
mod valuation;
mod walk;

use exact::Exact;
pub use format::format;
pub use model::*;

/// A loaded ledger. Freed by drop.
///
/// Besides the public lists it keeps what `at_cost`, `value` and `holdings` need (hledger): every `P`
/// line and the exact amounts and dates of each transaction, so that those questions read nothing
/// again. A Beancount ledger answers them from its public lists.
///
/// A set of files can hold several ledgers: each Beancount file is one, and all the hledger files
/// together are one (spec/beancount/several-files.md). Such a `Ledger` keeps the ledgers it was
/// built from as `parts`, lists the merged transactions, postings, prices and findings, and answers
/// `at_cost`, `value` and `holdings` from its parts. A set of one ledger is that ledger, as it
/// always was.
#[derive(Debug, Default)]
pub struct Ledger {
    transactions: Vec<Transaction>,
    postings: Vec<FlatPosting>,
    findings: Vec<Finding>,
    /// Parallel to `transactions` (hledger): the exact amounts, dates and cost tags, for `at_cost`,
    /// `value` and `holdings`.
    exact: Vec<Exact>,
    /// Every `P` line, in parse order (hledger).
    prices: Vec<prices::Price>,
    /// The same lines as `prices()` gives them: sorted by date.
    price_list: Vec<PriceDirective>,
    beancount: bool,
    /// The ledgers of a set of several, in the order of the manifest position of their first file;
    /// empty for a set of one ledger.
    parts: Vec<Ledger>,
    /// With `parts`, parallel to `transactions`: the part a merged transaction comes from and its
    /// index in that part's `transactions`.
    origin: Vec<(usize, usize)>,
}

impl Ledger {
    /// Transactions in the order the reference tool prints them: by date, then source order.
    ///
    /// A set of several ledgers (see [`load`]): by date, then the position in `files` of the
    /// transaction's file, then the order of its own ledger.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// One row per posting and commodity, with the header fields of its transaction.
    ///
    /// Beancount: booked postings, by date, then file and line. hledger: by date, then the order
    /// the files were read; the postings of a transaction as written; the commodities of a
    /// posting in code point order (spec/hledger/postings-flat.md). A transaction with a line that
    /// was refused, or that was dropped, gives no rows, as in `transactions()`. A set of several
    /// ledgers: by date, then the position in `files` of the posting's file, then the order of its
    /// own ledger.
    pub fn postings(&self) -> &[FlatPosting] {
        &self.postings
    }

    /// The `P` directives of the files, as `hledger prices` lists them (hledger only).
    ///
    /// Sorted by date; directives with the same date stay in the order they were read, which is
    /// the files as given, each from top to bottom (spec/hledger/prices-list.md). Prices are not
    /// taken from the transactions. A directive whose rate is followed by a price, a lot or a label
    /// is refused with an `unsupported-syntax` finding (`price-annotation`) and not listed.
    ///
    /// Beancount: the `price` directives, sorted by date, the same date in the order written (a stable
    /// sort, as the `prices` table lists them: spec/beancount/price-table.md). Nothing is dropped:
    /// a directive written twice is listed twice. `rate.quantity` is the number as `number(amount)`
    /// prints it, so a rate below 0.000001 has an exponent (`1E-7`). Empty when the file has an
    /// `unsupported-syntax` finding: a Beancount file with a refused construct lists nothing.
    ///
    /// A set of several ledgers (see [`load`]): by date, then the position in `files` of the
    /// directive's file, then the order of its own ledger.
    pub fn prices(&self) -> &[PriceDirective] {
        &self.price_list
    }

    /// Problems found while loading, in the order the reference tool meets them, which is not line
    /// order: the first error is therefore the reference's first error.
    ///
    /// hledger: file by file, as given (hledger stops at the first file that has an error). In a
    /// file: what the lines gave (`parse-error`, `unsupported-syntax`), in line order; then the
    /// problems of the transactions without a balance assignment (`unbalanced-transaction`), in
    /// file order; then the walk over the postings by date, ties by position in the file
    /// (spec/hledger/assertion-failures.md): the `assertion-failed` findings of plain postings
    /// and, for a transaction with an assignment, taken as a block at its date, its
    /// `assignment-with-posting-date`, its `assertion-failed` and then its own
    /// `unbalanced-transaction`. A file with a line that was not read is not walked, as its
    /// balances would be wrong. A `=` or `~` rule is read and left out, with an `ignored-syntax`
    /// warning on its header line (`automated-rule`, `periodic-rule`), in line order with the
    /// errors of that file; it never changes the verdict (spec/hledger/rules-ignored.md).
    ///
    /// Beancount: in the order `bean-check` prints them, which is not line order: by step (parsing,
    /// booking, pads, balance assertions, then the validations), and within a step by date, with a
    /// `balance` before the rest of its day (spec/beancount/check-order.md). A warning
    /// (`ignored-syntax`) comes before the errors.
    ///
    /// A set of several ledgers (see [`load`]): each ledger's findings in the order above, stable-sorted
    /// by the position in `files` of the finding's `file` (a finding whose file is not in the set
    /// takes the position of its ledger's first file), so a ledger's own order is kept within a file.
    pub fn check(&self) -> &[Finding] {
        &self.findings
    }

    /// What each part answers, put back in the order of `transactions()` (a set of several ledgers).
    /// The error is the first part's.
    pub(crate) fn ask_parts(
        &self,
        ask: impl Fn(&Ledger) -> Result<Vec<Transaction>, Finding>,
    ) -> Result<Vec<Transaction>, Finding> {
        let mut answers = Vec::with_capacity(self.parts.len());
        for part in &self.parts {
            answers.push(ask(part)?.into_iter().map(Some).collect::<Vec<_>>());
        }
        Ok(self
            .origin
            .iter()
            .filter_map(|&(part, i)| answers.get_mut(part)?.get_mut(i)?.take())
            .collect())
    }
}

/// Parse the given files, in order.
///
/// `.beancount` and `.bean` files are Beancount, everything else is hledger. Each Beancount file is
/// its own ledger (an `include` is still refused), and all the hledger files together are one
/// ledger. With more than one ledger, `transactions()`, `postings()` and `prices()` are sorted by
/// date, ties by the position of the item's file in `files`, then in the order of its own ledger;
/// `check()` lists each ledger's findings sorted by the position of their file in `files`;
/// `at_cost`, `value` and `holdings` are answered by each ledger and combined (see their docs).
/// A Beancount file with a refused construct lists nothing and the other ledgers still load
/// (spec/beancount/several-files.md).
pub fn load(files: &[SourceFile]) -> Ledger {
    // Each ledger with the position of its first file, which orders the ledgers.
    let mut ledgers: Vec<(usize, Ledger)> = Vec::new();
    let mut hledger: Vec<&SourceFile> = Vec::new();
    let mut hledger_at = None;
    for (position, file) in files.iter().enumerate() {
        if beancount::is_beancount(&file.name) {
            ledgers.push((position, load_beancount(file)));
        } else {
            hledger_at.get_or_insert(position);
            hledger.push(file);
        }
    }
    if let Some(position) = hledger_at {
        ledgers.push((position, load_hledger(&hledger)));
    }
    ledgers.sort_by_key(|(position, _)| *position);
    if ledgers.len() > 1 {
        return merge_ledgers(files, ledgers);
    }
    // One ledger, or no file at all: an empty hledger ledger.
    ledgers
        .pop()
        .map_or_else(|| load_hledger(&[]), |(_, ledger)| ledger)
}

fn load_beancount(file: &SourceFile) -> Ledger {
    let loaded = beancount::load(file);
    Ledger {
        postings: beancount::flatten(&loaded.transactions),
        transactions: loaded.transactions,
        findings: loaded.findings,
        price_list: loaded.prices,
        beancount: true,
        ..Ledger::default()
    }
}

/// The position in `files` of the first file called `name`, or `otherwise`.
fn position_of(files: &[SourceFile], name: &str, otherwise: usize) -> usize {
    files
        .iter()
        .position(|f| f.name == name)
        .unwrap_or(otherwise)
}

/// `(part, index)` of the items of every part, sorted by `(date, position of their file)`. `keys`
/// gives the `(date, file)` of each item of a part; the sort is stable, so the rest of the order is
/// each ledger's own.
fn merged_order(
    files: &[SourceFile],
    parts: &[(usize, Ledger)],
    keys: impl for<'a> Fn(&'a Ledger) -> Vec<(&'a str, &'a str)>,
) -> Vec<(usize, usize)> {
    let mut keyed: Vec<(&str, usize, (usize, usize))> = Vec::new();
    for (part, (first, ledger)) in parts.iter().enumerate() {
        for (i, (date, file)) in keys(ledger).into_iter().enumerate() {
            keyed.push((date, position_of(files, file, *first), (part, i)));
        }
    }
    keyed.sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)));
    keyed.into_iter().map(|(_, _, at)| at).collect()
}

/// A ledger made of several, which are given in the order of their first file's position.
fn merge_ledgers(files: &[SourceFile], mut parts: Vec<(usize, Ledger)>) -> Ledger {
    let origin = merged_order(files, &parts, |l| {
        l.transactions
            .iter()
            .map(|t| (t.date.as_str(), t.source.file.as_str()))
            .collect()
    });
    let part = |at: usize| parts.get(at).map(|(_, ledger)| ledger);
    let transactions = origin
        .iter()
        .filter_map(|&(at, i)| part(at)?.transactions.get(i).cloned())
        .collect();
    let postings = merged_order(files, &parts, |l| {
        l.postings
            .iter()
            .map(|p| (p.date.as_str(), p.file.as_str()))
            .collect()
    })
    .into_iter()
    .filter_map(|(at, i)| part(at)?.postings.get(i).cloned())
    .collect();
    let price_list = merged_order(files, &parts, |l| {
        l.price_list
            .iter()
            .map(|p| (p.date.as_str(), p.file.as_str()))
            .collect()
    })
    .into_iter()
    .filter_map(|(at, i)| part(at)?.price_list.get(i).cloned())
    .collect();
    let mut findings: Vec<(usize, Finding)> = Vec::new();
    for (first, ledger) in &parts {
        for f in &ledger.findings {
            findings.push((position_of(files, &f.file, *first), f.clone()));
        }
    }
    findings.sort_by_key(|(position, _)| *position);
    // The parts answer `postings()` no more; the merged list does.
    for (_, ledger) in &mut parts {
        ledger.postings = Vec::new();
    }
    Ledger {
        transactions,
        postings,
        findings: findings.into_iter().map(|(_, f)| f).collect(),
        price_list,
        parts: parts.into_iter().map(|(_, ledger)| ledger).collect(),
        origin,
        ..Ledger::default()
    }
}

fn load_hledger(files: &[&SourceFile]) -> Ledger {
    let mut out = parse::Output {
        transactions: Vec::new(),
        prices: Vec::new(),
        findings: Vec::new(),
        layouts: Vec::new(),
    };
    for (index, file) in files.iter().enumerate() {
        parse::parse_file(index, file, &mut out);
    }
    // The sort is stable: same date keeps file order, then line order. Findings stay in the order
    // they were made in, which is hledger's: file by file; in each file what the lines gave, the
    // problems of plain transactions, then the walk's.
    out.transactions.sort_by_key(|(_, _, _, date)| *date);
    let mut transactions = Vec::new();
    let mut exact = Vec::new();
    let mut postings = Vec::new();
    for (transaction, kept, rows, _) in out.transactions {
        transactions.push(transaction);
        exact.push(kept);
        postings.extend(rows);
    }
    let mut listed: Vec<&prices::Price> = out.prices.iter().collect();
    listed.sort_by_key(|p| p.date);
    let price_list = listed
        .into_iter()
        .map(|p| PriceDirective {
            date: p.date.to_string(),
            commodity: p.from.clone(),
            rate: Quantity {
                commodity: p.to.clone(),
                quantity: p.listed.to_string(),
            },
            file: p.file.clone(),
            line: p.line,
        })
        .collect();
    Ledger {
        transactions,
        postings,
        exact,
        prices: out.prices,
        price_list,
        findings: out.findings.into_iter().map(|(_, f)| f).collect(),
        ..Ledger::default()
    }
}
