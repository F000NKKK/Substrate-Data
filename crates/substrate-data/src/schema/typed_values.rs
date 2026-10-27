//! The schemas of typed values: references, by kind.
use super::{PhantomData, Reader, Ref, Schema, Shape, Value, ValueRef, fmt, typed};

// ── Typed values ────────────────────────────────────────────────────────────

/// A kind of thing a reference names: `user`, `doc`, `file`, ... — a
/// format declares its own with [`kind!`](crate::kind).
pub trait Kind {
    const NAME: &'static str;
}

/// One kind or several (`(Item, Block)`): what a reference field takes.
pub trait Kinds {
    const NAMES: &'static [&'static str];
}

impl<K: Kind> Kinds for K {
    const NAMES: &'static [&'static str] = &[K::NAME];
}

impl<A: Kind, B: Kind> Kinds for (A, B) {
    const NAMES: &'static [&'static str] = &[A::NAME, B::NAME];
}

impl<A: Kind, B: Kind, C: Kind> Kinds for (A, B, C) {
    const NAMES: &'static [&'static str] = &[A::NAME, B::NAME, C::NAME];
}

/// Declares kinds: `substrate_data::kind!(User = "user", Doc = "doc");`.
#[macro_export]
macro_rules! kind {
    ($($name:ident = $kind:expr),* $(,)?) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name;
        impl $crate::Kind for $name {
            const NAME: &'static str = $kind;
        }
    )*};
}

/// The id of something of kind `K`: written `@file:images/logo.png`.
pub struct Id<K> {
    pub id: String,
    kind: PhantomData<K>,
}

impl<K> Id<K> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: PhantomData,
        }
    }
}

impl<K> Clone for Id<K> {
    fn clone(&self) -> Self {
        Self::new(self.id.clone())
    }
}
impl<K> PartialEq for Id<K> {
    fn eq(&self, o: &Self) -> bool {
        self.id == o.id
    }
}
impl<K> Eq for Id<K> {}
impl<K> std::hash::Hash for Id<K> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}
impl<K: Kind> fmt::Debug for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}:{}", K::NAME, self.id)
    }
}
impl<K> fmt::Display for Id<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id)
    }
}

/// `v` as a reference to one of `kinds` (any kind if empty),
/// written `@kind:id` — a bare id is an error that says how to write it.
pub fn read_ref(v: &ValueRef, cx: &mut Reader, kinds: &[&str]) -> Option<Ref> {
    let wanted = || {
        kinds
            .iter()
            .map(|k| format!("`@{k}:`"))
            .collect::<Vec<_>>()
            .join(" or ")
    };
    let r = match v {
        ValueRef::Ref { kind, id } => Ref::new(*kind, *id),
        ValueRef::Str(s) if s.starts_with('@') => match typed::classify(s) {
            Ok(Value::Ref(r)) => r,
            Ok(other) => {
                cx.problem(format!(
                    "expected a reference ({}), found {}",
                    if kinds.is_empty() {
                        "`@kind:id`".into()
                    } else {
                        wanted()
                    },
                    other.kind_name()
                ));
                return None;
            }
            Err(e) => {
                cx.problem_of(crate::names::problem::BAD_TYPED, e);
                return None;
            }
        },
        ValueRef::Str(s) => {
            let kind = kinds.first().copied().unwrap_or("kind");
            cx.problem_of(
                crate::names::problem::NOT_A_REFERENCE,
                format!("`{s}`: name it by a reference — `@{kind}:{s}`"),
            );
            return None;
        }
        other => {
            cx.expected("a reference `@kind:id`", other);
            return None;
        }
    };
    if !kinds.is_empty() && !kinds.contains(&r.kind.as_str()) {
        cx.problem_of(
            crate::names::problem::WRONG_KIND,
            format!(
                "`@{}:{}` is a reference to {} — this takes {}",
                r.kind,
                r.id,
                r.kind,
                wanted()
            ),
        );
        return None;
    }
    cx.note_ref(r.clone());
    Some(r)
}

impl<K: Kind> Schema for Id<K> {
    fn shape() -> Shape {
        Shape::Ref(<K as Kinds>::NAMES)
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        read_ref(v, cx, <K as Kinds>::NAMES).map(|r| Id::new(r.id))
    }
    fn write(&self) -> Value {
        Value::Ref(Ref::new(K::NAME, self.id.clone()))
    }
}

impl Schema for Ref {
    fn shape() -> Shape {
        Shape::Ref(&[])
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        if let ValueRef::Str(s) = v
            && !s.starts_with('@')
        {
            cx.problem(format!("expected a reference `@kind:id`, found {s:?}"));
            return None;
        }
        read_ref(v, cx, &[])
    }
    fn write(&self) -> Value {
        Value::Ref(self.clone())
    }
}
