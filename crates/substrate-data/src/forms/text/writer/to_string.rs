//! A whole [`Value`] as text.

use super::Writer;
use crate::model::value::Value;

/// `value` as compact text (typed values in their string forms).
pub fn to_string(value: &Value) -> String {
    let mut w = Writer::new();
    w.value(value);
    w.finish()
}

/// `value` as text indented by two spaces, one entry per line.
pub fn to_string_pretty(value: &Value) -> String {
    let mut w = Writer::pretty();
    w.value(value);
    w.finish()
}
