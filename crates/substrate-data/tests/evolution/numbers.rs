//! Numbers whose type changes.

use substrate_data::Schema;

use crate::verdict::Group::Numbers;
use crate::verdict::{Row, row};

/// Every row of this group.
pub(crate) const ROWS: &[fn() -> Row] = &[
    widen_unsigned,
    widen_over_old_range,
    widen_signed,
    unsigned_to_signed,
    signed_to_unsigned_wraps,
    int_to_float,
    f32_to_f64,
    f64_to_f32,
    list_of_ints_widened,
    list_of_floats_widened,
];

macro_rules! one {
    ($name:ident, $t:ty) => {
        #[derive(Schema, Debug, PartialEq)]
        struct $name {
            v: $t,
        }
    };
}

one!(U8, u8);
one!(U32, u32);
one!(I32, i32);
one!(I64, i64);
one!(U64, u64);
one!(F32, f32);
one!(F64, f64);

fn widen_unsigned() -> Row {
    row(
        Numbers,
        "`u8` to `u32`, values within `u8`",
        (U8 { v: 200 }, Some(U32 { v: 200 })),
        (U32 { v: 200 }, Some(U8 { v: 200 })),
    )
}

fn widen_over_old_range() -> Row {
    row(
        Numbers,
        "`u8` to `u32`, the new writer's value over 255",
        (U8 { v: 200 }, Some(U32 { v: 200 })),
        (U32 { v: 300 }, None),
    )
}

fn widen_signed() -> Row {
    row(
        Numbers,
        "`i32` to `i64`, a negative value",
        (I32 { v: -5 }, Some(I64 { v: -5 })),
        (I64 { v: -5 }, Some(I32 { v: -5 })),
    )
}

fn unsigned_to_signed() -> Row {
    row(
        Numbers,
        "`u32` to `i32`, values outside the other's range",
        (U32 { v: 4_000_000_000 }, None),
        (I32 { v: -5 }, None),
    )
}

fn signed_to_unsigned_wraps() -> Row {
    row(
        Numbers,
        "`i64` to `u64`, a negative value (a `u64` above `i64::MAX` is its bits as an `i64`)",
        (I64 { v: -5 }, None),
        (U64 { v: u64::MAX - 4 }, None),
    )
}

fn int_to_float() -> Row {
    row(
        Numbers,
        "`u32` to `f64`",
        (U32 { v: 7 }, Some(F64 { v: 7.0 })),
        (F64 { v: 7.0 }, Some(U32 { v: 7 })),
    )
}

fn f32_to_f64() -> Row {
    row(
        Numbers,
        "`f32` to `f64`",
        (F32 { v: 0.5 }, Some(F64 { v: 0.5 })),
        (F64 { v: 0.5 }, Some(F32 { v: 0.5 })),
    )
}

fn f64_to_f32() -> Row {
    row(
        Numbers,
        "`f64` to `f32`, a value that is not an `f32`",
        (F64 { v: 0.1 }, None),
        (F32 { v: 0.5 }, Some(F64 { v: 0.5 })),
    )
}

#[derive(Schema, Debug, PartialEq)]
struct Bytes {
    v: Vec<u8>,
    after: u8,
}

#[derive(Schema, Debug, PartialEq)]
struct Words {
    v: Vec<u32>,
    after: u8,
}

fn list_of_ints_widened() -> Row {
    row(
        Numbers,
        "`Vec<u8>` to `Vec<u32>`",
        (
            Bytes {
                v: vec![1, 255],
                after: 5,
            },
            Some(Words {
                v: vec![1, 255],
                after: 5,
            }),
        ),
        (
            Words {
                v: vec![1, 255],
                after: 5,
            },
            Some(Bytes {
                v: vec![1, 255],
                after: 5,
            }),
        ),
    )
}

#[derive(Schema, Debug, PartialEq)]
struct Singles {
    v: Vec<f32>,
    after: u8,
}

#[derive(Schema, Debug, PartialEq)]
struct Doubles {
    v: Vec<f64>,
    after: u8,
}

fn list_of_floats_widened() -> Row {
    row(
        Numbers,
        "`Vec<f32>` to `Vec<f64>`",
        (
            Singles {
                v: vec![0.5, 2.0],
                after: 5,
            },
            Some(Doubles {
                v: vec![0.5, 2.0],
                after: 5,
            }),
        ),
        (
            Doubles {
                v: vec![0.5, 2.0],
                after: 5,
            },
            Some(Singles {
                v: vec![0.5, 2.0],
                after: 5,
            }),
        ),
    )
}
