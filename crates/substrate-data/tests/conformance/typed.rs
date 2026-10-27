//! A case of a typed value: the same value in every form, made from the
//! reference implementation and cross-checked before it is written.

use std::fmt::Debug;

use substrate_data::frame::{read_frame, read_shaped, to_frame};
use substrate_data::{Schema, binary, text, to_bytes};

use crate::case::{Case, Frame, Reading};
use crate::render::same;

/// `x` in every form. `schema` names its schema in [`crate::schemas`].
///
/// The text is given only when `with_text`: a long list is no text a
/// person reads.
pub(crate) fn typed_case<T: Schema + PartialEq + Debug>(
    name: &str,
    description: &str,
    schema: &'static str,
    x: &T,
) -> Case {
    let case = typed_frame_case(name, description, schema, x);
    let value = case.value.clone().expect("a value");
    let spelled = text::to_string(&x.write());
    let parsed = text::parse_value(&spelled).expect("the writer's own text");
    assert!(
        same(&parsed, &substrate_data::typed::flatten(&value)),
        "{name}: text and binary disagree: {parsed:?} / {value:?}"
    );
    if value_is_short(&spelled) {
        return case.text(&spelled);
    }
    case
}

/// `x` as a frame and as binary, no text.
pub(crate) fn typed_frame_case<T: Schema + PartialEq + Debug>(
    name: &str,
    description: &str,
    schema: &'static str,
    x: &T,
) -> Case {
    let frame = to_frame(x);
    assert_eq!(read_frame::<T>(&frame).as_ref(), Some(x), "{name}: frame");
    let shaped = read_shaped(&frame, &T::shape()).expect("read by shape");
    let bytes = to_bytes(x);
    assert_eq!(
        substrate_data::read_bytes::<T>(&bytes).as_ref().ok(),
        Some(x),
        "{name}: binary"
    );
    let value = binary::decode(&bytes).expect("the writer's own bytes");
    let writer = Reading {
        schema,
        value: shaped,
    };
    let frame = Frame::new(name, writer, frame, vec![]);
    for old in std::iter::once(&frame.bytes).chain(&frame.v2) {
        assert_eq!(
            read_frame::<T>(old).as_ref(),
            Some(x),
            "{name}: a frame of either version"
        );
    }
    let case = Case::new(name, description)
        .value(value)
        .binary_v2(bytes)
        .legacy_binary()
        .frame(frame);
    if let Some(old) = &case.binary {
        let read = substrate_data::read_bytes::<T>(old);
        assert_eq!(
            read.as_ref().ok(),
            Some(x),
            "{name}: the version 1 bytes still read"
        );
    }
    case
}

/// Text longer than this stays out of the manifest.
const TEXT_MAX: usize = 400;

fn value_is_short(spelled: &str) -> bool {
    spelled.len() <= TEXT_MAX
}
