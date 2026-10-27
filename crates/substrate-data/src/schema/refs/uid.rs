//! [`Uid`]: a 128-bit id, spelled in hex.

use std::borrow::Cow;
use std::fmt;

use super::RefId;
use crate::schema::Expected;

/// A 128-bit id (a UUID's bits, a hash), spelled as 32 lower-case hex
/// digits: `@doc:0123456789abcdef0123456789abcdef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Uid(pub u128);

/// Hex digits of a [`Uid`].
const UID_DIGITS: usize = 32;

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0width$x}", self.0, width = UID_DIGITS)
    }
}

impl RefId for Uid {
    const EXPECTED: Expected = Expected::Uid;
    fn parse(spelled: &str) -> Option<Self> {
        if spelled.len() != UID_DIGITS
            || !spelled
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        u128::from_str_radix(spelled, 16).ok().map(Uid)
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

/// Stored as its hex text everywhere (a tool shows a `uid`).
impl crate::schema::NamedType for Uid {
    const NAME: &'static str = "uid";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.to_string()
    }
    fn from_stored(s: String) -> Option<Self> {
        <Uid as RefId>::parse(&s)
    }
}

crate::named_schema!(Uid);
