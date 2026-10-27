//! [`FieldDoc`]: a struct's or a variant's field as data.

use super::ShapeNode;
use super::definition_doc::non_empty;
use super::definitions::Definitions;
use crate::schema::Field;

/// A field: its name, doc, shape, whether it must be written, its range.
#[derive(Debug, Clone, PartialEq, crate::Schema)]
pub struct FieldDoc {
    /// Its key.
    pub name: String,
    /// Its doc comment.
    pub doc: Option<String>,
    /// What it holds.
    pub shape: ShapeNode,
    /// Must be written (no default).
    pub required: bool,
    /// The numbers it holds must be in `[min, max]`.
    pub range: Option<[f64; 2]>,
}

impl FieldDoc {
    /// `fields` as data, their structs and enums defined in `defs`.
    pub(super) fn all(fields: &[Field], defs: &mut Definitions) -> Vec<FieldDoc> {
        fields
            .iter()
            .map(|f| FieldDoc {
                name: f.name.to_owned(),
                doc: non_empty(f.doc),
                shape: ShapeNode::of(&(f.shape)(), defs),
                required: f.required,
                range: f.range.map(|(lo, hi)| [lo, hi]),
            })
            .collect()
    }
}
