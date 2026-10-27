//! [`StaticShape`]: a shape found for a slice of static keys.

/// Slots of the cache of shapes of static keys (a power of two).
pub(super) const STATIC_SHAPES: usize = 16;

/// A shape found for a static slice of keys and a choice of them.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct StaticShape {
    /// Where the slice is.
    pub(super) keys: usize,
    /// Which of the slice's keys the shape has.
    pub(super) present: u64,
    /// The shape's number + 1; 0 for a free slot.
    pub(super) shape: u32,
}
