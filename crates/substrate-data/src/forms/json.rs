//! JSON ⇄ the model (feature `json`): for what still speaks JSON — a
//! third-party file format, a tool's protocol.

use crate::model::value::{Map, Value};

/// `j` as a value (integers stay integers).
pub fn from_json(j: &serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Value::Float(n.as_f64().unwrap_or(0.0)), Value::Int),
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(a) => Value::List(a.iter().map(from_json).collect()),
        serde_json::Value::Object(o) => Value::Map(
            o.iter()
                .map(|(k, v)| (k.clone(), from_json(v)))
                .collect::<Map>(),
        ),
    }
}

/// JSON text as a value, `None` if it isn't JSON.
#[must_use]
pub fn from_json_text(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok().map(|j| from_json(&j))
}

/// `v` as JSON; typed values become their string forms (`"@file:images/logo.png"`).
pub fn to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::Value::from(*n),
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        Value::Map(m) => serde_json::Value::Object(
            m.iter()
                .map(|(k, item)| (k.to_string(), to_json(item)))
                .collect(),
        ),
        typed => serde_json::Value::String(crate::typed::spelling(typed).unwrap_or_default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let j = serde_json::json!({ "a": [1, 2.5, "x", null, true], "b": { "c": -3 } });
        assert_eq!(to_json(&from_json(&j)), j);
    }
}
