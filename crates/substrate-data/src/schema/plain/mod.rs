//! The schemas of plain types: numbers, strings, lists, maps.

mod dynamic;
mod floats;
mod ints;
mod lists;
mod maps;
mod one_or_list;
mod ordered;
mod result;
pub use result::{RESULT_ERR, RESULT_OK};
mod text;
mod tuples;
mod wrappers;

pub use one_or_list::OneOrList;
pub use ordered::Ordered;
