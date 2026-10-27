//! [`StaticShape`]: a shape found for a slice of static keys.

/// Slots of the cache of shapes of static keys (a power of two).
pub(in crate::wire::binary) const STATIC_SHAPES: usize = 16;

/// A shape found for a static slice of keys and a choice of them.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::wire::binary) struct StaticShape {
    /// Where the slice is.
    pub(in crate::wire::binary) keys: usize,
    /// Which of the slice's keys the shape has.
    pub(in crate::wire::binary) present: u64,
    /// The shape's number + 1; 0 for a free slot.
    pub(in crate::wire::binary) shape: u32,
}
