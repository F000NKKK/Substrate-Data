//! Variants added, moved and removed.

use substrate_data::{Schema, Value, from_text};

use crate::base::{Base, Grown, base, grown};
use crate::verdict::Group::Enums;
use crate::verdict::{Row, row};

/// Every row of this group.
pub(crate) const ROWS: &[fn() -> Row] = &[
    unit_append,
    unit_insert,
    unit_reorder,
    unit_remove,
    tagged_append,
    tagged_append_with_other,
    tagged_reorder,
    tagged_variant_gains_field,
    tagged_payload_grows,
];

#[derive(Schema, Debug, PartialEq)]
enum Two {
    A,
    B,
}

fn unit_append() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    enum New {
        A,
        B,
        C,
    }
    row(
        Enums,
        "unit enum: append a variant",
        (Two::B, Some(New::B)),
        (New::C, None),
    )
}

fn unit_insert() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    enum New {
        A,
        X,
        B,
    }
    row(
        Enums,
        "unit enum: insert a variant before the last",
        (Two::B, Some(New::B)),
        (New::X, None),
    )
}

fn unit_reorder() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    enum New {
        B,
        A,
    }
    row(
        Enums,
        "unit enum: swap two variants",
        (Two::A, Some(New::A)),
        (New::A, Some(Two::A)),
    )
}

fn unit_remove() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    enum New {
        A,
    }
    row(
        Enums,
        "unit enum: remove the last variant",
        (Two::B, None),
        (New::A, Some(Two::A)),
    )
}

#[derive(Schema, Debug, PartialEq)]
#[data(tag = "type")]
enum Tagged {
    A { x: u32 },
    N,
}

fn tagged_append() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    #[data(tag = "type")]
    enum New {
        A { x: u32 },
        N,
        C { y: String },
    }
    row(
        Enums,
        "tagged enum: append a variant",
        (Tagged::A { x: 1 }, Some(New::A { x: 1 })),
        (New::C { y: "s".into() }, None),
    )
}

/// The catch-all an older reader keeps for variants it does not know.
#[derive(Schema, Debug, PartialEq)]
#[data(tag = "type")]
enum Open {
    A {
        x: u32,
    },
    #[data(other)]
    Unknown(Value),
}

fn tagged_append_with_other() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    #[data(tag = "type")]
    enum New {
        A { x: u32 },
        C { y: String },
    }
    let kept = from_text::<Open>(r#"{ type: "c", y: "s" }"#)
        .expect("the catch-all reads any variant")
        .value;
    row(
        Enums,
        "tagged enum with an `other` variant (old): append a variant",
        (Open::A { x: 1 }, Some(New::A { x: 1 })),
        (New::C { y: "s".into() }, Some(kept)),
    )
}

fn tagged_reorder() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    #[data(tag = "type")]
    enum New {
        N,
        A { x: u32 },
    }
    row(
        Enums,
        "tagged enum: swap two variants",
        (Tagged::A { x: 1 }, Some(New::A { x: 1 })),
        (New::A { x: 1 }, Some(Tagged::A { x: 1 })),
    )
}

fn tagged_variant_gains_field() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    #[data(tag = "type")]
    enum New {
        A {
            x: u32,
            #[data(default)]
            z: u8,
        },
        N,
    }
    row(
        Enums,
        "tagged enum: a variant gains a field with `default`",
        (Tagged::A { x: 1 }, Some(New::A { x: 1, z: 0 })),
        (New::A { x: 1, z: 9 }, Some(Tagged::A { x: 1 })),
    )
}

#[derive(Schema, Debug, PartialEq)]
#[data(tag = "type")]
enum Holds {
    Rec(Base),
    N,
}

fn tagged_payload_grows() -> Row {
    #[derive(Schema, Debug, PartialEq)]
    #[data(tag = "type")]
    enum New {
        Rec(Grown),
        N,
    }
    row(
        Enums,
        "tagged enum: a variant's record gains a field with `default`",
        (Holds::Rec(base()), Some(New::Rec(grown(&[])))),
        (New::Rec(grown(&["a"])), Some(Holds::Rec(base()))),
    )
}
