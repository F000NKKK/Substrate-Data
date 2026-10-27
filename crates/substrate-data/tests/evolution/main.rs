//! What survives a change to a derived type, per form.
//!
//! Each row of the matrix in `docs/evolution.md` is a change to a type with
//! values written by one version and read by the other, in the frame form and
//! in the named forms (binary and text). The document's matrix is written from
//! these rows: `EVOLUTION_BLESS=1 cargo test --test evolution` rewrites it.

mod base;
mod document;
mod enums;
mod fields;
mod guarantees;
mod matrix;
mod nested;
mod numbers;
mod options;
mod verdict;
