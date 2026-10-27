//! A string literal: its plain runs, its escapes.

use std::borrow::Cow;

use super::build::{Build, Failure};
use super::parser::Parser;
use crate::forms::text::scan;

/// `\\u` escapes are hexadecimal.
const HEX_RADIX: u32 = 16;

impl<'t, B: Build<'t>, const SPANS: bool> Parser<'t, B, SPANS> {
    /// A string literal's text: borrowed from the source when it has no
    /// escape, built when it has.
    pub(super) fn string(&mut self) -> Result<Cow<'t, str>, Failure> {
        let open = self.i;
        self.i += 1;
        let text = self.text;
        let Some(n) = scan::string_stop(&self.src[self.i..]) else {
            return Err(self.error_at(open, "unclosed string"));
        };
        if self.src[self.i + n] == b'"' {
            let plain = &text[self.i..self.i + n];
            self.i += n + 1;
            return Ok(Cow::Borrowed(plain));
        }
        self.escaped(open)
    }

    /// The rest of a string that has an escape or an error in it.
    fn escaped(&mut self, open: usize) -> Result<Cow<'t, str>, Failure> {
        let mut out = String::new();
        loop {
            let Some(n) = scan::string_stop(&self.src[self.i..]) else {
                return Err(self.error_at(open, "unclosed string"));
            };
            out.push_str(&self.text[self.i..self.i + n]);
            self.i += n;
            match self.src[self.i] {
                b'"' => {
                    self.i += 1;
                    return Ok(Cow::Owned(out));
                }
                b'\\' => self.escape(&mut out)?,
                b'\n' => return Err(self.error("line break in a string (write `\\n`)")),
                _ => return Err(self.error("control character in a string")),
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), Failure> {
        let at = self.i;
        self.i += 1;
        let c = self.peek().ok_or_else(|| self.error("unclosed string"))?;
        self.i += 1;
        match c {
            b'"' => out.push('"'),
            b'\\' => out.push('\\'),
            b'/' => out.push('/'),
            b'b' => out.push('\u{8}'),
            b'f' => out.push('\u{c}'),
            b'n' => out.push('\n'),
            b'r' => out.push('\r'),
            b't' => out.push('\t'),
            b'u' => {
                let ch = self.unicode(at)?;
                out.push(ch);
            }
            _ => {
                return Err(self.error_at(
                    at,
                    "unknown escape (`\\n`, `\\t`, `\\\"`, `\\\\`, `\\uXXXX`, ...)",
                ));
            }
        }
        Ok(())
    }

    /// The character of a `\\u` escape (`at` is the backslash), a surrogate
    /// pair's two halves together.
    fn unicode(&mut self, at: usize) -> Result<char, Failure> {
        let hi = self.hex4(at)?;
        let ch = if (0xD800..0xDC00).contains(&hi) {
            if !self.src[self.i..].starts_with(b"\\u") {
                return Err(self.error_at(at, "unpaired surrogate in `\\u` escape"));
            }
            self.i += 2;
            let lo = self.hex4(at)?;
            if !(0xDC00..0xE000).contains(&lo) {
                return Err(self.error_at(at, "unpaired surrogate in `\\u` escape"));
            }
            char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))
        } else {
            char::from_u32(hi)
        };
        ch.ok_or_else(|| self.error_at(at, "unpaired surrogate in `\\u` escape"))
    }

    fn hex4(&mut self, at: usize) -> Result<u32, Failure> {
        let digits = self
            .text
            .get(self.i..self.i + 4)
            .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()));
        let d = digits.ok_or_else(|| self.error_at(at, "`\\u` needs four hex digits"))?;
        let code = u32::from_str_radix(d, HEX_RADIX)
            .map_err(|_| self.error_at(at, "`\\u` needs four hex digits"))?;
        self.i += 4;
        Ok(code)
    }
}
