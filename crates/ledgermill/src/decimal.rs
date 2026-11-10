//! Exact decimal numbers of any size: an integer mantissa and a count of decimal places, so
//! `-12.50` is `-1250` with 2 places. No floating point anywhere; the places are part of the
//! value because the reference tool reports them.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use std::cmp::Ordering;

/// Places a division (an inferred price, a lot cost from a total) is rounded to.
pub(crate) const DIVISION_PLACES: u32 = 255;
/// Places a reported quantity is rounded to.
pub(crate) const OUTPUT_PLACES: u32 = 10;

/// The mantissa: a machine integer while the value fits in an `i128`, a big integer beyond.
/// Ledger amounts are small, and the integer costs no allocation. It is always `Small` when the
/// value fits, so equal values are equal mantissas and `==` stays structural.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mantissa {
    Small(i128),
    Big(BigInt),
}

impl Default for Mantissa {
    fn default() -> Mantissa {
        Mantissa::Small(0)
    }
}

/// `10^n` for every power that fits in an `i128`.
const POW10: [i128; 39] = {
    let mut table = [1i128; 39];
    let mut i = 1;
    while i < table.len() {
        table[i] = table[i - 1] * 10;
        i += 1;
    }
    table
};

fn pow10_big(n: u32) -> BigInt {
    num_traits::pow(BigInt::from(10u8), usize::try_from(n).unwrap_or(usize::MAX))
}

/// `numerator / denominator` rounded half to even. `denominator` is not zero.
fn div_half_even_big(numerator: &BigInt, denominator: &BigInt) -> BigInt {
    let negative = numerator.is_negative() != denominator.is_negative();
    let (n, d) = (numerator.abs(), denominator.abs());
    let (mut q, r) = n.div_rem(&d);
    let twice: BigInt = r * 2u8;
    if twice > d || (twice == d && q.is_odd()) {
        q += 1u8;
    }
    if negative { -q } else { q }
}

impl Mantissa {
    fn from_big(n: BigInt) -> Mantissa {
        match n.to_i128() {
            Some(small) => Mantissa::Small(small),
            None => Mantissa::Big(n),
        }
    }

    /// A magnitude and a sign, which reach one more value below zero than above it.
    fn from_magnitude(negative: bool, magnitude: u128) -> Mantissa {
        match (i128::try_from(magnitude), negative) {
            (Ok(small), false) => Mantissa::Small(small),
            (Ok(small), true) => Mantissa::Small(-small),
            (Err(_), true) if magnitude == i128::MIN.unsigned_abs() => Mantissa::Small(i128::MIN),
            (Err(_), negative) => {
                let big = BigInt::from(magnitude);
                Mantissa::Big(if negative { -big } else { big })
            }
        }
    }

    fn to_big(&self) -> BigInt {
        match self {
            Mantissa::Small(small) => BigInt::from(*small),
            Mantissa::Big(big) => big.clone(),
        }
    }

    fn pow10(n: u32) -> Mantissa {
        match usize::try_from(n).ok().and_then(|i| POW10.get(i)) {
            Some(small) => Mantissa::Small(*small),
            None => Mantissa::Big(pow10_big(n)),
        }
    }

    fn is_zero(&self) -> bool {
        matches!(self, Mantissa::Small(0))
    }

    fn is_negative(&self) -> bool {
        match self {
            Mantissa::Small(small) => *small < 0,
            Mantissa::Big(big) => big.is_negative(),
        }
    }

    fn negated(&self) -> Mantissa {
        match self {
            Mantissa::Small(small) => small.checked_neg().map_or_else(
                || Mantissa::from_big(-BigInt::from(*small)),
                Mantissa::Small,
            ),
            Mantissa::Big(big) => Mantissa::from_big(-big),
        }
    }

    fn add(&self, other: &Mantissa) -> Mantissa {
        if let (Mantissa::Small(a), Mantissa::Small(b)) = (self, other)
            && let Some(sum) = a.checked_add(*b)
        {
            return Mantissa::Small(sum);
        }
        Mantissa::from_big(self.to_big() + other.to_big())
    }

    fn mul(&self, other: &Mantissa) -> Mantissa {
        if let (Mantissa::Small(a), Mantissa::Small(b)) = (self, other)
            && let Some(product) = a.checked_mul(*b)
        {
            return Mantissa::Small(product);
        }
        Mantissa::from_big(self.to_big() * other.to_big())
    }

    fn mul_pow10(&self, n: u32) -> Mantissa {
        if n == 0 {
            return self.clone();
        }
        self.mul(&Mantissa::pow10(n))
    }

    /// `self / divisor` rounded half to even. `divisor` is not zero.
    fn div_half_even(&self, divisor: &Mantissa) -> Mantissa {
        if let (Mantissa::Small(n), Mantissa::Small(d)) = (self, divisor) {
            // Magnitudes in a `u128`, since `i128::MIN` has no positive twin. The remainder is
            // below the divisor, at most 2^127, so doubling it cannot overflow.
            let (n_abs, d_abs) = (n.unsigned_abs(), d.unsigned_abs());
            let (mut q, r) = (n_abs / d_abs, n_abs % d_abs);
            let twice = r * 2;
            if twice > d_abs || (twice == d_abs && q % 2 == 1) {
                q += 1;
            }
            return Mantissa::from_magnitude((*n < 0) != (*d < 0), q);
        }
        Mantissa::from_big(div_half_even_big(&self.to_big(), &divisor.to_big()))
    }

    fn cmp(&self, other: &Mantissa) -> Ordering {
        match (self, other) {
            (Mantissa::Small(a), Mantissa::Small(b)) => a.cmp(b),
            (Mantissa::Big(a), Mantissa::Big(b)) => a.cmp(b),
            // A big mantissa lies outside the `i128` range, on the side of its sign.
            (Mantissa::Small(_), Mantissa::Big(b)) => {
                if b.is_negative() {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            (Mantissa::Big(a), Mantissa::Small(_)) => {
                if a.is_negative() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Decimal {
    mantissa: Mantissa,
    places: u32,
}

/// Which term a sum of two zeros keeps (see [`Decimal::combine`]).
#[derive(Debug, Clone, Copy)]
pub(crate) enum ZeroTie {
    KeepEarlier,
    KeepLater,
}

impl Decimal {
    pub(crate) fn new(mantissa: i128, places: u32) -> Decimal {
        Decimal::with_mantissa(Mantissa::Small(mantissa), places)
    }

    fn with_mantissa(mantissa: Mantissa, places: u32) -> Decimal {
        Decimal { mantissa, places }
    }

    pub(crate) fn zero() -> Decimal {
        Decimal::new(0, 0)
    }

    /// `digits` holds only ASCII digits (at least one).
    pub(crate) fn from_digits(digits: &str, places: u32) -> Option<Decimal> {
        let mantissa = match digits.parse::<i128>() {
            Ok(small) => Mantissa::Small(small),
            Err(_) => Mantissa::from_big(digits.parse().ok()?),
        };
        Some(Decimal::with_mantissa(mantissa, places))
    }

    /// A decimal string as the interface prints one: an optional sign, digits with an optional
    /// point, and an optional exponent (`1E-7`). `None` for anything else.
    pub(crate) fn parse(text: &str) -> Option<Decimal> {
        let (negative, rest) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (number, exponent) = match rest.split_once(['E', 'e']) {
            Some((number, exponent)) => (number, exponent.parse::<i64>().ok()?),
            None => (rest, 0),
        };
        let (int, frac) = number.split_once('.').unwrap_or((number, ""));
        let digits = format!("{int}{frac}");
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let value =
            Decimal::from_digits(&digits, u32::try_from(frac.len()).ok()?)?.with_exponent(exponent);
        Some(if negative { value.negated() } else { value })
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
        Decimal::with_mantissa(self.mantissa.negated(), self.places)
    }

    fn scaled_to(&self, places: u32) -> Mantissa {
        self.mantissa.mul_pow10(places.saturating_sub(self.places))
    }

    /// Exact sum; the result has as many places as the more precise term.
    pub(crate) fn add(&self, other: &Decimal) -> Decimal {
        let places = self.places.max(other.places);
        Decimal::with_mantissa(self.scaled_to(places).add(&other.scaled_to(places)), places)
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
        Decimal::with_mantissa(
            self.mantissa.mul(&other.mantissa),
            self.places + other.places,
        )
    }

    /// Multiply by `10^exponent`, keeping the value exact.
    pub(crate) fn with_exponent(&self, exponent: i64) -> Decimal {
        let places = i64::from(self.places) - exponent;
        match u32::try_from(places) {
            Ok(places) => Decimal::with_mantissa(self.mantissa.clone(), places),
            Err(_) => {
                let shift = u32::try_from(-places).unwrap_or(u32::MAX);
                Decimal::with_mantissa(self.mantissa.mul_pow10(shift), 0)
            }
        }
    }

    /// The same value with at least `places` places (zeros added; more places are kept).
    pub(crate) fn padded_to(&self, places: u32) -> Decimal {
        let places = places.max(self.places);
        Decimal::with_mantissa(self.scaled_to(places), places)
    }

    /// The same value without trailing zeros in the decimal part; zero has no places.
    pub(crate) fn normalized(&self) -> Decimal {
        let mut places = self.places;
        let mantissa = match &self.mantissa {
            Mantissa::Small(0) => return Decimal::zero(),
            Mantissa::Small(small) => {
                let mut small = *small;
                while places > 0 && small % 10 == 0 {
                    small /= 10;
                    places -= 1;
                }
                Mantissa::Small(small)
            }
            Mantissa::Big(big) => {
                let ten = BigInt::from(10u8);
                let mut big = big.clone();
                while places > 0 {
                    let (q, r) = big.div_rem(&ten);
                    if !r.is_zero() {
                        break;
                    }
                    big = q;
                    places -= 1;
                }
                Mantissa::from_big(big)
            }
        };
        Decimal::with_mantissa(mantissa, places)
    }

    /// Round half to even to at most `places` places; fewer places are left as they are.
    pub(crate) fn rounded(&self, places: u32) -> Decimal {
        if self.places <= places {
            return self.clone();
        }
        let mantissa = self
            .mantissa
            .div_half_even(&Mantissa::pow10(self.places - places));
        Decimal::with_mantissa(mantissa, places)
    }

    /// `self / divisor` rounded half to even to `places` places; `None` when dividing by zero.
    pub(crate) fn divided(&self, divisor: &Decimal, places: u32) -> Option<Decimal> {
        if divisor.is_zero() {
            return None;
        }
        let numerator = self.mantissa.mul_pow10(divisor.places + places);
        let denominator = divisor.mantissa.mul_pow10(self.places);
        Some(Decimal::with_mantissa(
            numerator.div_half_even(&denominator),
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
        let places = usize::try_from(self.places).unwrap_or(usize::MAX);
        let magnitude = match &self.mantissa {
            Mantissa::Small(small) => small.unsigned_abs(),
            Mantissa::Big(big) => {
                let digits = big.abs().to_string();
                if places == 0 {
                    return write!(f, "{sign}{digits}");
                }
                let padded = format!("{digits:0>width$}", width = places + 1);
                let (int, frac) = padded.split_at(padded.len() - places);
                return write!(f, "{sign}{int}.{frac}");
            }
        };
        if places == 0 {
            return write!(f, "{sign}{magnitude}");
        }
        // Past 10^38 the integer part is zero: a magnitude is at most 2^127.
        match 10u128.checked_pow(self.places) {
            Some(scale) => write!(
                f,
                "{sign}{}.{:0>places$}",
                magnitude / scale,
                magnitude % scale
            ),
            None => write!(f, "{sign}0.{magnitude:0>places$}"),
        }
    }
}
