//! A map's lookups, with and without its index.

use super::*;
use crate::model::value::Value;

/// A big map is searched by its index, which every change keeps current:
/// an insert, a removal (the later keys move up), a rewrite.
#[test]
fn a_big_maps_index_follows_its_changes() {
    let n = INDEXED_FROM * 4;
    let mut map: Map = (0..n)
        .map(|i| (format!("k{i}"), Value::Int(i64::try_from(i).unwrap())))
        .collect();
    assert_eq!(map.get("k3"), Some(&Value::Int(3)));
    assert_eq!(map.remove("k1"), Some(Value::Int(1)));
    assert_eq!(map.index_of("k2"), Some(1));
    assert_eq!(map.get("k1"), None);
    map.insert("new", Value::Bool(true));
    assert_eq!(map.index_of("new"), Some(n - 1));
    map.insert("k5", Value::Null);
    assert_eq!(map.get("k5"), Some(&Value::Null));
    map.retain(|k, _| k != "k7");
    assert_eq!(map.get("k7"), None);
    assert_eq!(map.get("k8"), Some(&Value::Int(8)));
    // One removed, one added, one not kept.
    assert_eq!(map.keys().count(), n - 1);
}

/// Equal maps are equal whether one's indexed or not.
#[test]
fn equality_is_the_entries() {
    let make = || -> Map {
        (0..INDEXED_FROM * 2)
            .map(|i| (format!("k{i}"), Value::Int(i64::try_from(i).unwrap())))
            .collect()
    };
    let (a, b) = (make(), make());
    assert!(a.get("k0").is_some());
    assert_eq!(a, b);
}

/// The index costs a value nothing: a value, whose biggest kind is a
/// map, is no bigger than the map itself.
#[test]
fn a_value_is_no_bigger_than_a_map() {
    assert!(size_of::<Value>() <= size_of::<Map>());
}
