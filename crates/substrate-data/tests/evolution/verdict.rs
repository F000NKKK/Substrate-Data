//! What a reader makes of another version's data, in the frame form and in
//! the named forms (binary and text).

use std::fmt::Debug;

use substrate_data::frame::{self, to_frame};
use substrate_data::{Schema, from_text, read_bytes, to_bytes, to_text};

/// What reading the other version's bytes came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The reader got the value the writer meant.
    Reads,
    /// The reader refused: an error, not a value.
    Rejects,
    /// The reader got a value, and not the one the writer meant.
    Garbage,
}

impl Verdict {
    /// How the matrix spells it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Verdict::Reads => "reads",
            Verdict::Rejects => "rejects",
            Verdict::Garbage => "**garbage**",
        }
    }
}

/// The verdicts of one direction: the frame form, the named binary form and
/// the text form (the last two nearly always agree).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cells {
    pub(crate) frame: Verdict,
    pub(crate) binary: Verdict,
    pub(crate) text: Verdict,
}

/// A change to a type, in both directions.
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) group: Group,
    pub(crate) change: &'static str,
    /// The newer type reading what the older one wrote.
    pub(crate) new_reads_old: Cells,
    /// The older type reading what the newer one wrote.
    pub(crate) old_reads_new: Cells,
}

/// The matrix's sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Fields,
    Numbers,
    Options,
    Enums,
    Nested,
}

/// `got` against `want`: `None` for `want` means no value is right.
fn judge<R: PartialEq>(got: Option<R>, want: Option<&R>) -> Verdict {
    match (got, want) {
        (None, _) => Verdict::Rejects,
        (Some(got), Some(want)) if got == *want => Verdict::Reads,
        _ => Verdict::Garbage,
    }
}

/// `written` read as an `R` in each form; `want` is what it should come to.
pub(crate) fn cells<W: Schema, R: Schema + PartialEq + Debug>(
    written: &W,
    want: Option<&R>,
) -> Cells {
    let frame = judge(frame::read_frame::<R>(&to_frame(written)), want);
    let binary = judge(read_bytes::<R>(&to_bytes(written)).ok(), want);
    let text = judge(
        from_text::<R>(&to_text(written))
            .ok()
            .map(|read| read.value),
        want,
    );
    Cells {
        frame,
        binary,
        text,
    }
}

/// A row from an older value and what a newer reader should make of it, and
/// a newer value and what an older reader should make of it.
pub(crate) fn row<O, N>(
    group: Group,
    change: &'static str,
    old: (O, Option<N>),
    new: (N, Option<O>),
) -> Row
where
    O: Schema + PartialEq + Debug,
    N: Schema + PartialEq + Debug,
{
    let ((old, old_want), (new, new_want)) = (old, new);
    Row {
        group,
        change,
        new_reads_old: cells::<O, N>(&old, old_want.as_ref()),
        old_reads_new: cells::<N, O>(&new, new_want.as_ref()),
    }
}
