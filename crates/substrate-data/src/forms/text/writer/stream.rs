//! [`Writer`]: text written piece by piece, from data that isn't a value.

use super::float::push_float;
use super::integer::push_int;
use super::string::{push_string, write_body};
use crate::model::value::Value;

/// How far a nested level is indented in the pretty form.
const INDENT: &[u8] = b"  ";

/// A list or map being written: how many of its entries are still to come,
/// and how many have been.
#[derive(Debug)]
struct Open {
    map: bool,
    remaining: usize,
    done: usize,
}

/// Writes the text form piece by piece, without a [`Value`] in between. A
/// list or map is given its length, then that many values (a map's each
/// after its `key`); it closes by itself after the last, so there are no
/// end calls. The pretty form puts one entry per line.
///
/// ```
/// let mut w = substrate_data::text::Writer::new();
/// w.map(2).key("id").str("docs/intro").key("pages").list(2);
/// w.int(1).int(2);
/// assert_eq!(w.finish(), r#"{"id":"docs/intro","pages":[1,2]}"#);
/// ```
#[derive(Debug)]
pub struct Writer {
    out: Vec<u8>,
    pretty: bool,
    open: Vec<Open>,
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

impl Writer {
    /// A writer of compact text.
    #[must_use]
    pub fn new() -> Self {
        Self::with_pretty(false)
    }

    /// A writer of text indented by two spaces, one entry per line, ending
    /// with a newline.
    #[must_use]
    pub fn pretty() -> Self {
        Self::with_pretty(true)
    }

    fn with_pretty(pretty: bool) -> Self {
        Self {
            out: Vec::with_capacity(256),
            pretty,
            open: Vec::new(),
        }
    }

    /// Null.
    pub fn null(&mut self) -> &mut Self {
        self.begin();
        self.out.extend_from_slice(b"null");
        self.end()
    }

    /// A bool.
    pub fn bool(&mut self, b: bool) -> &mut Self {
        self.begin();
        self.out.extend_from_slice(if b { b"true" } else { b"false" });
        self.end()
    }

    /// An integer.
    pub fn int(&mut self, n: i64) -> &mut Self {
        self.begin();
        push_int(n, &mut self.out);
        self.end()
    }

    /// A float, in the shortest spelling that reads back as it and as a
    /// float; `null` for a NaN or an infinity.
    pub fn float(&mut self, f: f64) -> &mut Self {
        self.begin();
        push_float(f, &mut self.out);
        self.end()
    }

    /// A string, as spelled (the text form has no other kind of text).
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.begin();
        push_string(s, &mut self.out);
        self.end()
    }

    /// A reference, as its string form `@kind:id`.
    pub fn reference(&mut self, kind: &str, id: &str) -> &mut Self {
        self.begin();
        self.out.extend_from_slice(b"\"@");
        write_body(kind, &mut self.out);
        self.out.push(b':');
        write_body(id, &mut self.out);
        self.out.push(b'"');
        self.end()
    }

    /// Bytes, as their string form `@bytes:<base64>`.
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        use base64::Engine as _;
        self.begin();
        self.out.extend_from_slice(b"\"@");
        self.out.extend_from_slice(crate::typed::BYTES.as_bytes());
        self.out.push(b':');
        let at = self.out.len();
        self.out.resize(
            at + base64::encoded_len(bytes.len(), true).unwrap_or(0),
            0,
        );
        let n = base64::engine::general_purpose::STANDARD
            .encode_slice(bytes, &mut self.out[at..])
            .unwrap_or(0);
        self.out.truncate(at + n);
        self.out.push(b'"');
        self.end()
    }

    /// A list of `len` values, which follow.
    pub fn list(&mut self, len: usize) -> &mut Self {
        self.container(len, false)
    }

    /// A map of `len` entries (`key`, then its value), which follow.
    pub fn map(&mut self, len: usize) -> &mut Self {
        self.container(len, true)
    }

    /// The key of the map's next entry; its value follows.
    pub fn key(&mut self, key: &str) -> &mut Self {
        self.separate();
        push_string(key, &mut self.out);
        self.out.push(b':');
        if self.pretty {
            self.out.push(b' ');
        }
        self
    }

    /// A whole [`Value`] (typed values in their string forms).
    pub fn value(&mut self, value: &Value) -> &mut Self {
        match value {
            Value::Null => self.null(),
            Value::Bool(b) => self.bool(*b),
            Value::Int(n) => self.int(*n),
            Value::Float(f) => self.float(*f),
            // As spelled: a plain string's leading `@` was doubled when it
            // was put in the value (`Schema::write`), a typed one's wasn't.
            Value::Str(s) => self.str(s),
            Value::List(items) => {
                self.list(items.len());
                for item in items {
                    self.value(item);
                }
                self
            }
            Value::Map(map) => {
                self.map(map.len());
                for (k, v) in map {
                    self.key(k).value(v);
                }
                self
            }
            Value::Ref(r) => self.reference(r.kind.as_str(), r.id.as_str()),
            Value::Bytes(b) => self.bytes(b),
        }
    }

    /// The text written.
    #[must_use]
    pub fn finish(mut self) -> String {
        if self.pretty {
            self.out.push(b'\n');
        }
        // Only what was written from `str`s and ASCII: always valid.
        String::from_utf8(self.out)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
    }

    /// The comma and the line break before an entry of a list or map.
    fn separate(&mut self) {
        let Some(top) = self.open.last() else {
            return;
        };
        if top.done > 0 {
            self.out.push(b',');
        }
        self.line_break(self.open.len());
    }

    fn line_break(&mut self, level: usize) {
        if self.pretty {
            self.out.push(b'\n');
            for _ in 0..level {
                self.out.extend_from_slice(INDENT);
            }
        }
    }

    /// Before a value: its separator, unless a key just wrote it.
    fn begin(&mut self) {
        if matches!(self.open.last(), Some(top) if !top.map) {
            self.separate();
        }
    }

    /// After a value: counts it in its container, and closes the containers
    /// that it was the last of.
    fn end(&mut self) -> &mut Self {
        while let Some(top) = self.open.last_mut() {
            top.done += 1;
            top.remaining -= 1;
            if top.remaining > 0 {
                break;
            }
            let closer = if top.map { b'}' } else { b']' };
            self.open.pop();
            self.line_break(self.open.len());
            self.out.push(closer);
        }
        self
    }

    fn container(&mut self, len: usize, map: bool) -> &mut Self {
        self.begin();
        if len == 0 {
            self.out.extend_from_slice(if map { b"{}" } else { b"[]" });
            return self.end();
        }
        self.out.push(if map { b'{' } else { b'[' });
        self.open.push(Open {
            map,
            remaining: len,
            done: 0,
        });
        self
    }
}
