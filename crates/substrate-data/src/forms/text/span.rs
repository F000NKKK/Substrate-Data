//! [`Span`]: a byte range of the source.

/// A byte range of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// The first byte.
    pub start: u32,
    /// The byte after the last.
    pub end: u32,
}

impl Span {
    /// `start..end`; offsets past `u32` saturate.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Span {
        Span {
            start: u32::try_from(start).unwrap_or(u32::MAX),
            end: u32::try_from(end).unwrap_or(u32::MAX),
        }
    }

    /// From this one's start to `other`'s end.
    #[must_use]
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start,
            end: other.end.max(self.end),
        }
    }
}
