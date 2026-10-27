//! [`Doc`]: a parsed document and where each of its parts is.

use super::{LineCol, Spans};
use crate::model::value::{Path, Seg, Value};

/// A parsed document: the value and where each of its parts is.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    /// The value parsed.
    pub value: Value,
    /// Where each of its parts is.
    pub spans: Spans,
}

impl Doc {
    /// The spans of the value at `path` — the closest existing parent's if
    /// the path goes further than the document.
    pub fn spans_at(&self, path: &Path) -> &Spans {
        let mut v = &self.value;
        let mut s = &self.spans;
        for seg in path.segments() {
            let i = match (seg, v) {
                (Seg::Key(k), Value::Map(m)) => m.index_of(k),
                (Seg::Index(i), Value::List(l)) if *i < l.len() => Some(*i),
                _ => None,
            };
            let Some(i) = i else { break };
            let Some(child) = s.children.get(i) else {
                break;
            };
            v = match v {
                Value::Map(m) => m.value_at(i).unwrap_or(v),
                Value::List(l) => &l[i],
                _ => v,
            };
            s = child;
        }
        s
    }

    /// The line and column of the value at `path` in `text` (the source this was parsed from).
    pub fn locate(&self, text: &str, path: &Path) -> LineCol {
        LineCol::of(text, self.spans_at(path).span.start as usize)
    }
}
