//! Dates as journals write them: `2026-01-05`, `2026/1/5`, `2026.01.05`, or `01-05` with a year
//! from elsewhere. Proleptic Gregorian calendar, year 0 included.

use crate::cursor::Cursor;
use crate::problem::{Parsed, Problem, Unsupported, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Date {
    year: u32,
    month: u32,
    day: u32,
}

impl Date {
    pub(crate) fn year(self) -> u32 {
        self.year
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_sep(c: char) -> bool {
    matches!(c, '-' | '/' | '.')
}

fn number(digits: &str) -> Parsed<u32> {
    digits.parse().or(invalid("This is not a valid date."))
}

/// Read a date. A date without a year takes `year`; with no year to take, the reference tool
/// would use the current year, so it is unsupported (the result would depend on the clock),
/// unless the date is wrong in every year.
pub(crate) fn parse_date(cur: &mut Cursor, year: Option<u32>) -> Parsed<Date> {
    let first = cur.take_while(|c| c.is_ascii_digit());
    if first.is_empty() {
        return invalid("A date was expected.");
    }
    let Some(sep) = cur.peek().filter(|&c| is_sep(c)) else {
        return invalid("A date separator was expected.");
    };
    cur.bump();
    let second = cur.take_while(|c| c.is_ascii_digit());
    if second.is_empty() {
        return invalid("A month or day was expected.");
    }
    let (written_year, month, day) = match cur.peek().filter(|&c| is_sep(c)) {
        Some(c) => {
            if c != sep {
                return invalid("This date mixes separators.");
            }
            cur.bump();
            let third = cur.take_while(|c| c.is_ascii_digit());
            if third.is_empty() {
                return invalid("A day was expected.");
            }
            if first.len() < 4 {
                return invalid("This is not a valid date.");
            }
            (Some(number(first)?), number(second)?, number(third)?)
        }
        None => (None, number(first)?, number(second)?),
    };
    let year = match written_year.or(year) {
        Some(year) => year,
        // 2000 is a leap year: a date wrong in 2000 is wrong in every year.
        None if !valid(2000, month, day) => return invalid("This is not a valid date."),
        None => return Err(Problem::Unsupported(Unsupported::DateWithoutYear)),
    };
    if !valid(year, month, day) {
        return invalid("This is not a valid date.");
    }
    Ok(Date { year, month, day })
}

fn valid(year: u32, month: u32, day: u32) -> bool {
    day >= 1 && day <= days_in_month(year, month)
}
