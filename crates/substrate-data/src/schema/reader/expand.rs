//! [`Expansion`]: what a reference read where a value may stand came to.

use super::Reader;
use crate::diagnostic::Problem;
use crate::model::value::{Ref, Value};

/// How many expanded values may be read inside one another: a value that
/// expands to itself (through references that expand to references) stops
/// here with a problem instead of never.
pub const MAX_EXPANSION_DEPTH: usize = 32;

/// What a reference read where a value may stand came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Expansion {
    /// The resolver keeps it a reference (or there is no resolver).
    Kept,
    /// The value in its place.
    Value(Value),
    /// The resolver refused it: the problem is reported.
    Failed,
}

impl Reader<'_> {
    /// `r`, at the current path, expanded by the resolver: what it came to
    /// (a problem if the resolver refuses it).
    pub fn expand_ref(&mut self, r: &Ref) -> Expansion {
        let path = self.path();
        let Some(resolver) = self.resolver.as_mut() else {
            return Expansion::Kept;
        };
        match resolver.expand(r, &path) {
            Ok(Some(value)) => Expansion::Value(value),
            Ok(None) => Expansion::Kept,
            Err(e) => {
                self.problem_of(Problem::Unresolved, e.to_string());
                Expansion::Failed
            }
        }
    }

    /// Runs `f`, which reads an expanded value, one expansion deeper; past
    /// [`MAX_EXPANSION_DEPTH`] it is a problem and `f` isn't run.
    pub fn expanded<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        if self.expanding >= MAX_EXPANSION_DEPTH {
            self.problem(format!(
                "references expand into one another more than {MAX_EXPANSION_DEPTH} deep"
            ));
            return None;
        }
        self.expanding += 1;
        let read = f(self);
        self.expanding -= 1;
        read
    }
}
