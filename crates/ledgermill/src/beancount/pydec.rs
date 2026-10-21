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
use num_traits::{Signed, Zero};
use std::cmp::Ordering;
use std::fmt;

/// Significant digits kept by every operation.
const PRECISION: u32 = 28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Num {
    coefficient: BigInt,
    exponent: i64,
}

fn pow10(n: u32) -> BigInt {
    num_traits::pow(BigInt::from(10u8), usize::try_from(n).unwrap_or(usize::MAX))
}

fn digit_count(n: &BigInt) -> u32 {
    if n.is_zero() {
        return 1;
    }
    u32::try_from(n.abs().to_string().len()).unwrap_or(u32::MAX)
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
        Num {
            coefficient,
            exponent,
        }
    }

    /// `mantissa` with `scale` decimal places, as written in a journal.
    pub(super) fn from_scaled(mantissa: BigInt, scale: u32) -> Num {
        Num::new(mantissa, -i64::from(scale))
    }

    pub(super) fn zero() -> Num {
        Num::new(BigInt::zero(), 0)
    }

    pub(super) fn is_zero(&self) -> bool {
        self.coefficient.is_zero()
    }

    pub(super) fn is_negative(&self) -> bool {
        self.coefficient.is_negative()
    }

    pub(super) fn negated(&self) -> Num {
        Num::new(-&self.coefficient, self.exponent)
    }

    pub(super) fn abs(&self) -> Num {
        Num::new(self.coefficient.abs(), self.exponent)
    }

    /// Decimal places written: zero for a number with a positive exponent.
    pub(super) fn places(&self) -> u32 {
        u32::try_from(-self.exponent).unwrap_or(0)
    }

    /// The value as an integer mantissa and a count of places, for `rust_decimal`.
    pub(super) fn scaled(&self) -> Option<(BigInt, u32)> {
        match u32::try_from(-self.exponent) {
            Ok(places) => Some((self.coefficient.clone(), places)),
            Err(_) => {
                let shift = u32::try_from(self.exponent).ok()?;
                (shift <= 60).then(|| (&self.coefficient * pow10(shift), 0))
            }
        }
    }

    /// Compare values, whatever the exponents.
    pub(super) fn cmp_value(&self, other: &Num) -> Ordering {
        let exponent = self.exponent.min(other.exponent);
        self.aligned(exponent).cmp(&other.aligned(exponent))
    }

    /// The coefficient at a smaller or equal exponent.
    fn aligned(&self, exponent: i64) -> BigInt {
        let shift = u32::try_from(self.exponent - exponent).unwrap_or(u32::MAX);
        &self.coefficient * pow10(shift)
    }

    /// Round to [`PRECISION`] significant digits, half to even.
    fn rounded(self) -> Num {
        let count = digit_count(&self.coefficient);
        if count <= PRECISION {
            return self;
        }
        let drop = count - PRECISION;
        let (mut coefficient, _) = divide_rounded(&self.coefficient, &pow10(drop));
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
        Num::new(self.aligned(0), 0)
    }

    pub(super) fn add(&self, other: &Num) -> Num {
        let exponent = self.exponent.min(other.exponent);
        Num::new(self.aligned(exponent) + other.aligned(exponent), exponent).rounded()
    }

    pub(super) fn mul(&self, other: &Num) -> Num {
        Num::new(
            &self.coefficient * &other.coefficient,
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
        let da = i64::from(digit_count(&self.coefficient));
        let db = i64::from(digit_count(&divisor.coefficient));
        let k = u32::try_from((i64::from(PRECISION) + 2 + db - da).max(0)).unwrap_or(0);
        let numerator = &self.coefficient * pow10(k);
        let (q, r) = numerator.div_rem(&divisor.coefficient);
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
            divide_rounded(&numerator, &(&divisor.coefficient * pow10(drop)));
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
            return Num::new(self.aligned(exponent), exponent);
        }
        let drop = u32::try_from(exponent - self.exponent).unwrap_or(u32::MAX);
        let (coefficient, _) = divide_rounded(&self.coefficient, &pow10(drop));
        Num::new(coefficient, exponent)
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.is_negative() { "-" } else { "" };
        let digits = self.coefficient.abs().to_string();
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
