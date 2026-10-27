//! The schemas of integers: checked against their type's range, bit-packed in lists.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Expected, Reader, Schema, Shape};

/// What a schema needs of an integer type: its range, and the `i64` and
/// bit-packed forms it is carried in, both ways, without a lossy cast.
trait Int: Copy + Sized {
    /// Its range, as the shape says it (`u64` and `usize` up to `i64::MAX`).
    fn range() -> (i64, i64);
    /// As the model's `i64` (a 64-bit unsigned number: the `i64` of its bits).
    fn to_i64(self) -> i64;
    /// From the model's `i64`, if it fits.
    fn of_i64(n: i64) -> Option<Self>;
    /// As a bit-packed list's bits: a signed number zigzagged.
    fn to_bits(self) -> u64;
    /// From a bit-packed list's bits, if it fits.
    fn of_bits(v: u64) -> Option<Self>;
}

macro_rules! signed {
    ($($t:ty),*) => {$(
        impl Int for $t {
            fn range() -> (i64, i64) {
                (i64::from(<$t>::MIN), i64::from(<$t>::MAX))
            }
            fn to_i64(self) -> i64 {
                i64::from(self)
            }
            fn of_i64(n: i64) -> Option<Self> {
                Self::try_from(n).ok()
            }
            fn to_bits(self) -> u64 {
                crate::binary::zigzag(i64::from(self))
            }
            fn of_bits(v: u64) -> Option<Self> {
                Self::try_from(crate::binary::unzigzag(v)).ok()
            }
        }
    )*};
}
signed!(i8, i16, i32, i64);

macro_rules! unsigned {
    ($($t:ty),*) => {$(
        impl Int for $t {
            fn range() -> (i64, i64) {
                (0, i64::from(<$t>::MAX))
            }
            fn to_i64(self) -> i64 {
                i64::from(self)
            }
            fn of_i64(n: i64) -> Option<Self> {
                Self::try_from(n).ok()
            }
            fn to_bits(self) -> u64 {
                u64::from(self)
            }
            fn of_bits(v: u64) -> Option<Self> {
                Self::try_from(v).ok()
            }
        }
    )*};
}
unsigned!(u8, u16, u32);

impl Int for u64 {
    fn range() -> (i64, i64) {
        (0, i64::MAX)
    }
    fn to_i64(self) -> i64 {
        i64::from_ne_bytes(self.to_ne_bytes())
    }
    fn of_i64(n: i64) -> Option<Self> {
        Some(u64::from_ne_bytes(n.to_ne_bytes()))
    }
    fn to_bits(self) -> u64 {
        self
    }
    fn of_bits(v: u64) -> Option<Self> {
        Some(v)
    }
}

impl Int for usize {
    fn range() -> (i64, i64) {
        (0, i64::MAX)
    }
    fn to_i64(self) -> i64 {
        u64::try_from(self).unwrap_or(u64::MAX).to_i64()
    }
    fn of_i64(n: i64) -> Option<Self> {
        Self::try_from(u64::of_i64(n)?).ok()
    }
    fn to_bits(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
    fn of_bits(v: u64) -> Option<Self> {
        Self::try_from(v).ok()
    }
}

macro_rules! int_schema {
    ($($t:ty),*) => {$(
        impl Schema for $t {
            fn shape() -> Shape {
                let (min, max) = <$t as Int>::range();
                Shape::Int { min, max }
            }
            fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
                let Some(n) = v.as_i64() else {
                    cx.expected(Expected::Integer, v);
                    return None;
                };
                <$t as Int>::of_i64(n).or_else(|| {
                    cx.problem(format!("{n} is out of range ({}..={})", <$t>::MIN, <$t>::MAX));
                    None
                })
            }
            fn write(&self) -> Value {
                Value::Int(self.to_i64())
            }
            fn encode(&self, w: &mut Writer) {
                w.int(self.to_i64());
            }
            fn decode(d: &mut Decoder<'_>) -> Option<Self> {
                d.int().and_then(<$t as Int>::of_i64)
            }
            fn pack(&self, p: &mut Packer) {
                p.put_i(self.to_i64());
            }
            fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
                u.int().and_then(<$t as Int>::of_i64)
            }
            fn pack_list(items: &[Self], p: &mut Packer) {
                p.put_uints(items.len(), items.iter().map(|&n| n.to_bits()));
            }
            fn unpack_list(u: &mut Unpacker<'_>) -> Option<Vec<Self>> {
                u.uints(<$t as Int>::of_bits)
            }
            fn encode_list(items: &[Self], w: &mut Writer) {
                w.ints(items.iter().map(|&n| n.to_i64()));
            }
            fn decode_list(d: &mut Decoder<'_>) -> Option<Vec<Self>> {
                let n = d.ints()?;
                let mut out = Vec::with_capacity(n);
                for _ in 0..n {
                    out.push(<$t as Int>::of_i64(d.packed_int()?)?);
                }
                Some(out)
            }
        }
    )*};
}
int_schema!(i8, i16, i32, i64, u8, u16, u32, u64, usize);
