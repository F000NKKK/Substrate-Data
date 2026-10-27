//! Conversions between the model's integers and floats, in one place:
//! an integer read as a number is the nearest float, and a float reads as
//! an integer only when it is whole and exact.

/// The largest float magnitude read as an integer: below 2^53, every whole
/// float is an exact integer.
const EXACT_LIMIT: f64 = 9.0e15;

/// `n` as the nearest float (exact below 2^53, as JSON readers have it).
#[expect(
    clippy::cast_precision_loss,
    reason = "a number read as a float is the nearest one; exact below 2^53"
)]
pub(crate) fn int_to_f64(n: i64) -> f64 {
    n as f64
}

/// `f` as an integer, if it is whole and below 2^53 (so exact).
#[expect(
    clippy::cast_possible_truncation,
    reason = "checked whole and below 2^53 first"
)]
pub(crate) fn whole_to_i64(f: f64) -> Option<i64> {
    (f.fract() == 0.0 && f.abs() < EXACT_LIMIT).then_some(f as i64)
}
