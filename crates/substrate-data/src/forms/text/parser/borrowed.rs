//! [`Borrowed`]: the parser's values as a [`ValueRef`] in an arena.

use std::borrow::Cow;

use bumpalo::Bump;

use super::build::Build;
use crate::model::borrowed::{MapRef, ValueRef};

/// Builds a [`ValueRef`]: strings without escapes point into the text, the
/// rest (lists, maps, unescaped strings) lives in the arena, so a parse
/// makes no allocation of its own per value.
#[derive(Debug)]
pub(super) struct Borrowed<'a> {
    arena: &'a Bump,
    items: Vec<ValueRef<'a>>,
    entries: Vec<(&'a str, ValueRef<'a>)>,
}

impl<'a> Borrowed<'a> {
    pub(super) fn new(arena: &'a Bump) -> Self {
        Self {
            arena,
            items: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn text(&self, s: Cow<'a, str>) -> &'a str {
        match s {
            Cow::Borrowed(b) => b,
            Cow::Owned(o) => self.arena.alloc_str(&o),
        }
    }
}

impl<'a> Build<'a> for Borrowed<'a> {
    type Value = ValueRef<'a>;

    fn null(&mut self) -> ValueRef<'a> {
        ValueRef::Null
    }

    fn boolean(&mut self, b: bool) -> ValueRef<'a> {
        ValueRef::Bool(b)
    }

    fn int(&mut self, n: i64) -> ValueRef<'a> {
        ValueRef::Int(n)
    }

    fn float(&mut self, f: f64) -> ValueRef<'a> {
        ValueRef::Float(f)
    }

    fn string(&mut self, s: Cow<'a, str>) -> ValueRef<'a> {
        ValueRef::Str(self.text(s))
    }

    fn list_mark(&self) -> usize {
        self.items.len()
    }

    fn push_item(&mut self, v: ValueRef<'a>) {
        self.items.push(v);
    }

    fn end_list(&mut self, mark: usize) -> ValueRef<'a> {
        let list = self.arena.alloc_slice_copy(&self.items[mark..]);
        self.items.truncate(mark);
        ValueRef::List(list)
    }

    fn map_mark(&self) -> usize {
        self.entries.len()
    }

    fn push_entry(&mut self, key: Cow<'a, str>, v: ValueRef<'a>) {
        let key = self.text(key);
        self.entries.push((key, v));
    }

    fn has_key(&self, mark: usize, key: &str) -> bool {
        self.entries[mark..].iter().any(|(k, _)| *k == key)
    }

    fn each_key(&self, mark: usize, f: &mut dyn FnMut(&str)) {
        self.entries[mark..].iter().for_each(|(k, _)| f(k));
    }

    fn end_map(&mut self, mark: usize) -> ValueRef<'a> {
        let entries = self.arena.alloc_slice_copy(&self.entries[mark..]);
        self.entries.truncate(mark);
        ValueRef::Map(MapRef(entries))
    }
}
