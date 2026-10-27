//! The neutral schemas the frame and diagnostic cases are written for
//! (users, docs, files, projects), and the registry that finds one by its
//! name in the manifest.

use substrate_data::{Errors, Id, Ref, Schema, Shape, Value, text};

substrate_data::kind!(User = "user": u64, Doc = "doc", File = "file", Project = "project");

/// One of each plain scalar.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Scalars {
    pub(crate) flag: bool,
    pub(crate) count: i32,
    pub(crate) big: u64,
    pub(crate) ratio: f32,
    pub(crate) precise: f64,
    pub(crate) name: String,
}

/// Strings that repeat, and a list of them.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Page {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) editor: String,
    pub(crate) tags: Vec<String>,
}

/// Optional fields, present and absent.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Options {
    pub(crate) count: Option<u32>,
    pub(crate) note: Option<String>,
    pub(crate) missing: Option<u8>,
}

/// References: a numeric id, a text id and one of any kind.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Links {
    pub(crate) owner: Id<User>,
    pub(crate) parent: Id<Doc>,
    pub(crate) attachment: Ref,
}

/// A payload kept as bytes.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Blob {
    pub(crate) name: String,
    #[data(with = "substrate_data::with::Raw")]
    pub(crate) data: Vec<u8>,
}

/// A unit enum.
#[derive(Schema, Debug, Clone, Copy, PartialEq)]
pub(crate) enum Role {
    Reader,
    Editor,
    Admin,
}

/// A record with an enum, listed in a folder.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) role: Role,
}

/// A record of records.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Folder {
    pub(crate) name: String,
    pub(crate) members: Vec<Member>,
    pub(crate) tags: Vec<String>,
}

/// A tagged enum: a frame holds it in its binary form.
#[derive(Schema, Debug, Clone, PartialEq)]
#[data(tag = "type")]
pub(crate) enum Event {
    Opened { at: u32 },
    Renamed { name: String },
    Closed,
}

/// A record holding a tagged enum.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Activity {
    pub(crate) id: u32,
    pub(crate) event: Event,
}

/// Lists of floats and ints.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Samples {
    pub(crate) narrow: Vec<f32>,
    pub(crate) exact: Vec<f64>,
    pub(crate) wide: Vec<f64>,
    pub(crate) signed: Vec<i32>,
    pub(crate) unsigned: Vec<u16>,
}

/// A project as it first was.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct ProjectV1 {
    pub(crate) name: String,
    pub(crate) files: u32,
}

/// The same project with fields appended.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct ProjectV2 {
    pub(crate) name: String,
    pub(crate) files: u32,
    #[data(default)]
    pub(crate) owner: Option<Id<User>>,
    #[data(default)]
    pub(crate) archived: bool,
}

/// A number with a range, for the range diagnostic.
#[derive(Schema, Debug, Clone, PartialEq)]
pub(crate) struct Limits {
    #[data(range = "1..=99")]
    pub(crate) copies: u32,
    pub(crate) owner: Id<User>,
    pub(crate) file: Id<File>,
}

/// The shape of the schema called `name`.
pub(crate) fn shape_of(name: &str) -> Option<Shape> {
    Some(match name {
        "scalars" => Scalars::shape(),
        "page" => Page::shape(),
        "options" => Options::shape(),
        "links" => Links::shape(),
        "blob" => Blob::shape(),
        "member" => Member::shape(),
        "folder" => Folder::shape(),
        "activity" => Activity::shape(),
        "samples" => Samples::shape(),
        "project_v1" => ProjectV1::shape(),
        "project_v2" => ProjectV2::shape(),
        "limits" => Limits::shape(),
        "uint" => u32::shape(),
        "bool" => bool::shape(),
        "int_list" => Vec::<i32>::shape(),
        "uint_list" => Vec::<u32>::shape(),
        "ulong_list" => Vec::<u64>::shape(),
        "text" => String::shape(),
        "any" => Value::shape(),
        _ => return None,
    })
}

/// The shape of `name` described as compact text: `{ root, defs }`.
pub(crate) fn shape_text(name: &str) -> String {
    let shape = shape_of(name).unwrap_or_else(|| panic!("no schema called {name}"));
    text::to_string(&shape.describe().write())
}

/// Reads `input` as the schema called `name`: `Err` with every problem.
pub(crate) fn read_text(name: &str, input: &str) -> Result<(), Errors> {
    match name {
        "links" => substrate_data::from_text::<Links>(input).map(drop),
        "limits" => substrate_data::from_text::<Limits>(input).map(drop),
        "project_v2" => substrate_data::from_text::<ProjectV2>(input).map(drop),
        _ => panic!("no text schema called {name}"),
    }
}
