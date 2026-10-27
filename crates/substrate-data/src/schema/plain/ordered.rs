//! [`Ordered`]: an object kept in file order.

use super::maps::read_map;
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;
use crate::schema::{Reader, Schema, Shape};

/// An object kept in file order, values of one schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Ordered<T>(pub Vec<(String, T)>);

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: Schema> Schema for Ordered<T> {
    fn shape() -> Shape {
        Shape::Map(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let mut out = Vec::new();
        read_map(v, cx, |k, x| out.push((k, x)))?;
        Some(Ordered(out))
    }
    fn write(&self) -> Value {
        Value::Map(self.0.iter().map(|(k, v)| (k.clone(), v.write())).collect())
    }
}
