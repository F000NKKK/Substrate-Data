//! `#[data(skip)]` on a variant's field.

use substrate_data::{Schema, from_value};

/// A variant with a skipped field: the derive binds it as `_`.
#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
enum Mark {
    Note {
        text: String,
        #[data(skip)]
        seen: bool,
    },
}

#[test]
fn a_skipped_variant_field_is_not_written_and_reads_as_default() {
    let mark = Mark::Note {
        text: "draft".into(),
        seen: true,
    };
    let written = mark.write();
    assert!(written.get("seen").is_none());
    let again = from_value::<Mark>(&written).unwrap().value;
    assert_eq!(
        again,
        Mark::Note {
            text: "draft".into(),
            seen: false
        }
    );
}
