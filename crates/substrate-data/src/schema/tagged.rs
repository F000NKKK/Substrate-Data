//! How a tagged enum's variant keeps what it wraps.

use super::{Schema, Shape};

/// Where a tagged enum's variant keeps what it wraps when that isn't a
/// struct (`Id(String)` is `{type: "id", value: "…"}`); a wrapped struct's
/// fields sit next to the tag instead.
pub const WRAPPED: &str = crate::names::Word::Value.as_str();

/// Whether a tagged enum's variant wrapping `T` lays `T`'s fields next to
/// the tag (a struct) or keeps `T` under [`WRAPPED`].
#[doc(hidden)]
#[must_use]
pub fn flattens<T: Schema>() -> bool {
    matches!(T::shape(), Shape::Struct(_))
}
