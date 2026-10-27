//! A record that gains a field, held in other things.

use std::fmt::Debug;

use substrate_data::{Schema, Stream};

use crate::base::{Doc, Grown, In, base, grown};
use crate::verdict::Group::Nested;
use crate::verdict::{Row, row};

/// Every row of this group.
pub(crate) const ROWS: &[fn() -> Row] = &[
    in_field, in_option, in_box, in_array, in_vec, in_tuple, in_stream,
];

/// A stream, equal when its items are (its bytes may differ with the item type).
#[derive(Schema, Debug)]
struct Items<T: Schema> {
    items: Stream<T>,
}

impl<T: Schema + PartialEq> PartialEq for Items<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items.to_vec() == other.items.to_vec()
    }
}

/// `Doc` grown to `Grown`, held by `wrap_old` and `wrap_new`, with a field after.
fn grows<O, N>(
    change: &'static str,
    wrap_old: impl Fn(Doc) -> O,
    wrap_new: impl Fn(Grown) -> N,
) -> Row
where
    O: Schema + PartialEq + Debug,
    N: Schema + PartialEq + Debug,
{
    let old = || In {
        v: wrap_old(base()),
        after: 5,
    };
    let new = |tags: &[&str]| In {
        v: wrap_new(grown(tags)),
        after: 5,
    };
    row(
        Nested,
        change,
        (old(), Some(new(&[]))),
        (new(&["a"]), Some(old())),
    )
}

fn in_field() -> Row {
    grows("a record in a field", |b| b, |g| g)
}

fn in_option() -> Row {
    grows("a record in an `Option`", Some, Some)
}

fn in_box() -> Row {
    grows("a record in a `Box`", Box::new, Box::new)
}

fn in_array() -> Row {
    grows("records in an array", |b| [b, base()], |g| [g, grown(&[])])
}

fn in_vec() -> Row {
    grows("records in a `Vec`", |b| vec![b], |g| vec![g])
}

fn in_tuple() -> Row {
    grows("a record in a tuple", |b| (b, 1u8), |g| (g, 1u8))
}

fn in_stream() -> Row {
    grows(
        "records in a `Stream`",
        |b| Items {
            items: Stream::from(vec![b]),
        },
        |g| Items {
            items: Stream::from(vec![g]),
        },
    )
}
