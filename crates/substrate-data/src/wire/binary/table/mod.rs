//! What a [`super::Writer`] (and a frame's packer) remembers while it
//! writes: the strings written so far, the map shapes, and the hash index
//! that finds both again.

mod hash_index;
pub(crate) mod interner;
pub(crate) mod shape_table;
