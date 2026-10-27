//! A model value as JSON with explicit type tags, and back: ints, floats,
//! strings, references and bytes are never confused with one another.
//!
//! `{"t": "int", "v": "-5"}` — ints are decimal strings (JSON numbers
//! lose bits past 2^53); `{"t": "float", "bits": "3ff0000000000000"}` —
//! floats are their IEEE 754 bits, which also spell `-0.0` and `NaN`.

use substrate_data::{Map, Ref, Value};

use crate::hex::{from_hex, to_hex};
use crate::json::Json;

/// `value` as tagged JSON.
pub(crate) fn value_json(value: &Value) -> Json {
    match value {
        Value::Null => Json::obj([("t", Json::text("null"))]),
        Value::Bool(b) => Json::obj([("t", Json::text("bool")), ("v", Json::Bool(*b))]),
        Value::Int(n) => Json::obj([("t", Json::text("int")), ("v", Json::text(n.to_string()))]),
        Value::Float(f) => float_json(*f),
        Value::Str(s) => Json::obj([("t", Json::text("str")), ("v", Json::text(s.as_str()))]),
        Value::List(items) if small_ints(items) => Json::obj([
            ("t", Json::text("ints")),
            ("v", Json::Arr(items.iter().map(int_number).collect())),
        ]),
        Value::List(items) => Json::obj([
            ("t", Json::text("list")),
            ("v", Json::Arr(items.iter().map(value_json).collect())),
        ]),
        Value::Map(map) => Json::obj([("t", Json::text("map")), ("v", entries_json(map))]),
        Value::Ref(r) => Json::obj([
            ("t", Json::text("ref")),
            ("kind", Json::text(r.kind.as_str())),
            ("id", Json::text(r.id.as_str())),
        ]),
        Value::Bytes(b) => Json::obj([("t", Json::text("bytes")), ("hex", Json::text(to_hex(b)))]),
    }
}

/// Ints as JSON numbers are exact up to this (2^53).
const EXACT: u64 = 1 << 53;

/// A non-empty list of ints all exact as JSON numbers: rendered as plain numbers.
fn small_ints(items: &[Value]) -> bool {
    !items.is_empty()
        && items
            .iter()
            .all(|v| matches!(v, Value::Int(n) if n.unsigned_abs() < EXACT))
}

fn int_number(v: &Value) -> Json {
    match v {
        Value::Int(n) => Json::Int(*n),
        _ => Json::Null,
    }
}

fn float_json(f: f64) -> Json {
    let mut entries = vec![
        ("t".to_owned(), Json::text("float")),
        (
            "bits".to_owned(),
            Json::text(format!("{:016x}", f.to_bits())),
        ),
    ];
    if f.is_finite() {
        entries.push(("v".to_owned(), Json::text(format!("{f:?}"))));
    }
    Json::Obj(entries)
}

fn entries_json(map: &Map) -> Json {
    Json::Arr(
        map.iter()
            .map(|(key, value)| Json::Arr(vec![Json::text(key), value_json(value)]))
            .collect(),
    )
}

/// The value `json` renders; `None` when it isn't a rendering.
pub(crate) fn value_of(json: &serde_json::Value) -> Option<Value> {
    let tag = json.get("t")?.as_str()?;
    let v = json.get("v");
    Some(match tag {
        "null" | "bool" | "int" | "float" => scalar_of(tag, json)?,
        "ints" => Value::List(
            v?.as_array()?
                .iter()
                .map(|n| n.as_i64().map(Value::Int))
                .collect::<Option<_>>()?,
        ),
        "str" => Value::Str(v?.as_str()?.to_owned()),
        "list" => Value::List(v?.as_array()?.iter().map(value_of).collect::<Option<_>>()?),
        "map" => Value::Map(map_of(v?.as_array()?)?),
        "ref" => {
            Value::Ref(Ref::parse(json.get("kind")?.as_str()?, json.get("id")?.as_str()?).ok()?)
        }
        "bytes" => Value::Bytes(from_hex(json.get("hex")?.as_str()?)?),
        _ => return None,
    })
}

fn scalar_of(tag: &str, json: &serde_json::Value) -> Option<Value> {
    let v = json.get("v");
    Some(match tag {
        "null" => Value::Null,
        "bool" => Value::Bool(v?.as_bool()?),
        "int" => Value::Int(v?.as_str()?.parse().ok()?),
        _ => Value::Float(f64::from_bits(
            u64::from_str_radix(json.get("bits")?.as_str()?, 16).ok()?,
        )),
    })
}

fn map_of(pairs: &[serde_json::Value]) -> Option<Map> {
    let mut map = Map::new();
    for pair in pairs {
        let [key, value] = pair.as_array()?.as_slice() else {
            return None;
        };
        map.insert(key.as_str()?, value_of(value)?);
    }
    Some(map)
}

/// Whether `a` and `b` are the same value, floats bit for bit (so `-0.0`
/// is not `0.0` and a `NaN` is itself).
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((kx, vx), (ky, vy))| kx == ky && same(vx, vy))
        }
        _ => a == b,
    }
}
