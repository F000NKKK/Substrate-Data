//! [`Decoder`] step by step, for a schema reading itself
//! ([`crate::Schema::decode`]): `None` for anything but what
//! [`crate::Schema::encode`] writes there.

use bumpalo::Bump;

use super::decoder::{F32_BYTES, F64_BYTES, MAP_ENTRY_MIN};
use super::{Decoder, tag, unzigzag};
use crate::model::number::int_to_f64;

impl<'b> Decoder<'b> {
    /// The next tag, if it's `t`: read.
    fn next_is(&mut self, t: u8) -> Option<()> {
        (self.b.get(self.i) == Some(&t)).then(|| self.i += 1)
    }

    /// Whether a null is next: read if it is.
    pub fn null(&mut self) -> bool {
        self.next_is(tag::NULL).is_some()
    }

    /// A bool, if one is next.
    pub fn bool(&mut self) -> Option<bool> {
        match *self.b.get(self.i)? {
            tag::FALSE | tag::TRUE => {
                self.i += 1;
                Some(self.b[self.i - 1] == tag::TRUE)
            }
            _ => None,
        }
    }

    /// An integer, if one is next.
    pub fn int(&mut self) -> Option<i64> {
        self.next_is(tag::INT)?;
        self.varint().ok().map(unzigzag)
    }

    /// A float — or an int, as a number.
    pub fn float(&mut self) -> Option<f64> {
        match *self.b.get(self.i)? {
            tag::FLOAT => {
                self.i += 1;
                self.fixed().ok().map(f64::from_le_bytes)
            }
            tag::FLOAT32 => {
                self.i += 1;
                self.fixed().ok().map(|b| f64::from(f32::from_le_bytes(b)))
            }
            tag::INT => self.int().map(int_to_f64),
            _ => None,
        }
    }

    /// A string, if one is next.
    pub fn str(&mut self) -> Option<&'b str> {
        self.next_is(tag::STR)?;
        self.string().ok()
    }

    /// Bytes, if they're next.
    pub fn bytes(&mut self) -> Option<&'b [u8]> {
        self.next_is(tag::BYTES)?;
        self.length_bytes().ok()
    }

    /// A list next: how many values; [`Decoder::leave`] after them.
    pub fn list(&mut self) -> Option<usize> {
        self.next_is(tag::LIST)?;
        let n = self.count(1).ok()?;
        self.deeper().ok()?;
        Some(n)
    }

    /// A packed list of ints next ([`Decoder::packed_int`] each): how many.
    pub fn ints(&mut self) -> Option<usize> {
        self.next_is(tag::INTS)?;
        self.count(1).ok()
    }

    /// A stream next: its body (count and values, strings its own).
    pub fn stream(&mut self) -> Option<&'b [u8]> {
        self.next_is(tag::STREAM)?;
        self.stream_body().ok()
    }

    /// What's left to read.
    pub(crate) fn rest(&self) -> &'b [u8] {
        &self.b[self.i..]
    }

    /// A stream body's count, first thing in it.
    pub(crate) fn stream_count(&mut self) -> Option<usize> {
        self.count(1).ok()
    }

    /// A packed list of floats next: each one.
    /// (Written narrow when they fit: widened back.)
    pub fn floats(&mut self) -> Option<Vec<f64>> {
        if self.next_is(tag::FLOATS32).is_some() {
            return Some(crate::frame::floats::widen(
                self.packed_floats::<F32_BYTES>()
                    .ok()?
                    .as_slice()
                    .as_flattened(),
            ));
        }
        self.next_is(tag::FLOATS)?;
        Some(
            self.packed_floats::<F64_BYTES>()
                .ok()?
                .map(|f| f64::from_le_bytes(*f))
                .collect(),
        )
    }

    /// A packed list of 32-bit floats next: each one.
    pub fn floats32(&mut self) -> Option<Vec<f32>> {
        self.next_is(tag::FLOATS32)?;
        Some(
            self.packed_floats::<F32_BYTES>()
                .ok()?
                .map(|f| f32::from_le_bytes(*f))
                .collect(),
        )
    }

    /// One int of a packed list.
    pub fn packed_int(&mut self) -> Option<i64> {
        self.varint().ok().map(unzigzag)
    }

    /// A map next: how many entries, each a [`Decoder::key`] and a value;
    /// [`Decoder::leave`] after them.
    pub fn map(&mut self) -> Option<usize> {
        self.next_is(tag::MAP)?;
        let n = self.count(MAP_ENTRY_MIN).ok()?;
        self.deeper().ok()?;
        Some(n)
    }

    /// The end of a list or map.
    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// A map's next key.
    pub fn key(&mut self) -> Option<&'b str> {
        self.string().ok()
    }

    /// The next value, read and left (its strings still join the table).
    pub fn skip(&mut self) -> Option<()> {
        let arena = Bump::new();
        self.value_ref(&arena).ok().map(drop)
    }
}
