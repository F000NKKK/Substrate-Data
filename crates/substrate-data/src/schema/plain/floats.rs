//! The schemas of floats: raw in frames, `f32`-exact lists narrowed.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Expected, Reader, Schema, Shape};

/// What a schema needs of a float type: its width, and the model's `f64`
/// both ways.
trait Float: Copy {
    /// Its width in bits.
    const BITS: u32;
    /// From the model's `f64` (an `f32` rounded to the nearest).
    fn of_f64(f: f64) -> Self;
    /// As the model's `f64`.
    fn to_f64(self) -> f64;
}

impl Float for f32 {
    const BITS: u32 = 32;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a number read as an f32 is rounded to one"
    )]
    fn of_f64(f: f64) -> Self {
        f as f32
    }
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
}

impl Float for f64 {
    const BITS: u32 = 64;
    fn of_f64(f: f64) -> Self {
        f
    }
    fn to_f64(self) -> f64 {
        self
    }
}

macro_rules! float_schema {
    ($($t:ty => $one:ident, $many:ident, $read_many:ident, $put:ident, $get:ident, $puts:ident, $gets:ident);*) => {$(
        impl Schema for $t {
            fn shape() -> Shape {
                Shape::Float { bits: <$t as Float>::BITS }
            }
            fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
                v.as_f64().map(<$t as Float>::of_f64).or_else(|| {
                    cx.expected(Expected::Number, v);
                    None
                })
            }
            fn write(&self) -> Value {
                Value::Float(self.to_f64())
            }
            fn encode(&self, w: &mut Writer) {
                w.$one(*self);
            }
            fn decode(d: &mut Decoder<'_>) -> Option<Self> {
                d.float().map(<$t as Float>::of_f64)
            }
            fn encode_list(items: &[Self], w: &mut Writer) {
                w.$many(items);
            }
            fn decode_list(d: &mut Decoder<'_>) -> Option<Vec<Self>> {
                d.$read_many()
            }
            fn pack(&self, p: &mut Packer) {
                p.$put(*self);
            }
            fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
                u.$get()
            }
            fn pack_list(items: &[Self], p: &mut Packer) {
                p.$puts(items);
            }
            fn unpack_list(u: &mut Unpacker<'_>) -> Option<Vec<Self>> {
                u.$gets()
            }
        }
    )*};
}
float_schema!(
    f32 => float32, floats32, floats32, put_f32, f32, put_f32s, f32s;
    f64 => float, floats, floats, put_f64, f64, put_f64s, f64s
);
