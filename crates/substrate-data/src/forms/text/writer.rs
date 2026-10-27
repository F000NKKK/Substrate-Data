//! Text out of a value: compact or indented, typed values in their string forms.

use crate::model::value::{Map, Value};

/// `value` as compact text (typed values in their string forms).
pub fn to_string(value: &Value) -> String {
    let mut out = String::with_capacity(256);
    write(value, None, 0, &mut out);
    out
}

/// `value` as text indented by two spaces, one entry per line.
pub fn to_string_pretty(value: &Value) -> String {
    let mut out = String::new();
    write(value, Some("  "), 0, &mut out);
    out.push('\n');
    out
}

fn write(value: &Value, indent: Option<&str>, level: usize, out: &mut String) {
    Layout { indent, out }.value(value, level);
}

/// Text being written, and the indent unit if it's pretty.
struct Layout<'a, 'o> {
    indent: Option<&'a str>,
    out: &'o mut String,
}

impl Layout<'_, '_> {
    fn value(&mut self, value: &Value, level: usize) {
        match value {
            Value::Null => self.out.push_str("null"),
            Value::Bool(b) => self.out.push_str(if *b { "true" } else { "false" }),
            Value::Int(n) => push_fmt(self.out, format_args!("{n}")),
            Value::Float(f) => write_float(*f, self.out),
            Value::List(items) => self.list(items, level),
            Value::Map(map) => self.map(map, level),
            // As spelled: a plain string's leading `@` was doubled when it
            // was put in the value (`Schema::write`), a typed one's wasn't.
            Value::Str(s) => write_string(s, self.out),
            typed => write_string(&crate::typed::spelling(typed).unwrap_or_default(), self.out),
        }
    }

    fn newline(&mut self, level: usize) {
        if let Some(unit) = self.indent {
            self.out.push('\n');
            for _ in 0..level {
                self.out.push_str(unit);
            }
        }
    }

    fn list(&mut self, items: &[Value], level: usize) {
        if items.is_empty() {
            self.out.push_str("[]");
            return;
        }
        self.out.push('[');
        for (i, v) in items.iter().enumerate() {
            if i > 0 {
                self.out.push(',');
            }
            self.newline(level + 1);
            self.value(v, level + 1);
        }
        self.newline(level);
        self.out.push(']');
    }

    fn map(&mut self, map: &Map, level: usize) {
        if map.is_empty() {
            self.out.push_str("{}");
            return;
        }
        self.out.push('{');
        for (i, (k, v)) in map.iter().enumerate() {
            if i > 0 {
                self.out.push(',');
            }
            self.newline(level + 1);
            write_string(k, self.out);
            self.out
                .push_str(if self.indent.is_some() { ": " } else { ":" });
            self.value(v, level + 1);
        }
        self.newline(level);
        self.out.push('}');
    }
}

/// Formats into `out`; writing to a `String` can't fail.
fn push_fmt(out: &mut String, args: std::fmt::Arguments<'_>) {
    use std::fmt::Write as _;
    out.write_fmt(args).unwrap_or_default();
}

fn write_float(f: f64, out: &mut String) {
    if f.is_finite() {
        out.push_str(&float_string(f));
    } else {
        out.push_str("null");
    }
}

/// A finite float in the shortest spelling that reads back as exactly it,
/// and as a float: `1.0`, `0.1`, `1e-7`, `1e16` (never `10000000000000000`,
/// which would read back as an integer).
pub(crate) fn float_string(f: f64) -> String {
    format!("{f:?}")
}

/// `s` as a JSON string literal.
pub fn write_string(s: &str, out: &mut String) {
    out.push('"');
    write_string_body(s, out);
    out.push('"');
}

fn write_string_body(s: &str, out: &mut String) {
    let bytes = s.as_bytes();
    let mut run = 0;
    while let Some(n) = bytes[run..]
        .iter()
        .position(|&b| b == b'"' || b == b'\\' || b < 0x20)
    {
        let i = run + n;
        out.push_str(&s[run..i]);
        match bytes[i] {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b => push_fmt(out, format_args!("\\u{b:04x}")),
        }
        run = i + 1;
    }
    out.push_str(&s[run..]);
}
