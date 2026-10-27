//! [`Entries`]: one kind's values by id, in the order they were added.

use std::collections::HashMap;

use crate::model::value::{RefKey, Value};

/// One kind's values by id, in the order they were added.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Entries {
    order: Vec<RefKey>,
    values: HashMap<RefKey, Value>,
}

impl Entries {
    /// Sets `id`'s value, returning the one it replaced (it keeps its place).
    pub(super) fn insert(&mut self, id: RefKey, value: Value) -> Option<Value> {
        let replaced = self.values.insert(id.clone(), value);
        if replaced.is_none() {
            self.order.push(id);
        }
        replaced
    }

    pub(super) fn get(&self, id: &RefKey) -> Option<&Value> {
        self.values.get(id)
    }

    pub(super) fn len(&self) -> usize {
        self.order.len()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&RefKey, &Value)> {
        self.order
            .iter()
            .filter_map(|id| self.values.get(id).map(|v| (id, v)))
    }
}
