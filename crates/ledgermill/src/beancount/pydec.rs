//! Numbers the way the reference tool computes and prints them
//! (spec/beancount/numbers-and-interpolation.md): a coefficient and an exponent, so `2.0E+2` and
//! `200` are the same value with different looks.
//!
//! - A sum keeps the finer of the two exponents; a product adds exponents.
//! - Every result is rounded half to even to 28 significant digits.
//! - A division whose result is exact keeps as few digits as it can without going past the
//!   exponent the operands suggest (the first minus the second); an inexact one has 28 digits.
//! - Printing is plain unless the exponent is positive or the number is smaller than `0.000001`;
//!   then it is `d.dddE+n`.
//!
//! `rust_decimal`, which the booking crate uses, rounds divisions to 28 decimal places instead,
//! so every figure that reaches the output is derived here.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt;

/// Significant digits kept by every operation.
const PRECISION: u32 = 28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Num {
    coefficient: Coef,
    exponent: i64,
}

/// A coefficient: a machine word while it fits, a big integer beyond. Amounts in a ledger are
/// small, and a word costs no allocation. It is always the word when the value fits, so equal
/// values are equal coefficients.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Coef {
    Small(i64),
    Big(BigInt),
}

impl Coef {
    fn from_big(n: BigInt) -> Coef {
        match n.to_i64() {
            Some(small) => Coef::Small(small),
            None => Coef::Big(n),
        }
    }

    fn to_big(&self) -> BigInt {
        match self {
            Coef::Small(small) => BigInt::from(*small),
            Coef::Big(big) => big.clone(),
        }
    }

    fn is_zero(&self) -> bool {
        matches!(self, Coef::Small(0))
    }

    fn is_negative(&self) -> bool {
        match self {
            Coef::Small(small) => *small < 0,
            Coef::Big(big) => big.is_negative(),
        }
    }

    fn negated(&self) -> Coef {
        match self {
            Coef::Small(small) => small
                .checked_neg()
                .map_or_else(|| Coef::from_big(-BigInt::from(*small)), Coef::Small),
            Coef::Big(big) => Coef::from_big(-big),
        }
    }

    fn abs(&self) -> Coef {
        if self.is_negative() {
            self.negated()
        } else {
            self.clone()
        }
    }

    fn add(&self, other: &Coef) -> Coef {
        if let (Coef::Small(a), Coef::Small(b)) = (self, other)
            && let Some(sum) = a.checked_add(*b)
        {
            return Coef::Small(sum);
        }
        Coef::from_big(self.to_big() + other.to_big())
    }

    fn mul(&self, other: &Coef) -> Coef {
        if let (Coef::Small(a), Coef::Small(b)) = (self, other)
            && let Some(product) = a.checked_mul(*b)
        {
            return Coef::Small(product);
        }
        Coef::from_big(self.to_big() * other.to_big())
    }

    fn times_pow10(&self, shift: u32) -> Coef {
        if shift == 0 {
            return self.clone();
        }
        if let Coef::Small(small) = self
            && shift <= 18
            && let Some(product) = small.checked_mul(10i64.pow(shift))
        {
            return Coef::Small(product);
        }
        Coef::from_big(self.to_big() * pow10(shift))
    }

    fn cmp(&self, other: &Coef) -> Ordering {
        match (self, other) {
            (Coef::Small(a), Coef::Small(b)) => a.cmp(b),
            _ => self.to_big().cmp(&other.to_big()),
        }
    }

    /// Whether it certainly has no more than [`PRECISION`] digits: below 2^93, which has 28.
    fn within_precision(&self) -> bool {
        match self {
            Coef::Small(_) => true,
            Coef::Big(big) => big.bits() < 94,
        }
    }

    fn digit_count(&self) -> u32 {
        match self {
            // Zero has one digit.
            Coef::Small(small) => small
                .unsigned_abs()
                .checked_ilog10()
                .map_or(1, |log| log + 1),
            Coef::Big(big) => u32::try_from(big.magnitude().to_string().len()).unwrap_or(u32::MAX),
        }
    }

    /// The digits of the magnitude, without a sign.
    fn digits(&self) -> String {
        match self {
            Coef::Small(small) => small.unsigned_abs().to_string(),
            Coef::Big(big) => big.magnitude().to_string(),
        }
    }
}

fn pow10(n: u32) -> BigInt {
    // Most shifts are a few places: one word, no repeated squaring.
    if n <= 19 {
        return BigInt::from(10u64.pow(n));
    }
    num_traits::pow(BigInt::from(10u8), usize::try_from(n).unwrap_or(usize::MAX))
}

fn digit_count(n: &BigInt) -> u32 {
    Coef::from_big(n.clone()).digit_count()
}

/// `numerator / denominator` rounded half to even, with whether the division was exact.
fn divide_rounded(numerator: &BigInt, denominator: &BigInt) -> (BigInt, bool) {
    let negative = numerator.is_negative() != denominator.is_negative();
    let (n, d) = (numerator.abs(), denominator.abs());
    let (mut q, r) = n.div_rem(&d);
    let exact = r.is_zero();
    let twice: BigInt = r * 2u8;
    if twice > d || (twice == d && q.is_odd()) {
        q += 1u8;
    }
    (if negative { -q } else { q }, exact)
}

impl Num {
    pub(super) fn new(coefficient: BigInt, exponent: i64) -> Num {
        Num::of(Coef::from_big(coefficient), exponent)
    }

    fn of(coefficient: Coef, exponent: i64) -> Num {
        Num {
            coefficient,
            exponent,
        }
    }

    /// `mantissa` with `scale` decimal places, as written in a journal.
    pub(super) fn from_scaled(mantissa: BigInt, scale: u32) -> Num {
        Num::new(mantissa, -i64::from(scale))
    }

    /// The same, from the mantissa of a `rust_decimal`.
    pub(super) fn from_scaled_i128(mantissa: i128, scale: u32) -> Num {
        match i64::try_from(mantissa) {
            Ok(small) => Num::of(Coef::Small(small), -i64::from(scale)),
            Err(_) => Num::from_scaled(BigInt::from(mantissa), scale),
        }
    }

    /// The number as `Display` or `plain` writes it back: an optional `-`, digits with an optional
    /// point, and an optional `E` with a signed exponent. `None` for anything else.
    pub(super) fn parse(text: &str) -> Option<Num> {
        let (negative, rest) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (mantissa, exponent) = match rest.split_once('E') {
            Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
            None => (rest, 0),
        };
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = format!("{whole}{fraction}");
        if whole.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let places = i64::try_from(fraction.len()).ok()?;
        let coefficient = match digits.parse::<i64>() {
            // Not negative yet, so negating cannot overflow.
            Ok(small) => Coef::Small(if negative { -small } else { small }),
            Err(_) => {
                let big: BigInt = digits.parse().ok()?;
                Coef::from_big(if negative { -big } else { big })
            }
        };
        Some(Num::of(coefficient, exponent - places))
    }

    pub(super) fn zero() -> Num {
        Num::of(Coef::Small(0), 0)
    }

    pub(super) fn is_zero(&self) -> bool {
        self.coefficient.is_zero()
    }

    pub(super) fn is_negative(&self) -> bool {
        self.coefficient.is_negative()
    }

    pub(super) fn negated(&self) -> Num {
        Num::of(self.coefficient.negated(), self.exponent)
    }

    pub(super) fn abs(&self) -> Num {
        Num::of(self.coefficient.abs(), self.exponent)
    }

    /// Decimal places written: zero for a number with a positive exponent.
    pub(super) fn places(&self) -> u32 {
        u32::try_from(-self.exponent).unwrap_or(0)
    }

    /// The value as an integer mantissa and a count of places, for `rust_decimal`; `None` when
    /// it does not fit.
    pub(super) fn scaled_i128(&self) -> Option<(i128, u32)> {
        match u32::try_from(-self.exponent) {
            Ok(places) => {
                let mantissa = match &self.coefficient {
                    Coef::Small(small) => i128::from(*small),
                    Coef::Big(big) => big.to_i128()?,
                };
                Some((mantissa, places))
            }
            Err(_) => {
                let shift = u32::try_from(self.exponent).ok()?;
                if shift > 60 {
                    return None;
                }
                let mantissa = self.coefficient.to_big() * pow10(shift);
                Some((mantissa.to_i128()?, 0))
            }
        }
    }

    /// Compare values, whatever the exponents.
    pub(super) fn cmp_value(&self, other: &Num) -> Ordering {
        let exponent = self.exponent.min(other.exponent);
        match self.exponent.cmp(&other.exponent) {
            Ordering::Equal => self.coefficient.cmp(&other.coefficient),
            Ordering::Less => self.coefficient.cmp(&other.aligned(exponent)),
            Ordering::Greater => self.aligned(exponent).cmp(&other.coefficient),
        }
    }

    /// The coefficient at a smaller or equal exponent.
    fn aligned(&self, exponent: i64) -> Coef {
        let shift = u32::try_from(self.exponent - exponent).unwrap_or(u32::MAX);
        self.coefficient.times_pow10(shift)
    }

    /// Round to [`PRECISION`] significant digits, half to even.
    fn rounded(self) -> Num {
        if self.coefficient.within_precision() {
            return self;
        }
        let count = self.coefficient.digit_count();
        if count <= PRECISION {
            return self;
        }
        let drop = count - PRECISION;
        let (mut coefficient, _) = divide_rounded(&self.coefficient.to_big(), &pow10(drop));
        let mut exponent = self.exponent + i64::from(drop);
        if digit_count(&coefficient) > PRECISION {
            coefficient /= 10u8;
            exponent += 1;
        }
        Num::new(coefficient, exponent)
    }

    /// The same value written without a positive exponent (`2.0E+2` becomes `200`).
    pub(super) fn without_positive_exponent(&self) -> Num {
        if self.exponent <= 0 {
            return self.clone();
        }
        Num::of(self.aligned(0), 0)
    }

    pub(super) fn add(&self, other: &Num) -> Num {
        let exponent = self.exponent.min(other.exponent);
        // The term that already has the finer exponent needs no copy.
        let sum = match self.exponent.cmp(&other.exponent) {
            Ordering::Equal => self.coefficient.add(&other.coefficient),
            Ordering::Less => self.coefficient.add(&other.aligned(exponent)),
            Ordering::Greater => self.aligned(exponent).add(&other.coefficient),
        };
        Num::of(sum, exponent).rounded()
    }

    pub(super) fn mul(&self, other: &Num) -> Num {
        Num::of(
            self.coefficient.mul(&other.coefficient),
            self.exponent + other.exponent,
        )
        .rounded()
    }

    /// `self / divisor`; `None` when the divisor is zero.
    pub(super) fn div(&self, divisor: &Num) -> Option<Num> {
        if divisor.is_zero() {
            return None;
        }
        let ideal = self.exponent - divisor.exponent;
        if self.is_zero() {
            return Some(Num::new(BigInt::zero(), ideal));
        }
        // Scale so the integer quotient has at least PRECISION + 1 digits.
        let da = i64::from(self.coefficient.digit_count());
        let db = i64::from(divisor.coefficient.digit_count());
        let k = u32::try_from((i64::from(PRECISION) + 2 + db - da).max(0)).unwrap_or(0);
        let divisor_coefficient = divisor.coefficient.to_big();
        let numerator = self.coefficient.to_big() * pow10(k);
        let (q, r) = numerator.div_rem(&divisor_coefficient);
        let digits = digit_count(&q);
        let exponent = ideal - i64::from(k);
        if r.is_zero() {
            // Exact: drop trailing zeros, but not past the exponent the operands suggest.
            let ten = BigInt::from(10u8);
            let (mut coefficient, mut exponent) = (q, exponent);
            while exponent < ideal {
                let (next, rest) = coefficient.div_rem(&ten);
                if !rest.is_zero() {
                    break;
                }
                coefficient = next;
                exponent += 1;
            }
            return Some(Num::new(coefficient, exponent).rounded());
        }
        let drop = digits.saturating_sub(PRECISION);
        let (mut coefficient, _) =
            divide_rounded(&numerator, &(&divisor_coefficient * pow10(drop)));
        let mut exponent = exponent + i64::from(drop);
        if digit_count(&coefficient) > PRECISION {
            coefficient /= 10u8;
            exponent += 1;
        }
        Some(Num::new(coefficient, exponent))
    }

    /// Round half to even to exactly `places` places, padding with zeros when it has fewer.
    pub(super) fn quantized(&self, places: u32) -> Num {
        let exponent = -i64::from(places);
        if self.exponent >= exponent {
            return Num::of(self.aligned(exponent), exponent);
        }
        let drop = u32::try_from(exponent - self.exponent).unwrap_or(u32::MAX);
        let (coefficient, _) = divide_rounded(&self.coefficient.to_big(), &pow10(drop));
        Num::new(coefficient, exponent)
    }
}

impl Num {
    /// The value in full, never with an exponent: `0.0000005`, not `5E-7`. Padding narrations
    /// print numbers this way (spec/beancount/pad.md).
    pub(super) fn plain(&self) -> String {
        let sign = if self.is_negative() { "-" } else { "" };
        let digits = self.coefficient.digits();
        let body = match usize::try_from(-self.exponent) {
            Ok(places) if places > 0 => {
                let padded = format!("{digits:0>width$}", width = places + 1);
                let (int, frac) = padded.split_at(padded.len() - places);
                format!("{int}.{frac}")
            }
            _ => {
                let zeros = usize::try_from(self.exponent).unwrap_or(0);
                format!("{digits}{}", "0".repeat(zeros))
            }
        };
        format!("{sign}{body}")
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.is_negative() { "-" } else { "" };
        let digits = self.coefficient.digits();
        let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        let adjusted = self.exponent + len - 1;
        if self.exponent <= 0 && adjusted >= -6 {
            if self.exponent == 0 {
                return write!(f, "{sign}{digits}");
            }
            let places = -self.exponent;
            if len > places {
                let point = usize::try_from(len - places).unwrap_or(0);
                let (int, frac) = digits.split_at(point);
                return write!(f, "{sign}{int}.{frac}");
            }
            let zeros = "0".repeat(usize::try_from(places - len).unwrap_or(0));
            return write!(f, "{sign}0.{zeros}{digits}");
        }
        let (head, tail) = digits.split_at(1);
        let point = if tail.is_empty() { "" } else { "." };
        let exponent_sign = if adjusted >= 0 { "+" } else { "-" };
        write!(
            f,
            "{sign}{head}{point}{tail}E{exponent_sign}{}",
            adjusted.abs()
        )
    }
}
