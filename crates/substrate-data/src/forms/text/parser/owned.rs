//! [`Owned`]: the parser's values as an owned [`Value`] tree.

use std::borrow::Cow;
use std::collections::HashSet;

use super::build::Build;
use crate::model::quick_hash::QuickBuild;
use crate::model::value::{Map, Value};
use crate::sync::Arc;

/// Keys longer than this are not shared: a long key rarely repeats.
const SHARED_KEY_MAX: usize = 32;

/// How many distinct keys are shared at most, so a map of a million
/// different keys doesn't also fill a table of them.
const SHARED_KEYS_MAX: usize = 4096;

/// Builds a [`Value`]. A key that was seen before is the same `Arc<str>` as
/// the first one, as in a decoded document.
#[derive(Debug, Default)]
pub(super) struct Owned {
    items: Vec<Value>,
    entries: Vec<(Arc<str>, Value)>,
    keys: HashSet<Arc<str>, QuickBuild>,
}

impl Owned {
    fn key(&mut self, key: &str) -> Arc<str> {
        if let Some(known) = self.keys.get(key) {
            return Arc::clone(known);
        }
        let made: Arc<str> = Arc::from(key);
        if key.len() <= SHARED_KEY_MAX && self.keys.len() < SHARED_KEYS_MAX {
            self.keys.insert(Arc::clone(&made));
        }
        made
    }
}

impl<'t> Build<'t> for Owned {
    type Value = Value;

    fn null(&mut self) -> Value {
        Value::Null
    }

    fn boolean(&mut self, b: bool) -> Value {
        Value::Bool(b)
    }

    fn int(&mut self, n: i64) -> Value {
        Value::Int(n)
    }

    fn float(&mut self, f: f64) -> Value {
        Value::Float(f)
    }

    fn string(&mut self, s: Cow<'t, str>) -> Value {
        Value::Str(s.into_owned())
    }

    fn list_mark(&self) -> usize {
        self.items.len()
    }

    fn push_item(&mut self, v: Value) {
        self.items.push(v);
    }

    fn end_list(&mut self, mark: usize) -> Value {
        Value::List(split_off(&mut self.items, mark))
    }

    fn map_mark(&self) -> usize {
        self.entries.len()
    }

    fn push_entry(&mut self, key: Cow<'t, str>, v: Value) {
        let key = self.key(&key);
        self.entries.push((key, v));
    }

    fn has_key(&self, mark: usize, key: &str) -> bool {
        self.entries[mark..].iter().any(|(k, _)| &**k == key)
    }

    fn each_key(&self, mark: usize, f: &mut dyn FnMut(&str)) {
        self.entries[mark..].iter().for_each(|(k, _)| f(k));
    }

    fn end_map(&mut self, mark: usize) -> Value {
        Value::Map(Map::from_entries(split_off(&mut self.entries, mark)))
    }
}

/// What is on `stack` from `mark` on, in a vector of exactly its size.
fn split_off<T>(stack: &mut Vec<T>, mark: usize) -> Vec<T> {
    let mut taken = stack.split_off(mark);
    if mark == 0 {
        // The whole stack, with the room it grew to; the result is kept.
        taken.shrink_to_fit();
    }
    taken
}
