//! The process's one table of names: installed by the runtime, else the crate's own.

use super::local;
use crate::sync::OnceLock;

/// The table's two functions: a name's number (made the first time), and
/// a number's name (`None`: no such atom).
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// A name's number, made the first time.
    pub intern: fn(&str) -> u32,
    /// A number's name (`None`: no such atom).
    pub name: fn(u32) -> Option<&'static str>,
}

static TABLE: OnceLock<Table> = OnceLock::new();

/// Points this copy of the crate at the process's table (the runtime's);
/// before any atom is made. False if it has one already.
pub fn install(table: Table) -> bool {
    TABLE.set(table).is_ok()
}

/// The process's table: the installed one, else this crate's own.
pub(super) fn table() -> &'static Table {
    TABLE.get_or_init(|| Table {
        intern: local::intern,
        name: local::name,
    })
}
