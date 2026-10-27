//! The benchmarks' shared, deterministic corpus: neutral documents (users,
//! projects, docs, files) in three sizes, as value trees and as typed
//! structs. Included by each bench with `mod corpus;` (and by
//! `examples/sizes.rs` through `#[path]`); every file of this directory
//! belongs to it, so a bench uses only part of it.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    reason = "generated numbers kept in range by construction"
)]

pub(crate) mod each;
pub(crate) mod formats;
pub(crate) mod lcg;
pub(crate) mod sample;
pub(crate) mod shapes;
pub(crate) mod typed;
pub(crate) mod values;
