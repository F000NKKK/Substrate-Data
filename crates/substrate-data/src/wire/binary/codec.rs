//! Values to and from the binary form, whole.

use bumpalo::Bump;

use super::{DecodeError, Decoder, Writer};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;

/// `value` in the binary form.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut w = Writer::new();
    w.value(value);
    w.finish()
}

/// Reads the binary form.
///
/// # Errors
///
/// [`DecodeError`] when the bytes aren't the binary form.
pub fn decode(bytes: &[u8]) -> Result<Value, DecodeError> {
    let mut d = Decoder::open(bytes)?;
    let v = d.value()?;
    d.end()?;
    Ok(v)
}

/// Reads the binary form borrowed: every string points into `bytes`,
/// lists and maps go in `arena` — nothing is copied.
///
/// # Errors
///
/// [`DecodeError`] when the bytes aren't the binary form.
pub fn decode_ref<'a>(bytes: &'a [u8], arena: &'a Bump) -> Result<ValueRef<'a>, DecodeError> {
    let mut d = Decoder::open(bytes)?;
    let v = d.value_ref(arena)?;
    d.end()?;
    Ok(v)
}
