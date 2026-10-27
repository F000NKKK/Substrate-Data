//! [`Atom`]: a name kept once a process, a number in frames.

use std::fmt;

use super::table::table;

/// A name kept once a process; see the module.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Atom(u32);

impl Atom {
    /// `name`'s atom.
    #[must_use]
    pub fn new(name: &str) -> Atom {
        Atom((table().intern)(name))
    }

    /// The atom numbered `n`, if there's one.
    #[must_use]
    pub fn from_number(n: u32) -> Option<Atom> {
        (table().name)(n).map(|_| Atom(n))
    }

    /// Its number in this process's table.
    #[must_use]
    pub fn number(self) -> u32 {
        self.0
    }

    /// Its text.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        (table().name)(self.0).unwrap_or_default()
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Atom({:?})", self.as_str())
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Atom {
    fn from(name: &str) -> Atom {
        Atom::new(name)
    }
}

impl From<String> for Atom {
    fn from(name: String) -> Atom {
        Atom::new(&name)
    }
}

impl From<&String> for Atom {
    fn from(name: &String) -> Atom {
        Atom::new(name)
    }
}

impl PartialEq<str> for Atom {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Atom {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Atom {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Atom> for String {
    fn eq(&self, other: &Atom) -> bool {
        self == other.as_str()
    }
}

impl std::ops::Deref for Atom {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Default for Atom {
    /// The empty name's atom.
    fn default() -> Atom {
        Atom::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_one_atom() {
        let a = Atom::new("test:atom_a");
        assert_eq!(a, Atom::new("test:atom_a"));
        assert_ne!(a, Atom::new("test:atom_b"));
        assert_eq!(Atom::from_number(a.number()), Some(a));
        assert_eq!(Atom::from_number(u32::MAX), None);
        assert_eq!(a.to_string(), "test:atom_a");
    }
}
