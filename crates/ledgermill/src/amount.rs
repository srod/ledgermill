//! The one amount form this slice reads:
//!
//! ```text
//! [-] NUMBER [ ][SYMBOL]      3.50   -3.50 EUR   3.50€
//! [-] SYMBOL [ ] NUMBER       $3.50  -EUR 3.50   EUR3.50
//!     SYMBOL [ ] - NUMBER     $-3.50 EUR -3.50
//! ```
//!
//! Anything else is refused by the caller, never guessed.

use crate::number::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedAmount {
    pub commodity: String,
    pub number: Decimal,
}

/// Characters a commodity symbol may be made of: letters, plus the common currency signs.
fn is_symbol_char(c: char) -> bool {
    (c.is_alphabetic() && !c.is_numeric()) || "$€£¥¢₹₽₩₪₫₺₿฿₴₦₱".contains(c)
}

fn is_number_char(c: char) -> bool {
    c.is_ascii_digit() || c == '.'
}

/// Split `text` at the end of its leading run of characters matching `pred`.
fn split_run(text: &str, pred: fn(char) -> bool) -> (&str, &str) {
    let end = text.find(|c| !pred(c)).unwrap_or(text.len());
    text.split_at(end)
}

/// Parse an amount; `text` has no surrounding blanks. `None` means "not a form we read".
pub(crate) fn parse_amount(text: &str) -> Option<ParsedAmount> {
    let (mut negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };

    let (commodity, number_text) = if rest.starts_with(is_symbol_char) {
        // SYMBOL [ ] [-] NUMBER
        let (symbol, after) = split_run(rest, is_symbol_char);
        let after = after.strip_prefix(' ').unwrap_or(after);
        let after = match after.strip_prefix('-') {
            Some(after) if !negative => {
                negative = true;
                after
            }
            Some(_) => return None,
            None => after,
        };
        (symbol, after)
    } else {
        // NUMBER [ ][SYMBOL]
        let (number, after) = split_run(rest, is_number_char);
        let symbol = after.strip_prefix(' ').unwrap_or(after);
        if !symbol.is_empty() && !symbol.chars().all(is_symbol_char) {
            return None;
        }
        (symbol, number)
    };

    let number = Decimal::parse_unsigned(number_text)?;
    let number = if negative { number.negated() } else { number };
    Some(ParsedAmount { commodity: commodity.to_string(), number })
}
