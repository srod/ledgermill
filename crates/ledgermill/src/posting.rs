//! One posting line: status, account, amount, price, lot, comment.
//! See `spec/hledger/posting-lines.md`, `spec/hledger/prices-and-lots.md`.

use crate::amount::{Default, ReadAmount, Styles, read_amount};
use crate::chars::{is_blank, trim_blank};
use crate::cursor::Cursor;
use crate::date::parse_date;
use crate::decimal::Decimal;
use crate::model::PostingKind;
use crate::problem::{Parsed, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CostKind {
    Unit,
    Total,
}

/// A written `@`/`@@` price. A total's value is already signed like the quantity.
#[derive(Debug, Clone)]
pub(crate) struct WrittenCost {
    pub kind: CostKind,
    pub commodity: String,
    pub value: Decimal,
    /// Inferred unit prices are reported without trailing zeros; written prices as written.
    pub inferred_unit: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LotCost {
    pub commodity: String,
    pub value: Decimal,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WrittenLot {
    pub cost: Option<LotCost>,
    pub date: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct WrittenAmount {
    pub commodity: String,
    pub quantity: Decimal,
    pub cost: Option<WrittenCost>,
    pub lot: Option<WrittenLot>,
}

/// A balance assertion written after a posting's amount (spec/hledger/assertion-syntax.md).
#[derive(Debug, Clone)]
pub(crate) struct WrittenAssertion {
    /// `==`: every commodity of the account is compared, not only the asserted one.
    pub total: bool,
    /// `=*`: the account's sub-accounts count too.
    pub inclusive: bool,
    /// The asserted amount; its price is kept (an assignment uses it) and never checked.
    pub amount: WrittenAmount,
    /// Byte offset of the first `=` in the posting content (indentation removed).
    pub offset: usize,
}

#[derive(Debug)]
pub(crate) struct PostingLine {
    pub kind: PostingKind,
    pub account: String,
    pub amount: Option<WrittenAmount>,
    pub assertion: Option<WrittenAssertion>,
    /// Text after the first `;`, untrimmed; `None` without one.
    pub comment: Option<String>,
}

/// Where the account name ends: at two whitespace characters in a row, or at a tab followed by
/// whitespace; a lone tab or no-break space is part of the name (and reads as a space).
fn split_account(text: &str) -> (&str, &str) {
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if is_blank(c) && chars.peek().is_some_and(|&(_, d)| is_blank(d)) {
            return (text.get(..i).unwrap_or(""), text.get(i..).unwrap_or(""));
        }
    }
    (text, "")
}

fn account_name(text: &str) -> String {
    trim_blank(text)
        .chars()
        .map(|c| if is_blank(c) { ' ' } else { c })
        .collect()
}

/// Read a posting line's content (indentation removed). `year` is the transaction's year, for
/// lot dates without one.
pub(crate) fn parse_posting(content: &str, styles: &Styles, year: u32) -> Parsed<PostingLine> {
    let mut cur = Cursor::new(content);
    // A posting status is read and not reported.
    if cur.eat('*') || cur.eat('!') {
        cur.skip_blanks();
    }
    let (raw_account, rest) = split_account(cur.rest());
    // A posting with no amount needs two spaces before its comment; otherwise `;` is part of
    // the name (the reference tool does the same).
    let (raw_account, rest, comment) = if rest.is_empty() {
        (raw_account, "", None)
    } else {
        let rest = rest.trim_start_matches(is_blank);
        match rest.strip_prefix(';') {
            Some(c) => (raw_account, "", Some(c.to_string())),
            None => (raw_account, rest, None),
        }
    };
    let trimmed = trim_blank(raw_account);
    let (kind, name) = match (trimmed.strip_prefix('('), trimmed.strip_prefix('[')) {
        (Some(inner), _) if inner.ends_with(')') => (
            PostingKind::Virtual,
            inner.strip_suffix(')').unwrap_or(inner),
        ),
        (_, Some(inner)) if inner.ends_with(']') => (
            PostingKind::BalancedVirtual,
            inner.strip_suffix(']').unwrap_or(inner),
        ),
        _ => (PostingKind::Regular, trimmed),
    };
    let account = match kind {
        PostingKind::Regular => account_name(name),
        // Inside brackets the name keeps its spaces as written (`( a )` is ` a `).
        _ => name
            .chars()
            .map(|c| if is_blank(c) { ' ' } else { c })
            .collect(),
    };
    if rest.is_empty() {
        return Ok(PostingLine {
            kind,
            account,
            amount: None,
            assertion: None,
            comment,
        });
    }
    let mut cur = Cursor::new(rest);
    let amount = if cur.peek() == Some('=') {
        None
    } else {
        Some(parse_amount_tail(&mut cur, styles, year)?)
    };
    cur.skip_blanks();
    let assertion = if cur.peek() == Some('=') {
        let offset = content.len() - cur.rest().len();
        let assertion = parse_assertion(&mut cur, styles, year, offset)?;
        cur.skip_blanks();
        Some(assertion)
    } else {
        None
    };
    let comment = if cur.eat(';') {
        Some(cur.rest().to_string())
    } else if cur.at_end() {
        None
    } else if assertion.is_some() {
        return invalid("Unexpected text after the balance assertion.");
    } else {
        return invalid("Unexpected text after the amount.");
    };
    Ok(PostingLine {
        kind,
        account,
        amount,
        assertion,
        comment,
    })
}

/// `=`, `==`, `=*` or `==*`, then the asserted amount. The cursor is on the first `=`. The
/// stars must touch the equals signs; the amount may carry a price but no lot.
fn parse_assertion(
    cur: &mut Cursor,
    styles: &Styles,
    year: u32,
    offset: usize,
) -> Parsed<WrittenAssertion> {
    cur.eat('=');
    let total = cur.eat('=');
    if cur.peek() == Some('=') {
        return invalid("A balance assertion has at most two equals signs.");
    }
    let inclusive = cur.eat('*');
    cur.skip_blanks();
    let amount = parse_amount_tail(cur, styles, year)?;
    if amount.lot.is_some() {
        return invalid("The amount of a balance assertion has no lot.");
    }
    Ok(WrittenAssertion {
        total,
        inclusive,
        amount,
        offset,
    })
}

/// The amount and its `@`/`@@` price and lot parts, in any order, each at most once.
fn parse_amount_tail(cur: &mut Cursor, styles: &Styles, year: u32) -> Parsed<WrittenAmount> {
    let main = read_amount(cur, styles, Default::Apply)?;
    let mut cost = None;
    let mut lot: Option<WrittenLot> = None;
    let mut lot_cost_total = None;
    loop {
        let mut probe = *cur;
        probe.skip_blanks();
        match probe.peek() {
            Some('@') => {
                if cost.is_some() {
                    return invalid("An amount has at most one price.");
                }
                probe.bump();
                let kind = if probe.eat('@') {
                    CostKind::Total
                } else {
                    CostKind::Unit
                };
                if probe.peek() == Some('@') {
                    return invalid("Unexpected `@`.");
                }
                probe.skip_blanks();
                let price = read_amount(&mut probe, styles, Default::Apply)?;
                cost = Some((kind, price));
            }
            Some('{') => {
                let l = lot.get_or_insert_with(WrittenLot::default);
                if l.cost.is_some() || lot_cost_total.is_some() {
                    return invalid("An amount has at most one lot cost.");
                }
                probe.bump();
                let total = probe.eat('{');
                probe.skip_blanks();
                probe.eat('=');
                probe.skip_blanks();
                let close = if total { "}}" } else { "}" };
                if !probe.eat_str(close) {
                    let value = read_amount(&mut probe, styles, Default::Apply)?;
                    probe.skip_blanks();
                    if !probe.eat_str(close) {
                        return invalid("A lot cost is not closed.");
                    }
                    if total {
                        lot_cost_total = Some(value);
                    } else {
                        l.cost = Some(LotCost {
                            commodity: value.commodity,
                            value: value.quantity,
                        });
                    }
                } else if total {
                    lot_cost_total = None;
                }
            }
            Some('[') => {
                let l = lot.get_or_insert_with(WrittenLot::default);
                if l.date.is_some() {
                    return invalid("An amount has at most one lot date.");
                }
                probe.bump();
                probe.skip_blanks();
                let date = parse_date(&mut probe, Some(year))?;
                probe.skip_blanks();
                if !probe.eat(']') {
                    return invalid("A lot date is not closed.");
                }
                l.date = Some(date.to_string());
            }
            Some('(') => {
                let l = lot.get_or_insert_with(WrittenLot::default);
                if l.label.is_some() {
                    return invalid("An amount has at most one lot label.");
                }
                probe.bump();
                let text = probe.take_while(|c| c != ')' && c != '\n');
                if !probe.eat(')') {
                    return invalid("A lot label is not closed.");
                }
                l.label = Some(trim_blank(text).to_string());
            }
            _ => break,
        }
        *cur = probe;
    }
    let ReadAmount {
        commodity,
        quantity,
        ..
    } = main;
    if let (Some(total), Some(l)) = (lot_cost_total, lot.as_mut()) {
        // `{{T}}` is reported per unit: T / quantity, so a negative quantity flips the sign.
        let per_unit = total
            .quantity
            .divided(&quantity, crate::decimal::OUTPUT_PLACES)
            .map(|v| if v.is_zero() { v } else { v.normalized() });
        l.cost = Some(LotCost {
            commodity: total.commodity,
            value: per_unit.unwrap_or_else(|| total.quantity.clone()),
        });
    }
    let cost = cost.map(|(kind, price)| WrittenCost {
        kind,
        commodity: price.commodity,
        value: match kind {
            CostKind::Unit => price.quantity,
            // A total takes the sign of the quantity it prices; a zero quantity counts as
            // positive.
            CostKind::Total if quantity.is_negative() => price.quantity.normalized().negated(),
            CostKind::Total => price.quantity.normalized(),
        },
        inferred_unit: false,
    });
    Ok(WrittenAmount {
        commodity,
        quantity,
        cost,
        lot,
    })
}
