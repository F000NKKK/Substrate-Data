//! A shaped map read step by step.

/// A shaped map being read step by step: which shape, and how far.
#[derive(Debug, Clone, Copy)]
pub(in crate::wire::binary) struct Walk {
    /// The nesting depth of the map.
    pub(in crate::wire::binary) depth: usize,
    pub(in crate::wire::binary) shape: usize,
    /// The next key to hand out.
    pub(in crate::wire::binary) next: usize,
}
