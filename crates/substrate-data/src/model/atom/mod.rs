//! Atoms: names that repeat across messages (`core:open`, a live
//! record's state) kept once a process and sent in frames as their number.
//! An [`Atom`] is a `u32`: copied, compared and packed as one; its text is
//! the table's, alive as long as the process. A frame carries the number,
//! every other form (text, the tagged binary form, files) the text — the
//! numbers mean nothing outside the process.
//!
//! The table is the host's: a plugin's copy of this crate (another dynamic
//! library) is pointed at it ([`install`]) so every plugin and the host
//! agree on the numbers. Without one (tools, tests) the crate keeps its own.
//!
//! ```
//! use substrate_data::Atom;
//!
//! let open = Atom::new("core:open");
//! assert_eq!(open, Atom::new("core:open"));
//! assert_eq!(open.as_str(), "core:open");
//! ```

#[expect(clippy::module_inception, reason = "the atom module's one type")]
mod atom;
pub mod local;
mod table;

pub use atom::Atom;
pub use table::{Table, install};
