//! Compact owned types and 32-bit floats, written straight.

use substrate_data::Schema;

/// Compact owned types (no spare capacity) and 32-bit floats, written straight.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Compact {
    pub(crate) name: Box<str>,
    pub(crate) shared: std::sync::Arc<str>,
    pub(crate) ids: Box<[u32]>,
    pub(crate) weights: Vec<f32>,
    pub(crate) scale: f32,
}

#[test]
fn compact_types_round_trip() {
    let c = Compact {
        name: "a".into(),
        shared: "b".into(),
        ids: vec![1, 2].into_boxed_slice(),
        weights: vec![0.5, 1.25],
        scale: 2.5,
    };
    let bytes = substrate_data::to_bytes(&c);
    assert_eq!(substrate_data::read_bytes::<Compact>(&bytes).unwrap(), c);
    assert_eq!(
        substrate_data::from_text::<Compact>(&substrate_data::to_text(&c))
            .unwrap()
            .value,
        c
    );
}
