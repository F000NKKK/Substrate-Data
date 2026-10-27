//! The schemas of a value of any shape, `bool` and `()`.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Expected, Reader, Schema, Shape};

impl Schema for Value {
    fn shape() -> Shape {
        Shape::Any
    }
    fn read(v: &ValueRef<'_>, _: &mut Reader<'_>) -> Option<Self> {
        Some(v.to_value())
    }
    fn write(&self) -> Value {
        self.clone()
    }
}

impl Schema for bool {
    fn shape() -> Shape {
        Shape::Bool
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        v.as_bool().or_else(|| {
            cx.expected(Expected::Bool, v);
            None
        })
    }
    fn write(&self) -> Value {
        Value::Bool(*self)
    }
    fn encode(&self, w: &mut Writer) {
        w.bool(*self);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        d.bool()
    }
    fn pack(&self, p: &mut Packer) {
        p.put_bool(*self);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.bool()
    }
}

/// A number as carried (an `i64`) back to `T`: out of `T`'s range, none.
/// Nothing: what a call answers when it only says it's done.
impl Schema for () {
    fn shape() -> Shape {
        Shape::Any
    }
    fn read(_v: &ValueRef<'_>, _cx: &mut Reader<'_>) -> Option<Self> {
        Some(())
    }
    fn write(&self) -> Value {
        Value::Null
    }
}
