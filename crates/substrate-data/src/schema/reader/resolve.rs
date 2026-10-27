//! [`Resolve`]: what a tool makes of the references it reads, and
//! [`Unresolved`], why one is wrong.

use std::fmt;

use crate::model::value::{Path, Ref, RefKey};

/// What a tool makes of the references it reads: a build checks each one
/// against what there is (and may rewrite its id), an editor shows them.
pub trait Resolve {
    /// A reference at `path`: the id to keep, or why it's wrong.
    ///
    /// # Errors
    ///
    /// [`Unresolved`] when the reference names nothing there is, or names
    /// it in a way the tool refuses.
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<RefKey, Unresolved>;
}

/// Why a reference doesn't resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// There is nothing it names.
    Missing(Ref),
    /// It names something, but not in a way this tool takes (a reason for a person).
    Refused {
        /// The reference.
        reference: Ref,
        /// Why, for a person.
        reason: String,
    },
}

impl fmt::Display for Unresolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unresolved::Missing(r) => write!(f, "`{r}`: there is no such {}", r.kind),
            Unresolved::Refused { reference, reason } => write!(f, "`{reference}`: {reason}"),
        }
    }
}

impl std::error::Error for Unresolved {}
