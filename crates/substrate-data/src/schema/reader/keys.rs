//! The source-only keys every object may have.

/// A note for whoever edits the file — every object may have one.
pub const COMMENT_KEY: &str = "comment";
/// What a source-only key starts with: `$…` (a note on an entry: `$<key>`), `_…`.
pub const META_PREFIX: char = '$';
/// What a private source-only key starts with: `_…`.
pub const PRIVATE_PREFIX: char = '_';

/// Source-only keys every object may have: `$…`, `_…`, `comment`.
#[must_use]
pub fn is_metadata_key(key: &str) -> bool {
    key.starts_with(META_PREFIX) || key.starts_with(PRIVATE_PREFIX) || key == COMMENT_KEY
}
