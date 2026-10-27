//! A shape as data ([`ShapeDoc`]), for tools in other languages: an
//! editor builds property grids, pickers and hover docs from it, docs
//! generate their field tables from it. Its written form is the published
//! language of `docs/format.md` (§ Schemas, "describe").

mod definition_doc;
mod definitions;
mod field_doc;
mod shape_doc;
mod shape_node;
mod variant_doc;

pub use definition_doc::DefinitionDoc;
pub use field_doc::FieldDoc;
pub use shape_doc::ShapeDoc;
pub use shape_node::ShapeNode;
pub use variant_doc::VariantDoc;
