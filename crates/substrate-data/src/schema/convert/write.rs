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

/// `v` in the binary form, appended to `out` (a whole document, header
/// first): a buffer that is kept and reused grows once.
pub fn to_bytes_into<T: Schema>(v: &T, out: &mut Vec<u8>) {
    let mut w = Writer::appending(std::mem::take(out));
    v.encode(&mut w);
    *out = w.finish();
}

/// `value` as text, indented: written straight, no value tree built.
pub fn to_text<T: Schema>(value: &T) -> String {
    let mut w = text::Writer::pretty();
    value.write_text(&mut w);
    w.finish()
}
