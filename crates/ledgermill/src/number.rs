//! Numbers as journals write them: digit groups, a decimal mark that is `.` or `,`, an optional
//! exponent. Reading happens in two steps because the meaning of a lone separator (`1,000`)
//! depends on directives and on the commodity, which may come after the number.
//! The rules are in `spec/hledger/numbers.md`.

use crate::chars::is_group_space;
use crate::cursor::Cursor;
use crate::decimal::Decimal;
use crate::problem::{Parsed, Problem, Unsupported, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    Point,
    Comma,
}

impl Mark {
    pub(crate) fn of(c: char) -> Option<Mark> {
        match c {
            '.' => Some(Mark::Point),
            ',' => Some(Mark::Comma),
            _ => None,
        }
    }

    fn char(self) -> char {
        match self {
            Mark::Point => '.',
            Mark::Comma => ',',
        }
    }
}

/// Exponents beyond this are refused: the value would be absurdly large to hold.
const MAX_EXPONENT: i64 = 4096;

/// A number as written, before its decimal mark is known.
#[derive(Debug)]
pub(crate) struct RawNumber<'a> {
    /// Starts with `.` (`.5`).
    leading_point: bool,
    /// Runs of digits; `seps[i]` sits between `runs[i]` and `runs[i + 1]`.
    runs: Vec<&'a str>,
    seps: Vec<char>,
    /// Ends with a `.` or `,` that no digit follows (`5.`, `1,000.`).
    trailing: Option<char>,
    exponent: Option<i64>,
}

/// A number with its meaning settled.
#[derive(Debug)]
pub(crate) struct Number {
    pub value: Decimal,
    /// The decimal mark the number used, if it had one.
    pub mark: Option<Mark>,
}

fn is_mark(c: char) -> bool {
    c == '.' || c == ','
}

fn inconsistent<T>() -> Parsed<T> {
    invalid("This number uses its separators inconsistently.")
}

/// Read a number at the cursor (no sign). `None`, with the cursor unmoved, when there is no
/// number here.
pub(crate) fn read_number<'a>(cur: &mut Cursor<'a>) -> Option<Parsed<RawNumber<'a>>> {
    let start = *cur;
    let leading_point =
        cur.peek() == Some('.') && cur.peek_nth(1).is_some_and(|c| c.is_ascii_digit());
    if leading_point {
        cur.bump();
    }
    let first = cur.take_while(|c| c.is_ascii_digit());
    if first.is_empty() {
        *cur = start;
        return None;
    }
    let mut raw = RawNumber {
        leading_point,
        runs: vec![first],
        seps: Vec::new(),
        trailing: None,
        exponent: None,
    };
    while let Some(c) = cur.peek().filter(|&c| is_mark(c) || is_group_space(c)) {
        let next = cur.peek_nth(1);
        if next.is_some_and(|d| d.is_ascii_digit()) {
            if leading_point {
                return Some(inconsistent());
            }
            cur.bump();
            raw.seps.push(c);
            raw.runs.push(cur.take_while(|c| c.is_ascii_digit()));
        } else {
            if is_mark(c) {
                if leading_point || next.is_some_and(is_mark) {
                    return Some(inconsistent());
                }
                cur.bump();
                raw.trailing = Some(c);
            }
            break;
        }
    }
    match read_exponent(cur) {
        Ok(e) => raw.exponent = e,
        Err(p) => return Some(Err(p)),
    }
    Some(Ok(raw))
}

fn read_exponent(cur: &mut Cursor) -> Parsed<Option<i64>> {
    if !matches!(cur.peek(), Some('e' | 'E')) {
        return Ok(None);
    }
    let mut probe = *cur;
    probe.bump();
    let negative = probe.eat('-');
    let signed = negative || probe.eat('+');
    let digits = probe.take_while(|c| c.is_ascii_digit());
    if digits.is_empty() {
        // `1e` is 1 of the commodity `e`; `1e+` is an error.
        return if signed {
            invalid("An exponent needs digits.")
        } else {
            Ok(None)
        };
    }
    *cur = probe;
    let value: i64 = digits.parse().unwrap_or(i64::MAX);
    if value > MAX_EXPONENT {
        return Err(Problem::Unsupported(Unsupported::NumberTooLarge));
    }
    Ok(Some(if negative { -value } else { value }))
}

impl RawNumber<'_> {
    /// Settle the meaning. `declared` decides a lone `.` or `,` between digits; with nothing
    /// declared it is the decimal mark.
    pub(crate) fn settle(&self, declared: Option<Mark>) -> Parsed<Number> {
        let (decimal_at, mark) = self.decimal_position(declared)?;
        let groups = match decimal_at {
            Some(i) => i,
            None => self.seps.len(),
        };
        if self.exponent.is_some() && groups > 0 {
            return invalid("Digit groups and exponents cannot be combined.");
        }
        let (int_runs, frac) = match decimal_at {
            Some(i) => (
                &self.runs[..=i],
                self.runs.get(i + 1).copied().unwrap_or(""),
            ),
            None => (&self.runs[..], ""),
        };
        let (int_runs, frac) = if self.leading_point {
            (&[][..], self.runs[0])
        } else {
            (int_runs, frac)
        };
        let mut digits = int_runs.concat();
        digits.push_str(frac);
        let places =
            u32::try_from(frac.len()).or(Err(Problem::Unsupported(Unsupported::NumberTooLarge)))?;
        let value = Decimal::from_digits(&digits, places)
            .ok_or(Problem::Invalid("This is not a valid number."))?;
        let value = match self.exponent {
            Some(e) => value.with_exponent(e),
            None => value,
        };
        Ok(Number { value, mark })
    }

    /// Index of the separator that is the decimal mark (`seps.len()` for a trailing one), and
    /// the mark.
    fn decimal_position(&self, declared: Option<Mark>) -> Parsed<(Option<usize>, Option<Mark>)> {
        if self.leading_point {
            return Ok((None, Some(Mark::Point)));
        }
        let seps = &self.seps;
        let spaced = seps.iter().any(|&c| is_group_space(c));
        let marks = seps.iter().filter(|&&c| is_mark(c)).count();

        if let Some(t) = self.trailing {
            // Everything before a trailing mark is digit groups of one kind.
            let uniform = seps.windows(2).all(|w| w[0] == w[1]);
            if !uniform || seps.contains(&t) {
                return inconsistent();
            }
            return Ok((Some(seps.len()), Mark::of(t)));
        }
        let Some(&last) = seps.last() else {
            return Ok((None, None));
        };
        if spaced {
            // Space groups first, then at most one mark, which is the decimal mark.
            match marks {
                0 => return Ok((None, None)),
                1 if is_mark(last) => return Ok((Some(seps.len() - 1), Mark::of(last))),
                1 => return invalid("This number has digits after its decimal part."),
                _ => return inconsistent(),
            }
        }
        let before = &seps[..seps.len() - 1];
        if before.iter().all(|&c| c == last) {
            if !before.is_empty() {
                return Ok((None, None));
            }
            let decimal = declared.is_none_or(|m| m.char() == last);
            return Ok(if decimal {
                (Some(0), Mark::of(last))
            } else {
                (None, None)
            });
        }
        // Two marks: the last one, used once, is the decimal mark; the other groups digits.
        let group = before[0];
        if before.iter().any(|&c| c != group) {
            return inconsistent();
        }
        Ok((Some(seps.len() - 1), Mark::of(last)))
    }
}
