//! The schemas of `Option` and `Box`: their content's.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Reader, Schema, Shape};

impl<T: Schema> Schema for Option<T> {
    fn shape() -> Shape {
        Shape::Optional(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        if v.is_null() {
            Some(None)
        } else {
            T::read(v, cx).map(Some)
        }
    }
    fn write(&self) -> Value {
        self.as_ref().map_or(Value::Null, T::write)
    }
    fn write_text(&self, w: &mut crate::text::Writer) {
        match self {
            Some(v) => v.write_text(w),
            None => {
                w.null();
            }
        }
    }
    fn encode(&self, w: &mut Writer) {
        match self {
            Some(v) => v.encode(w),
            None => {
                w.null();
            }
        }
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        if d.null() {
            Some(None)
        } else {
            T::decode(d).map(Some)
        }
    }
    fn pack(&self, p: &mut Packer) {
        p.put_bool(self.is_some());
        if let Some(v) = self {
            v.pack(p);
        }
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        if u.bool()? {
            T::unpack(u).map(Some)
        } else {
            Some(None)
        }
    }
    fn absent() -> Option<Self> {
        Some(None)
    }
    fn skip(&self) -> bool {
        self.is_none()
    }
}

impl<T: Schema> Schema for Box<T> {
    fn shape() -> Shape {
        T::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        T::read(v, cx).map(Box::new)
    }
    fn write(&self) -> Value {
        (**self).write()
    }
    fn write_text(&self, w: &mut crate::text::Writer) {
        (**self).write_text(w);
    }
    fn encode(&self, w: &mut Writer) {
        (**self).encode(w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        T::decode(d).map(Box::new)
    }
    fn pack(&self, p: &mut Packer) {
        (**self).pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack(u).map(Box::new)
    }
    fn absent() -> Option<Self> {
        T::absent().map(Box::new)
    }
    fn skip(&self) -> bool {
        (**self).skip()
    }
}
