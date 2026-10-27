//! A fast hasher for the crate's own short-key tables (a map's index, the
//! keys a parse has seen), where `SipHash`'s cost per key is most of a lookup.

mod quick_build;
mod quick_hasher;

#[cfg(test)]
mod tests;

pub(crate) use quick_build::QuickBuild;
