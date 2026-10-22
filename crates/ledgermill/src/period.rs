//! Period expressions, as a periodic rule (`~`) header holds them: `monthly`, `every 2nd day of
//! month`, `from 2026-01 to 2026-06`. Nothing here computes a period: the rule is never applied,
//! so the only question is whether the reference tool accepts the text. The grammar is in
//! `spec/hledger/rules-ignored.md`.
//!
//! The grammar is written as a set of alternatives: every function returns all the places where
//! what it reads can end, so a word that could be read two ways (`every month aug`, `2026-13`) is
//! tried both ways.

/// What the reference tool says about an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Valid,
    Invalid,
    /// An open range sign glued after a date (`2026-01-01-`, `feb..`): the reference tool takes
    /// some of these and not others, by a rule the spec notes do not pin down.
    Unsettled,
}

/// The expression in the text after the `~` of a header: past the blanks, up to the first `;`, two
/// blanks in a row (a description follows) or the end of the line.
pub(crate) fn header_expression(after_sign: &str) -> &str {
    let text = after_sign.trim_start_matches([' ', '\t']);
    let bytes = text.as_bytes();
    let is_blank = |b: u8| b == b' ' || b == b'\t';
    let end = (0..bytes.len())
        .find(|&i| {
            bytes[i] == b';'
                || (is_blank(bytes[i]) && bytes.get(i + 1).is_some_and(|&n| is_blank(n)))
        })
        .unwrap_or(bytes.len());
    text.get(..end).unwrap_or(text)
}

/// Judge an expression: the text after the `~` of a header, up to the two blanks, `;` or end of
/// line that end it.
pub(crate) fn judge(expression: &str) -> Verdict {
    let text = expression.trim_matches([' ', '\t']);
    if text.is_empty() || !text.is_ascii() {
        return Verdict::Invalid;
    }
    let folded = text.to_ascii_lowercase();
    if Scan::new(&folded).accepts() {
        return Verdict::Valid;
    }
    // A range sign with nothing after it, which the reference tool sometimes takes.
    let open = folded
        .strip_suffix("..")
        .or_else(|| folded.strip_suffix('-'))
        .map(|rest| rest.trim_end_matches([' ', '\t']))
        .filter(|rest| !rest.is_empty());
    match open {
        Some(rest) if Scan::new(rest).accepts() => Verdict::Unsettled,
        _ => Verdict::Invalid,
    }
}

type Ends = Vec<usize>;

struct Scan<'a> {
    text: &'a [u8],
}

const UNITS: [&str; 5] = ["day", "week", "month", "quarter", "year"];
const MONTHS: [(&str, u32); 23] = [
    ("january", 1),
    ("february", 2),
    ("march", 3),
    ("april", 4),
    ("may", 5),
    ("june", 6),
    ("july", 7),
    ("august", 8),
    ("september", 9),
    ("october", 10),
    ("november", 11),
    ("december", 12),
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];
const WEEKDAYS: [&str; 16] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "mon",
    "tue",
    "wed",
    "thu",
    "fri",
    "sat",
    "sun",
    "weekday",
    "weekendday",
];

fn is_leap(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// Days in `month` (1-12) of `year`; 0 for no such month.
fn days_in(year: u64, month: u64) -> u64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A run of digits as a number, saturating: a number this large is out of every range checked.
fn value(digits: &[u8]) -> u64 {
    digits.iter().fold(0u64, |n, d| {
        n.saturating_mul(10).saturating_add(u64::from(d - b'0'))
    })
}

/// Union of what `f` gives for each end.
fn then(ends: &[usize], f: impl Fn(usize) -> Ends) -> Ends {
    let mut out = Ends::new();
    for &end in ends {
        for next in f(end) {
            if !out.contains(&next) {
                out.push(next);
            }
        }
    }
    out
}

impl<'a> Scan<'a> {
    fn new(text: &'a str) -> Scan<'a> {
        Scan {
            text: text.as_bytes(),
        }
    }

    fn accepts(&self) -> bool {
        let len = self.text.len();
        let mut ends = self.span(0);
        for interval in self.interval(0) {
            ends.push(interval);
            ends.extend(self.span(interval));
            ends.extend(self.span(self.blanks(interval)));
        }
        ends.into_iter().any(|end| self.blanks(end) == len)
    }

    /// Past the blanks (a lone tab reads as a space) at `at`.
    fn blanks(&self, mut at: usize) -> usize {
        while matches!(self.text.get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        at
    }

    /// The end of `word` if it is at `at`.
    fn word(&self, at: usize, word: &str) -> Option<usize> {
        let end = at.checked_add(word.len())?;
        (self.text.get(at..end)? == word.as_bytes()).then_some(end)
    }

    /// The end of the first of `words` at `at`, for each that is there.
    fn words(&self, at: usize, words: &[&str]) -> Ends {
        words.iter().filter_map(|w| self.word(at, w)).collect()
    }

    /// A whole run of digits at `at`.
    fn digits(&self, at: usize) -> Option<(usize, &'a [u8])> {
        let rest = self.text.get(at..)?;
        let len = rest.iter().take_while(|b| b.is_ascii_digit()).count();
        (len > 0).then(|| (at + len, &rest[..len]))
    }

    fn month_name(&self, at: usize) -> Ends {
        MONTHS
            .iter()
            .filter_map(|(name, _)| self.word(at, name))
            .collect()
    }

    fn weekday_name(&self, at: usize) -> Ends {
        self.words(at, &WEEKDAYS[..14])
    }

    // ───────────── intervals ─────────────

    fn interval(&self, at: usize) -> Ends {
        let mut out = self.words(
            at,
            &[
                "daily",
                "weekly",
                "monthly",
                "quarterly",
                "yearly",
                "biweekly",
                "fortnightly",
                "bimonthly",
            ],
        );
        if let Some(end) = self.word(at, "every") {
            out.extend(self.every(self.blanks(end)));
        }
        out
    }

    /// What follows `every`.
    fn every(&self, at: usize) -> Ends {
        let mut out = self.words(at, &UNITS);
        // `every N days`: the plural only.
        if let Some((end, _)) = self.digits(at) {
            out.extend(self.words(
                self.blanks(end),
                &["days", "weeks", "months", "quarters", "years"],
            ));
        }
        // `every weekday`, `every weekendday`, `every mon`
        out.extend(self.words(at, &WEEKDAYS));
        // `every 5th day [of week|month]`, `every 2nd thursday [of month]`, `every 5th jan [of year]`
        if let Some(nth) = self.nth(at) {
            let after = self.blanks(nth);
            if let Some(day) = self.word(after, "day") {
                out.push(day);
                out.extend(self.of(day, &["week", "month"]));
            }
            for weekday in self.weekday_name(after) {
                out.push(weekday);
                out.extend(self.of(weekday, &["month"]));
            }
            for month in self.month_name(after) {
                out.push(month);
                out.extend(self.of(month, &["year"]));
            }
        }
        // `every jan 5th [of year]`
        for month in self.month_name(at) {
            if let Some(nth) = self.nth(self.blanks(month)) {
                out.push(nth);
                out.extend(self.of(nth, &["year"]));
            }
        }
        // `every 1/15 [of year]`
        for end in self.month_and_day(at) {
            out.push(end);
            out.extend(self.of(end, &["year"]));
        }
        out
    }

    /// A number and its ordinal ending: `1st`, `22nd`, `3rd`, `4th`, whatever the number.
    fn nth(&self, at: usize) -> Option<usize> {
        let (end, _) = self.digits(at)?;
        self.words(end, &["st", "nd", "rd", "th"]).first().copied()
    }

    /// `of WORD`, the blank before and after being optional.
    fn of(&self, at: usize, words: &[&str]) -> Ends {
        let Some(of) = self.word(self.blanks(at), "of") else {
            return Ends::new();
        };
        self.words(self.blanks(of), words)
    }

    /// `M/D`, `M-D` or `M.D`: a month and a day that exists in it (29 February included).
    fn month_and_day(&self, at: usize) -> Ends {
        let Some((sep_at, month)) = self.digits(at) else {
            return Ends::new();
        };
        let Some(b'/' | b'-' | b'.') = self.text.get(sep_at) else {
            return Ends::new();
        };
        let Some((end, day)) = self.digits(sep_at + 1) else {
            return Ends::new();
        };
        let (month, day) = (value(month), value(day));
        if (1..=12).contains(&month) && (1..=days_in(4, month)).contains(&day) {
            vec![end]
        } else {
            Ends::new()
        }
    }

    // ───────────── spans ─────────────

    /// The range signs: `to`, `..` and `-`.
    fn sign(&self, at: usize) -> Ends {
        let mut out = self.words(at, &["to", "..", "-"]);
        out.dedup();
        out
    }

    /// A sign and a second date, after the first one that ends at `date`.
    fn range_end(&self, date: usize) -> Ends {
        then(&self.sign(self.blanks(date)), |sign| {
            self.date(self.blanks(sign))
        })
    }

    fn span(&self, at: usize) -> Ends {
        let mut out = Ends::new();
        for keyword in ["from", "since"] {
            if let Some(end) = self.word(at, keyword) {
                for date in self.date(self.blanks(end)) {
                    out.push(date);
                    out.extend(self.range_end(date));
                }
            }
        }
        for keyword in ["to", "until", "in"] {
            if let Some(end) = self.word(at, keyword) {
                out.extend(self.date(self.blanks(end)));
            }
        }
        // `..DATE`, `-DATE`
        out.extend(then(&self.sign(at), |sign| self.date(self.blanks(sign))));
        for date in self.date(at) {
            out.push(date);
            out.extend(self.range_end(date));
        }
        out
    }

    // ───────────── dates ─────────────

    fn date(&self, at: usize) -> Ends {
        let mut out = self.numeric_date(at);
        out.extend(self.quarter(at));
        out.extend(self.month_name(at));
        out.extend(self.words(at, &["today", "yesterday", "tomorrow"]));
        out.extend(self.relative(at));
        if let Some(end) = self.word(at, "in") {
            out.extend(self.relative(self.blanks(end)));
        }
        out
    }

    /// `qN`
    fn quarter(&self, at: usize) -> Ends {
        match self.word(at, "q").and_then(|q| self.text.get(q)) {
            Some(b'1'..=b'4') => vec![at + 2],
            _ => Ends::new(),
        }
    }

    /// `last year`, `next monday`, `this feb`, `3 days ago`, `in 2 weeks`: the forms that count
    /// from today.
    fn relative(&self, at: usize) -> Ends {
        let mut out = Ends::new();
        for keyword in ["last", "this", "next"] {
            let Some(end) = self.word(at, keyword) else {
                continue;
            };
            let after = self.blanks(end);
            out.extend(self.unit_then_ago(after));
            out.extend(self.weekday_name(after));
            out.extend(self.month_name(after));
        }
        // N units
        let number = match self.text.get(at) {
            Some(b'-' | b'+') => at + 1,
            _ => at,
        };
        if let Some((end, _)) = self.digits(number) {
            out.extend(self.unit_then_ago(self.blanks(end)));
        }
        out
    }

    /// `day`, `days`, `week`... and an optional `ago` or `ahead`.
    fn unit_then_ago(&self, at: usize) -> Ends {
        let mut out = Ends::new();
        for unit in self.words(at, &UNITS) {
            let plural = unit + usize::from(self.text.get(unit) == Some(&b's'));
            for end in [unit, plural] {
                out.push(end);
                out.extend(self.words(self.blanks(end), &["ago", "ahead"]));
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// `2026`, `31`, `2026-01`, `2026/1/5`, `1/15`, `2026q2`, `20260115`.
    fn numeric_date(&self, at: usize) -> Ends {
        let Some((end, first)) = self.digits(at) else {
            return Ends::new();
        };
        let mut out = Ends::new();
        let len = first.len();
        if len < 4 {
            // Short runs are a day of the month, whatever their zeros.
            if (1..=31).contains(&value(first)) {
                out.push(end);
            }
        } else {
            // Eight digits that make a date are that date, and more than eight that start
            // with one are an error; anything else of four or more is a year.
            let starts_a_date = len >= 8 && real_date(&first[..8]);
            if !starts_a_date || len == 8 {
                out.push(end);
            }
            out.extend(self.quarter(end));
        }
        // A year and a month (`199512`) or a whole date is complete: only a `-` can go on, as
        // the sign of a range.
        let complete = (len == 6 && (1..=12).contains(&value(&first[4..])))
            || (len >= 8 && real_date(&first[..8]));
        if complete {
            return out;
        }
        let Some(&sep) = self
            .text
            .get(end)
            .filter(|s| matches!(s, b'-' | b'/' | b'.'))
        else {
            return out;
        };
        let Some((month_end, second)) = self.digits(end + 1) else {
            return out;
        };
        // When the longer reading is a date, it is the one taken: `2004-5-1/5` is not `2004-5`,
        // a sign and `1/5`.
        if len >= 4 {
            let (year, month) = (value(first), value(second));
            if (1..=12).contains(&month) {
                if self.text.get(month_end) == Some(&sep)
                    && let Some((day_end, day)) = self.digits(month_end + 1)
                    && (1..=days_in(year, month)).contains(&value(day))
                {
                    return vec![day_end];
                }
                return vec![month_end];
            }
        } else {
            let (month, day) = (value(first), value(second));
            if (1..=12).contains(&month) && (1..=days_in(4, month)).contains(&day) {
                return vec![month_end];
            }
        }
        out
    }
}

/// Eight digits `YYYYMMDD` that are a day of the calendar.
fn real_date(digits: &[u8]) -> bool {
    let (year, month, day) = (
        value(&digits[..4]),
        value(&digits[4..6]),
        value(&digits[6..8]),
    );
    (1..=days_in(year, month)).contains(&day)
}
