//! Exact decimal numbers of any size: an integer mantissa and a count of decimal places, so
//! `-12.50` is `-1250` with 2 places. No floating point anywhere; the places are part of the
//! value because the reference tool reports them.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, Zero};
use std::cmp::Ordering;

/// Places a division (an inferred price, a lot cost from a total) is rounded to.
pub(crate) const DIVISION_PLACES: u32 = 255;
/// Places a reported quantity is rounded to.
pub(crate) const OUTPUT_PLACES: u32 = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Decimal {
    mantissa: BigInt,
    places: u32,
}

/// Which term a sum of two zeros keeps (see [`Decimal::combine`]).
#[derive(Debug, Clone, Copy)]
pub(crate) enum ZeroTie {
    KeepEarlier,
    KeepLater,
}

fn pow10(n: u32) -> BigInt {
    num_traits::pow(BigInt::from(10u8), usize::try_from(n).unwrap_or(usize::MAX))
}

/// `numerator / denominator` rounded half to even. `denominator` is not zero.
fn div_half_even(numerator: &BigInt, denominator: &BigInt) -> BigInt {
    let negative = numerator.is_negative() != denominator.is_negative();
    let (n, d) = (numerator.abs(), denominator.abs());
    let (mut q, r) = n.div_rem(&d);
    let twice: BigInt = r * 2u8;
    if twice > d || (twice == d && q.is_odd()) {
        q += 1u8;
    }
    if negative { -q } else { q }
}

impl Decimal {
    pub(crate) fn new(mantissa: BigInt, places: u32) -> Decimal {
        Decimal { mantissa, places }
    }

    pub(crate) fn zero() -> Decimal {
        Decimal::new(BigInt::zero(), 0)
    }

    /// `digits` holds only ASCII digits (at least one).
    pub(crate) fn from_digits(digits: &str, places: u32) -> Option<Decimal> {
        Some(Decimal::new(digits.parse().ok()?, places))
    }

    pub(crate) fn places(&self) -> u32 {
        self.places
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.mantissa.is_zero()
    }

    pub(crate) fn is_negative(&self) -> bool {
        self.mantissa.is_negative()
    }

    pub(crate) fn negated(&self) -> Decimal {
        Decimal::new(-&self.mantissa, self.places)
    }

    fn scaled_to(&self, places: u32) -> BigInt {
        &self.mantissa * pow10(places.saturating_sub(self.places))
    }

    /// Exact sum; the result has as many places as the more precise term.
    pub(crate) fn add(&self, other: &Decimal) -> Decimal {
        let places = self.places.max(other.places);
        Decimal::new(self.scaled_to(places) + other.scaled_to(places), places)
    }

    /// Sum as the reference tool adds two amounts: a zero term gives back the other term,
    /// decimal places included; `tie` says which one two zeros give.
    pub(crate) fn combine(&self, other: &Decimal, tie: ZeroTie) -> Decimal {
        match (self.is_zero(), other.is_zero(), tie) {
            (true, true, ZeroTie::KeepEarlier) | (false, true, _) => self.clone(),
            (true, _, _) => other.clone(),
            (false, false, _) => self.add(other),
        }
    }

    /// Exact product; places add up.
    pub(crate) fn mul(&self, other: &Decimal) -> Decimal {
        Decimal::new(&self.mantissa * &other.mantissa, self.places + other.places)
    }

    /// Multiply by `10^exponent`, keeping the value exact.
    pub(crate) fn with_exponent(&self, exponent: i64) -> Decimal {
        let places = i64::from(self.places) - exponent;
        match u32::try_from(places) {
            Ok(places) => Decimal::new(self.mantissa.clone(), places),
            Err(_) => {
                let shift = u32::try_from(-places).unwrap_or(u32::MAX);
                Decimal::new(&self.mantissa * pow10(shift), 0)
            }
        }
    }

    /// The same value with at least `places` places (zeros added; more places are kept).
    pub(crate) fn padded_to(&self, places: u32) -> Decimal {
        let places = places.max(self.places);
        Decimal::new(self.scaled_to(places), places)
    }

    /// The same value without trailing zeros in the decimal part; zero has no places.
    pub(crate) fn normalized(&self) -> Decimal {
        if self.is_zero() {
            return Decimal::zero();
        }
        let ten = BigInt::from(10u8);
        let (mut mantissa, mut places) = (self.mantissa.clone(), self.places);
        while places > 0 {
            let (q, r) = mantissa.div_rem(&ten);
            if !r.is_zero() {
                break;
            }
            mantissa = q;
            places -= 1;
        }
        Decimal::new(mantissa, places)
    }

    /// Round half to even to at most `places` places; fewer places are left as they are.
    pub(crate) fn rounded(&self, places: u32) -> Decimal {
        if self.places <= places {
            return self.clone();
        }
        let mantissa = div_half_even(&self.mantissa, &pow10(self.places - places));
        Decimal::new(mantissa, places)
    }

    /// `self / divisor` rounded half to even to `places` places; `None` when dividing by zero.
    pub(crate) fn divided(&self, divisor: &Decimal, places: u32) -> Option<Decimal> {
        if divisor.is_zero() {
            return None;
        }
        let numerator = &self.mantissa * pow10(divisor.places + places);
        let denominator = &divisor.mantissa * pow10(self.places);
        Some(Decimal::new(
            div_half_even(&numerator, &denominator),
            places,
        ))
    }

    /// Compare values, whatever the places.
    pub(crate) fn cmp_value(&self, other: &Decimal) -> Ordering {
        let places = self.places.max(other.places);
        self.scaled_to(places).cmp(&other.scaled_to(places))
    }

    /// The decimal string the interface reports: rounded to [`OUTPUT_PLACES`].
    pub(crate) fn output(&self) -> String {
        self.rounded(OUTPUT_PLACES).to_string()
    }
}

impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.is_negative() { "-" } else { "" };
        let digits = self.mantissa.abs().to_string();
        let places = usize::try_from(self.places).unwrap_or(usize::MAX);
        if places == 0 {
            return write!(f, "{sign}{digits}");
        }
        let padded = format!("{digits:0>width$}", width = places + 1);
        let (int, frac) = padded.split_at(padded.len() - places);
        write!(f, "{sign}{int}.{frac}")
    }
}
