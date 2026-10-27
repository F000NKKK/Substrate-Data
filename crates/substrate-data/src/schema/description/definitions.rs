//! [`Definitions`]: the structs and enums a description has met, each
//! written once and referred to by name — a recursive type finds itself.

use super::DefinitionDoc;
use crate::schema::Ordered;

/// The definitions met so far, in the order met; one being written is `None`.
#[derive(Debug, Default)]
pub(super) struct Definitions(Vec<(String, Option<DefinitionDoc>)>);

impl Definitions {
    /// Whether `name` is defined or being defined.
    pub(super) fn has(&self, name: &str) -> bool {
        self.0.iter().any(|(n, _)| n == name)
    }

    /// `name` is being defined: a recursive use of it finds it here.
    pub(super) fn reserve(&mut self, name: &str) {
        self.0.push((name.to_owned(), None));
    }

    /// `name`'s definition, written.
    pub(super) fn set(&mut self, name: &str, definition: DefinitionDoc) {
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| n == name) {
            slot.1 = Some(definition);
        }
    }

    /// Every definition, in the order met.
    pub(super) fn finish(self) -> Ordered<DefinitionDoc> {
        Ordered(
            self.0
                .into_iter()
                .filter_map(|(name, d)| Some((name, d?)))
                .collect(),
        )
    }
}
