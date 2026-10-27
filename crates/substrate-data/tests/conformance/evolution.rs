//! Frame cases for records that grow, and for frames whose root is no record.

use substrate_data::Id;
use substrate_data::frame::{read_frame, read_shaped, to_frame};

use crate::case::{Case, Frame, Reading};
use crate::schemas::{ProjectV1, ProjectV2, shape_of};
use crate::typed::typed_case;

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = evolution();
    all.extend(roots());
    all
}

/// What reading `bytes` by the shape of `schema` gives.
fn reading(schema: &'static str, bytes: &[u8]) -> Reading {
    let shape = shape_of(schema).expect("a schema");
    Reading {
        schema,
        value: read_shaped(bytes, &shape).expect("readable"),
    }
}

fn evolution() -> Vec<Case> {
    let newer = ProjectV2 {
        name: "notes".into(),
        files: 12,
        owner: Some(Id::new(7u64)),
        archived: true,
    };
    let older = ProjectV1 {
        name: "notes".into(),
        files: 12,
    };
    let newer_bytes = to_frame(&newer);
    let older_bytes = to_frame(&older);
    assert_eq!(
        read_frame::<ProjectV2>(&older_bytes),
        Some(ProjectV2 {
            name: "notes".into(),
            files: 12,
            owner: None,
            archived: false
        }),
        "an older frame fills the appended fields' defaults"
    );
    assert_eq!(
        read_frame::<ProjectV1>(&newer_bytes),
        Some(older),
        "a reader that knows fewer fields skips the rest"
    );
    vec![
        Case::new(
            "frame_record_appended_fields",
            "Fields are only ever appended. A frame packed with `owner` and `archived` is read by a schema that knows only `name` and `files`: the record's length lets it skip the rest.",
        )
        .frame(Frame::new(
            "frame_record_appended_fields",
            reading("project_v2", &newer_bytes),
            newer_bytes.clone(),
            vec![reading("project_v1", &newer_bytes)],
        )),
        Case::new(
            "frame_record_older_frame",
            "A frame packed before the fields were appended, read by a newer schema: the missing trailing fields are left out of the value read by shape, and take their defaults when the schema says `default` (owner none, archived false).",
        )
        .frame(Frame::new(
            "frame_record_older_frame",
            reading("project_v1", &older_bytes),
            older_bytes.clone(),
            vec![reading("project_v2", &older_bytes)],
        )),
    ]
}

fn roots() -> Vec<Case> {
    vec![
        typed_case(
            "frame_root_uint",
            "A frame whose root is a number: the varint, then an empty table (length 0).",
            "uint",
            &300u32,
        ),
        typed_case(
            "frame_root_text",
            "A frame whose root is a string: the table index 0, then the table holding the string.",
            "text",
            &"hi".to_owned(),
        ),
        typed_case(
            "frame_root_text_at",
            "A frame holds a string raw; reading it by shape gives the model's spelling, a leading `@` doubled.",
            "text",
            &"@home".to_owned(),
        ),
    ]
}
