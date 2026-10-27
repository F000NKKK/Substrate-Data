//! Reading a shape: written, or named.

use super::seen_keys::SeenKeys;
use crate::binary::{DecodeError, Decoder, NEW_STRING};

impl Decoder<'_> {
    /// A shape: written (odd marker, its key count, then the keys as
    /// strings) or named (even marker, its number). The shape's number.
    pub(in crate::wire::binary) fn shape_spec(&mut self) -> Result<usize, DecodeError> {
        let marker = self.varint()?;
        if marker & NEW_STRING == 0 {
            let shape = usize::try_from(marker >> 1).unwrap_or(usize::MAX);
            return if shape < self.shapes.len() {
                Ok(shape)
            } else {
                Err(self.error(format!("shape {shape} isn't defined")))
            };
        }
        let left = self.b.len() - self.i;
        let n = match usize::try_from(marker >> 1) {
            Ok(0) => return Err(self.error("a shape with no keys")),
            Ok(n) if n <= left => n,
            _ => {
                let n = marker >> 1;
                return Err(self.error(format!("length {n} past the end ({left} bytes left)")));
            }
        };
        let mut seen = SeenKeys::default();
        for _ in 0..n {
            let at = self.i;
            let string = self.string_index()?;
            if !seen.insert(self.strings[string]) {
                return Err(DecodeError::duplicate_key(self.strings[string], at));
            }
            self.shapes.push_key(string);
        }
        Ok(self.shapes.seal())
    }

    /// How many values shape `shape` takes, checked against the bytes left
    /// (each value takes one byte at least).
    pub(in crate::wire::binary) fn shape_width(&self, shape: usize) -> Result<usize, DecodeError> {
        let n = self.shapes.width(shape);
        let left = self.b.len() - self.i;
        if n <= left {
            Ok(n)
        } else {
            Err(self.error(format!("length {n} past the end ({left} bytes left)")))
        }
    }
}
