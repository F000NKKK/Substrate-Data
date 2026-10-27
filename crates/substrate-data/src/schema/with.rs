//! Field codecs: `#[data(with = "…")]` reads and writes a field through a
//! type other than its own — like serde's `with`. The field keeps the plain
//! type a program uses (`String`), the format says what it holds (a
//! reference to a file), checked and resolved by the [`Reader`]'s
//! [`Resolve`](crate::schema::Resolve).
//!
//! ```
//! use substrate_data::with::{Opt, RefTo};
//!
//! substrate_data::kind!(File = "file", User = "user");
//!
//! #[derive(substrate_data::Schema)]
//! struct Attachment {
//!     /// `@file:…`, kept as the plain id.
//!     #[data(with = "RefTo<File>")]
//!     file: String,
//!     #[data(with = "Opt<RefTo<User>>")]
//!     owner: Option<String>,
//! }
//!
//! let a: Attachment =
//!     substrate_data::from_text(r#"{ file: "@file:images/logo.png" }"#).unwrap().value;
//! assert_eq!(a.file, "images/logo.png");
//! assert_eq!(a.owner, None);
//! ```

use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Map, Ref, Seg, Value};
use crate::schema::{Kinds, Reader, Schema, Shape, is_metadata_key};

/// How a field of type `F` is read and written.
pub trait With<F> {
    /// The field's shape.
    fn shape() -> Shape;
    /// Reads the field.
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<F>;
    /// Writes the field.
    fn write(f: &F) -> Value;
    /// The field's value when it isn't there (`None`: it's required).
    #[must_use]
    fn absent() -> Option<F> {
        None
    }
    /// Whether a struct leaves the field out.
    fn skip(_f: &F) -> bool {
        false
    }
    /// The field in the binary form, straight: its value encoded unless
    /// the codec knows better.
    fn encode(f: &F, w: &mut Writer) {
        w.value(&Self::write(f));
    }
    /// The field from the binary form, straight; `None` sends the read the
    /// slow way (see [`Schema::decode`]).
    fn decode(d: &mut Decoder<'_>) -> Option<F> {
        let arena = bumpalo::Bump::new();
        let v = d.value_ref(&arena).ok()?;
        let mut cx = Reader::new();
        let f = Self::read(&v, &mut cx)?;
        cx.problems.is_empty().then_some(f)
    }
    /// The field packed into a frame: its binary form as bytes unless the
    /// codec knows better.
    fn pack(f: &F, p: &mut Packer) {
        let mut w = Writer::body();
        Self::encode(f, &mut w);
        p.put_bytes(&w.finish());
    }
    /// The field out of a frame.
    fn unpack(u: &mut Unpacker<'_>) -> Option<F> {
        let mut d = Decoder::body(u.bytes()?);
        let f = Self::decode(&mut d)?;
        d.at_end().then_some(f)
    }
}

/// A reference of kind(s) `K`, kept as the resolved id.
#[derive(Debug)]
pub struct RefTo<K>(PhantomData<K>);

impl<K: Kinds> With<K::Id> for RefTo<K> {
    fn shape() -> Shape {
        super::ref_shape::<K>()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<K::Id> {
        super::read_id::<K>(v, cx)
    }
    fn write(f: &K::Id) -> Value {
        Value::Ref(Ref::new(
            K::first(),
            crate::model::value::RefKey::spelled(&crate::schema::RefId::spell(f)),
        ))
    }
    fn pack(f: &K::Id, p: &mut Packer) {
        f.pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<K::Id> {
        K::Id::unpack(u)
    }
}

/// Text as it's spelled — markup's attribute, which may spell a reference
/// (`@file:images/logo.png`): a string kept as written, a typed value as its
/// spelling. Nothing is checked or resolved here.
#[derive(Debug)]
pub struct Spelled;

impl With<String> for Spelled {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<String> {
        if let ValueRef::Str(s) = v {
            return Some(s.to_string());
        }
        let spelled = crate::typed::spelling(&v.to_value());
        if spelled.is_none() {
            cx.expected(super::Expected::Text, v);
        }
        spelled
    }
    fn write(f: &String) -> Value {
        Value::Str(f.clone())
    }
}

/// An optional field through `W`.
#[derive(Debug)]
pub struct Opt<W>(PhantomData<W>);

impl<F, W: With<F>> With<Option<F>> for Opt<W> {
    fn shape() -> Shape {
        Shape::Optional(W::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Option<F>> {
        if v.is_null() {
            Some(None)
        } else {
            W::read(v, cx).map(Some)
        }
    }
    fn write(f: &Option<F>) -> Value {
        f.as_ref().map_or(Value::Null, W::write)
    }
    fn absent() -> Option<Option<F>> {
        Some(None)
    }
    fn skip(f: &Option<F>) -> bool {
        f.is_none()
    }
}

/// A list, each item through `W`.
#[derive(Debug)]
pub struct List<W>(PhantomData<W>);

impl<F, W: With<F>> With<Vec<F>> for List<W> {
    fn shape() -> Shape {
        Shape::List(W::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Vec<F>> {
        let ValueRef::List(items) = v else {
            cx.expected(super::Expected::List, v);
            return None;
        };
        let mut out = Vec::with_capacity(items.len());
        let mut ok = true;
        for (i, item) in items.iter().enumerate() {
            match cx.at(Seg::Index(i), |cx| W::read(item, cx)) {
                Some(x) => out.push(x),
                None => ok = false,
            }
        }
        ok.then_some(out)
    }
    fn write(f: &Vec<F>) -> Value {
        Value::List(f.iter().map(W::write).collect())
    }
}

/// An object of any keys, each value through `W`.
#[derive(Debug)]
pub struct Values<W>(PhantomData<W>);

impl<F, W: With<F>> With<BTreeMap<String, F>> for Values<W> {
    fn shape() -> Shape {
        Shape::Map(W::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<BTreeMap<String, F>> {
        read_values::<F, W, _>(v, cx)
    }
    fn write(f: &BTreeMap<String, F>) -> Value {
        write_values::<F, W>(f.iter())
    }
}

impl<F, W: With<F>> With<HashMap<String, F>> for Values<W> {
    fn shape() -> Shape {
        Shape::Map(W::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<HashMap<String, F>> {
        read_values::<F, W, _>(v, cx)
    }
    fn write(f: &HashMap<String, F>) -> Value {
        write_values::<F, W>(f.iter())
    }
}

/// An object's entries (its notes left out), each value through `W`.
fn read_values<F, W: With<F>, M: Default + Extend<(String, F)>>(
    v: &ValueRef<'_>,
    cx: &mut Reader<'_>,
) -> Option<M> {
    let map = cx.map(v)?;
    let mut out = M::default();
    let mut ok = true;
    for (k, item) in map.iter().filter(|(k, _)| !is_metadata_key(k)) {
        match cx.at(Seg::Key(k.to_string()), |cx| W::read(item, cx)) {
            Some(x) => out.extend([(k.to_string(), x)]),
            None => ok = false,
        }
    }
    ok.then_some(out)
}

fn write_values<'a, F: 'a, W: With<F>>(
    entries: impl Iterator<Item = (&'a String, &'a F)>,
) -> Value {
    Value::Map(
        entries
            .map(|(k, v)| (k.clone(), W::write(v)))
            .collect::<Map>(),
    )
}

/// The field's own schema — `with` spelled out (`List<Plain>` for a `Vec<T: Schema>`).
#[derive(Debug)]
pub struct Plain;

impl<F: Schema> With<F> for Plain {
    fn shape() -> Shape {
        F::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<F> {
        F::read(v, cx)
    }
    fn write(f: &F) -> Value {
        f.write()
    }
    fn absent() -> Option<F> {
        F::absent()
    }
    fn skip(f: &F) -> bool {
        f.skip()
    }
    fn encode(f: &F, w: &mut Writer) {
        f.encode(w);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<F> {
        F::decode(d)
    }
    fn pack(f: &F, p: &mut Packer) {
        f.pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<F> {
        F::unpack(u)
    }
}

/// Bytes as they are (`Vec<u8>`): a payload a format doesn't look into.
#[derive(Debug)]
pub struct Raw;

impl With<Vec<u8>> for Raw {
    fn shape() -> Shape {
        Shape::Bytes
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Vec<u8>> {
        match v {
            ValueRef::Bytes(b) => Some(b.to_vec()),
            // As text: `@bytes:<base64>`.
            ValueRef::Str(s) => {
                if let crate::typed::Spelled::Bytes(b) = crate::typed::classify(s)
                    .map_err(|e| cx.problem_of(crate::Problem::BadTyped, e.to_string()))
                    .ok()?
                {
                    Some(b)
                } else {
                    cx.expected(super::Expected::Bytes, v);
                    None
                }
            }
            other => {
                cx.expected(super::Expected::Bytes, other);
                None
            }
        }
    }
    fn write(f: &Vec<u8>) -> Value {
        Value::Bytes(f.clone())
    }
    fn encode(f: &Vec<u8>, w: &mut Writer) {
        w.bytes(f);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Vec<u8>> {
        d.bytes().map(<[u8]>::to_vec)
    }
    fn pack(f: &Vec<u8>, p: &mut Packer) {
        p.put_bytes(f);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Vec<u8>> {
        u.bytes().map(<[u8]>::to_vec)
    }
    fn absent() -> Option<Vec<u8>> {
        Some(Vec::new())
    }
}
