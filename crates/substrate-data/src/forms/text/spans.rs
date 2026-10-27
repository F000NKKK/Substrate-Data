//! [`Spans`]: where each value of a parsed document is.

use super::Span;

/// Where each value of a parsed document is: the value's span, its key's
/// (map entries), and its children's, in the same order as the value's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spans {
    /// The value's span.
    pub span: Span,
    /// Its key's span, for a map entry.
    pub key: Option<Span>,
    /// Its children's spans, in the order of the value's.
    pub children: Vec<Spans>,
}
