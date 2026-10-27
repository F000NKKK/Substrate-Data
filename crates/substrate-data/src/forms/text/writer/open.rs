//! [`Open`]: a list or map being written.

/// A list or map being written: how many of its entries are still to come,
/// and how many have been.
#[derive(Debug)]
pub(super) struct Open {
    pub(super) map: bool,
    pub(super) remaining: usize,
    pub(super) done: usize,
}
