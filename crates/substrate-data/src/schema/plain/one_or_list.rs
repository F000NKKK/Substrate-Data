//! One value, or a list of them: `"a"` or `["a", "b"]` — what a format
//! accepts where several will do (a field's accepted types, the files to
//! pick one from).

use crate::frame::{Packer, Unpacker};
use crate::schema::{Reader, Schema, Shape};
use crate::{Value, ValueRef};

/// One value or several, read either way and written back the same way (one
/// as itself). Never empty: an empty list is a problem.
#[derive(Debug, Clone, PartialEq)]
pub struct OneOrList<T>(pub Vec<T>);

impl<T: Schema> Schema for OneOrList<T> {
    fn shape() -> Shape {
        Shape::List(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        match v {
            ValueRef::List([]) => {
                cx.problem("an empty list: at least one");
                None
            }
            ValueRef::List(_) => Vec::<T>::read(v, cx).map(OneOrList),
            _ => T::read(v, cx).map(|one| OneOrList(vec![one])),
        }
    }
    fn write(&self) -> Value {
        match self.0.as_slice() {
            [one] => one.write(),
            many => Value::List(many.iter().map(Schema::write).collect()),
        }
    }
    /// In a frame it's a list (one or more), as its shape says.
    fn pack(&self, p: &mut Packer) {
        T::pack_list(&self.0, p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        T::unpack_list(u)
            .filter(|items| !items.is_empty())
            .map(OneOrList)
    }
}
