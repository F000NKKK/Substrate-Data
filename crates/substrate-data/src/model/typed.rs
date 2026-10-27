//! The typed values' string forms — one grammar for every format:
//!
//! | Written | Value |
//! |---|---|
//! | `@asset:meshes/cube.mesh`, `@entity:player` | [`Value::Ref`] |
//! | `@bytes:iVBORw0…` (base64) | [`Value::Bytes`] |
//! | `@@text` | the plain string `@text` |
//!
//! A string that starts with `@` is always one of these (or an error), in
//! any field — so a typo in a reference is never silently plain text.

use crate::model::value::{Ref, Value};
use crate::names::typed;

use base64::Engine as _;

/// `s` as a typed value if it's written as one (`@…`), the plain string
/// otherwise; `@@` escapes a leading `@`.
pub fn classify(s: &str) -> Result<Value, String> {
    let Some(rest) = s.strip_prefix('@') else {
        return Ok(Value::Str(s.to_string()));
    };
    if let Some(literal) = rest.strip_prefix('@') {
        return Ok(Value::Str(format!("@{literal}")));
    }
    let (kind, id) = rest.split_once(':').ok_or_else(|| {
        format!("`{s}`: a reference is `@kind:id` (`@asset:meshes/cube.mesh`); write `@@` for a literal `@`")
    })?;
    if !is_kind(kind) {
        return Err(format!(
            "`{s}`: `{kind}` isn't a kind (lower case letters, digits and `_`: `asset`, `entity`, ...)"
        ));
    }
    if kind == typed::BYTES {
        return base64::engine::general_purpose::STANDARD
            .decode(id)
            .map(Value::Bytes)
            .map_err(|e| format!("`@bytes:`: not base64 ({e})"));
    }
    if !is_id(id) {
        return Err(format!(
            "`{s}`: `{id}` isn't an id (not empty, no spaces or control characters)"
        ));
    }
    Ok(Value::Ref(Ref::new(kind, id)))
}

/// The string form of a typed value (`None` for the untyped ones).
pub fn typed_string(v: &Value) -> Option<String> {
    Some(match v {
        Value::Ref(r) => r.to_string(),
        Value::Bytes(b) => format!(
            "@{}:{}",
            typed::BYTES,
            base64::engine::general_purpose::STANDARD.encode(b)
        ),
        Value::Str(t) => escape_at(t),
        _ => return None,
    })
}

/// `v` with its typed values as their strings (`@asset:x`): plain values
/// only, for a tool that knows no typed ones.
pub fn flatten(v: &Value) -> Value {
    match v {
        Value::List(items) => Value::List(items.iter().map(flatten).collect()),
        Value::Map(m) => Value::Map(m.iter().map(|(k, x)| (k.to_string(), flatten(x))).collect()),
        Value::Str(_) => v.clone(),
        typed => typed_string(typed).map_or_else(|| typed.clone(), Value::Str),
    }
}

/// `s` with a leading `@` doubled, so it reads back as plain text.
pub fn escape_at(s: &str) -> String {
    if s.starts_with('@') {
        format!("@{s}")
    } else {
        s.to_string()
    }
}

/// A kind: lower case ASCII letters, digits and `_`, starting with a letter.
pub fn is_kind(s: &str) -> bool {
    let mut b = s.bytes();
    b.next().is_some_and(|c| c.is_ascii_lowercase())
        && b.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

/// An id: anything but empty, whitespace or control characters — what it
/// names is the format's business.
pub fn is_id(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_form() {
        assert_eq!(classify("plain").unwrap(), Value::Str("plain".into()));
        assert_eq!(classify("@@me").unwrap(), Value::Str("@me".into()));
        assert_eq!(
            classify("@asset:meshes/cube.mesh").unwrap(),
            Value::Ref(Ref::new("asset", "meshes/cube.mesh"))
        );
        assert_eq!(
            classify("@entity:core:player").unwrap(),
            Value::Ref(Ref::new("entity", "core:player"))
        );
        assert_eq!(
            classify("@bytes:AQID").unwrap(),
            Value::Bytes(vec![1, 2, 3])
        );
    }

    #[test]
    fn rejects_typos() {
        for bad in [
            "@",
            "@cube",
            "@Asset:cube",
            "@asset:",
            "@asset:a b",
            "@bytes:!!",
        ] {
            assert!(classify(bad).is_err(), "{bad} should fail");
        }
    }

    #[test]
    fn round_trips() {
        for s in [
            "@asset:meshes/cube.mesh",
            "@prefab:enemy[0]",
            "@bytes:AQID",
            "@@at",
            "plain",
        ] {
            assert_eq!(typed_string(&classify(s).unwrap()).unwrap(), s);
        }
    }
}
