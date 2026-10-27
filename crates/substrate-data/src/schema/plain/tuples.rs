//! The schemas of tuples: lists of their items in order.

use crate::frame::{Packer, Unpacker};
use crate::model::borrowed::ValueRef;
use crate::model::value::{Seg, Value};
use crate::schema::{Expected, Reader, Schema, Shape};

/// A tuple, written as a list of its items in order: a pair `[a, b]` (a
/// point, a size), a triple `[x, y, z]` (a position).
macro_rules! tuples {
    ($($n:literal => ($($t:ident $i:tt),+);)+) => {$(
        impl<$($t: Schema),+> Schema for ($($t,)+) {
            fn shape() -> Shape {
                Shape::List(Value::shape)
            }
            fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
                let ValueRef::List(items) = v else {
                    cx.expected(Expected::Items($n), v);
                    return None;
                };
                if items.len() != $n {
                    cx.problem(format!("expected {} items, found {}", $n, items.len()));
                    return None;
                }
                let read = ($(cx.at(Seg::Index($i), |cx| $t::read(&items[$i], cx)),)+);
                Some(($(read.$i?,)+))
            }
            fn write(&self) -> Value {
                Value::List(vec![$(self.$i.write()),+])
            }
            /// In a frame it's a list of any values, as its shape says:
            /// each item in its binary form.
            fn pack(&self, p: &mut Packer) {
                let at = p.begin_list($n);
                $(p.put_tagged(&self.$i);)+
                p.end(at);
            }
            fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
                let (n, end) = u.list()?;
                if n != $n {
                    return None;
                }
                let items = ($(u.tagged::<$t>()?,)+);
                u.skip_to(end)?;
                Some(items)
            }
        }
    )+};
}

tuples! {
    2 => (A 0, B 1);
    3 => (A 0, B 1, C 2);
    4 => (A 0, B 1, C 2, D 3);
}
