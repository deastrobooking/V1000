//! Canonical time model.
//!
//! Edit math must be exact and rate-agnostic: `24 frames @ 24 fps` and
//! `30 frames @ 30 fps` are the *same instant* (one second) and must compare
//! equal. Floating-point seconds drift and a frame-count-plus-rate pair cannot
//! be ordered across rates, so positions and durations on the timeline use
//! [`Time`] — an exact rational number of seconds — and frame rates use
//! [`Rational`]. Conversion to/from a frame index is always explicit and names
//! the rate it applies.

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

/// An exact rational number, kept normalized with a positive denominator.
///
/// Used for frame rates (e.g. `24000/1001` for 23.976 fps) and as the backing
/// representation of [`Time`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i64,
    den: i64,
}

impl Rational {
    /// Creates `num/den`, reduced to lowest terms with a positive denominator.
    ///
    /// # Panics
    /// Panics if `den == 0`.
    pub fn new(num: i64, den: i64) -> Self {
        assert!(den != 0, "denominator must be non-zero");
        let sign = if den < 0 { -1 } else { 1 };
        let (n, d) = (i128::from(num) * sign, i128::from(den) * sign);
        let g = gcd(n, d);
        Self {
            num: (n / g) as i64,
            den: (d / g) as i64,
        }
    }

    /// The whole number `n` as `n/1`.
    pub const fn from_int(n: i64) -> Self {
        Self { num: n, den: 1 }
    }

    /// Numerator (sign lives here; denominator is always positive).
    pub fn numerator(self) -> i64 {
        self.num
    }

    /// Denominator (always positive).
    pub fn denominator(self) -> i64 {
        self.den
    }

    /// Lossy conversion to `f64`, for display and GPU upload only.
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    fn from_i128(num: i128, den: i128) -> Self {
        debug_assert!(den != 0);
        let sign = if den < 0 { -1 } else { 1 };
        let (n, d) = (num * sign, den * sign);
        let g = gcd(n, d);
        Self {
            num: (n / g) as i64,
            den: (d / g) as i64,
        }
    }
}

impl Add for Rational {
    type Output = Rational;
    fn add(self, rhs: Rational) -> Rational {
        let num =
            i128::from(self.num) * i128::from(rhs.den) + i128::from(rhs.num) * i128::from(self.den);
        Rational::from_i128(num, i128::from(self.den) * i128::from(rhs.den))
    }
}

impl Sub for Rational {
    type Output = Rational;
    fn sub(self, rhs: Rational) -> Rational {
        self + (-rhs)
    }
}

impl Neg for Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        Self {
            num: -self.num,
            den: self.den,
        }
    }
}

impl Mul for Rational {
    type Output = Rational;
    fn mul(self, rhs: Rational) -> Rational {
        Rational::from_i128(
            i128::from(self.num) * i128::from(rhs.num),
            i128::from(self.den) * i128::from(rhs.den),
        )
    }
}

impl Div for Rational {
    type Output = Rational;
    fn div(self, rhs: Rational) -> Rational {
        assert!(rhs.num != 0, "division by zero");
        Rational::from_i128(
            i128::from(self.num) * i128::from(rhs.den),
            i128::from(self.den) * i128::from(rhs.num),
        )
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // den > 0 for both, so cross-multiplication preserves the inequality.
        (i128::from(self.num) * i128::from(other.den))
            .cmp(&(i128::from(other.num) * i128::from(self.den)))
    }
}

/// An exact position or duration on a timeline, measured in seconds.
///
/// Comparisons and arithmetic are exact and independent of any frame rate.
/// Convert to/from a frame index with [`Time::from_frame`] / [`Time::to_frame`],
/// always naming the rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    seconds: Rational,
}

impl Time {
    /// The zero instant.
    pub const ZERO: Time = Time {
        seconds: Rational::from_int(0),
    };

    /// A time of exactly `seconds`.
    pub fn from_seconds(seconds: Rational) -> Self {
        Self { seconds }
    }

    /// Approximates `seconds` to microsecond precision. For wall-clock input
    /// (e.g. a playhead advanced by frame delta-time); edit points should use
    /// [`Time::from_frame`] to stay exact.
    pub fn from_seconds_f64(seconds: f64) -> Self {
        Self {
            seconds: Rational::new((seconds * 1_000_000.0).round() as i64, 1_000_000),
        }
    }

    /// The exact time of `frame` at the rate `fps`.
    pub fn from_frame(frame: i64, fps: Rational) -> Self {
        Self {
            seconds: Rational::from_int(frame) / fps,
        }
    }

    /// The frame index this time lands on at rate `fps`, rounded to nearest
    /// (half away from zero is not needed; ties round up).
    pub fn to_frame(self, fps: Rational) -> i64 {
        let v = self.seconds * fps; // frames, exact
        let (n, d) = (i128::from(v.numerator()), i128::from(v.denominator())); // d > 0
        let q = n.div_euclid(d);
        let r = n.rem_euclid(d);
        if 2 * r >= d {
            (q + 1) as i64
        } else {
            q as i64
        }
    }

    /// Exact seconds as a [`Rational`].
    pub fn seconds(self) -> Rational {
        self.seconds
    }

    /// Lossy seconds, for display and GPU upload only.
    pub fn as_seconds_f64(self) -> f64 {
        self.seconds.to_f64()
    }
}

impl Add for Time {
    type Output = Time;
    fn add(self, rhs: Time) -> Time {
        Time {
            seconds: self.seconds + rhs.seconds,
        }
    }
}

impl Sub for Time {
    type Output = Time;
    fn sub(self, rhs: Time) -> Time {
        Time {
            seconds: self.seconds - rhs.seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_reduces_and_signs() {
        let r = Rational::new(2, -4);
        assert_eq!(r.numerator(), -1);
        assert_eq!(r.denominator(), 2);
    }

    #[test]
    fn rational_orders_across_denominators() {
        assert!(Rational::new(1, 3) < Rational::new(1, 2));
        assert_eq!(Rational::new(2, 4), Rational::new(1, 2));
    }

    #[test]
    fn same_instant_compares_equal_across_rates() {
        // 24 frames @ 24fps and 30 frames @ 30fps are both exactly 1 second.
        let a = Time::from_frame(24, Rational::new(24, 1));
        let b = Time::from_frame(30, Rational::new(30, 1));
        assert_eq!(a, b);
        assert_eq!(a.as_seconds_f64(), 1.0);
    }

    #[test]
    fn ntsc_rate_is_exact() {
        // 23.976 fps = 24000/1001. 1001 frames is exactly 1001/24000*1001... check round trip.
        let fps = Rational::new(24000, 1001);
        let t = Time::from_frame(48, fps);
        assert_eq!(t.to_frame(fps), 48);
    }

    #[test]
    fn to_frame_rounds_to_nearest() {
        let fps = Rational::new(30, 1);
        // 1.51s -> 45.3 -> 45
        assert_eq!(Time::from_seconds_f64(1.51).to_frame(fps), 45);
        // exactly 0.5 frame ties round up: 0.5/30 s -> frame 0.5 -> 1
        let half = Time::from_seconds(Rational::new(1, 60));
        assert_eq!(half.to_frame(fps), 1);
    }

    #[test]
    fn time_arithmetic_is_exact() {
        let a = Time::from_frame(10, Rational::new(30, 1));
        let b = Time::from_frame(20, Rational::new(30, 1));
        assert_eq!(a + b, Time::from_frame(30, Rational::new(30, 1)));
        assert_eq!(b - a, a);
        assert!(Time::ZERO < a);
    }
}
