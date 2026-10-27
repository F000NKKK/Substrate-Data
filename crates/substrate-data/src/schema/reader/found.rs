//! [`Found`]: something found while reading, with where.

use crate::model::value::Path;

/// Where a reference was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found<T> {
    /// What was found.
    pub value: T,
    /// Where.
    pub path: Path,
}
