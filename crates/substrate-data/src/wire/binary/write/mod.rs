//! Writing the binary form: the writer, and what it keeps to find shapes again.

mod shape_cache;
mod static_shape;
pub(crate) mod writer;
mod writer_packed;
mod writer_record;
mod writer_value;

pub use writer::Writer;
