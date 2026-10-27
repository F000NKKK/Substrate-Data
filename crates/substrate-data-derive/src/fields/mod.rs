//! Named fields: read from their attributes, and the code for each form.

mod binary;
mod frame;
mod named_field;
mod value;

pub(crate) use binary::{decode_fields, encode_fields, write_text_fields};
pub(crate) use frame::{pack_fields, unpack_fields};
pub(crate) use named_field::{NamedField, named_fields};
pub(crate) use value::{field_shapes, read_fields, write_fields};
