//! [`DefinitionDoc`]: a struct's or an enum's definition as data.

use super::definitions::Definitions;
use super::{FieldDoc, ShapeNode, VariantDoc};
use crate::schema::{EnumShape, StructShape};

/// A struct or an enum, as written once in a description's `defs`.
#[derive(Debug, Clone, PartialEq, crate::Schema)]
#[data(tag = "kind")]
pub enum DefinitionDoc {
    /// A struct: an object of these fields.
    Struct {
        /// Its doc comment.
        doc: Option<String>,
        /// Its fields, in order.
        fields: Vec<FieldDoc>,
    },
    /// An enum: one of these variants.
    Enum {
        /// Its doc comment.
        doc: Option<String>,
        /// The key naming the variant; none for an enum of names.
        tag: Option<String>,
        /// Its variants, in order.
        variants: Vec<VariantDoc>,
    },
}

impl DefinitionDoc {
    /// `s` defined in `defs` (once), and a reference to it.
    pub(super) fn define_struct(s: &StructShape, defs: &mut Definitions) -> ShapeNode {
        if !defs.has(s.name) {
            defs.reserve(s.name);
            let fields = FieldDoc::all(&s.fields, defs);
            let doc = non_empty(s.doc);
            defs.set(s.name, DefinitionDoc::Struct { doc, fields });
        }
        ShapeNode::Def {
            name: s.name.to_owned(),
        }
    }

    /// `e` defined in `defs` (once), and a reference to it.
    pub(super) fn define_enum(e: &EnumShape, defs: &mut Definitions) -> ShapeNode {
        if !defs.has(e.name) {
            defs.reserve(e.name);
            let variants = e
                .variants
                .iter()
                .map(|v| VariantDoc::of(v, e.tag.is_some(), defs))
                .collect();
            let definition = DefinitionDoc::Enum {
                doc: non_empty(e.doc),
                tag: e.tag.map(str::to_owned),
                variants,
            };
            defs.set(e.name, definition);
        }
        ShapeNode::Def {
            name: e.name.to_owned(),
        }
    }
}

/// A doc comment, none when it's empty.
pub(super) fn non_empty(doc: &str) -> Option<String> {
    (!doc.is_empty()).then(|| doc.to_owned())
}
