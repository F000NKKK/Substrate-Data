//! [`Keys`]: the keys of the map being read, so a repeat is refused.

use std::borrow::Cow;
use std::collections::HashSet;

use super::build::Build;
use crate::model::quick_hash::QuickBuild;

/// Maps up to this many keys are checked for a repeat by a scan; past it, a set.
const SCAN_MAX: usize = 16;

/// The keys of the map being read, for refusing a repeat: the builder's
/// scan while it's small, a set once it grows.
#[derive(Default)]
pub(super) struct Keys<'t> {
    count: usize,
    set: Option<HashSet<Cow<'t, str>, QuickBuild>>,
}

impl<'t> Keys<'t> {
    /// Whether the map begun at `mark` has `key` already; if not, notes it.
    pub(super) fn repeats<B: Build<'t>>(
        &mut self,
        build: &B,
        mark: usize,
        key: Cow<'t, str>,
    ) -> bool {
        self.count += 1;
        if let Some(set) = &mut self.set {
            return !set.insert(key);
        }
        if build.has_key(mark, &key) {
            return true;
        }
        if self.count > SCAN_MAX {
            let mut set = HashSet::with_hasher(QuickBuild::default());
            build.each_key(mark, &mut |k| {
                set.insert(Cow::Owned(k.to_owned()));
            });
            set.insert(key);
            self.set = Some(set);
        }
        false
    }
}
