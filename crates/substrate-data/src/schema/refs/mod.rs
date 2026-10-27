//! Typed references: kinds, their id types, and the schemas that read
//! `@kind:id` as them.

mod id;
mod kind;
mod read_ref;
mod ref_id;
mod uid;
mod untyped;

pub use id::Id;
pub use kind::{Kind, Kinds};
pub use read_ref::{read_id, read_ref, ref_shape};
pub use ref_id::RefId;
pub use uid::Uid;
