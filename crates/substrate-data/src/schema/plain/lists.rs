//! The schemas of lists, sets, boxed slices and arrays.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Seg, Value};
use crate::schema::{Expected, Reader, Schema, Shape};

impl<T: Schema> Schema for Vec<T> {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let ValueRef::List(items) = v else {
            cx.expected(Expected::List, v);
            return None;
        };
        let mut out = Vec::with_capacity(items.len());
        let mut ok = true;
        for (i, item) in items.iter().enumerate() {
            match cx.at(Seg::Index(i), |cx| T::read(item, cx)) {
                Some(x) => out.push(x),
                None => ok = false,
            }
        }
        ok.then_some(out)
    }
    fn write(&self) -> Value {
        Value::List(self.iter().map(T::write).collect())
    }
    fn encode(&self, w: &mut Writer) {
        T::encode_list(self, w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        T::decode_list(d)
    }
    fn pack(&self, p: &mut Packer) {
        T::pack_list(self, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u)
    }
}

/// A set: a list, in order, each once.
impl<T: Schema + Ord + Clone> Schema for std::collections::BTreeSet<T> {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        Vec::<T>::read(v, cx).map(Self::from_iter)
    }
    fn write(&self) -> Value {
        Value::List(self.iter().map(T::write).collect())
    }
    fn encode(&self, w: &mut Writer) {
        // As a list of them is written (numbers packed).
        T::encode_list(&self.iter().cloned().collect::<Vec<_>>(), w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        Vec::<T>::decode(d).map(Self::from_iter)
    }
    /// In a frame it's a list, as its shape says.
    fn pack(&self, p: &mut Packer) {
        T::pack_list(&self.iter().cloned().collect::<Vec<_>>(), p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u).map(Self::from_iter)
    }
}

/// A string that won't change: a pointer and a length, no spare capacity
/// (`String`'s third word) — what big tables of names keep.
/// A list that won't change: no spare capacity.
impl<T: Schema> Schema for Box<[T]> {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        Vec::<T>::read(v, cx).map(Vec::into_boxed_slice)
    }
    fn write(&self) -> Value {
        Value::List(self.iter().map(T::write).collect())
    }
    fn encode(&self, w: &mut Writer) {
        T::encode_list(self, w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        T::decode_list(d).map(Vec::into_boxed_slice)
    }
    fn pack(&self, p: &mut Packer) {
        T::pack_list(self, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u).map(Vec::into_boxed_slice)
    }
}

impl<T: Schema, const N: usize> Schema for [T; N] {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let items = Vec::<T>::read(v, cx)?;
        let len = items.len();
        items.try_into().ok().or_else(|| {
            cx.problem(format!("expected {N} items, found {len}"));
            None
        })
    }
    fn write(&self) -> Value {
        Value::List(self.iter().map(T::write).collect())
    }
    fn encode(&self, w: &mut Writer) {
        T::encode_list(self, w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        T::decode_list(d)?.try_into().ok()
    }
    fn pack(&self, p: &mut Packer) {
        T::pack_list(self, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u)?.try_into().ok()
    }
}
