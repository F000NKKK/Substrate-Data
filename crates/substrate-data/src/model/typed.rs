//! The typed values' string forms — one grammar for every format:
//!
//! | Written | Value |
//! |---|---|
//! | `@file:images/logo.png`, `@user:alice` | [`Value::Ref`] |
//! | `@bytes:iVBORw0…` (base64) | [`Value::Bytes`] |
//! | `@@text` | the plain string `@text` |
//!
//! A string that starts with `@` is always one of these (or an error), in
//! any field — so a typo in a reference is never silently plain text.

use std::fmt;

use base64::Engine as _;

use crate::model::value::{KindName, Ref, Value, ValueKind};
use crate::names::TYPED;

/// The kind word of bytes written as text (`@bytes:…`); no reference has it.
const BYTES: &str = "bytes";

/// What a string spells.
#[derive(Debug, Clone, PartialEq)]
pub enum Spelled {
    /// Plain text (its `@@` escape undone).
    Plain(String),
    /// A reference.
    Ref(Ref),
    /// Bytes.
    Bytes(Vec<u8>),
}

impl Spelled {
    /// Which value it is, for messages.
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            Spelled::Plain(_) => ValueKind::Str,
            Spelled::Ref(_) => ValueKind::Ref,
            Spelled::Bytes(_) => ValueKind::Bytes,
        }
    }

    /// As a value (plain text as spelled again: a leading `@` doubled).
    #[must_use]
    pub fn into_value(self) -> Value {
        match self {
            Spelled::Plain(s) => Value::Str(escape_at(&s)),
            Spelled::Ref(r) => Value::Ref(r),
            Spelled::Bytes(b) => Value::Bytes(b),
        }
    }
}

/// A string written as a typed value that isn't one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::module_name_repetitions,
    reason = "re-exported at the crate root, where `Error` alone would say nothing"
)]
pub enum TypedError {
    /// No `kind:` (`@logo`).
    NoKind {
        /// The string as written.
        spelled: String,
    },
    /// A kind that isn't one (`@File:x`).
    BadKind {
        /// The kind as written.
        kind: String,
    },
    /// An id that isn't one (`@file:a b`), of `kind` when it's known.
    BadId {
        /// Its reference's kind, when it's known.
        kind: Option<KindName>,
        /// The id as written.
        id: String,
    },
    /// `@bytes:` that isn't base64.
    BadBytes(base64::DecodeError),
}

impl fmt::Display for TypedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypedError::NoKind { spelled } => write!(
                f,
                "`{spelled}`: a reference is `@kind:id` (`@file:images/logo.png`); write `@@` for a literal `@`"
            ),
            TypedError::BadKind { kind } => write!(
                f,
                "`{kind}` isn't a kind (lower case letters, digits and `_`: `user`, `doc`, `file`, ...)"
            ),
            TypedError::BadId {
                kind: Some(kind),
                id,
            } => write!(
                f,
                "`@{kind}:{id}`: `{id}` isn't an id (not empty, no spaces or control characters)"
            ),
            TypedError::BadId { kind: None, id } => write!(
                f,
                "`{id}` isn't an id (not empty, no spaces or control characters)"
            ),
            TypedError::BadBytes(e) => write!(f, "`@bytes:`: not base64 ({e})"),
        }
    }
}

impl std::error::Error for TypedError {}

/// What `s` spells: a typed value if it's written as one (`@…`), plain
/// text otherwise; `@@` escapes a leading `@`.
///
/// # Errors
///
/// [`TypedError`] when `s` starts with `@` but isn't a typed value.
pub fn classify(s: &str) -> Result<Spelled, TypedError> {
    let Some(rest) = s.strip_prefix(TYPED) else {
        return Ok(Spelled::Plain(s.to_string()));
    };
    if rest.starts_with(TYPED) {
        return Ok(Spelled::Plain(rest.to_string()));
    }
    let (kind, id) = rest.split_once(':').ok_or_else(|| TypedError::NoKind {
        spelled: s.to_owned(),
    })?;
    if kind == BYTES {
        return base64::engine::general_purpose::STANDARD
            .decode(id)
            .map(Spelled::Bytes)
            .map_err(TypedError::BadBytes);
    }
    Ref::parse(kind, id).map(Spelled::Ref)
}

/// The string form of a typed value (`None` for the untyped ones).
pub fn spelling(v: &Value) -> Option<String> {
    Some(match v {
        Value::Ref(r) => r.to_string(),
        Value::Bytes(b) => format!(
            "{TYPED}{BYTES}:{}",
            base64::engine::general_purpose::STANDARD.encode(b)
        ),
        _ => return None,
    })
}

/// `v` with its typed values as their strings (`@file:x`): plain values
/// only, for a tool that knows no typed ones.
pub fn flatten(v: &Value) -> Value {
    match v {
        Value::List(items) => Value::List(items.iter().map(flatten).collect()),
        Value::Map(m) => Value::Map(m.iter().map(|(k, x)| (k.to_string(), flatten(x))).collect()),
        other => spelling(other).map_or_else(|| other.clone(), Value::Str),
    }
}

/// `s` with a leading `@` doubled, so it reads back as plain text.
#[must_use]
pub fn escape_at(s: &str) -> String {
    if s.starts_with(TYPED) {
        format!("{TYPED}{s}")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(kind: &str, id: &str) -> Spelled {
        Spelled::Ref(Ref::parse(kind, id).unwrap())
    }

    #[test]
    fn reads_every_form() {
        assert_eq!(classify("plain").unwrap(), Spelled::Plain("plain".into()));
        assert_eq!(classify("@@me").unwrap(), Spelled::Plain("@me".into()));
        assert_eq!(
            classify("@file:images/logo.png").unwrap(),
            r("file", "images/logo.png")
        );
        assert_eq!(
            classify("@user:team:alice").unwrap(),
            r("user", "team:alice")
        );
        assert_eq!(
            classify("@bytes:AQID").unwrap(),
            Spelled::Bytes(vec![1, 2, 3])
        );
    }

    #[test]
    fn rejects_typos() {
        assert!(matches!(classify("@"), Err(TypedError::NoKind { .. })));
        assert!(matches!(classify("@logo"), Err(TypedError::NoKind { .. })));
        assert!(matches!(
            classify("@File:logo"),
            Err(TypedError::BadKind { .. })
        ));
        assert!(matches!(
            classify("@file:"),
            Err(TypedError::BadId { kind: Some(_), .. })
        ));
        assert!(matches!(
            classify("@file:a b"),
            Err(TypedError::BadId { kind: Some(_), .. })
        ));
        assert!(matches!(
            classify("@bytes:!!"),
            Err(TypedError::BadBytes(_))
        ));
    }

    #[test]
    fn round_trips() {
        for s in [
            "@file:images/logo.png",
            "@doc:guides/intro[0]",
            "@bytes:AQID",
        ] {
            assert_eq!(spelling(&classify(s).unwrap().into_value()).unwrap(), s);
        }
        assert_eq!(
            classify("@@at").unwrap().into_value(),
            Value::Str("@@at".into())
        );
    }
}
