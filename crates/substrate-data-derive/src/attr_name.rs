//! The words of `#[data(...)]`: the attribute itself and each of its options.

pub(crate) const CRATE: &str = "crate";
pub(crate) const DATA: &str = "data";
pub(crate) const DEFAULT: &str = "default";
pub(crate) const DOC: &str = "doc";
pub(crate) const OTHER: &str = "other";
pub(crate) const RANGE: &str = "range";
pub(crate) const RENAME: &str = "rename";
pub(crate) const RENAME_ALL: &str = "rename_all";
/// `rename_all`'s one case: fields (a struct's) or variants (an enum's) in lower camel case.
pub(crate) const CAMEL_CASE: &str = "camelCase";
pub(crate) const SKIP: &str = "skip";
pub(crate) const TAG: &str = "tag";
pub(crate) const VIEW: &str = "view";
pub(crate) const WITH: &str = "with";
