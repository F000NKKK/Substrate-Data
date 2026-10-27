//! [`NamedType`]: a type of a format's own (`color`, `date`, `money`) that
//! is stored as another schema's data, and the shape tools know it by.

use super::{Reader, Schema, Shape, ShapeFn};
use crate::ValueRef;

/// A type of a format's own, stored as [`NamedType::Stored`] in every form
/// (text, binary, frames) and described by [`NamedType::NAME`], so a tool
/// that knows the name shows its own editor (a colour picker, a calendar)
/// and one that doesn't edits the stored data. [`crate::named_schema!`]
/// implements [`Schema`] for it.
pub trait NamedType: Sized {
    /// What tools know it by: `"color"`, `"date"`.
    const NAME: &'static str;
    /// What it's stored as.
    type Stored: Schema;
    /// Its stored form.
    fn to_stored(&self) -> Self::Stored;
    /// The value `stored` holds, if it's a valid one.
    fn from_stored(stored: Self::Stored) -> Option<Self>;
}

/// The shape of a [`NamedType`]: its name and its stored shape.
#[derive(Debug, Clone, Copy)]
pub struct NamedShape {
    name: &'static str,
    stored: ShapeFn,
}

impl NamedShape {
    /// The shape of `T`.
    pub fn of<T: NamedType>() -> NamedShape {
        NamedShape {
            name: T::NAME,
            stored: <T::Stored as Schema>::shape,
        }
    }

    /// What tools know it by.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// What it's stored as.
    pub fn stored(&self) -> Shape {
        (self.stored)()
    }
}

/// Reads a [`NamedType`] through its stored form (what [`crate::named_schema!`] calls).
pub fn read_named<T: NamedType>(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<T> {
    let stored = T::Stored::read(v, cx)?;
    T::from_stored(stored).or_else(|| {
        cx.problem(format!("not a valid {}", T::NAME));
        None
    })
}

/// Implements [`Schema`] for a [`NamedType`]: every form is its stored
/// form's, so a reader that only knows the shape reads it as stored.
#[macro_export]
macro_rules! named_schema {
    ($t:ty) => {
        impl $crate::Schema for $t {
            fn shape() -> $crate::Shape {
                $crate::Shape::named::<$t>()
            }
            fn read(
                v: &$crate::ValueRef<'_>,
                cx: &mut $crate::Reader<'_>,
            ) -> ::std::option::Option<Self> {
                $crate::schema::read_named::<$t>(v, cx)
            }
            fn write(&self) -> $crate::Value {
                $crate::Schema::write(&$crate::schema::NamedType::to_stored(self))
            }
            fn encode(&self, w: &mut $crate::binary::Writer) {
                $crate::Schema::encode(&$crate::schema::NamedType::to_stored(self), w);
            }
            fn decode(d: &mut $crate::binary::Decoder<'_>) -> ::std::option::Option<Self> {
                <<$t as $crate::schema::NamedType>::Stored as $crate::Schema>::decode(d)
                    .and_then(<$t as $crate::schema::NamedType>::from_stored)
            }
            fn pack(&self, p: &mut $crate::frame::Packer) {
                $crate::Schema::pack(&$crate::schema::NamedType::to_stored(self), p);
            }
            fn unpack(u: &mut $crate::frame::Unpacker<'_>) -> ::std::option::Option<Self> {
                <<$t as $crate::schema::NamedType>::Stored as $crate::Schema>::unpack(u)
                    .and_then(<$t as $crate::schema::NamedType>::from_stored)
            }
        }
    };
}
