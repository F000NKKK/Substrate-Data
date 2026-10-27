//! Version 1 documents read, and upgrade to version 2 without losing
//! anything: the property that makes "read v1, write v2" safe. Documents
//! are generated deterministically, covering every tag version 1 has (the
//! packed lists, the single `f32`, streams with a table of their own),
//! and every value version 1 can hold: an int at the edges, a float of any
//! bits, strings that look like references, keys in every order.

#[path = "support/generate.rs"]
mod generate;
#[path = "support/v1.rs"]
mod v1;

use generate::{Rng, document};
use substrate_data::binary::{self, LEGACY_VERSION, VERSION};
use substrate_data::{Bump, Value};

/// Whether `a` and `b` are the same value, floats bit for bit.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|((kx, vx), (ky, vy))| kx == ky && same(vx, vy))
        }
        _ => a == b,
    }
}

const DOCUMENTS: usize = 4000;

#[test]
fn every_version_1_document_reads_and_upgrades_to_the_same_value() {
    let mut rng = Rng::new(7);
    for n in 0..DOCUMENTS {
        let (v1, value) = document(&mut rng);
        assert_eq!(v1[4], LEGACY_VERSION);
        let read = binary::decode(&v1).unwrap_or_else(|e| panic!("#{n}: {e}"));
        assert!(
            same(&read, &value),
            "#{n}: version 1 reads {read:?}, not {value:?}"
        );
        let arena = Bump::new();
        let in_arena = binary::decode_ref(&v1, &arena).unwrap_or_else(|e| panic!("#{n}: {e}"));
        assert!(same(&in_arena.to_value(), &value), "#{n}: decode_ref");

        let v2 = binary::encode(&read);
        assert_eq!(v2[4], VERSION, "#{n}: written as version 2");
        assert_eq!(binary::upgrade(&v1).expect("upgrades"), v2, "#{n}: upgrade");
        let back = binary::decode(&v2).unwrap_or_else(|e| panic!("#{n}: {e}"));
        assert!(
            same(&back, &value),
            "#{n}: version 2 reads {back:?}, not {value:?}"
        );
        let upgraded = binary::decode_ref(&v2, &arena).unwrap_or_else(|e| panic!("#{n}: {e}"));
        assert!(same(&upgraded.to_value(), &value), "#{n}: decode_ref of v2");
        assert_eq!(binary::encode(&back), v2, "#{n}: writing is a fixed point");
    }
    for kind in [
        "null", "float32", "bytes", "ref", "ints", "floats", "floats32", "stream", "map", "records",
    ] {
        assert!(
            rng.1.get(kind).copied().unwrap_or(0) > 100,
            "{kind} was rarely generated"
        );
    }
}

#[test]
fn every_model_value_survives_version_1_then_version_2() {
    let mut rng = Rng::new(11);
    for n in 0..DOCUMENTS {
        let (_, value) = document(&mut rng);
        let v1 = v1::encode(&value);
        let read = binary::decode(&v1).unwrap_or_else(|e| panic!("#{n}: {e}"));
        assert!(same(&read, &value), "#{n}: version 1");
        let v2 = binary::upgrade(&v1).expect("upgrades");
        assert!(
            same(&binary::decode(&v2).expect("reads"), &value),
            "#{n}: version 2"
        );
        assert_eq!(
            v2,
            binary::encode(&value),
            "#{n}: the same bytes from the value"
        );
    }
}

#[test]
fn every_cut_of_either_version_fails_loudly() {
    let mut rng = Rng::new(3);
    for _ in 0..300 {
        let (v1, value) = document(&mut rng);
        let v2 = binary::encode(&value);
        for bytes in [&v1, &v2] {
            for cut in 0..bytes.len() {
                assert!(
                    binary::decode(&bytes[..cut]).is_err(),
                    "a cut at {cut} of {} read",
                    bytes.len()
                );
            }
        }
    }
}

#[test]
fn another_version_byte_is_an_error() {
    let (v1, _) = document(&mut Rng::new(5));
    for version in [0u8, 3, 4, 127, 255] {
        let mut changed = v1.clone();
        changed[4] = version;
        let err = binary::decode(&changed).expect_err("an unknown version");
        assert_eq!(err.offset, 5);
        assert!(err.message.contains("versions 1 and 2"), "{err}");
    }
}

#[test]
fn a_version_1_document_with_a_version_2_tag_is_an_error() {
    // Tag 15 (a shaped map), tag 16 (records), a tag that is an int or a string by itself.
    for tag in [15u8, 16, 32, 63, 128, 255] {
        let err =
            binary::decode(&[b'S', b'D', b'A', b'T', 1, tag, 0, 0, 0]).expect_err("version 1");
        assert!(err.message.contains(&format!("unknown tag {tag}")), "{err}");
    }
    // The same bytes in version 2 are a value (or another error), not "unknown tag".
    assert_eq!(
        binary::decode(&[b'S', b'D', b'A', b'T', 2, 128]).expect("an int"),
        Value::Int(-16)
    );
}
