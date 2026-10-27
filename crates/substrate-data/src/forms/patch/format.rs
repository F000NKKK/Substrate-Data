//! [`format`]: a value written the way editors write it; [`same`]: values equal as written.

use std::fmt;

use crate::model::value::{Map, Value};
use crate::text::write_string;

/// A line no longer than this is written on one line by [`format`].
const ONE_LINE: usize = 72;
/// One indent step.
pub(super) const INDENT: &str = "  ";

/// `value` as editors write it at indent `level` (two spaces each):
/// short lists of numbers and short objects of scalars on one line
/// (`"margins": [0, 0, 16, 16]`), the rest one entry a line.
pub fn format(value: &Value, level: usize) -> String {
    let mut out = String::new();
    write_value(value, level, &mut out);
    out
}

/// `value` onto `out`: on one line if it fits, else an entry a line.
fn write_value(value: &Value, level: usize, out: &mut String) {
    let start = out.len();
    if write_inline(value, out) {
        return;
    }
    out.truncate(start);
    match value {
        Value::List(items) => {
            out.push_str("[\n");
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                push_indent(out, level + 1);
                write_value(v, level + 1, out);
            }
            out.push('\n');
            push_indent(out, level);
            out.push(']');
        }
        Value::Map(map) => {
            out.push_str("{\n");
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                push_indent(out, level + 1);
                write_string(k, out);
                out.push_str(": ");
                write_value(v, level + 1, out);
            }
            out.push('\n');
            push_indent(out, level);
            out.push('}');
        }
        other => {
            write_scalar(other, out);
        }
    }
}

fn push_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str(INDENT);
    }
}

pub(super) fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    write_string(s, &mut out);
    out
}

/// A finite float in the shortest spelling that reads back as exactly it,
/// and as a float: `1.0`, `-0.0`, `0.1`, `1e-7`, `1e20` — nothing rounded
/// away, a whole one still a float. (An integer is a `Value::Int`, written
/// as one; a value that didn't change keeps the source's spelling.)
pub(super) fn number(n: f64) -> String {
    crate::text::float_string(n)
}

/// A scalar onto `out`; `false` (nothing written) for a list or a map.
fn write_scalar(v: &Value, out: &mut String) -> bool {
    match v {
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => {
            fmt::Write::write_fmt(out, format_args!("{n}")).ok();
        }
        Value::Float(f) if f.is_finite() => out.push_str(&number(*f)),
        Value::Null | Value::Float(_) => out.push_str("null"),
        Value::List(_) | Value::Map(_) => return false,
        // As the source has it: a typed value (`@file:…`) or an escaped `@@`, unread.
        Value::Str(s) => write_string(s, out),
        other => out.push_str(&crate::text::to_string(other)),
    }
    true
}

/// `v` onto `out` on one line, if it can be and fits ([`ONE_LINE`]); else
/// `false`, with part of it written.
fn write_inline(v: &Value, out: &mut String) -> bool {
    if write_scalar(v, out) {
        return true;
    }
    let start = out.len();
    let written = match v {
        Value::List(items) if items.is_empty() => {
            out.push_str("[]");
            return true;
        }
        Value::List(items) => write_numbers(items, out),
        Value::Map(map) if map.is_empty() => {
            out.push_str("{}");
            return true;
        }
        Value::Map(map) => write_scalars(map, out),
        _ => false,
    };
    written && out.len() - start <= ONE_LINE
}

/// A list of numbers and bools as `[1, 2]`; `false` if it has anything else.
fn write_numbers(items: &[Value], out: &mut String) -> bool {
    if !items
        .iter()
        .all(|x| matches!(x, Value::Int(_) | Value::Float(_) | Value::Bool(_)))
    {
        return false;
    }
    out.push('[');
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_scalar(x, out);
    }
    out.push(']');
    true
}

/// A map with no map in it as `{ "a": 1 }`; `false` if an entry can't be on one line.
fn write_scalars(map: &Map, out: &mut String) -> bool {
    out.push_str("{ ");
    for (i, (k, x)) in map.iter().enumerate() {
        if matches!(x, Value::Map(_)) {
            return false;
        }
        if i > 0 {
            out.push_str(", ");
        }
        write_string(k, out);
        out.push_str(": ");
        if !write_inline(x, out) {
            return false;
        }
    }
    out.push_str(" }");
    true
}

/// `value` as compact JSON for a tool: maps in their order, strings as the
/// source has them (a typed value's text unread).
pub fn to_json(value: &Value) -> String {
    let mut out = String::new();
    json_into(value, &mut out);
    out
}

fn json_into(value: &Value, out: &mut String) {
    match value {
        Value::List(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_into(v, out);
            }
            out.push(']');
        }
        Value::Map(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                json_into(v, out);
            }
            out.push('}');
        }
        Value::Str(s) => write_string(s, out),
        Value::Float(f) if f.is_finite() => out.push_str(&f.to_string()),
        Value::Float(_) => out.push_str("null"),
        other => out.push_str(&crate::text::to_string(other)),
    }
}

/// Whether two values are the same, a number however it's spelled (`1` = `1.0`).
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {
            // `-0.0` is not `0`: its sign is kept.
            match (a.as_f64(), b.as_f64()) {
                (Some(x), Some(y)) => x.to_bits() == y.to_bits(),
                _ => false,
            }
        }
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((ka, p), (kb, q))| ka == kb && same(p, q))
        }
        _ => a == b,
    }
}
