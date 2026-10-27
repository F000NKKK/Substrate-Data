//! [`Dictionary`]: the values references expand to, kept per kind.

#[expect(
    clippy::module_inception,
    reason = "the module of the dictionary and its parts"
)]
mod dictionary;
mod entries;

pub use dictionary::Dictionary;
