//! The parser: text to a value, with the spans of its parts when asked.

use super::{Doc, LineCol, Span, Spans, SyntaxError, scan};
use crate::model::value::{Map, Value};

/// `\\u` escapes are hexadecimal.
const HEX_RADIX: u32 = 16;

/// Deeper than this is an error, not a stack overflow.
pub const MAX_DEPTH: usize = 256;

/// Parses text.
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
pub fn parse(text: &str) -> Result<Doc, SyntaxError> {
    let (value, spans) = run::<true>(text)?;
    Ok(Doc { value, spans })
}

/// Parses text, without the positions (faster).
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
pub fn parse_value(text: &str) -> Result<Value, SyntaxError> {
    run::<false>(text).map(|(v, _)| v)
}

fn run<const SPANS: bool>(text: &str) -> Result<(Value, Spans), SyntaxError> {
    let mut p = Parser::<SPANS> {
        src: text.as_bytes(),
        text,
        i: 0,
        depth: 0,
    };
    if text.starts_with('\u{feff}') {
        p.i = 3;
    }
    p.ws()?;
    let (value, spans) = p.value()?;
    p.ws()?;
    if p.i < p.src.len() {
        return Err(p.error("unexpected text after the value"));
    }
    Ok((value, spans))
}

/// `SPANS`: whether to record where each value is.
#[derive(Debug)]
struct Parser<'a, const SPANS: bool> {
    src: &'a [u8],
    text: &'a str,
    i: usize,
    depth: usize,
}

impl<const SPANS: bool> Parser<'_, SPANS> {
    fn error(&self, message: impl Into<String>) -> SyntaxError {
        self.error_at(self.i, message)
    }

    fn error_at(&self, offset: usize, message: impl Into<String>) -> SyntaxError {
        let mut message = message.into();
        if offset >= self.src.len() {
            message.push_str(", found the end of the file");
        }
        SyntaxError {
            message,
            at: LineCol::of(self.text, offset),
            offset,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.i).copied()
    }

    /// Skips whitespace and comments.
    fn ws(&mut self) -> Result<(), SyntaxError> {
        loop {
            let rest = &self.src[self.i..];
            self.i += rest
                .iter()
                .position(|&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
                .unwrap_or(rest.len());
            if self.peek() != Some(b'/') {
                return Ok(());
            }
            match self.src.get(self.i + 1) {
                Some(b'/') => {
                    self.i += scan::line_end(&self.src[self.i..]);
                }
                Some(b'*') => {
                    let start = self.i;
                    match scan::block_comment_end(&self.src[self.i + 2..]) {
                        Some(end) => self.i += 2 + end + 2,
                        None => return Err(self.error_at(start, "unclosed `/*` comment")),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn value(&mut self) -> Result<(Value, Spans), SyntaxError> {
        let start = self.i;
        let (value, children) = match self.peek() {
            Some(b'{') => self.nested(Parser::map)?,
            Some(b'[') => self.nested(Parser::list)?,
            Some(b'"') => (Value::Str(self.string()?), Vec::new()),
            Some(b'-' | b'0'..=b'9') => (self.number()?, Vec::new()),
            Some(b't') if self.src[self.i..].starts_with(b"true") => {
                self.i += 4;
                (Value::Bool(true), Vec::new())
            }
            Some(b'f') if self.src[self.i..].starts_with(b"false") => {
                self.i += 5;
                (Value::Bool(false), Vec::new())
            }
            Some(b'n') if self.src[self.i..].starts_with(b"null") => {
                self.i += 4;
                (Value::Null, Vec::new())
            }
            Some(b'\'') => return Err(self.error("strings use double quotes")),
            _ => return Err(self.error("expected a value")),
        };
        let span = Span::new(start, self.i);
        Ok((
            value,
            Spans {
                span,
                key: None,
                children,
            },
        ))
    }

    fn nested(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<(Value, Vec<Spans>), SyntaxError>,
    ) -> Result<(Value, Vec<Spans>), SyntaxError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error(format!("nested deeper than {MAX_DEPTH} levels")));
        }
        let r = f(self);
        self.depth -= 1;
        r
    }

    fn list(&mut self) -> Result<(Value, Vec<Spans>), SyntaxError> {
        self.i += 1;
        let mut items = Vec::new();
        let mut spans = Vec::new();
        loop {
            self.ws()?;
            if self.peek() == Some(b']') {
                self.i += 1;
                break;
            }
            let (v, s) = self.value()?;
            items.push(v);
            if SPANS {
                spans.push(s);
            }
            self.ws()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.error("expected `,` or `]`")),
            }
        }
        Ok((Value::List(items), spans))
    }

    fn map(&mut self) -> Result<(Value, Vec<Spans>), SyntaxError> {
        self.i += 1;
        let mut map = Map::new();
        let mut spans = Vec::new();
        // Duplicate keys: a scan for small objects, a set once they grow.
        let mut seen: Option<std::collections::HashSet<String>> = None;
        loop {
            self.ws()?;
            if self.peek() == Some(b'}') {
                self.i += 1;
                break;
            }
            let key_start = self.i;
            let key = self.key()?;
            let key_span = Span::new(key_start, self.i);
            if is_duplicate(&map, &mut seen, &key) {
                return Err(
                    self.error_at(key_start, format!("`{key}` is already set in this object"))
                );
            }
            self.ws()?;
            if self.peek() != Some(b':') {
                return Err(self.error("expected `:` after the key"));
            }
            self.i += 1;
            self.ws()?;
            let (v, mut s) = self.value()?;
            s.key = Some(key_span);
            map.push_unchecked(key.into(), v);
            if SPANS {
                spans.push(s);
            }
            self.ws()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.error("expected `,` or `}`")),
            }
        }
        Ok((Value::Map(map), spans))
    }

    /// An object's key: a string, or a bare name.
    fn key(&mut self) -> Result<String, SyntaxError> {
        let start = self.i;
        match self.peek() {
            Some(b'"') => self.string(),
            Some(first) if first.is_ascii_alphabetic() || first == b'_' || first == b'$' => {
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
                {
                    self.i += 1;
                }
                Ok(self.text[start..self.i].to_string())
            }
            _ => Err(self.error("expected a key")),
        }
    }

    fn string(&mut self) -> Result<String, SyntaxError> {
        let open = self.i;
        self.i += 1;
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
                    return Ok(out);
                }
                b'\\' => self.escape(&mut out)?,
                b'\n' => return Err(self.error("line break in a string (write `\\n`)")),
                _ => return Err(self.error("control character in a string")),
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), SyntaxError> {
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
                out.push(
                    ch.ok_or_else(|| self.error_at(at, "unpaired surrogate in `\\u` escape"))?,
                );
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

    fn hex4(&mut self, at: usize) -> Result<u32, SyntaxError> {
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

    fn number(&mut self) -> Result<Value, SyntaxError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let s = p.i;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.i += 1;
            }
            p.i - s
        };
        let int_start = self.i;
        if digits(self) == 0 {
            return Err(self.error("expected digits"));
        }
        if self.src[int_start] == b'0' && self.i - int_start > 1 {
            return Err(self.error_at(int_start, "numbers don't start with 0"));
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            self.i += 1;
            float = true;
            if digits(self) == 0 {
                return Err(self.error("expected digits after `.`"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            float = true;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if digits(self) == 0 {
                return Err(self.error("expected digits in the exponent"));
            }
        }
        let s = &self.text[start..self.i];
        if !float && let Ok(n) = s.parse::<i64>() {
            return Ok(Value::Int(n));
        }
        Ok(Value::Float(
            s.parse::<f64>()
                .map_err(|_| self.error_at(start, "invalid number"))?,
        ))
    }
}

/// Whether `key` is already in `map`: a scan for small objects, a set
/// (built in `seen` the first time) once they grow.
fn is_duplicate(
    map: &Map,
    seen: &mut Option<std::collections::HashSet<String>>,
    key: &str,
) -> bool {
    match seen {
        Some(set) => !set.insert(key.to_string()),
        None if map.len() < 16 => map.get(key).is_some(),
        None => {
            let mut set: std::collections::HashSet<String> =
                map.keys().map(str::to_string).collect();
            let dup = !set.insert(key.to_string());
            *seen = Some(set);
            dup
        }
    }
}
