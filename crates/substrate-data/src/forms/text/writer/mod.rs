//! Text out of a value, or out of data that isn't one: compact or indented,
//! typed values in their string forms.

mod float;
mod integer;
mod open;
mod sink;
mod string;
mod to_string;
#[expect(clippy::module_inception, reason = "the module's one type")]
mod writer;

pub(crate) use float::float_string;
pub use string::write_string;
pub use to_string::{to_string, to_string_pretty};
pub use writer::Writer;
