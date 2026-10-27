//! A reference's two parts, each checked once where it's made: its kind
//! ([`KindName`], interned like an [`Atom`]) and its id ([`RefKey`]).

use std::fmt;

use crate::Atom;
use crate::typed::TypedError;

/// A reference's kind: lower case ASCII letters, digits and `_`, starting
/// with a letter (`user`, `doc`, `file`). Checked when it's made, then a
/// number: copied and compared as one.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct KindName(Atom);

impl KindName {
    /// `name` as a kind, if it's one.
    ///
    /// # Errors
    ///
    /// [`TypedError::BadKind`] when `name` isn't lower case letters, digits and `_`
    /// starting with a letter.
    pub fn new(name: &str) -> Result<KindName, TypedError> {
        if Self::is_valid(name) {
            Ok(KindName(Atom::new(name)))
        } else {
            Err(TypedError::BadKind {
                kind: name.to_owned(),
            })
        }
    }

    /// A name `kind!` already checked at compile time.
    #[must_use]
    pub fn declared(name: &'static str) -> KindName {
        debug_assert!(Self::is_valid(name), "`{name}` isn't a kind");
        KindName(Atom::new(name))
    }

    /// Whether `name` is a kind — at compile time too (`kind!` checks its
    /// names with it).
    #[must_use]
    pub const fn is_valid(name: &str) -> bool {
        let b = name.as_bytes();
        if b.is_empty() || !b[0].is_ascii_lowercase() {
            return false;
        }
        let mut i = 1;
        while i < b.len() {
            let c = b[i];
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_') {
                return false;
            }
            i += 1;
        }
        true
    }

    /// As written.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        self.0.as_str()
    }

    /// Its number in this process's atom table.
    #[must_use]
    pub fn atom(self) -> Atom {
        self.0
    }
}

impl fmt::Debug for KindName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KindName({:?})", self.as_str())
    }
}

impl fmt::Display for KindName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialOrd for KindName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KindName {
    /// By name, not by number.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialEq<str> for KindName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for KindName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

/// A reference's id as written, untyped: anything but empty, whitespace or
/// control characters. What it names is the format's business; a typed
/// field reads it as its kind's id type ([`crate::RefId`]).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RefKey(Box<str>);

impl RefKey {
    /// `id` as a reference's id, if it's one.
    ///
    /// # Errors
    ///
    /// [`TypedError::BadId`] when `id` is empty or has whitespace or control characters.
    pub fn new(id: &str) -> Result<RefKey, TypedError> {
        if Self::is_valid(id) {
            Ok(RefKey(id.into()))
        } else {
            Err(TypedError::BadId {
                kind: None,
                id: id.to_owned(),
            })
        }
    }

    /// Whether `id` may be a reference's id.
    #[must_use]
    pub fn is_valid(id: &str) -> bool {
        !id.is_empty() && !id.chars().any(|c| c.is_whitespace() || c.is_control())
    }

    /// A typed id's spelling, as is: it's checked when it's read back.
    pub(crate) fn spelled(id: &str) -> RefKey {
        RefKey(id.into())
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RefKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RefKey({:?})", &*self.0)
    }
}

impl fmt::Display for RefKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for RefKey {
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for RefKey {
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl std::ops::Deref for KindName {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for KindName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<String> for KindName {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl std::ops::Deref for RefKey {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RefKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A map keyed by ids is looked up by a `&str`: an id hashes and orders as its text.
impl std::borrow::Borrow<str> for RefKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<String> for RefKey {
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<RefKey> for String {
    fn eq(&self, other: &RefKey) -> bool {
        self.as_str() == &*other.0
    }
}

impl PartialEq<RefKey> for str {
    fn eq(&self, other: &RefKey) -> bool {
        self == &*other.0
    }
}
