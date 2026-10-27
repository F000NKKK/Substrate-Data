//! Text out of a value, or out of data that isn't one: compact or indented,
//! typed values in their string forms.

mod float;
mod integer;
mod stream;
mod string;
mod to_string;

pub(crate) use float::float_string;
pub use stream::Writer;
pub use string::write_string;
pub use to_string::{to_string, to_string_pretty};
