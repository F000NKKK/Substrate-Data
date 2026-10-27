//! A number literal: an integer when it has no fraction or exponent and fits.

use super::build::Build;
use super::parser::Parser;
use super::build::Failure;

/// Integers of up to this many digits fit an `i64` without a check.
const SAFE_DIGITS: usize = 18;

impl<'t, B: Build<'t>, const SPANS: bool> Parser<'t, B, SPANS> {
    /// Skips ASCII digits; how many.
    fn digits(&mut self) -> usize {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        self.i - start
    }

    pub(super) fn number(&mut self) -> Result<B::Value, Failure> {
        let start = self.i;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.i += 1;
        }
        let int_start = self.i;
        let int_digits = self.digits();
        if int_digits == 0 {
            return Err(self.error("expected digits"));
        }
        if self.src[int_start] == b'0' && int_digits > 1 {
            return Err(self.error_at(int_start, "numbers don't start with 0"));
        }
        let float = self.fraction_and_exponent()?;
        if !float {
            if int_digits <= SAFE_DIGITS {
                let n = self.src[int_start..self.i]
                    .iter()
                    .fold(0_i64, |n, &d| n * 10 + i64::from(d - b'0'));
                return Ok(self.build.int(if negative { -n } else { n }));
            }
            if let Ok(n) = self.text[start..self.i].parse::<i64>() {
                return Ok(self.build.int(n));
            }
        }
        let f = self.text[start..self.i]
            .parse::<f64>()
            .map_err(|_| self.error_at(start, "invalid number"))?;
        Ok(self.build.float(f))
    }

    /// Skips a number's `.digits` and `e±digits` parts; whether it had one.
    fn fraction_and_exponent(&mut self) -> Result<bool, Failure> {
        let mut float = false;
        if self.peek() == Some(b'.') {
            self.i += 1;
            float = true;
            if self.digits() == 0 {
                return Err(self.error("expected digits after `.`"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            float = true;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                return Err(self.error("expected digits in the exponent"));
            }
        }
        Ok(float)
    }
}
