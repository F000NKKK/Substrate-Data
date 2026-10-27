//! Map shapes (version 2): the keys of a map, listed once and named by
//! number after that, so a list of records with the same keys writes them
//! once.

mod seen_keys;
mod shapes;
mod spec;
mod walk;

pub(super) use shapes::Shapes;
pub(super) use walk::Walk;
