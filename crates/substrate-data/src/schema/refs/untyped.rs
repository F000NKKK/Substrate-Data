//! The schemas of a reference of any kind and of a kind's name.

use super::read_ref;
use crate::diagnostic::Problem;
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{KindName, Ref, RefKey, Value};
use crate::schema::{Expected, Reader, Schema, Shape};

/// A reference of any kind: in a frame, its kind and its id, as text.
impl Schema for Ref {
    fn shape() -> Shape {
        Shape::Ref {
            kinds: &[],
            id: String::shape,
        }
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        if let ValueRef::Str(s) = v
            && !s.starts_with(crate::names::TYPED)
        {
            cx.expected(Expected::Reference, v);
            return None;
        }
        read_ref(v, cx, &[])
    }
    fn write(&self) -> Value {
        Value::Ref(self.clone())
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self.kind.as_str());
        p.put_str(self.id.as_str());
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        let kind = KindName::new(u.str()?).ok()?;
        let id = RefKey::new(u.str()?).ok()?;
        Some(Ref::new(kind, id))
    }
}

/// A kind's name as text: checked when it's read.
impl Schema for KindName {
    fn shape() -> Shape {
        Shape::Atom
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let ValueRef::Str(s) = v else {
            cx.expected(Expected::Name, v);
            return None;
        };
        KindName::new(s)
            .map_err(|e| cx.problem_of(Problem::BadTyped, e.to_string()))
            .ok()
    }
    fn write(&self) -> Value {
        Value::Str(self.as_str().to_owned())
    }
    fn pack(&self, p: &mut Packer) {
        self.atom().pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        KindName::new(crate::Atom::unpack(u)?.as_str()).ok()
    }
}
