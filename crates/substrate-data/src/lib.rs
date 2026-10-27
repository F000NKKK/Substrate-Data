//! Substrate Data: one data model, text syntax, binary form, packed frames
//! and schema system — for a program's files, saves, caches and wire
//! messages alike. See `docs/format.md`.
//!
//! - [`Value`]: the model — JSON's values plus typed ones (references
//!   `@kind:id`, bytes);
//! - [`text`]: JSON with comments, trailing commas and unquoted keys,
//!   parsed with the position of every value; [`patch`] edits a document
//!   in place, keeping its comments and layout;
//! - [`typed`]: the one grammar of the typed values' string forms
//!   (`@file:images/logo.png`, `@bytes:…`);
//! - [`binary`]: the compact, self-describing tagged form;
//! - [`frame`]: the packed form for messages whose schema both sides know;
//! - [`Schema`] (`#[derive(Schema)]`): a format written once — read with
//!   the path of every problem, written, encoded, packed, and described
//!   ([`Shape`]) for editors and docs.

// `::substrate_data::…` paths of `#[derive(Schema)]` work inside this crate too.
extern crate self as substrate_data;

pub mod diagnostic;
mod forms;
mod model;
pub mod names;
mod sync;
mod wire;

pub use atom::Atom;
pub use bumpalo::Bump;
pub use diagnostic::{Code, Diagnostic, Errors, Problem, Severity};
#[cfg(feature = "json")]
pub use forms::json;
#[cfg(feature = "toml")]
pub use forms::toml;
pub use forms::{patch, source, text};
pub use model::borrowed::{MapRef, ValueRef};
pub use model::value::{KindName, Map, Path, Ref, RefKey, Seg, Value, ValueKind};
pub use model::{atom, typed};
pub use schema::with;
pub use text::{Doc, LineCol, Span, Spans, SyntaxError};
pub use wire::stream::{Stream, StreamWriter};
pub use wire::{binary, frame, io};
pub mod schema;

pub use schema::{
    Expected, Id, Kind, Kinds, NamedType, OneOrList, Ordered, Read, Reader, RefId, Resolve, Schema,
    Shape, ShapeDoc, Uid, Unresolved, from_doc_with, from_text, from_value, from_value_with,
    placed, read_bytes, read_ref_value, read_value, to_bytes, to_bytes_into, to_text,
};
pub use substrate_data_derive::Schema;

#[cfg(doctest)]
mod readme;
