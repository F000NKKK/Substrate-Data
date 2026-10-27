//! The schemas of typed values: references, by kind, with typed ids.
use std::borrow::Cow;

use super::{Expected, PhantomData, Reader, Ref, Schema, Shape, Value, ValueRef, fmt, typed};
use crate::diagnostic::Problem;
use crate::frame::{Packer, Unpacker};
use crate::model::value::{KindName, RefKey};

// ── Ids ─────────────────────────────────────────────────────────────────────

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
impl super::NamedType for Uid {
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

// ── Kinds ───────────────────────────────────────────────────────────────────

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
    fn kind_name() -> KindName {
        KindName::declared(Self::NAME)
    }
}

/// One kind or several (`(File, Doc)`, sharing an id type): what a
/// reference field takes.
pub trait Kinds {
    const NAMES: &'static [&'static str];
    type Id: RefId;

    /// The first of them: what a reference this reads is written as.
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

// ── Typed references ────────────────────────────────────────────────────────

/// The id of something of kind `K`, of `K`'s id type: written
/// `@file:images/logo.png`, `@user:42`.
pub struct Id<K: Kind> {
    pub id: K::Id,
    kind: PhantomData<K>,
}

impl<K: Kind> Id<K> {
    pub fn new(id: impl Into<K::Id>) -> Self {
        Self {
            id: id.into(),
            kind: PhantomData,
        }
    }

    /// As a reference.
    pub fn to_ref(&self) -> Ref {
        Ref::of::<K>(&self.id)
    }
}

impl<K: Kind> Clone for Id<K> {
    fn clone(&self) -> Self {
        Self::new(self.id.clone())
    }
}
impl<K: Kind> PartialEq for Id<K> {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id
    }
}
impl<K: Kind> Eq for Id<K> where K::Id: Eq {}
impl<K: Kind> std::hash::Hash for Id<K>
where
    K::Id: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}
impl<K: Kind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}:{}", K::NAME, self.id.spell())
    }
}
impl<K: Kind> fmt::Display for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id.spell())
    }
}

/// `v` as a reference to one of `kinds` (any kind if empty), written
/// `@kind:id` — a bare id is an error that says how to write it. The id
/// is read untyped ([`read_id`] reads it as a kind's id type).
pub fn read_ref(v: &ValueRef, cx: &mut Reader, kinds: &[&str]) -> Option<Ref> {
    let wanted = || {
        kinds
            .iter()
            .map(|k| format!("`@{k}:`"))
            .collect::<Vec<_>>()
            .join(" or ")
    };
    let r = match v {
        ValueRef::Ref { kind, id } => Ref::new(*kind, RefKey::spelled(id)),
        ValueRef::Str(s) if s.starts_with(crate::names::TYPED) => match typed::classify(s) {
            Ok(typed::Spelled::Ref(r)) => r,
            Ok(other) => {
                cx.problem_of(
                    Problem::Expected,
                    format!(
                        "expected a reference ({}), found {}",
                        if kinds.is_empty() {
                            "`@kind:id`".into()
                        } else {
                            wanted()
                        },
                        other.kind()
                    ),
                );
                return None;
            }
            Err(e) => {
                cx.problem_of(Problem::BadTyped, e.to_string());
                return None;
            }
        },
        ValueRef::Str(s) => {
            let kind = kinds.first().copied().unwrap_or("kind");
            cx.problem_of(
                Problem::NotAReference,
                format!("`{s}`: name it by a reference — `@{kind}:{s}`"),
            );
            return None;
        }
        other => {
            cx.expected(Expected::Reference, other);
            return None;
        }
    };
    if !kinds.is_empty() && !kinds.contains(&r.kind.as_str()) {
        cx.problem_of(
            Problem::WrongKind,
            format!(
                "`{r}` is a reference to {} — this takes {}",
                r.kind,
                wanted()
            ),
        );
        return None;
    }
    cx.note_ref(r.clone());
    Some(r)
}

/// `id` (of a reference of `kind`) as an `I`, or a problem.
fn parse_id<I: RefId>(cx: &mut Reader, kind: KindName, id: &str) -> Option<I> {
    I::parse(id).or_else(|| {
        cx.problem_of(
            Problem::BadId,
            format!(
                "`@{kind}:{id}`: `{id}` isn't a {kind} id (expected {})",
                I::EXPECTED
            ),
        );
        None
    })
}

/// `v` as a reference to one of `K`'s kinds, its id read as `K::Id` — as
/// written, or as the [`crate::Resolve`] rewrote it.
pub fn read_id<K: Kinds>(v: &ValueRef, cx: &mut Reader) -> Option<K::Id> {
    // The usual case, a reference of a kind it takes, nothing kept or resolved.
    if let ValueRef::Ref { kind, id } = v
        && K::NAMES.contains(&kind.as_str())
        && !cx.keeps_found()
        && !cx.resolves()
    {
        return parse_id(cx, *kind, id);
    }
    let r = read_ref(v, cx, K::NAMES)?;
    let id = parse_id::<K::Id>(cx, r.kind, r.id.as_str())?;
    if !cx.resolves() {
        return Some(id);
    }
    let kind = r.kind;
    let key = cx.take_ref(r)?;
    parse_id(cx, kind, key.as_str())
}

/// The shape of a reference to `K`.
pub fn ref_shape<K: Kinds>() -> Shape {
    Shape::Ref {
        kinds: K::NAMES,
        id: <K::Id as Schema>::shape,
    }
}

/// In a frame, only its id (the kind is the field's).
impl<K: Kind> Schema for Id<K> {
    fn shape() -> Shape {
        ref_shape::<K>()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        read_id::<K>(v, cx).map(Id::new)
    }
    fn write(&self) -> Value {
        Value::Ref(self.to_ref())
    }
    fn pack(&self, p: &mut Packer) {
        self.id.pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        K::Id::unpack(u).map(Id::new)
    }
}

/// A reference of any kind: in a frame, its kind and its id, as text.
impl Schema for Ref {
    fn shape() -> Shape {
        Shape::Ref {
            kinds: &[],
            id: String::shape,
        }
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        if let ValueRef::Str(s) = v
            && !s.starts_with(crate::names::TYPED)
        {
            cx.expected(Expected::Reference, v);
            return None;
        }
        read_ref(v, cx, &[])
    }
    fn write(&self) -> Value {
        Value::Ref(self.clone())
    }
    fn pack(&self, p: &mut Packer) {
        p.put_str(self.kind.as_str());
        p.put_str(self.id.as_str());
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        let kind = KindName::new(u.get_str()?).ok()?;
        let id = RefKey::new(u.get_str()?).ok()?;
        Some(Ref::new(kind, id))
    }
}

/// A kind's name as text: checked when it's read.
impl Schema for KindName {
    fn shape() -> Shape {
        Shape::Atom
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        let ValueRef::Str(s) = v else {
            cx.expected(Expected::Name, v);
            return None;
        };
        KindName::new(s)
            .map_err(|e| cx.problem_of(Problem::BadTyped, e.to_string()))
            .ok()
    }
    fn write(&self) -> Value {
        Value::Str(self.as_str().to_owned())
    }
    fn pack(&self, p: &mut Packer) {
        self.atom().pack(p);
    }
    fn unpack(u: &mut Unpacker<'_>) -> Option<Self> {
        KindName::new(crate::Atom::unpack(u)?.as_str()).ok()
    }
}
