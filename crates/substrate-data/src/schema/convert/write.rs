//! A schema's value out: to the binary form, to text.

use crate::binary::Writer;
use crate::schema::Schema;
use crate::text;

/// `v` in the binary form, written straight ([`Schema::encode`]).
pub fn to_bytes<T: Schema>(v: &T) -> Vec<u8> {
    let mut w = Writer::new();
    v.encode(&mut w);
    w.finish()
}

/// `value` as text, indented.
pub fn to_text<T: Schema>(value: &T) -> String {
    text::to_string_pretty(&value.write())
}
