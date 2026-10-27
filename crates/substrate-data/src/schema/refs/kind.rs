//! [`Kind`] and [`Kinds`]: what a reference names, declared with `kind!`.

use super::RefId;
use crate::model::value::KindName;

/// A kind of thing a reference names: `user`, `doc`, `file`, ... — a
/// format declares its own with [`kind!`](crate::kind), each with the type
/// of its ids.
pub trait Kind {
    /// As written: lower case letters, digits and `_` (`kind!` checks it
    /// when it compiles).
    const NAME: &'static str;
    /// Its ids.
    type Id: RefId;

    /// [`Kind::NAME`] as a kind.
    #[must_use]
    fn kind_name() -> KindName {
        KindName::declared(Self::NAME)
    }
}

/// One kind or several (`(File, Doc)`, sharing an id type): what a
/// reference field takes.
pub trait Kinds {
    /// Every kind's name, the first the one written.
    const NAMES: &'static [&'static str];
    /// Their ids' type, the same for every kind.
    type Id: RefId;

    /// The first of them: what a reference this reads is written as.
    #[must_use]
    fn first() -> KindName {
        KindName::declared(Self::NAMES.first().copied().unwrap_or_default())
    }
}

impl<K: Kind> Kinds for K {
    const NAMES: &'static [&'static str] = &[K::NAME];
    type Id = K::Id;
}

impl<A: Kind, B: Kind<Id = A::Id>> Kinds for (A, B) {
    const NAMES: &'static [&'static str] = &[A::NAME, B::NAME];
    type Id = A::Id;
}

impl<A: Kind, B: Kind<Id = A::Id>, C: Kind<Id = A::Id>> Kinds for (A, B, C) {
    const NAMES: &'static [&'static str] = &[A::NAME, B::NAME, C::NAME];
    type Id = A::Id;
}

/// Declares kinds, each with its id type (`String` when none is said):
/// `substrate_data::kind!(User = "user": u64, Doc = "doc");`.
#[macro_export]
macro_rules! kind {
    ($($name:ident = $kind:literal $(: $id:ty)?),* $(,)?) => {$(
        #[doc = concat!("The `", $kind, "` kind of reference.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name;
        impl $crate::Kind for $name {
            const NAME: &'static str = {
                assert!(
                    $crate::KindName::is_valid($kind),
                    "a kind is lower case letters, digits and `_`, starting with a letter"
                );
                $kind
            };
            type Id = $crate::__id_or_string!($($id)?);
        }
    )*};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __id_or_string {
    () => {
        ::std::string::String
    };
    ($id:ty) => {
        $id
    };
}
