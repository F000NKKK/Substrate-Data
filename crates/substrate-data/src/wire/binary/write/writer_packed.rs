//! [`Writer`] writing packed lists of numbers: no tag for each.

use super::Writer;
use crate::binary::{tag, varint, zigzag};

impl Writer {
    /// A list of floats, packed, no tag each: four bytes each when every
    /// one is an `f32` exactly ([`tag::FLOATS32`]), else eight.
    pub fn floats(&mut self, items: &[f64]) -> &mut Self {
        let narrow = crate::frame::floats::fits_f32(items);
        self.out
            .push(if narrow { tag::FLOATS32 } else { tag::FLOATS });
        varint(&mut self.out, items.len() as u64);
        if narrow {
            crate::frame::floats::put_narrow(&mut self.out, items);
        } else {
            crate::frame::floats::put_wide(&mut self.out, items);
        }
        self
    }

    /// A list of 32-bit floats, packed: four bytes each, no tag.
    pub fn floats32(&mut self, items: &[f32]) -> &mut Self {
        self.out.push(tag::FLOATS32);
        varint(&mut self.out, items.len() as u64);
        crate::frame::floats::put_f32s(&mut self.out, items);
        self
    }

    /// A list of ints, packed (no tag each).
    pub fn ints(&mut self, items: impl ExactSizeIterator<Item = i64>) -> &mut Self {
        self.out.push(tag::INTS);
        varint(&mut self.out, items.len() as u64);
        for n in items {
            varint(&mut self.out, zigzag(n));
        }
        self
    }
}
