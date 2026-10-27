//! [`SeenKeys`]: the keys of the map being read, so a repeat is refused.

use std::collections::HashSet;

use crate::model::quick_hash::QuickBuild;

/// Maps up to this many keys are checked by scanning.
const SCAN_MAX: usize = 16;

/// The keys read so far of one map: a scan while it is small, a set once it
/// grows.
#[derive(Debug, Default)]
pub(super) struct SeenKeys<'a> {
    list: Vec<&'a str>,
    set: Option<HashSet<&'a str, QuickBuild>>,
}

impl<'a> SeenKeys<'a> {
    /// Notes `key`; `false` when the map already had it.
    pub(super) fn insert(&mut self, key: &'a str) -> bool {
        if let Some(set) = &mut self.set {
            return set.insert(key);
        }
        if self.list.contains(&key) {
            return false;
        }
        self.list.push(key);
        if self.list.len() > SCAN_MAX {
            self.set = Some(self.list.iter().copied().collect());
        }
        true
    }
}
