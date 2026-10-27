//! Values to and from the binary form, whole.

use bumpalo::Bump;

use super::writer::START_BYTES;
use super::{DecodeError, Decoder, Writer};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;

/// `value` in the binary form.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::with_capacity(START_BYTES);
    encode_into(value, &mut out);
    out
}

/// `value` in the binary form, appended to `out` (a whole document, header
/// first): a buffer that is kept and reused grows once.
pub fn encode_into(value: &Value, out: &mut Vec<u8>) {
    let mut w = Writer::appending(std::mem::take(out));
    w.value(value);
    *out = w.finish();
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

/// A document of version 1 or 2 as a version 2 one: what reading an old
/// file and saving it again does. Nothing is lost: the value read is the
/// value written, and every value version 1 holds has a version 2 form (a
/// [`crate::Stream`] is the list of its items, as it is read anywhere
/// without a schema).
///
/// # Errors
///
/// [`DecodeError`] when the bytes aren't the binary form; nothing is written then.
pub fn upgrade(bytes: &[u8]) -> Result<Vec<u8>, DecodeError> {
    decode(bytes).map(|value| encode(&value))
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
