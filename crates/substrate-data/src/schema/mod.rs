//! Schemas: a format is a Rust type implementing [`Schema`] (usually
//! `#[derive(Schema)]`). One schema reads and writes the format, checks it
//! with the path of every problem, collects its references for the build to
//! check, and describes itself ([`Shape`]) for editors (property grids, completion) and the
//! docs.

mod convert;
mod description;
mod fields;
mod format;
mod named_type;
mod plain;
mod reader;
mod refs;
mod shape;
mod tagged;
pub mod with;

#[cfg(feature = "toml")]
pub use convert::from_toml;
pub use convert::{
    field_names, from_doc, from_doc_with, from_text, from_value, from_value_with, placed,
    read_both, read_bytes, read_ref_value, read_value, to_bytes, to_bytes_into, to_text,
};
pub use description::{DefinitionDoc, FieldDoc, ShapeDoc, ShapeNode, VariantDoc};
pub use fields::{Fields, field_name};
pub use format::Schema;
pub use named_type::{NamedShape, NamedType, read_named};
pub use plain::{OneOrList, Ordered, RESULT_ERR, RESULT_OK};
pub use reader::{
    COMMENT_KEY, Expected, Found, META_PREFIX, Numbers, PRIVATE_PREFIX, Read, Reader, Resolve,
    Unresolved, is_metadata_key,
};
pub use refs::{Id, Kind, Kinds, RefId, Uid, read_id, read_ref, ref_shape};
pub use shape::{EnumShape, Field, Shape, ShapeFn, StructShape, Variant};
pub use tagged::{WRAPPED, flattens};
