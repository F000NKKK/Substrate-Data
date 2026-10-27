//! [`Build`]: what the parser makes values with, so one parser fills an
//! owned [`crate::Value`] and a borrowed [`crate::ValueRef`] alike.

use std::borrow::Cow;

use crate::forms::text::SyntaxError;

/// The parser's error, boxed so a result stays small: errors are rare.
#[derive(Debug)]
pub(super) struct Failure(Box<SyntaxError>);

impl Failure {
    pub(super) fn new(error: SyntaxError) -> Self {
        Self(Box::new(error))
    }

    pub(super) fn into_inner(self) -> SyntaxError {
        *self.0
    }
}

/// Makes the parser's values. Lists and maps are filled on a stack the
/// builder keeps: [`Build::mark`] says where a container starts, its items
/// are pushed, and the closing call turns everything since the mark into one
/// value of exactly the size — no list grows by doubling.
pub(super) trait Build<'t> {
    /// A value of the model.
    type Value;

    fn null(&mut self) -> Self::Value;
    fn boolean(&mut self, b: bool) -> Self::Value;
    fn int(&mut self, n: i64) -> Self::Value;
    fn float(&mut self, f: f64) -> Self::Value;
    /// A string value: borrowed from the text unless it had escapes.
    fn string(&mut self, s: Cow<'t, str>) -> Self::Value;

    /// Where a list starts on the item stack.
    fn list_mark(&self) -> usize;
    fn push_item(&mut self, v: Self::Value);
    /// The items since `mark`, as a list.
    fn end_list(&mut self, mark: usize) -> Self::Value;

    /// Where a map starts on the entry stack.
    fn map_mark(&self) -> usize;
    fn push_entry(&mut self, key: Cow<'t, str>, v: Self::Value);
    /// Whether the map begun at `mark` has `key`, by a scan.
    fn has_key(&self, mark: usize, key: &str) -> bool;
    /// Calls `f` with each key of the map begun at `mark`, in order.
    fn each_key(&self, mark: usize, f: &mut dyn FnMut(&str));
    /// The entries since `mark`, as a map.
    fn end_map(&mut self, mark: usize) -> Self::Value;
}
