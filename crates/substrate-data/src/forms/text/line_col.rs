//! [`LineCol`]: a 1-based line and column.

use std::fmt;

/// A 1-based line and column (in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// The line, from 1.
    pub line: u32,
    /// The column in characters, from 1.
    pub col: u32,
}

impl LineCol {
    /// Where byte `offset` of `text` is.
    #[must_use]
    pub fn of(text: &str, offset: usize) -> LineCol {
        let offset = offset.min(text.len());
        let before = &text.as_bytes()[..offset];
        let lines = memchr::memchr_iter(b'\n', before).count() + 1;
        let line_start = memchr::memrchr(b'\n', before).map_or(0, |i| i + 1);
        let col = text
            .get(line_start..offset)
            .map_or(offset - line_start, |s| s.chars().count())
            + 1;
        LineCol {
            line: u32::try_from(lines).unwrap_or(u32::MAX),
            col: u32::try_from(col).unwrap_or(u32::MAX),
        }
    }
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[cfg(test)]
mod tests {
    use super::LineCol;

    /// The scalar computation `LineCol::of` used before `memchr`.
    fn scalar(text: &str, offset: usize) -> LineCol {
        let offset = offset.min(text.len());
        let before = &text.as_bytes()[..offset];
        let lines = before.split(|&b| b == b'\n').count();
        let line_start = before
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let col = text
            .get(line_start..offset)
            .map_or(offset - line_start, |s| s.chars().count())
            + 1;
        LineCol {
            line: u32::try_from(lines).unwrap_or(u32::MAX),
            col: u32::try_from(col).unwrap_or(u32::MAX),
        }
    }

    #[test]
    fn matches_the_scalar_computation_at_every_offset() {
        for text in [
            "",
            "one line",
            "a\nb\n\nnaïve\n日本語 😀\nend",
            "\n\n\n",
            "tail\n",
        ] {
            for offset in 0..=text.len() + 2 {
                assert_eq!(
                    LineCol::of(text, offset),
                    scalar(text, offset),
                    "{text:?} {offset}"
                );
            }
        }
    }
}
