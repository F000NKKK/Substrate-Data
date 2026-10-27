//! A value's first byte: what it is.

/// The tags, each a constant, and [`ALL`] of them by name.
macro_rules! tags {
    ($($(#[$doc:meta])* $name:ident = $value:expr;)*) => {
        $($(#[$doc])* pub const $name: u8 = $value;)*
        /// Every tag, by its name (what other languages' readers are generated from).
        pub const ALL: &[(&str, u8)] = &[$((stringify!($name), $value)),*];
    };
}

tags! {
    /// Null.
    NULL = 0;
    /// `false`.
    FALSE = 1;
    /// `true`.
    TRUE = 2;
    /// An integer, zigzag varint.
    INT = 3;
    /// A float, an `f64` little-endian.
    FLOAT = 4;
    /// A string: new (its bytes) or the table's.
    STR = 5;
    /// A list: a count, then the values.
    LIST = 6;
    /// A map: a count, then key and value pairs.
    MAP = 7;
    /// A reference: its kind and id strings.
    REF = 8;
    /// Bytes: a length, then them.
    BYTES = 9;
    /// A list of ints without a tag each: what a schema of numbers writes its lists as.
    INTS = 10;
    /// A list in a body of its own (its own strings): read lazily, cut out whole ([`crate::Stream`]).
    STREAM = 11;
    /// A float that is an `f32` exactly, 4 bytes.
    FLOAT32 = 12;
    /// Lists of floats without a tag each.
    FLOATS = 13;
    /// Lists of `f32`s without a tag each.
    FLOATS32 = 14;
}
