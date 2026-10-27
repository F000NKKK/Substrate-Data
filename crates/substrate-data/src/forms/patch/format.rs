//! [`format`]: a value written the way editors write it; [`same`]: values equal as written.

use crate::model::value::Value;
use crate::text::write_string;

/// A line no longer than this is written on one line by [`format`].
const ONE_LINE: usize = 72;
/// One indent step.
pub(super) const INDENT: &str = "  ";

/// `value` as editors write it at indent `level` (two spaces each):
/// short lists of numbers and short objects of scalars on one line
/// (`"margins": [0, 0, 16, 16]`), the rest one entry a line.
pub fn format(value: &Value, level: usize) -> String {
    if let Some(one) = inline(value) {
        return one;
    }
    let pad = INDENT.repeat(level + 1);
    let end = INDENT.repeat(level);
    match value {
        Value::List(items) => {
            let body: Vec<String> = items
                .iter()
                .map(|v| format!("{pad}{}", format(v, level + 1)))
                .collect();
            format!("[\n{}\n{end}]", body.join(",\n"))
        }
        Value::Map(map) => {
            let body: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{pad}{}: {}", quoted(k), format(v, level + 1)))
                .collect();
            format!("{{\n{}\n{end}}}", body.join(",\n"))
        }
        other => scalar(other).unwrap_or_default(),
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

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::Bool(b) => Some(b.to_string()),
        Value::Int(n) => Some(n.to_string()),
        Value::Float(f) if f.is_finite() => Some(number(*f)),
        Value::Null | Value::Float(_) => Some("null".into()),
        Value::List(_) | Value::Map(_) => None,
        // As the source has it: a typed value (`@file:…`) or an escaped `@@`, unread.
        Value::Str(s) => Some(quoted(s)),
        other => Some(crate::text::to_string(other)),
    }
}

fn inline(v: &Value) -> Option<String> {
    if let Some(s) = scalar(v) {
        return Some(s);
    }
    let out = match v {
        Value::List(items) if items.is_empty() => return Some("[]".into()),
        Value::List(items) => {
            if !items
                .iter()
                .all(|x| matches!(x, Value::Int(_) | Value::Float(_) | Value::Bool(_)))
            {
                return None;
            }
            format!(
                "[{}]",
                items
                    .iter()
                    .filter_map(scalar)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Value::Map(map) if map.is_empty() => return Some("{}".into()),
        Value::Map(map) => {
            let mut parts = Vec::with_capacity(map.len());
            for (k, x) in map.iter() {
                if matches!(x, Value::Map(_)) {
                    return None;
                }
                parts.push(format!("{}: {}", quoted(k), inline(x)?));
            }
            format!("{{ {} }}", parts.join(", "))
        }
        _ => return None,
    };
    (out.len() <= ONE_LINE).then_some(out)
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
