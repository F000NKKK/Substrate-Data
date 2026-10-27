//! Cases for the packed list tags and streams, written by the typed writers.

use substrate_data::binary::Writer;
use substrate_data::{Schema, Stream, Value, to_bytes};

use crate::case::Case;
use crate::v1::V1Writer;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = packed_ints();
    all.extend(packed_floats());
    all.extend(streams());
    all
}

fn ints(items: &[i64]) -> Value {
    Value::List(items.iter().map(|&n| Value::Int(n)).collect())
}

fn floats(items: &[f64]) -> Value {
    Value::List(items.iter().map(|&f| Value::Float(f)).collect())
}

/// A case whose value is written as the version 1 bytes `old` and the
/// version 2 bytes `new` of a typed writer, and as text.
fn written(name: &str, description: &str, value: Value, old: Vec<u8>, new: Vec<u8>) -> Case {
    let text = substrate_data::text::to_string(&value);
    Case::new(name, description)
        .text(&text)
        .value(value)
        .binary(old)
        .binary_v2(new)
}

fn packed_ints() -> Vec<Case> {
    let sets: [(&str, &[i64]); 4] = [
        ("empty", &[]),
        ("small", &[0, -1, 1, 63, -64]),
        ("varints", &[64, -65, 8192, 2_147_483_648]),
        ("extremes", &[i64::MIN, i64::MAX, 0]),
    ];
    sets.iter()
        .map(|&(name, items)| {
            let mut w = Writer::new();
            w.ints(items.iter().copied());
            let mut old = V1Writer::new();
            old.ints(items);
            written(
                &format!("ints_packed_{name}"),
                "A list of ints packed: tag 10, a count, then a zigzag varint per item with no tag each. Decodes to the same list as tag 6 would.",
                ints(items),
                old.finish(),
                w.finish(),
            )
        })
        .collect()
}

/// The version 1 and the version 2 bytes a writer makes from the same calls.
macro_rules! both {
    (|$w:ident| $body:expr) => {{
        let mut old = V1Writer::new();
        {
            let $w = &mut old;
            $body;
        }
        let mut new = Writer::new();
        {
            let $w = &mut new;
            $body;
        }
        (old.finish(), new.finish())
    }};
}

fn packed_floats() -> Vec<Case> {
    let (single_old, single_new) = both!(|w| w.float32(0.5));
    let (narrow_old, narrow_new) = both!(|w| w.floats(&[0.5, 1.5, -0.0]));
    let (wide_old, wide_new) = both!(|w| w.floats(&[0.1, 0.2]));
    let (mixed_old, mixed_new) = both!(|w| w.floats(&[0.5, 0.1]));
    vec![
        written(
            "float32_single",
            "A float that is an f32: tag 12, four bytes little-endian. Decodes to the f64 of the same value.",
            Value::Float(0.5),
            single_old,
            single_new,
        ),
        written(
            "floats_packed_narrow",
            "A writer picks tag 14 (f32 each) when every f64 survives the round trip through f32 bit for bit; -0.0 does.",
            floats(&[0.5, 1.5, -0.0]),
            narrow_old,
            narrow_new,
        ),
        written(
            "floats_packed_wide",
            "Tag 13 (f64 each) when any one does not: 0.1 is not an f32.",
            floats(&[0.1, 0.2]),
            wide_old,
            wide_new,
        ),
        written(
            "floats_packed_mixed",
            "One value that is not an f32 widens the whole list, 0.5 included.",
            floats(&[0.5, 0.1]),
            mixed_old,
            mixed_new,
        ),
    ]
    .into_iter()
    .chain(more_floats())
    .collect()
}

fn more_floats() -> Vec<Case> {
    let (empty_old, empty_new) = both!(|w| w.floats(&[]));
    let (f32s_old, f32s_new) = both!(|w| w.floats32(&[0.5, f32::MAX]));
    let (nan_old, nan_new) = both!(|w| w.floats(&[f64::NAN, f64::INFINITY]));
    vec![
        written(
            "floats_packed_empty",
            "An empty list of floats picks the narrow tag 14.",
            Value::List(vec![]),
            empty_old,
            empty_new,
        ),
        written(
            "floats32_packed",
            "A list of f32s: tag 14 always, 4 bytes each.",
            floats(&[0.5, f64::from(f32::MAX)]),
            f32s_old,
            f32s_new,
        ),
        nan_case(nan_old, nan_new),
    ]
}

fn nan_case(old: Vec<u8>, new: Vec<u8>) -> Case {
    Case::new(
        "floats_packed_non_finite",
        "Binary only (text writes null for them): a quiet NaN and infinity survive f32 bit for bit, so a typed writer uses the narrow tag 14. A model value's writer never narrows a NaN (see the floats rule in the format).",
    )
    .value(floats(&[f64::NAN, f64::INFINITY]))
    .binary(old)
    .binary_v2(new)
}

/// A log: a name and a stream of entries.
#[derive(Schema, Debug)]
struct Log {
    name: String,
    entries: Stream<String>,
}

#[derive(Schema, Debug)]
struct Counter {
    hits: Stream<u32>,
}

fn streams() -> Vec<Case> {
    let entries =
        |items: &[&str]| -> Stream<String> { items.iter().map(|s| (*s).to_owned()).collect() };
    let empty = Log {
        name: "audit".into(),
        entries: entries(&[]),
    };
    let repeats = Log {
        name: "audit".into(),
        entries: entries(&["opened", "saved", "opened", "audit"]),
    };
    let counter = Counter {
        hits: (0..6u32).map(|i| i * 1000).collect(),
    };
    vec![
        schema_case(
            "stream_empty",
            "An empty stream: tag 11, a u32 byte length (here 1), then the count 0.",
            &empty,
        ),
        schema_case(
            "stream_own_table",
            "A stream has its own string table: `audit` is written again inside it although the document already has it, and `opened` is written once inside.",
            &repeats,
        ),
        schema_case(
            "stream_ints",
            "Stream items are values: six ints, each tag 3 and a zigzag varint in version 1; a small int is one byte in version 2.",
            &counter,
        ),
    ]
}

/// A case of a typed value: its text, its model value and its binary form.
fn schema_case<T: Schema>(name: &str, description: &str, x: &T) -> Case {
    let bytes = to_bytes(x);
    let value = substrate_data::binary::decode(&bytes).expect("the writer's own bytes");
    let text = substrate_data::text::to_string(&value);
    let case = Case::new(name, description)
        .text(&text)
        .value(value)
        .binary_v2(bytes)
        .legacy_binary();
    let old = case
        .binary
        .as_deref()
        .expect("the version 1 file of a stream case");
    let read = substrate_data::binary::decode(old).expect("version 1 still reads");
    assert!(
        crate::render::same(&read, case.value.as_ref().expect("a value")),
        "{name}: version 1 and 2 read differently"
    );
    case
}
