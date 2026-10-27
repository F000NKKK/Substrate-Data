//! `#[derive(Schema)]` — see `substrate_data::Schema`.
//!
//! - A struct with named fields is an object. Field attributes:
//!   `#[data(rename = "name")]`, `#[data(default)]` /
//!   `#[data(default = "path::to_fn")]`, `#[data(skip)]` (never read or
//!   written; `Default`). `Option` fields may be left out.
//! - A struct with one unnamed field is that field's format.
//! - An enum of unit variants is a string: the variant's name in
//!   `snake_case`, or `#[data(rename = "...")]`.
//! - `#[data(tag = "type")]` on an enum: an object whose `type` names the
//!   variant; a variant is a unit, has named fields, or wraps one value —
//!   a struct's fields sit next to the tag, anything else under `value`
//!   (`substrate_data::schema::WRAPPED`).
//!
//! - `#[data(view)]` on a struct: `NameView<'a>` too, the struct read out
//!   of a frame without copying (`substrate_data::frame::View`).
//!
//! Every type also packs itself into frames (`pack` / `unpack`: fields by
//! position, unit enums by index) and writes the binary form straight
//! (`encode` / `decode`).
//!
//! Doc comments become the shape's docs (an editor's hover text and the
//! docs' field tables).

mod attr_name;
mod attrs;
mod case;
mod codegen;
mod docs;
mod enums;
mod expand;
mod fields;
mod marker;
mod struct_schema;
#[cfg(test)]
mod tests;
mod view;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// `#[derive(Schema)]`: see the crate's docs.
#[proc_macro_derive(Schema, attributes(data))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct CrateReadmeDoctests;
