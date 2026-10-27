//! The ways in: a document with its spans, a bare value, a borrowed value.

use bumpalo::Bump;

use super::borrowed::Borrowed;
use super::build::{Build, Failure};
use super::owned::Owned;
use super::parser::Parser;
use crate::forms::text::{Doc, Spans, SyntaxError};
use crate::model::borrowed::ValueRef;
use crate::model::value::Value;

/// Deeper than this is an error, not a stack overflow.
pub const MAX_DEPTH: usize = 256;

/// Parses text.
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
pub fn parse(text: &str) -> Result<Doc, SyntaxError> {
    let (value, spans) = run::<Owned, true>(text, Owned::default())?;
    Ok(Doc { value, spans })
}

/// Parses text, without the positions (faster).
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
pub fn parse_value(text: &str) -> Result<Value, SyntaxError> {
    run::<Owned, false>(text, Owned::default()).map(|(v, _)| v)
}

/// Parses text straight to a borrowed value, the way a schema reads it: no
/// positions, and nothing is allocated on the heap per value — strings
/// without escapes point into `text`, lists and maps are in `arena`.
///
/// # Errors
///
/// [`SyntaxError`] when the text doesn't parse.
///
/// ```
/// let arena = substrate_data::Bump::new();
/// let v = substrate_data::text::parse_ref(r#"{ name: "docs", pages: [1, 2] }"#, &arena).unwrap();
/// assert_eq!(v.get("name").and_then(|n| n.as_str()), Some("docs"));
/// ```
pub fn parse_ref<'a>(text: &'a str, arena: &'a Bump) -> Result<ValueRef<'a>, SyntaxError> {
    run::<Borrowed<'a>, false>(text, Borrowed::new(arena)).map(|(v, _)| v)
}

fn run<'t, B: Build<'t>, const SPANS: bool>(
    text: &'t str,
    builder: B,
) -> Result<(B::Value, Spans), SyntaxError> {
    parse_all::<B, SPANS>(text, builder).map_err(Failure::into_inner)
}

fn parse_all<'t, B: Build<'t>, const SPANS: bool>(
    text: &'t str,
    builder: B,
) -> Result<(B::Value, Spans), Failure> {
    let mut p = Parser::<B, SPANS>::new(text, builder);
    if text.starts_with('\u{feff}') {
        p.skip(3);
    }
    p.ws()?;
    let mut root = Vec::new();
    let value = p.value(None, &mut root)?;
    p.ws()?;
    if !p.at_end() {
        return Err(p.error("unexpected text after the value"));
    }
    Ok((value, root.pop().unwrap_or_default()))
}
