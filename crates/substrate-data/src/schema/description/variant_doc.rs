//! [`VariantDoc`]: an enum's variant as data.

use super::FieldDoc;
use super::definition_doc::non_empty;
use super::definitions::Definitions;
use crate::schema::Variant;

/// A variant: its name, doc, and (in a tagged enum) its fields.
#[derive(Debug, Clone, PartialEq, crate::Schema)]
pub struct VariantDoc {
    /// Its name.
    pub name: String,
    /// Its doc comment.
    pub doc: Option<String>,
    /// Its fields; none in an enum of names.
    pub fields: Option<Vec<FieldDoc>>,
}

impl VariantDoc {
    /// `v` as data; its fields only when the enum is `tagged`.
    pub(super) fn of(v: &Variant, tagged: bool, defs: &mut Definitions) -> VariantDoc {
        VariantDoc {
            name: v.name.to_owned(),
            doc: non_empty(v.doc),
            fields: tagged.then(|| FieldDoc::all(&v.fields, defs)),
        }
    }
}
