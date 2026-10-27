//! The ordered map: order, replacement, the index of big maps, retain and
//! iteration.

use substrate_data::{Map, Value};

#[test]
fn a_map_keeps_order_and_replaces_in_place() {
    let mut m = Map::new();
    assert!(m.is_empty());
    assert_eq!(m.insert("a", Value::Int(1)), None);
    assert_eq!(m.insert("b", Value::Int(2)), None);
    assert_eq!(m.insert("a", Value::Int(3)), Some(Value::Int(1)));
    assert_eq!((m.len(), m.index_of("b")), (2, Some(1)));
    assert_eq!(m.value_at(0), Some(&Value::Int(3)));
    assert_eq!(m.value_at(99), None);
    assert_eq!(m.keys().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(m.values().count(), 2);
}

fn big_map() -> Map {
    (0..40).map(|i| (format!("k{i}"), Value::Int(i))).collect()
}

#[test]
fn a_big_map_finds_keys_through_its_index_and_keeps_it_current() {
    let mut m = big_map();
    assert_eq!(m.get("k39"), Some(&Value::Int(39)));
    assert_eq!(m.remove("k0"), Some(Value::Int(0)));
    assert_eq!(m.remove("k0"), None);
    assert_eq!(m.index_of("k1"), Some(0));
    m.insert("k40", Value::Int(40));
    assert_eq!(m.index_of("k40"), Some(39));
    *m.get_mut("k5").expect("a value") = Value::Null;
    assert_eq!(m.get("k5"), Some(&Value::Null));
}

#[test]
fn a_map_keeps_only_what_retain_says() {
    let mut m = big_map();
    assert_eq!(m.get("k7"), Some(&Value::Int(7)));
    m.retain(|k, _| k == "k1" || k == "k7");
    assert_eq!(m.keys().collect::<Vec<_>>(), ["k1", "k7"]);
    assert_eq!(m.get("k7"), Some(&Value::Int(7)));
    assert_eq!(m.get("k2"), None);
}

#[test]
fn a_map_iterates_by_reference_and_by_value() {
    let mut m = Map::new();
    m.insert("x", Value::Null);
    assert!(format!("{m:?}").starts_with("Map("));
    let borrowed: Vec<_> = (&m).into_iter().map(|(k, _)| k.to_owned()).collect();
    let owned: Vec<_> = m.clone().into_iter().map(|(k, _)| k).collect();
    assert_eq!(borrowed, owned);
    let collected: Map = [("x", Value::Null)].into_iter().collect();
    assert_eq!(collected, m);
}
