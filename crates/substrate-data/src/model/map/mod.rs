//! A map: entries in file order, keys unique. A small map is
//! searched by a scan, the fastest there is for few keys. Past
//! [`INDEXED_FROM`] keys, a lookup builds an index of them once, and every
//! later change keeps it current. Decoding never builds one, so a document
//! read and never searched pays nothing for it.

#[expect(clippy::module_inception, reason = "the map module's one type")]
mod map;
#[cfg(test)]
mod tests;

#[cfg(test)]
use map::INDEXED_FROM;
pub use map::Map;
