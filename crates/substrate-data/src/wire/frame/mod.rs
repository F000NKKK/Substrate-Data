//! Frames: data packed for messages — both sides know the schema, so
//! nothing says what a value is or what it's called. A record's fields go
//! in the order they're declared, after the record's length (so a reader
//! that knows fewer fields skips the rest; one that knows more takes their
//! defaults: fields are only ever appended). Strings go in a table at the
//! end, each once; the body refers to them by index. `docs/format.md`
//! ("Frames") has the layout of every value.
//!
//! Nothing is aligned, every value takes at least a byte (a count is
//! checked against the bytes left), and a malformed frame reads as `None`,
//! never a panic.
//!
//! ```
//! #[derive(substrate_data::Schema, Debug, PartialEq)]
//! struct Hit { id: String, count: u32, ratio: f32, next: Option<Box<Hit>> }
//!
//! let hit = Hit { id: "docs/intro".into(), count: 3, ratio: 0.5, next: None };
//! let frame = substrate_data::frame::to_frame(&hit);
//! assert_eq!(substrate_data::frame::read_frame::<Hit>(&frame), Some(hit));
//! ```

pub mod floats;
pub mod lanes;
mod packer;
mod shaped;
mod sizes;
mod strings;
mod unpacker;
mod view;

pub use packer::{Packer, to_frame};
pub use shaped::{read_shaped, unpack_shaped};
pub use unpacker::Unpacker;
#[expect(
    clippy::module_name_repetitions,
    reason = "`frame::to_frame` and `frame::read_frame` say what they make"
)]
pub use unpacker::read_frame;
pub use view::{Floats, FromF64, Ints, List, ListIter, View, list, read_view};
