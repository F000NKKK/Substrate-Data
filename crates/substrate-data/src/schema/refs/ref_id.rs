//! [`RefId`]: the type of a kind's ids, and its impls for text and numbers.

use std::borrow::Cow;
use std::fmt;

use crate::model::value::RefKey;
use crate::schema::{Expected, Schema};

/// The type of a kind's ids: how one is spelled in a reference
/// (`@user:42`), read back, and stored in every other form (a frame packs
/// it as itself — a number as a number).
pub trait RefId: Schema + Clone + PartialEq + fmt::Debug {
    /// What an id of this type looks like, for messages.
    const EXPECTED: Expected;
    /// The id `spelled` is, if it's one of this type.
    fn parse(spelled: &str) -> Option<Self>;
    /// As written after `@kind:`.
    fn spell(&self) -> Cow<'_, str>;
}

impl RefId for String {
    const EXPECTED: Expected = Expected::Text;
    fn parse(spelled: &str) -> Option<Self> {
        RefKey::is_valid(spelled).then(|| spelled.to_owned())
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

macro_rules! number_ids {
    ($($t:ty => $what:ident),*) => {$(
        impl RefId for $t {
            const EXPECTED: Expected = Expected::$what;
            fn parse(spelled: &str) -> Option<Self> {
                // Digits only: no sign, no spaces — one spelling per id.
                let digits = spelled.strip_prefix('-').unwrap_or(spelled);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                spelled.parse().ok()
            }
            fn spell(&self) -> Cow<'_, str> {
                Cow::Owned(self.to_string())
            }
        }
    )*};
}
number_ids!(u32 => Unsigned, u64 => Unsigned, i64 => Integer);
