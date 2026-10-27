//! The words the formats share, defined once: diagnostic codes, the typed
//! value grammar's characters and words, the keys tagged enums use.

/// What starts a typed value: `@asset:meshes/cube.mesh`.
pub const TYPED: &str = "@";

/// The kinds of typed value that aren't references.
pub mod typed {
    /// Bytes as they are, base64 in text: `@bytes:iVBORw0…`.
    pub const BYTES: &str = "bytes";
}

/// Keys a format's own structure uses.
pub mod word {
    /// The key naming a tagged enum's variant: `{ type: "sphere", … }`.
    pub const TYPE: &str = "type";
    /// Where a tagged enum's variant keeps what it wraps when that isn't a struct.
    pub const VALUE: &str = "value";
}

/// What's wrong with a value: the code of each [`crate::Diagnostic`]
/// (`SD…`), stable so tools can match on them.
pub mod problem {
    /// Any other problem with a value.
    pub const INVALID: &str = "SD0000";
    /// Text that doesn't parse.
    pub const SYNTAX: &str = "SD0001";
    /// A value of another type than its field's.
    pub const EXPECTED: &str = "SD0002";
    /// A bare id where a reference goes (`@asset:…`).
    pub const NOT_A_REFERENCE: &str = "SD0003";
    /// A reference to another kind than its field takes.
    pub const WRONG_KIND: &str = "SD0004";
    pub const UNKNOWN_FIELD: &str = "SD0005";
    pub const MISSING_FIELD: &str = "SD0006";
    pub const OUT_OF_RANGE: &str = "SD0007";
    /// A string written as a typed value that isn't one (`@Asset:x`).
    pub const BAD_TYPED: &str = "SD0008";
    /// Binary data that won't decode.
    pub const BAD_BINARY: &str = "SD0009";
}

/// The words a [`crate::Shape`] is described by ([`crate::Shape::describe`]).
pub mod shape_kind {
    pub const ANY: &str = "any";
    pub const BOOL: &str = "bool";
    pub const INT: &str = "int";
    pub const FLOAT: &str = "float";
    pub const STRING: &str = "string";
    pub const ATOM: &str = "atom";
    pub const BYTES: &str = "bytes";
    pub const REF: &str = "ref";
    pub const NAMED: &str = "named";
    pub const LIST: &str = "list";
    pub const MAP: &str = "map";
    pub const OPTIONAL: &str = "optional";
    pub const STRUCT: &str = "struct";
    pub const ENUM: &str = "enum";
    /// A struct or enum written once in `defs`, referred to by name.
    pub const DEF: &str = "def";
}
