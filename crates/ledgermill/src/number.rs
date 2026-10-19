//! Exact decimal numbers, just enough for amounts: an integer mantissa and a count of decimal
//! places, so `-12.50` stays `-1250` with 2 places.

/// Most digits (integer and decimal together) an amount may have. Keeps every mantissa well
/// inside what a JavaScript number holds exactly, which is what the reference tool's JSON
/// consumers rely on.
const MAX_DIGITS: usize = 15;
/// Most decimal places an amount may have. The reference tool rounds past ten.
const MAX_PLACES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Decimal {
    mantissa: i128,
    places: u32,
}

impl Decimal {
    /// Parse `DIGITS` or `DIGITS.DIGITS` (no sign, no grouping, no exponent), with no leading
    /// zero in the integer part unless it is exactly `0`.
    pub(crate) fn parse_unsigned(text: &str) -> Option<Decimal> {
        let (int, frac) = match text.split_once('.') {
            Some((int, frac)) => (int, frac),
            None => (text, ""),
        };
        let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
        if int.is_empty() || !all_digits(int) || !all_digits(frac) {
            return None;
        }
        if text.contains('.') && frac.is_empty() {
            return None;
        }
        if int.len() > 1 && int.starts_with('0') {
            return None;
        }
        if int.len() + frac.len() > MAX_DIGITS || frac.len() > MAX_PLACES {
            return None;
        }
        let mantissa = format!("{int}{frac}").parse().ok()?;
        Some(Decimal { mantissa, places: frac.len() as u32 })
    }

    pub(crate) fn negated(self) -> Decimal {
        Decimal { mantissa: -self.mantissa, ..self }
    }

    pub(crate) fn is_zero(self) -> bool {
        self.mantissa == 0
    }

    /// Sum with exact arithmetic; the result has as many places as the most precise term.
    /// `None` only if the arithmetic would overflow.
    pub(crate) fn sum(terms: &[Decimal]) -> Option<Decimal> {
        let places = terms.iter().map(|t| t.places).max().unwrap_or(0);
        let mut mantissa: i128 = 0;
        for t in terms {
            let scale = 10i128.checked_pow(places - t.places)?;
            mantissa = mantissa.checked_add(t.mantissa.checked_mul(scale)?)?;
        }
        Some(Decimal { mantissa, places })
    }
}

impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.mantissa < 0 { "-" } else { "" };
        let digits = self.mantissa.unsigned_abs().to_string();
        let places = self.places as usize;
        if places == 0 {
            return write!(f, "{sign}{digits}");
        }
        let padded = format!("{digits:0>width$}", width = places + 1);
        let (int, frac) = padded.split_at(padded.len() - places);
        write!(f, "{sign}{int}.{frac}")
    }
}
