//! An integer's digits, without going through `fmt`.

/// Every two-digit number, `00` to `99`, back to back.
const PAIRS: &[u8; 200] = b"0001020304050607080910111213141516171819\
2021222324252627282930313233343536373839\
4041424344454647484950515253545556575859\
6061626364656667686970717273747576777879\
8081828384858687888990919293949596979899";

/// The most digits of a `u64`.
pub(super) const MAX_DIGITS: usize = 20;

/// Appends `n` in decimal.
pub(super) fn push_int(n: i64, out: &mut Vec<u8>) {
    if n < 0 {
        out.push(b'-');
    }
    let mut buf = [0_u8; MAX_DIGITS];
    out.extend_from_slice(digits_of(n.unsigned_abs(), &mut buf));
}

/// The decimal digits of `n`, written at the end of `buf`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "each cast is of a value below 100, or an index below 200"
)]
pub(super) fn digits_of(mut n: u64, buf: &mut [u8; MAX_DIGITS]) -> &[u8] {
    let mut at = MAX_DIGITS;
    while n >= 100 {
        let pair = (n % 100) as usize * 2;
        n /= 100;
        at -= 2;
        buf[at..at + 2].copy_from_slice(&PAIRS[pair..pair + 2]);
    }
    if n >= 10 {
        let pair = n as usize * 2;
        at -= 2;
        buf[at..at + 2].copy_from_slice(&PAIRS[pair..pair + 2]);
    } else {
        at -= 1;
        buf[at] = b'0' + n as u8;
    }
    &buf[at..]
}

#[cfg(test)]
mod tests {
    use super::push_int;

    fn text(n: i64) -> String {
        let mut out = Vec::new();
        push_int(n, &mut out);
        String::from_utf8(out).unwrap_or_default()
    }

    #[test]
    fn matches_display_across_every_digit_count() {
        let mut n: i64 = 1;
        loop {
            for v in [n - 1, n, n + 1, -n, -n - 1, 7 * n] {
                assert_eq!(text(v), v.to_string());
            }
            match n.checked_mul(10) {
                Some(next) => n = next,
                None => break,
            }
        }
        for v in [0, 9, 10, 99, 100, i64::MAX, i64::MIN] {
            assert_eq!(text(v), v.to_string());
        }
    }
}
