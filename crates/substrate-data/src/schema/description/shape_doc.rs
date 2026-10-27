//! [`ShapeDoc`]: a whole shape as data — its root and the definitions of
//! the structs and enums in it.

use super::definitions::Definitions;
use super::{DefinitionDoc, ShapeNode};
use crate::schema::{Ordered, Shape};

/// A shape as data: written `{ root, defs }`, structs and enums once in
/// `defs` and referred to by name.
#[derive(Debug, Clone, PartialEq, crate::Schema)]
pub struct ShapeDoc {
    /// The shape described.
    pub root: ShapeNode,
    /// Every struct and enum in it, by name, in the order met.
    pub defs: Ordered<DefinitionDoc>,
}

impl ShapeDoc {
    /// `shape` described.
    #[must_use]
    pub fn of(shape: &Shape) -> ShapeDoc {
        let mut defs = Definitions::default();
        let root = ShapeNode::of(shape, &mut defs);
        ShapeDoc {
            root,
            defs: defs.finish(),
        }
    }

    /// The definition named `name`.
    #[must_use]
    pub fn definition(&self, name: &str) -> Option<&DefinitionDoc> {
        self.defs.0.iter().find(|(n, _)| n == name).map(|(_, d)| d)
    }
}
