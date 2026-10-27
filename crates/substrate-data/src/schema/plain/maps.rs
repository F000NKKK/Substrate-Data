//! The schemas of maps by string keys.

use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use crate::model::borrowed::ValueRef;
use crate::model::value::{Seg, Value};
use crate::schema::{Reader, Schema, Shape, is_metadata_key};

/// `v`'s entries read as `T`s into `put`, source-only keys skipped.
pub(super) fn read_map<T: Schema>(
    v: &ValueRef<'_>,
    cx: &mut Reader<'_>,
    mut put: impl FnMut(String, T),
) -> Option<()> {
    let map = cx.map(v)?;
    let mut ok = true;
    for (k, item) in map.iter() {
        if is_metadata_key(k) {
            continue;
        }
        match cx.at(Seg::Key(k.to_string()), |cx| T::read(item, cx)) {
            Some(x) => put(k.to_string(), x),
            None => ok = false,
        }
    }
    ok.then_some(())
}

impl<T: Schema> Schema for BTreeMap<String, T> {
    fn shape() -> Shape {
        Shape::Map(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let mut out = BTreeMap::new();
        read_map(v, cx, |k, x| {
            out.insert(k, x);
        })?;
        Some(out)
    }
    fn write(&self) -> Value {
        Value::Map(self.iter().map(|(k, v)| (k.clone(), v.write())).collect())
    }
}

impl<T: Schema, S: BuildHasher + Default> Schema for HashMap<String, T, S> {
    fn shape() -> Shape {
        Shape::Map(T::shape)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let mut out = HashMap::default();
        read_map(v, cx, |k, x| {
            out.insert(k, x);
        })?;
        Some(out)
    }
    fn write(&self) -> Value {
        let mut keys: Vec<&String> = self.keys().collect();
        keys.sort();
        Value::Map(
            keys.into_iter()
                .map(|k| (k.clone(), self[k].write()))
                .collect(),
        )
    }
}
