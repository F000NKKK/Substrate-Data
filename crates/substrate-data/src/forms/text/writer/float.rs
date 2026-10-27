//! A float's spelling: Rust's `{:?}` (the specification's), from `zmij`'s
//! shortest digits instead of `fmt`'s, which cost several times as much.

use super::integer::{MAX_DIGITS, digits_of};

/// Positional spelling from this decimal exponent (of the first digit) on.
const POSITIONAL_FROM: i32 = -4;

/// Exponent spelling from this decimal exponent on.
const EXPONENT_FROM: i32 = 16;

/// Past this many binary places, an exact value has more than 18 decimal
/// digits (`5^26` is below 1e18), and a spelling of at most 17 is no tie.
const MAX_TIE_PLACES: u32 = 26;

/// The most digits a shortest `f64` has, with room for a point and zeros.
const SCRATCH: usize = 40;

/// Appends `f`, or `null` for a NaN or an infinity (which text can't spell).
pub(super) fn push_float(f: f64, out: &mut Vec<u8>) {
    if f.is_finite() {
        push_finite(f, out);
    } else {
        out.extend_from_slice(b"null");
    }
}

/// A finite float in the shortest spelling that reads back as exactly it,
/// and as a float: `1.0`, `0.1`, `1e-7`, `1e16` (never `10000000000000000`,
/// which would read back as an integer).
pub(crate) fn float_string(f: f64) -> String {
    let mut out = Vec::new();
    push_finite(f, &mut out);
    String::from_utf8(out).unwrap_or_default()
}

fn push_finite(f: f64, out: &mut Vec<u8>) {
    if f == 0.0 {
        out.extend_from_slice(if f.is_sign_negative() {
            b"-0.0"
        } else {
            b"0.0"
        });
        return;
    }
    let mut buffer = zmij::Buffer::new();
    let shortest = buffer.format_finite(f);
    let tie = tie_digits(f, shortest);
    // The common spelling, positional and with the point: zmij's too.
    if tie.is_none() && f.abs() >= 1e-4 && !shortest.as_bytes().contains(&b'e') {
        out.extend_from_slice(shortest.as_bytes());
        return;
    }
    respell(shortest, tie, out);
}

/// Where `f` is exactly halfway between two spellings of the shortest
/// length, the digits `fmt` takes: the upper one (zmij takes the even one,
/// and the specification says `fmt`'s). `None` when it is not halfway.
///
/// That takes `f` to be a fraction of a power of two whose exact decimal
/// digits are one more than `shortest`'s, the last a 5: an integer, or a
/// fraction with more than 18 digits (any with 27 or more binary places),
/// never is.
fn tie_digits(f: f64, shortest: &str) -> Option<u64> {
    let bits = f.to_bits();
    let biased = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1 << 52), biased - 1075)
    };
    let places = mantissa.trailing_zeros();
    let binary_places = u32::try_from(-(exponent + i32::try_from(places).unwrap_or(0))).ok()?;
    if binary_places == 0 || binary_places > MAX_TIE_PLACES {
        return None;
    }
    // Exactly the digits of `mantissa * 5^places` (no trailing zero: odd).
    let exact = u128::from(mantissa >> places) * u128::from(5_u64.pow(binary_places));
    if usize::try_from(exact.ilog10()).ok()? != significant_digits(shortest) {
        return None;
    }
    u64::try_from(exact / 10 + 1).ok()
}

/// How many digits `shortest` has, not counting the zeros that only place it.
fn significant_digits(shortest: &str) -> usize {
    let mut first = None;
    let mut last = 0;
    let digits = shortest
        .bytes()
        .take_while(|&b| b != b'e')
        .filter(u8::is_ascii_digit);
    for (at, digit) in digits.enumerate() {
        if digit != b'0' {
            first.get_or_insert(at);
            last = at;
        }
    }
    first.map_or(0, |first| last - first + 1)
}

/// `shortest` (zmij's spelling) in Rust's: the same digits, placed by
/// Rust's thresholds.
fn respell(shortest: &str, tie: Option<u64>, out: &mut Vec<u8>) {
    let (negative, unsigned) = match shortest.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, shortest),
    };
    let (mantissa, exponent) = unsigned.split_once('e').unwrap_or((unsigned, "0"));
    let exponent: i32 = exponent.trim_start_matches('+').parse().unwrap_or(0);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut scratch = [0_u8; SCRATCH];
    let all = whole.bytes().chain(fraction.bytes());
    let mut len = 0;
    for (slot, digit) in scratch.iter_mut().zip(all) {
        *slot = digit;
        len += 1;
    }
    let leading = scratch[..len].iter().take_while(|&&d| d == b'0').count();
    let digits = &scratch[leading..len];
    let digits = trim_zeros(digits);
    // Where the point goes, counted from the first digit.
    let mut point =
        i32::try_from(whole.len()).unwrap_or(0) + exponent - i32::try_from(leading).unwrap_or(0);
    if negative {
        out.push(b'-');
    }
    let mut upper = [0_u8; MAX_DIGITS];
    let Some(tie) = tie else {
        return place(digits, point, out);
    };
    // The upper spelling of a tie: the same length, or one more if it carried.
    let upper = digits_of(tie, &mut upper);
    point += i32::try_from(upper.len() - digits.len()).unwrap_or(0);
    place(trim_zeros(upper), point, out);
}

/// `digits` without the zeros at its end.
fn trim_zeros(digits: &[u8]) -> &[u8] {
    &digits[..digits.iter().rposition(|&d| d != b'0').map_or(0, |p| p + 1)]
}

/// `digits` with the decimal point `point` digits in, as Rust spells it.
fn place(digits: &[u8], point: i32, out: &mut Vec<u8>) {
    let exponent = point - 1;
    if !(POSITIONAL_FROM..EXPONENT_FROM).contains(&exponent) {
        let (first, rest) = digits.split_at(1);
        out.extend_from_slice(first);
        if !rest.is_empty() {
            out.push(b'.');
            out.extend_from_slice(rest);
        }
        out.push(b'e');
        out.extend_from_slice(exponent.to_string().as_bytes());
        return;
    }
    let count = i32::try_from(digits.len()).unwrap_or(0);
    if point <= 0 {
        out.extend_from_slice(b"0.");
        out.extend(std::iter::repeat_n(
            b'0',
            usize::try_from(-point).unwrap_or(0),
        ));
        out.extend_from_slice(digits);
    } else if point >= count {
        out.extend_from_slice(digits);
        out.extend(std::iter::repeat_n(
            b'0',
            usize::try_from(point - count).unwrap_or(0),
        ));
        out.extend_from_slice(b".0");
    } else {
        let (whole, fraction) = digits.split_at(usize::try_from(point).unwrap_or(0));
        out.extend_from_slice(whole);
        out.push(b'.');
        out.extend_from_slice(fraction);
    }
}

#[cfg(test)]
mod tests {
    use super::float_string;

    /// A small deterministic generator.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "a narrowing is what is wanted"
    )]
    fn single_of(f: f64) -> f32 {
        f as f32
    }

    fn same_as_debug(f: f64) {
        assert_eq!(
            float_string(f),
            format!("{f:?}"),
            "bits {:#018x}",
            f.to_bits()
        );
    }

    #[test]
    fn the_spellings_the_format_names() {
        for (f, text) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (0.1, "0.1"),
            (0.0001, "0.0001"),
            (0.000_01, "1e-5"),
            (1e-7, "1e-7"),
            (123_456_789_012_345.0, "123456789012345.0"),
            (1e16, "1e16"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e308"),
            (-2.5e-5, "-2.5e-5"),
            (1.5e300, "1.5e300"),
        ] {
            assert_eq!(float_string(f), text);
            same_as_debug(f);
        }
    }

    #[test]
    fn edges_of_the_thresholds_and_neighbours_of_every_power_of_ten() {
        for exp in -330..=308 {
            let power: f64 = format!("1e{exp}").parse().unwrap_or(1.0);
            for bits in [
                power.to_bits().wrapping_sub(1),
                power.to_bits(),
                power.to_bits() + 1,
            ] {
                let f = f64::from_bits(bits);
                if f.is_finite() {
                    same_as_debug(f);
                    same_as_debug(-f);
                }
            }
        }
        for exp in 0..1023 {
            let f = 2.0_f64.powi(exp);
            same_as_debug(f);
            same_as_debug(f64::from_bits(f.to_bits() + 1));
        }
    }

    #[test]
    fn any_bit_pattern_spells_as_debug_does() {
        let mut rng = Lcg(5);
        for _ in 0..300_000 {
            let f = f64::from_bits(rng.next());
            if f.is_finite() {
                same_as_debug(f);
            }
        }
    }

    #[test]
    fn short_decimals_and_whole_numbers_spell_as_debug_does() {
        let mut rng = Lcg(9);
        for _ in 0..100_000 {
            let n = f64::from(u32::try_from(rng.next() >> 33).unwrap_or(0));
            for scale in [1.0, 10.0, 100.0, 1e3, 1e6, 1e10, 1e-3, 1e-6, 1e-9] {
                same_as_debug(n / scale);
                same_as_debug(n * scale);
            }
        }
    }

    /// Halfway cases: a widened `f32` is a fraction of a power of two, many of
    /// whose exact values end in a 5 one digit past the shortest spelling.
    #[test]
    fn widened_singles_spell_as_debug_does() {
        let mut rng = Lcg(3);
        for _ in 0..200_000 {
            let single = f32::from_bits(u32::try_from(rng.next() >> 32).unwrap_or(0));
            if single.is_finite() {
                same_as_debug(f64::from(single));
            }
            let unit = f64::from(u32::try_from(rng.next() >> 40).unwrap_or(0)) / 16_777_216.0;
            same_as_debug(f64::from(single_of(unit * 100.0)));
            same_as_debug(unit * 4096.0);
        }
    }
}
