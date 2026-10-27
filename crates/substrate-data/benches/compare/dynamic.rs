//! Value trees against `serde_json`'s: the dynamic route, where a program
//! holds a document it doesn't have a type for. JSON, CBOR and `MessagePack`
//! all go through `serde_json::Value`; typed values and references are
//! their string spellings there, as in this crate's text form.

use serde_json::Value as Tree;
use substrate_data::{Value, typed};

/// `value` as a `serde_json` tree.
pub(crate) fn tree(value: &Value) -> Tree {
    convert(&typed::flatten(value))
}

fn convert(value: &Value) -> Tree {
    match value {
        Value::Null | Value::Ref(_) | Value::Bytes(_) => Tree::Null,
        Value::Bool(b) => Tree::Bool(*b),
        Value::Int(i) => Tree::from(*i),
        Value::Float(f) => serde_json::Number::from_f64(*f).map_or(Tree::Null, Tree::Number),
        Value::Str(s) => Tree::String(s.clone()),
        Value::List(items) => Tree::Array(items.iter().map(convert).collect()),
        Value::Map(m) => Tree::Object(m.iter().map(|(k, v)| (k.to_owned(), convert(v))).collect()),
    }
}
