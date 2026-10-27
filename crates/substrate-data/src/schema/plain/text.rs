//! The schemas of text: a leading `@` doubled so it reads back as text.

use crate::binary::{Decoder, Writer};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Expected, Reader, Schema, Shape};
use crate::typed;

/// What a typed value's string starts with (`@file:…`).
fn typed_mark() -> char {
    crate::names::TYPED
}

/// Plain text as a value: a leading `@` doubled, so it reads back as text
/// and not as a typed value.
fn plain(s: &str) -> Value {
    Value::Str(typed::escape_at(s))
}

/// Plain text written straight, its leading `@` doubled like [`plain`]'s.
fn put_plain(w: &mut Writer, s: &str) {
    if s.starts_with(typed_mark()) {
        w.str(&typed::escape_at(s));
    } else {
        w.str(s);
    }
}

impl Schema for String {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        match v {
            // Plain text, the usual case: copied once, into the result.
            ValueRef::Str(s) if !s.starts_with('@') => Some(s.to_string()),
            ValueRef::Str(s) => match typed::classify(s) {
                Ok(typed::Spelled::Plain(s)) => Some(s),
                Ok(other) => {
                    cx.problem(format!(
                        "expected plain text, found {} (write `@@` for a literal `@`)",
                        other.kind()
                    ));
                    None
                }
                Err(e) => {
                    cx.problem_of(crate::diagnostic::Problem::BadTyped, e.to_string());
                    None
                }
            },
            other => {
                cx.expected(Expected::Str, other);
                None
            }
        }
    }
    fn write(&self) -> Value {
        plain(self)
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, self);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        // A typed value (`@…`) is the slow way's to tell apart.
        d.str()
            .filter(|s| !s.starts_with(typed_mark()))
            .map(str::to_owned)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.str().map(str::to_owned)
    }
}

/// Text that may be a constant: borrowed where it's written, owned once read.
impl Schema for std::borrow::Cow<'static, str> {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        String::read(v, cx).map(Self::Owned)
    }
    fn write(&self) -> Value {
        plain(self)
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, self);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        String::decode(d).map(Self::Owned)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        String::unpack(u).map(Self::Owned)
    }
}

/// A file's path, as text (a path that isn't Unicode loses what isn't).
impl Schema for std::path::PathBuf {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        String::read(v, cx).map(Self::from)
    }
    fn write(&self) -> Value {
        plain(&self.to_string_lossy())
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, &self.to_string_lossy());
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        String::decode(d).map(Self::from)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(&self.to_string_lossy());
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.str().map(Self::from)
    }
}

/// Text everywhere but in a frame, which carries its number.
impl Schema for crate::Atom {
    fn shape() -> Shape {
        Shape::Atom
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        String::read(v, cx).map(|s| crate::Atom::new(&s))
    }
    fn write(&self) -> Value {
        plain(self.as_str())
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, self.as_str());
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        d.str()
            .filter(|s| !s.starts_with(typed_mark()))
            .map(crate::Atom::new)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_u(u64::from(self.number()));
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        crate::Atom::from_number(u32::try_from(u.uint()?).ok()?)
    }
}

impl Schema for Box<str> {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        String::read(v, cx).map(String::into_boxed_str)
    }
    fn write(&self) -> Value {
        plain(self)
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, self);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        String::decode(d).map(String::into_boxed_str)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.str().map(Into::into)
    }
}

/// A string shared without copying.
impl Schema for crate::sync::Arc<str> {
    fn shape() -> Shape {
        Shape::Str
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        String::read(v, cx).map(Into::into)
    }
    fn write(&self) -> Value {
        plain(self)
    }
    fn encode(&self, w: &mut Writer) {
        put_plain(w, self);
    }
    fn decode(d: &mut Decoder<'_>) -> Option<Self> {
        String::decode(d).map(Into::into)
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        u.shared_str()
    }
}
