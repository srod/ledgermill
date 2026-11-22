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

/// Largest exponent expanded into plain digits (`plain()` / `without_positive_exponent()`).
const MAX_PLAIN_EXPONENT: i64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Num {
    coefficient: Coef,
    exponent: i64,
    /// A zero that carries a minus sign (`0.00 * -1`): the arithmetic keeps it and it prints as
    /// `-0.00`. Always false for a number that is not zero.
    negative_zero: bool,
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
        if shift == 0 || self.is_zero() {
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
    BigInt::from(10u8).pow(n)
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
            negative_zero: false,
        }
    }

    /// The same number as a zero with a minus sign, when it is a zero and `negative` is set.
    fn signed_zero(mut self, negative: bool) -> Num {
        self.negative_zero = negative && self.coefficient.is_zero();
        self
    }

    /// The sign the arithmetic goes by: a zero can be negative too.
    fn has_minus(&self) -> bool {
        self.coefficient.is_negative() || self.negative_zero
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
        // A minus sign in front of a zero stays (`-0.00`): it is a zero that prints with its sign.
        Some(Num::of(coefficient, exponent.checked_sub(places)?).signed_zero(negative))
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

    /// The unary minus: a zero stays a plain zero (`-(0.00 * -1)` is `0.00`).
    pub(super) fn negated(&self) -> Num {
        Num::of(self.coefficient.negated(), self.exponent)
    }

    /// The subtrahend as the subtraction adds it: the sign flips, a zero's too (`-0.00 - 0.00`
    /// is `-0.00`).
    fn flipped(&self) -> Num {
        Num::of(self.coefficient.negated(), self.exponent).signed_zero(!self.has_minus())
    }

    /// `self - other`.
    pub(super) fn sub(&self, other: &Num) -> Num {
        self.add(&other.flipped())
    }

    pub(super) fn abs(&self) -> Num {
        Num::of(self.coefficient.abs(), self.exponent)
    }

    /// Decimal places written: zero for a number with a positive exponent.
    pub(super) fn places(&self) -> u32 {
        self.exponent
            .checked_neg()
            .and_then(|p| u32::try_from(p).ok())
            .unwrap_or(0)
    }

    /// Canonical `(signed_digits, exponent)` with trailing zeros stripped, so equal values map to
    /// the same key (`100`, `100.00` -> `("1", 2)`; any zero -> `("0", 0)`).
    pub(super) fn canonical(&self) -> (String, i64) {
        if self.is_zero() {
            return ("0".to_string(), 0);
        }
        let raw = self.coefficient.digits();
        let trimmed = raw.trim_end_matches('0');
        let stripped = i64::try_from(raw.len() - trimmed.len()).unwrap_or(0);
        let digits = if self.is_negative() {
            format!("-{trimmed}")
        } else {
            trimmed.to_string()
        };
        (digits, self.exponent.saturating_add(stripped))
    }

    /// The value as an integer mantissa and a count of places, for `rust_decimal`; `None` when
    /// it does not fit.
    pub(super) fn scaled_i128(&self) -> Option<(i128, u32)> {
        if let Some(places) = self
            .exponent
            .checked_neg()
            .and_then(|p| u32::try_from(p).ok())
        {
            let mantissa = match &self.coefficient {
                Coef::Small(small) => i128::from(*small),
                Coef::Big(big) => big.to_i128()?,
            };
            Some((mantissa, places))
        } else {
            let shift = u32::try_from(self.exponent).ok()?;
            if shift > 60 {
                return None;
            }
            let mantissa = self.coefficient.to_big() * pow10(shift);
            Some((mantissa.to_i128()?, 0))
        }
    }

    /// Position of the most significant digit: `exponent + digit_count - 1`.
    fn adjusted_exponent(&self) -> i64 {
        self.exponent
            .saturating_add(i64::from(self.coefficient.digit_count()) - 1)
    }

    /// Compare values, whatever the exponents, without scaling across large exponent gaps.
    pub(super) fn cmp_value(&self, other: &Num) -> Ordering {
        match (self.is_zero(), other.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if other.is_negative() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if self.is_negative() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }
        match (self.is_negative(), other.is_negative()) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let by_scale = self.adjusted_exponent().cmp(&other.adjusted_exponent());
        if by_scale != Ordering::Equal {
            return if self.is_negative() {
                by_scale.reverse()
            } else {
                by_scale
            };
        }
        let exponent = self.exponent.min(other.exponent);
        match self.exponent.cmp(&other.exponent) {
            Ordering::Equal => self.coefficient.cmp(&other.coefficient),
            Ordering::Less => self.coefficient.cmp(&other.aligned(exponent)),
            Ordering::Greater => self.aligned(exponent).cmp(&other.coefficient),
        }
    }

    /// The coefficient at a smaller or equal exponent.
    fn aligned(&self, exponent: i64) -> Coef {
        if self.is_zero() {
            return Coef::Small(0);
        }
        let shift = u32::try_from(self.exponent.saturating_sub(exponent)).unwrap_or(u32::MAX);
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
        let mut exponent = self.exponent.saturating_add(i64::from(drop));
        if digit_count(&coefficient) > PRECISION {
            coefficient /= 10u8;
            exponent = exponent.saturating_add(1);
        }
        Num::new(coefficient, exponent)
    }

    /// The same value written without a positive exponent (`2.0E+2` becomes `200`).
    pub(super) fn without_positive_exponent(&self) -> Num {
        if self.exponent <= 0 {
            return self.clone();
        }
        if self.is_zero() {
            return Num::of(Coef::Small(0), 0).signed_zero(self.negative_zero);
        }
        if self.exponent > MAX_PLAIN_EXPONENT {
            return self.clone();
        }
        Num::of(self.aligned(0), 0).signed_zero(self.negative_zero)
    }

    pub(super) fn add(&self, other: &Num) -> Num {
        // A zero reached by cancelling is plain; two zeros are negative only when both are.
        let negative_zero = self.negative_zero && other.negative_zero;
        match (self.is_zero(), other.is_zero()) {
            (true, true) => {
                return Num::of(Coef::Small(0), self.exponent.min(other.exponent))
                    .signed_zero(negative_zero);
            }
            (true, false) => return add_zero(other, self.exponent),
            (false, true) => return add_zero(self, other.exponent),
            (false, false) => {}
        }
        // When one term is more than PRECISION + 2 orders of magnitude smaller, it can only
        // contribute a sticky bit at PRECISION + 2 digits below the larger term; avoid `pow10`
        // over huge exponent gaps.
        let (self_adj, other_adj) = (self.adjusted_exponent(), other.adjusted_exponent());
        let (big, big_adj, small, gap) = if self_adj >= other_adj {
            (self, self_adj, other, self_adj.saturating_sub(other_adj))
        } else {
            (other, other_adj, self, other_adj.saturating_sub(self_adj))
        };
        if gap > i64::from(PRECISION) + 2 {
            let target = big
                .exponent
                .min(big_adj.saturating_sub(i64::from(PRECISION) + 2));
            let sticky = Coef::Small(if small.is_negative() { -1 } else { 1 });
            let sum = big.aligned(target).add(&sticky);
            return Num::of(sum, target).rounded();
        }
        let exponent = self.exponent.min(other.exponent);
        // The term that already has the finer exponent needs no copy.
        let sum = match self.exponent.cmp(&other.exponent) {
            Ordering::Equal => self.coefficient.add(&other.coefficient),
            Ordering::Less => self.coefficient.add(&other.aligned(exponent)),
            Ordering::Greater => self.aligned(exponent).add(&other.coefficient),
        };
        Num::of(sum, exponent).rounded().signed_zero(negative_zero)
    }

    pub(super) fn mul(&self, other: &Num) -> Num {
        Num::of(
            self.coefficient.mul(&other.coefficient),
            self.exponent.saturating_add(other.exponent),
        )
        .rounded()
        .signed_zero(self.has_minus() != other.has_minus())
    }

    /// `self / divisor`; `None` when the divisor is zero.
    pub(super) fn div(&self, divisor: &Num) -> Option<Num> {
        if divisor.is_zero() {
            return None;
        }
        let ideal = self.exponent.saturating_sub(divisor.exponent);
        if self.is_zero() {
            return Some(
                Num::new(BigInt::zero(), ideal)
                    .signed_zero(self.has_minus() != divisor.has_minus()),
            );
        }
        // Scale so the integer quotient has at least PRECISION + 1 digits.
        let da = i64::from(self.coefficient.digit_count());
        let db = i64::from(divisor.coefficient.digit_count());
        let k = u32::try_from((i64::from(PRECISION) + 2 + db - da).max(0)).unwrap_or(0);
        let divisor_coefficient = divisor.coefficient.to_big();
        let numerator = self.coefficient.to_big() * pow10(k);
        let (q, r) = numerator.div_rem(&divisor_coefficient);
        let digits = digit_count(&q);
        let exponent = ideal.saturating_sub(i64::from(k));
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
                exponent = exponent.saturating_add(1);
            }
            return Some(Num::new(coefficient, exponent).rounded());
        }
        let drop = digits.saturating_sub(PRECISION);
        let (mut coefficient, _) =
            divide_rounded(&numerator, &(&divisor_coefficient * pow10(drop)));
        let mut exponent = exponent.saturating_add(i64::from(drop));
        if digit_count(&coefficient) > PRECISION {
            coefficient /= 10u8;
            exponent = exponent.saturating_add(1);
        }
        Some(Num::new(coefficient, exponent))
    }

    /// Round half to even to exactly `places` places, padding with zeros when it has fewer.
    pub(super) fn quantized(&self, places: u32) -> Num {
        let exponent = -i64::from(places);
        if self.is_zero() {
            return Num::of(Coef::Small(0), exponent).signed_zero(self.negative_zero);
        }
        if self.exponent >= exponent {
            return Num::of(self.aligned(exponent), exponent).signed_zero(self.negative_zero);
        }
        let drop = exponent.saturating_sub(self.exponent);
        // Strictly smaller than half a ULP at `exponent`: rounds to zero without `pow10(drop)`.
        if drop > i64::from(self.coefficient.digit_count()) + 1 {
            return Num::of(Coef::Small(0), exponent);
        }
        let drop = u32::try_from(drop).unwrap_or(u32::MAX);
        let (coefficient, _) = divide_rounded(&self.coefficient.to_big(), &pow10(drop));
        Num::new(coefficient, exponent)
    }
}

/// Add a zero at `zero_exp` to a non-zero `nz`, preserving exact places up to `PRECISION` digits
/// without allocating `pow10` across huge exponent gaps.
fn add_zero(nz: &Num, zero_exp: i64) -> Num {
    if zero_exp >= nz.exponent {
        return nz.clone().rounded();
    }
    let min_exp = nz
        .adjusted_exponent()
        .saturating_sub(i64::from(PRECISION) - 1);
    let target = zero_exp.max(min_exp);
    Num::of(nz.aligned(target), target).rounded()
}

impl Num {
    /// The value in full, never with an exponent: `0.0000005`, not `5E-7`. Padding narrations
    /// print numbers this way (spec/beancount/pad.md).
    pub(super) fn plain(&self) -> String {
        let sign = if self.has_minus() { "-" } else { "" };
        if self.is_zero() && self.exponent >= 0 {
            return format!("{sign}0");
        }
        if self.exponent.unsigned_abs() > u64::try_from(MAX_PLAIN_EXPONENT).unwrap_or(u64::MAX) {
            return self.to_string();
        }
        let digits = self.coefficient.digits();
        let body = match self
            .exponent
            .checked_neg()
            .and_then(|p| usize::try_from(p).ok())
        {
            Some(places) if places > 0 => {
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
        let sign = if self.has_minus() { "-" } else { "" };
        let digits = self.coefficient.digits();
        let len = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        let adjusted = self.exponent.saturating_add(len - 1);
        if self.exponent <= 0 && adjusted >= -6 {
            if self.exponent == 0 {
                return write!(f, "{sign}{digits}");
            }
            let places = self.exponent.saturating_neg();
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
            adjusted.unsigned_abs()
        )
    }
}
