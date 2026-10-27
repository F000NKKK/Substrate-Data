//! A frame's lists of floats, four at a time: whether a list of `f64` is
//! `f32` exactly (then it's kept as `f32`s, half the bytes), the narrowing
//! and the widening back — AVX2 where the processor has it, chosen once;
//! the same bytes either way.

#[cfg(target_arch = "x86_64")]
mod avx2;
mod narrowing;
mod portable;

pub use narrowing::{fits_f32, put_f32s, put_narrow, put_wide, widen};

use crate::wire::frame::sizes::F32_BYTES;
