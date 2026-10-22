//! The seam to the two crates we build on: `beancount-parser-lima` (reads the text) and
//! `limabean-booking` (matches reductions to lots). Both are our forks, pinned by commit.
//!
//! Nothing outside this module names a type of either crate: they come in as text and plain
//! values and go out as the plain values of `plain.rs`. Numbers are `rust_decimal::Decimal` here
//! because the booking crate needs it; they cross the seam as `pydec::Num`.
//!
//! Only the booking algorithm is taken from the booking crate. Every figure that reaches the
//! output (per-unit prices, interpolated amounts) is derived by `book.rs`, because the crate's
//! arithmetic differs from the reference tool's (spec/beancount/numbers-and-interpolation.md).

use super::plain::{
    BookFailure, Day, Item, Lot, Method, ParseIssue, RawCost, RawPosting, RawPrice, RawTransaction,
    Read, Row,
};
use super::pydec::Num as Number;
use super::text::{line_of, line_starts};
use beancount_parser_lima as parser;
use beancount_parser_lima::Report;
use limabean_booking as booking;
use num_bigint::BigInt;
use std::collections::HashMap;
use std::sync::Arc;

// ───────────── reading ─────────────

fn number(d: rust_decimal::Decimal) -> Number {
    Number::from_scaled(BigInt::from(d.mantissa()), d.scale())
}

/// A number written as a plain literal; anything computed is refused by the caller.
fn literal(value: &parser::ExprValue) -> Option<Number> {
    match value.expr() {
        parser::Expr::Value(d) => Some(number(*d)),
        parser::Expr::Neg(inner) => match inner.as_ref() {
            parser::Expr::Value(d) => Some(number(*d).negated()),
            _ => None,
        },
        _ => None,
    }
}

/// The parser's date type is not exported, so this reads it through a macro.
macro_rules! day {
    ($date:expr) => {{
        let d = $date.item();
        Day {
            year: d.year(),
            month: u8::from(d.month()),
            day: d.day(),
        }
    }};
}

fn flag_char(flag: parser::Flag) -> char {
    match flag {
        parser::Flag::Asterisk => '*',
        parser::Flag::Exclamation => '!',
        parser::Flag::Ampersand => '&',
        parser::Flag::Hash => '#',
        parser::Flag::Question => '?',
        parser::Flag::Percent => '%',
        parser::Flag::Letter(letter) => letter.char(),
    }
}

fn method(booking: parser::Booking) -> Method {
    match booking {
        parser::Booking::Strict => Method::Strict,
        parser::Booking::StrictWithSize => Method::StrictWithSize,
        parser::Booking::None => Method::None,
        parser::Booking::Average => Method::Average,
        parser::Booking::Fifo => Method::Fifo,
        parser::Booking::Lifo => Method::Lifo,
        parser::Booking::Hifo => Method::Hifo,
    }
}

fn scoped(value: &parser::ScopedExprValue) -> (&parser::ExprValue, bool) {
    match value {
        parser::ScopedExprValue::PerUnit(v) => (v, false),
        parser::ScopedExprValue::Total(v) => (v, true),
    }
}

/// A posting, or the construct that makes it unreadable for now.
fn posting(
    p: &parser::Spanned<parser::Posting<'_>>,
    starts: &[usize],
) -> Result<RawPosting, &'static str> {
    let units = match p.amount() {
        Some(value) => Some(literal(value.item()).ok_or("number-expression")?),
        None => None,
    };
    let cost = match p.cost_spec() {
        Some(spec) => {
            let spec = spec.item();
            let per_unit = match spec.per_unit() {
                Some(v) => Some(literal(v.item()).ok_or("number-expression")?),
                None => None,
            };
            let total = match spec.total() {
                Some(v) => Some(literal(v.item()).ok_or("number-expression")?),
                None => None,
            };
            Some(RawCost {
                per_unit,
                total,
                currency: spec.currency().map(|c| c.item().to_string()),
                date: spec.date().map(|d| day!(d)),
                label: spec.label().map(|l| (*l.item()).to_string()),
                merge: spec.merge(),
            })
        }
        None => None,
    };
    let price = match p.price_annotation() {
        None => None,
        Some(spec) => Some(match spec.item() {
            parser::PriceSpec::CurrencyAmount(s, currency) => {
                let (value, total) = scoped(s);
                RawPrice {
                    number: Some(literal(value).ok_or("number-expression")?),
                    total,
                    currency: Some(currency.to_string()),
                }
            }
            parser::PriceSpec::BareAmount(s) => {
                let (value, total) = scoped(s);
                RawPrice {
                    number: Some(literal(value).ok_or("number-expression")?),
                    total,
                    currency: None,
                }
            }
            parser::PriceSpec::BareCurrency(currency) => RawPrice {
                number: None,
                total: false,
                currency: Some(currency.to_string()),
            },
            parser::PriceSpec::Unspecified => RawPrice {
                number: None,
                total: false,
                currency: None,
            },
        }),
    };
    // A price to work out next to a cost is not mapped by the reference (nor by us).
    if cost.is_some()
        && price
            .as_ref()
            .is_some_and(|p| p.number.is_none() || p.currency.is_none())
    {
        return Err("inferred-price");
    }
    Ok(RawPosting {
        account: p.account().item().to_string(),
        flag: p.flag().map(|f| flag_char(*f.item())),
        line: line_of(starts, p.span().start),
        units,
        currency: p.currency().map(|c| c.item().to_string()),
        cost,
        price,
    })
}

/// Read one file's text. `text` has no `include` line (the caller refuses those first).
pub(super) fn read(text: &str) -> Result<Read, Vec<ParseIssue>> {
    // Offsets from the parser refer to the rewritten text, so lines are counted in it too.
    let negative_tolerance = super::text::negative_tolerance_lines(text);
    let normalized = super::text::normalize(text);
    let starts = line_starts(&normalized);
    let sources = parser::BeancountSources::from(normalized);
    let reader = parser::BeancountParser::new(&sources);
    let success = match reader.parse() {
        Ok(success) => success,
        Err(failure) => {
            return Err(failure
                .errors
                .iter()
                .map(|e| ParseIssue {
                    line: line_of(&starts, e.span().start),
                    reason: e.reason().to_string(),
                })
                .collect());
        }
    };
    let default_method = success
        .options
        .booking_method()
        .map_or(Method::Strict, |m| method(*m.item()));
    let mut items = Vec::new();
    for directive in &success.directives {
        let line = line_of(&starts, directive.span().start);
        match directive.variant() {
            parser::DirectiveVariant::Transaction(t) => {
                let mut postings = Vec::new();
                let mut refused = None;
                for p in t.postings() {
                    match posting(p, &starts) {
                        Ok(p) => postings.push(p),
                        Err(construct) => {
                            refused = Some(construct);
                            break;
                        }
                    }
                }
                if let Some(construct) = refused {
                    items.push(Item::Refused { line, construct });
                    continue;
                }
                items.push(Item::Transaction(RawTransaction {
                    date: day!(directive.date()),
                    line,
                    flag: flag_char(*t.flag().item()),
                    payee: t.payee().map(|s| (*s.item()).to_string()),
                    narration: t.narration().map(|s| (*s.item()).to_string()),
                    postings,
                }));
            }
            parser::DirectiveVariant::Open(open) => items.push(Item::Open {
                date: day!(directive.date()),
                line,
                account: open.account().item().to_string(),
                currencies: open.currencies().map(|c| c.item().to_string()).collect(),
                method: open.booking().map(|b| method(*b.item())),
            }),
            parser::DirectiveVariant::Close(close) => items.push(Item::Close {
                date: day!(directive.date()),
                line,
                account: close.account().item().to_string(),
            }),
            parser::DirectiveVariant::Commodity(commodity) => items.push(Item::Commodity {
                date: day!(directive.date()),
                line,
                currency: commodity.currency().item().to_string(),
            }),
            parser::DirectiveVariant::Note(note) => items.push(Item::Note {
                date: day!(directive.date()),
                line,
                account: note.account().item().to_string(),
            }),
            parser::DirectiveVariant::Document(document) => items.push(Item::Document {
                date: day!(directive.date()),
                line,
                account: document.account().item().to_string(),
            }),
            parser::DirectiveVariant::Pad(pad) => items.push(Item::Pad {
                date: day!(directive.date()),
                line,
                account: pad.account().item().to_string(),
                source: pad.source().item().to_string(),
            }),
            parser::DirectiveVariant::Balance(balance) => {
                let atol = balance.atol().item();
                let amount = atol.amount().item();
                match literal(amount.number().item()) {
                    Some(expected) => items.push(Item::Balance {
                        date: day!(directive.date()),
                        line,
                        account: balance.account().item().to_string(),
                        expected,
                        tolerance: atol.tolerance().map(|t| {
                            let t = number(*t.item());
                            if negative_tolerance.contains(&line) {
                                t.negated()
                            } else {
                                t
                            }
                        }),
                        currency: amount.currency().item().to_string(),
                    }),
                    None => items.push(Item::Refused {
                        line,
                        construct: "number-expression",
                    }),
                }
            }
            // These do not change what transactions and postings report.
            _ => {}
        }
    }
    Ok(Read {
        items,
        default_method,
    })
}

// ───────────── booking ─────────────

/// One posting handed to the booking algorithm. The numbers are already per unit.
#[derive(Debug, Clone)]
pub(super) struct Feed {
    pub account: String,
    pub units: Option<Number>,
    pub currency: Option<String>,
    pub cost: Option<FeedCost>,
    pub price: Option<(Number, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct FeedCost {
    pub per_unit: Option<Number>,
    pub currency: Option<String>,
    pub date: Option<Day>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Types;

impl booking::BookingTypes for Types {
    type Account = Arc<str>;
    type Date = Day;
    type Currency = Arc<str>;
    type Number = rust_decimal::Decimal;
    type Label = Arc<str>;
}

/// For the crate, which only matches lots and tests balance with it: more than 28 places are
/// rounded away (`rust_decimal` holds no more).
fn to_decimal(n: &Number) -> Option<rust_decimal::Decimal> {
    let n = if n.places() > 28 {
        n.quantized(28)
    } else {
        n.clone()
    };
    let (mantissa, places) = n.scaled()?;
    let mantissa = i128::try_from(mantissa).ok()?;
    rust_decimal::Decimal::try_from_i128_with_scale(mantissa, places).ok()
}

#[derive(Debug, Clone)]
struct PostingIn {
    account: Arc<str>,
    units: Option<rust_decimal::Decimal>,
    currency: Option<Arc<str>>,
    cost: Option<CostIn>,
    price: Option<PriceIn>,
}

#[derive(Debug, Clone)]
struct CostIn {
    per_unit: Option<rust_decimal::Decimal>,
    currency: Option<Arc<str>>,
    date: Option<Day>,
    label: Option<Arc<str>>,
}

#[derive(Debug, Clone)]
struct PriceIn {
    per_unit: rust_decimal::Decimal,
    currency: Arc<str>,
}

impl booking::PostingSpec for PostingIn {
    type Types = Types;
    type CostSpec = CostIn;
    type PriceSpec = PriceIn;

    fn account(&self) -> Arc<str> {
        self.account.clone()
    }

    fn units(&self) -> Option<rust_decimal::Decimal> {
        self.units
    }

    fn currency(&self) -> Option<Arc<str>> {
        self.currency.clone()
    }

    fn cost(&self) -> Option<&CostIn> {
        self.cost.as_ref()
    }

    fn price(&self) -> Option<&PriceIn> {
        self.price.as_ref()
    }
}

impl booking::CostSpec for CostIn {
    type Types = Types;

    fn date(&self) -> Option<Day> {
        self.date
    }

    fn per_unit(&self) -> Option<rust_decimal::Decimal> {
        self.per_unit
    }

    fn total(&self) -> Option<rust_decimal::Decimal> {
        None
    }

    fn currency(&self) -> Option<Arc<str>> {
        self.currency.clone()
    }

    fn label(&self) -> Option<Arc<str>> {
        self.label.clone()
    }

    fn merge(&self) -> bool {
        false
    }
}

impl booking::PriceSpec for PriceIn {
    type Types = Types;

    fn per_unit(&self) -> Option<rust_decimal::Decimal> {
        Some(self.per_unit)
    }

    fn total(&self) -> Option<rust_decimal::Decimal> {
        None
    }

    fn currency(&self) -> Option<Arc<str>> {
        Some(self.currency.clone())
    }
}

/// The reference tool's default tolerance rule, as the booking crate reads it. `book.rs` makes
/// its own balance check, so the multiplier only matters for lot matching.
#[derive(Debug, Clone)]
struct Tolerance {
    multiplier: rust_decimal::Decimal,
    /// What a currency with no decimal place written may be off by; `None` means exact.
    default: Option<rust_decimal::Decimal>,
}

impl booking::Tolerance for Tolerance {
    type Types = Types;

    fn inferred_tolerance_default(&self, _currency: &Arc<str>) -> Option<rust_decimal::Decimal> {
        self.default
    }

    fn inferred_tolerance_multiplier(&self) -> Option<rust_decimal::Decimal> {
        Some(self.multiplier)
    }
}

fn failure(error: &booking::BookingError) -> BookFailure {
    use booking::{BookingError as E, PostingBookingError as P, TransactionBookingError as T};
    match error {
        E::Transaction(T::UnsupportedBookingMethod(..)) => {
            BookFailure::Booking("unsupported-booking-method")
        }
        E::Transaction(T::TooManyMissingNumbers) => {
            BookFailure::Booking("too-many-missing-numbers")
        }
        E::Transaction(T::Unbalanced(_)) => BookFailure::Booking("unbalanced"),
        E::Transaction(T::CannotDetermineCurrencyForBalancing) => {
            BookFailure::Booking("cannot-determine-currency")
        }
        E::Transaction(T::AutoPostMultipleBuckets(currencies)) => {
            BookFailure::SeveralCurrencies(currencies.clone())
        }
        // Matching a reduction against the lots is reported at the transaction.
        E::Posting(_, P::AmbiguousMatches) => BookFailure::Booking("ambiguous-lot-match"),
        E::Posting(_, P::MultipleCostCurrenciesMatch) => {
            BookFailure::Booking("several-cost-currencies-match")
        }
        E::Posting(_, P::NotEnoughLotsToReduce) => BookFailure::Booking("not-enough-lots"),
        E::Posting(_, P::NoPositionMatches) => BookFailure::Booking("no-matching-lot"),
        // What cannot be categorized or worked out is reported at its posting.
        E::Posting(at, P::AmbiguousAutoPost) => {
            BookFailure::OnPosting("ambiguous-auto-posting", *at)
        }
        E::Posting(at, P::CannotInferUnits) => BookFailure::OnPosting("cannot-infer-units", *at),
        E::Posting(at, P::CannotInferCurrency) => {
            BookFailure::OnPosting("cannot-infer-currency", *at)
        }
        E::Posting(at, P::CannotInferAnything) => {
            BookFailure::OnPosting("cannot-infer-anything", *at)
        }
        E::Posting(at, P::CannotInferPricePerUnit) => {
            BookFailure::OnPosting("cannot-infer-price", *at)
        }
        E::Posting(at, P::CannotInferPriceCurrency) => {
            BookFailure::OnPosting("cannot-infer-price-currency", *at)
        }
        E::Posting(at, P::CannotInferPrice) => BookFailure::OnPosting("cannot-infer-price", *at),
        E::Posting(at, P::InferredNegativeCostPerUnit) => {
            BookFailure::OnPosting("negative-cost", *at)
        }
        E::Posting(at, P::InferredNegativePricePerUnit) => {
            BookFailure::OnPosting("negative-price", *at)
        }
    }
}

/// What each account holds, from the transactions booked so far.
pub(super) struct Books {
    inventory: HashMap<Arc<str>, booking::Positions<Types>>,
    /// The cost of each lot as first written (the crate keeps it as `rust_decimal`, which loses
    /// the exponent the output shows). Keyed by account, cost currency, date, label and value.
    costs: HashMap<(String, String, Day, Option<String>, String), Number>,
    methods: HashMap<String, Method>,
    default: Method,
}

fn cost_key(
    account: &str,
    currency: &str,
    date: Day,
    label: Option<&str>,
    per_unit: &rust_decimal::Decimal,
) -> (String, String, Day, Option<String>, String) {
    (
        account.to_string(),
        currency.to_string(),
        date,
        label.map(str::to_string),
        per_unit.normalize().to_string(),
    )
}

fn booking_method(m: Method) -> Option<booking::Booking> {
    Some(match m {
        Method::Strict => booking::Booking::Strict,
        Method::StrictWithSize => booking::Booking::StrictWithSize,
        Method::None => booking::Booking::None,
        Method::Fifo => booking::Booking::Fifo,
        Method::Lifo => booking::Booking::Lifo,
        Method::Hifo => booking::Booking::Hifo,
        // The crate does not implement it (its code stops with `todo!`): never hand it over.
        Method::Average => return None,
    })
}

impl Books {
    pub(super) fn new(methods: HashMap<String, Method>, default: Method) -> Books {
        Books {
            inventory: HashMap::new(),
            costs: HashMap::new(),
            methods,
            default,
        }
    }

    fn method_of(&self, account: &str) -> Method {
        self.methods.get(account).copied().unwrap_or(self.default)
    }

    /// Whether the account books by AVERAGE, which `book` refuses before anything else.
    pub(super) fn averages(&self, account: &str) -> bool {
        self.method_of(account) == Method::Average
    }

    /// Whether a posting of these units adds a lot: the account holds none of the commodity with
    /// the opposite sign (which the posting would reduce).
    pub(super) fn adds_lot(&self, account: &str, currency: &str, units: &Number) -> bool {
        let negative = units.is_negative();
        self.inventory.get(account).is_none_or(|positions| {
            !positions.iter().any(|p| {
                &*p.currency == currency
                    && !p.units.is_zero()
                    && p.units.is_sign_negative() != negative
            })
        })
    }

    /// Book one transaction's postings; on success the inventory moves on.
    pub(super) fn book(&mut self, date: Day, feed: &[Feed]) -> Result<Vec<Row>, BookFailure> {
        // Refuse AVERAGE here, before any call that could reach the crate's `todo!`.
        if let Some(f) = feed.iter().find(|f| self.averages(&f.account)) {
            return Err(BookFailure::Average(f.account.clone()));
        }
        // A lot added here shows the cost as written, even when an equal lot is held with other
        // digits; a reduction shows the digits of the lot held.
        let adds: Vec<bool> = feed
            .iter()
            .map(|f| match (&f.cost, &f.units, &f.currency) {
                (Some(_), Some(units), Some(currency)) => {
                    self.adds_lot(&f.account, currency, units)
                }
                _ => false,
            })
            .collect();
        let postings: Vec<PostingIn> = feed
            .iter()
            .map(to_posting)
            .collect::<Option<_>>()
            .ok_or(BookFailure::TooPrecise)?;
        // A negative posting with a cost, in an account that holds none of its commodity, adds a
        // negative lot; the crate would call it a reduction of a missing lot. A zero position of
        // that commodity makes it an addition (spec/beancount/lots-and-booking.md).
        for (f, p) in feed.iter().zip(&postings) {
            let (Some(currency), Some(_), Some(units)) = (&p.currency, &f.cost, &p.units) else {
                continue;
            };
            if units.is_sign_negative() {
                let positions = self.inventory.entry(p.account.clone()).or_default();
                if !positions.iter().any(|pos| &pos.currency == currency) {
                    positions.accumulate(
                        rust_decimal::Decimal::ZERO,
                        currency.clone(),
                        None,
                        booking::Booking::Strict,
                    );
                }
            }
        }
        let refs: Vec<&PostingIn> = postings.iter().collect();
        let inventory = &self.inventory;
        let lookup = |account: Arc<str>| inventory.get(&account);
        let method = |account: Arc<str>| {
            booking_method(self.method_of(&account)).unwrap_or(booking::Booking::Strict)
        };
        let half = rust_decimal::Decimal::new(5, 1);
        let strict = Tolerance {
            multiplier: half,
            default: None,
        };
        let mut result = booking::book(date, &refs, &strict, lookup, method);
        if let Err(booking::BookingError::Transaction(
            booking::TransactionBookingError::Unbalanced(_),
        )) = &result
        {
            // The crate's balance check differs from ours at its edges; ours decides, so book
            // again with the check out of the way.
            let huge = rust_decimal::Decimal::from(10_000_000i64);
            let loose = Tolerance {
                multiplier: huge,
                default: Some(huge),
            };
            result = booking::book(date, &refs, &loose, lookup, method);
        }
        let booked = result.map_err(|e| failure(&e))?;
        // Remember the written cost of every new lot before looking any up.
        for f in feed {
            let Some(cost) = &f.cost else { continue };
            let (Some(per_unit), Some(currency)) = (&cost.per_unit, &cost.currency) else {
                continue;
            };
            if let Some(value) = to_decimal(per_unit) {
                let key = cost_key(
                    &f.account,
                    currency,
                    cost.date.unwrap_or(date),
                    cost.label.as_deref(),
                    &value,
                );
                self.costs.entry(key).or_insert_with(|| per_unit.clone());
            }
        }
        let mut rows = Vec::new();
        for (origin, p) in booked.interpolated_postings.iter().enumerate() {
            let units_currency = p.currency.to_string();
            match &p.cost {
                Some(costs) => {
                    let account = feed.get(origin).map_or("", |f| f.account.as_str());
                    let mut lots: Vec<_> = costs.iter().collect();
                    // The crate lists the lots in its own order (oldest first) when a sale takes
                    // everything held; the reference lists them in the order they were consumed,
                    // and the sort is stable, so equal lots keep the order given.
                    if adds.get(origin) != Some(&true) {
                        match self.method_of(account) {
                            Method::Fifo => lots.sort_by_key(|(_, c)| c.date),
                            Method::Lifo => lots.sort_by_key(|(_, c)| std::cmp::Reverse(c.date)),
                            Method::Hifo => {
                                lots.sort_by_key(|(_, c)| std::cmp::Reverse(c.per_unit))
                            }
                            _ => {}
                        }
                    }
                    for (cost_currency, c) in lots {
                        let label = c.label.as_ref().map(|l| l.to_string());
                        let key = cost_key(
                            account,
                            cost_currency.as_ref(),
                            c.date,
                            label.as_deref(),
                            &c.per_unit,
                        );
                        // Every new lot was written or filled in by `book.rs` and remembered above.
                        let written = feed
                            .get(origin)
                            .and_then(|f| f.cost.as_ref())
                            .and_then(|c| c.per_unit.clone())
                            .filter(|_| adds.get(origin) == Some(&true));
                        let per_unit = match written {
                            Some(written) => written,
                            None => self
                                .costs
                                .get(&key)
                                .cloned()
                                .ok_or(BookFailure::Unsupported("inferred-cost"))?,
                        };
                        rows.push(Row {
                            origin,
                            units: number(c.units),
                            currency: units_currency.clone(),
                            lot: Some(Lot {
                                per_unit,
                                currency: cost_currency.to_string(),
                                date: c.date,
                                label: c.label.as_ref().map(|l| l.to_string()),
                            }),
                        });
                    }
                }
                None => rows.push(Row {
                    origin,
                    units: number(p.units),
                    currency: units_currency,
                    lot: None,
                }),
            }
        }
        for (account, positions) in booked.updated_inventory {
            self.inventory.insert(account, positions);
        }
        Ok(rows)
    }
}

fn to_posting(f: &Feed) -> Option<PostingIn> {
    let cost = match &f.cost {
        Some(c) => Some(CostIn {
            per_unit: match &c.per_unit {
                Some(n) => Some(to_decimal(n)?),
                None => None,
            },
            currency: c.currency.as_deref().map(Arc::from),
            date: c.date,
            label: c.label.as_deref().map(Arc::from),
        }),
        None => None,
    };
    let price = match &f.price {
        Some((n, currency)) => Some(PriceIn {
            per_unit: to_decimal(n)?,
            currency: Arc::from(currency.as_str()),
        }),
        None => None,
    };
    Some(PostingIn {
        account: Arc::from(f.account.as_str()),
        units: match &f.units {
            Some(n) => Some(to_decimal(n)?),
            None => None,
        },
        currency: f.currency.as_deref().map(Arc::from),
        cost,
        price,
    })
}
