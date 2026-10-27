//! [`Id`]: the id of something of a kind, of that kind's id type.

use std::fmt;
use std::marker::PhantomData;

use super::{Kind, RefId, read_id, ref_shape};
use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Ref, Value};
use crate::schema::{Reader, Schema, Shape};

/// The id of something of kind `K`, of `K`'s id type: written
/// `@file:images/logo.png`, `@user:42`.
pub struct Id<K: Kind> {
    /// The id.
    pub id: K::Id,
    kind: PhantomData<K>,
}

impl<K: Kind> Id<K> {
    /// The id `id` of something of kind `K`.
    pub fn new(id: impl Into<K::Id>) -> Self {
        Self {
            id: id.into(),
            kind: PhantomData,
        }
    }

    /// As a reference.
    pub fn to_ref(&self) -> Ref {
        Ref::of::<K>(&self.id)
    }
}

impl<K: Kind> Clone for Id<K> {
    fn clone(&self) -> Self {
        Self::new(self.id.clone())
    }
}
impl<K: Kind> PartialEq for Id<K> {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id
    }
}
impl<K: Kind> Eq for Id<K> where K::Id: Eq {}
impl<K: Kind> std::hash::Hash for Id<K>
where
    K::Id: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}
impl<K: Kind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}:{}", K::NAME, self.id.spell())
    }
}
impl<K: Kind> fmt::Display for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id.spell())
    }
}

/// In a frame, only its id (the kind is the field's).
impl<K: Kind> Schema for Id<K> {
    fn shape() -> Shape {
        ref_shape::<K>()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        read_id::<K>(v, cx).map(Id::new)
    }
    fn write(&self) -> Value {
        Value::Ref(self.to_ref())
    }
    fn pack(&self, p: &mut Packer) {
        self.id.pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        K::Id::unpack(u).map(Id::new)
    }
}
