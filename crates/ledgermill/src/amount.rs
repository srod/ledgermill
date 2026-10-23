//! Amounts as journals write them: a number with an optional commodity symbol on either side,
//! signs, quotes. See `spec/hledger/amount-forms.md` and `spec/hledger/numbers.md`.

use crate::chars::is_symbol_char;
use crate::cursor::Cursor;
use crate::decimal::Decimal;
use crate::number::{Mark, read_number};
use crate::problem::{Parsed, invalid};
use std::collections::HashMap;

/// What a file has declared about numbers so far (`decimal-mark`, `commodity`, `D`). Directives
/// only affect the lines after them, in their own file.
#[derive(Debug, Default, Clone)]
pub(crate) struct Styles {
    pub decimal_mark: Option<Mark>,
    pub commodity_marks: HashMap<String, Mark>,
    /// `D`: the commodity of amounts written without one, and the mark of its sample amount.
    pub default_commodity: Option<(String, Option<Mark>)>,
    /// The decimal places of `D`'s sample amount: a `P` rate with no commodity is listed with at
    /// least as many.
    pub default_places: u32,
    /// Nobody reads the values read with these styles (a rule's amounts): only their shape is
    /// checked.
    pub shape_only: bool,
}

impl Styles {
    /// What decides a lone `.`/`,` in an amount of `commodity`: a `decimal-mark` directive, else
    /// a `commodity` directive for that commodity, else a `D` directive.
    fn mark_for(&self, commodity: &str) -> Option<Mark> {
        self.decimal_mark
            .or_else(|| self.commodity_marks.get(commodity).copied())
            .or_else(|| self.default_commodity.as_ref().and_then(|(_, mark)| *mark))
    }
}

/// An amount as read: its commodity and exact quantity, and the decimal mark it used.
#[derive(Debug, Clone)]
pub(crate) struct ReadAmount {
    pub commodity: String,
    pub quantity: Decimal,
    pub mark: Option<Mark>,
    /// A commodity symbol was written (not taken from `D`, nor empty).
    pub named: bool,
}

/// Whether to give a commodity-less amount the `D` commodity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Default {
    Apply,
    Ignore,
}

fn read_sign(cur: &mut Cursor) -> bool {
    if cur.eat('-') {
        true
    } else {
        cur.eat('+');
        false
    }
}

/// A commodity symbol: `"quoted"` or a run of symbol characters. `None` if there is none here.
pub(crate) fn read_symbol(cur: &mut Cursor) -> Option<Parsed<String>> {
    if cur.eat('"') {
        let text = cur.take_while(|c| c != '"' && c != '\n');
        if !cur.eat('"') {
            return Some(invalid("A quoted commodity symbol is not closed."));
        }
        return Some(Ok(text.to_string()));
    }
    let text = cur.take_while(is_symbol_char);
    (!text.is_empty()).then(|| Ok(text.to_string()))
}

/// Read an amount at the cursor. The cursor stops right after it.
pub(crate) fn read_amount(
    cur: &mut Cursor,
    styles: &Styles,
    default: Default,
) -> Parsed<ReadAmount> {
    // One sign before everything; a second one is allowed only after a left-hand symbol, and
    // two minus signs cancel out.
    let mut negative = read_sign(cur);
    cur.skip_blanks();
    let left = read_symbol(cur).transpose()?;
    if left.is_some() {
        cur.skip_blanks();
        negative ^= read_sign(cur);
        cur.skip_blanks();
    }
    if matches!(cur.peek(), Some('-' | '+')) {
        return invalid("An amount has at most one sign before its number.");
    }
    let Some(raw) = read_number(cur).transpose()? else {
        return invalid("An amount was expected.");
    };
    let symbol = match left {
        Some(symbol) => Some(symbol),
        None => {
            let mut after = *cur;
            after.skip_blanks();
            match read_symbol(&mut after).transpose()? {
                Some(symbol) => {
                    *cur = after;
                    Some(symbol)
                }
                None => None,
            }
        }
    };
    let named = symbol.is_some();
    let commodity = match (symbol, &styles.default_commodity, default) {
        (Some(symbol), _, _) => symbol,
        (None, Some((symbol, _)), Default::Apply) => symbol.clone(),
        (None, _, _) => String::new(),
    };
    let number = raw.settle(styles.mark_for(&commodity), styles.shape_only)?;
    // Exact as written; rounding to ten places happens only when a quantity is reported.
    let value = number.value;
    let quantity = if negative { value.negated() } else { value };
    Ok(ReadAmount {
        commodity,
        quantity,
        mark: number.mark,
        named,
    })
}
