//! Reading a value against a schema: its problems, with their paths.

mod expected;
mod found;
mod keys;
mod numbers;
mod read;
#[expect(
    clippy::module_inception,
    reason = "the module of the reader and its parts"
)]
mod reader;
mod resolve;
mod suggest;

pub use expected::Expected;
pub use found::Found;
pub use keys::{COMMENT_KEY, META_PREFIX, PRIVATE_PREFIX, is_metadata_key};
pub use numbers::Numbers;
pub use read::Read;
pub use reader::Reader;
pub use resolve::{Resolve, Unresolved};
