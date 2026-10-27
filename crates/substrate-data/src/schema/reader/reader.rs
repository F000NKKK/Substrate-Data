//! [`Reader`]: the state of reading a value against a schema.

use super::Found;
use super::keys::is_metadata_key;
use super::suggest::{closest, found_text};
use crate::Diagnostic;
use crate::diagnostic::Problem;
use crate::model::borrowed::{MapRef, ValueRef};
use crate::model::value::{Path, Ref, RefKey, Seg};
use crate::schema::{Expected, Resolve, Schema};

/// The state of reading a value: where it is, the problems so far, and the
/// references met (for the build to check against the content there is).
#[derive(Default)]
pub struct Reader<'r> {
    /// Where it is, as steps: a field's name stays `&'static` until a
    /// problem or a reference needs the whole [`Path`].
    pub(crate) path: Vec<Step>,
    /// Don't keep the references met (who only wants the value).
    pub(crate) value_only: bool,
    /// What the tool makes of references (none: kept as written).
    pub(crate) resolver: Option<&'r mut dyn Resolve>,
    /// The problems found so far.
    pub problems: Vec<Diagnostic>,
    /// The references met so far (unless it only reads values).
    pub refs: Vec<Found<Ref>>,
    /// Keys a struct being read doesn't report as unknown (an enum's tag).
    pub(crate) allowed: Vec<&'static str>,
    /// How many expanded values are being read inside one another.
    pub(crate) expanding: usize,
}

impl std::fmt::Debug for Reader<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reader")
            .field("path", &self.path())
            .field("value_only", &self.value_only)
            .field("resolves", &self.resolver.is_some())
            .field("problems", &self.problems)
            .field("refs", &self.refs)
            .finish_non_exhaustive()
    }
}

/// A step of [`Reader`]'s path.
pub(crate) enum Step {
    Seg(Seg),
    Field(&'static str),
}

impl<'r> Reader<'r> {
    /// A reader at the root, keeping the references it meets.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A reader handing references to `resolver`.
    pub fn with_resolver(resolver: &'r mut dyn Resolve) -> Self {
        Self {
            resolver: Some(resolver),
            ..Self::default()
        }
    }

    /// `r` resolved (a problem if it's wrong); its id without a resolver.
    pub fn resolve_ref(&mut self, r: &Ref) -> Option<RefKey> {
        let result = match &mut self.resolver {
            Some(res) => res.reference(r, &Self::built(&self.path)),
            None => Ok(r.id.clone()),
        };
        result
            .map_err(|e| self.problem_of(Problem::Unresolved, e.to_string()))
            .ok()
    }

    /// [`Reader::resolve_ref`], taking `r`: without a resolver the id moves.
    pub fn take_ref(&mut self, r: Ref) -> Option<RefKey> {
        if self.resolver.is_none() {
            return Some(r.id);
        }
        self.resolve_ref(&r)
    }

    /// Whether references are kept for the caller ([`read_value`](crate::schema::read_value) doesn't).
    #[must_use]
    pub fn keeps_found(&self) -> bool {
        !self.value_only
    }

    /// Whether references go through a resolver (else kept as written).
    #[must_use]
    pub fn resolves(&self) -> bool {
        self.resolver.is_some()
    }

    /// Where the reader is.
    #[must_use]
    pub fn path(&self) -> Path {
        Self::built(&self.path)
    }

    fn built(steps: &[Step]) -> Path {
        let mut p = Path::root();
        for s in steps {
            p.push(match s {
                Step::Seg(s) => s.clone(),
                Step::Field(k) => Seg::Key((*k).to_string()),
            });
        }
        p
    }

    /// Reports a problem with the value being read.
    pub fn problem(&mut self, message: impl Into<String>) {
        self.problem_of(Problem::Invalid, message);
    }

    /// Reports a `problem` with the value being read.
    pub fn problem_of(&mut self, problem: Problem, message: impl Into<String>) {
        let found = Diagnostic::error(problem, message.into()).at_path(self.path());
        self.problems.push(found);
    }

    /// "expected X, found Y".
    pub fn expected(&mut self, what: Expected, found: &ValueRef<'_>) {
        self.problem_of(
            Problem::Expected,
            format!("expected {what}, found {}", found_text(found)),
        );
    }

    /// Runs `f` one step deeper.
    pub fn at<T>(&mut self, seg: Seg, f: impl FnOnce(&mut Self) -> T) -> T {
        self.path.push(Step::Seg(seg));
        let r = f(self);
        self.path.pop();
        r
    }

    /// Runs `f` in field `key` (no allocation for the step).
    fn in_field<T>(&mut self, key: &'static str, f: impl FnOnce(&mut Self) -> T) -> T {
        self.path.push(Step::Field(key));
        let r = f(self);
        self.path.pop();
        r
    }

    /// Keeps `r`, met at the current path (unless it only reads values).
    pub fn note_ref(&mut self, r: Ref) {
        if self.value_only {
            return;
        }
        let path = self.path();
        self.refs.push(Found { value: r, path });
    }

    /// `v` as an object, or a problem.
    pub fn map<'v>(&mut self, v: &ValueRef<'v>) -> Option<MapRef<'v>> {
        match v {
            ValueRef::Map(m) => Some(*m),
            other => {
                self.expected(Expected::Object, other);
                None
            }
        }
    }

    /// Reports keys of `map` that aren't `known` (nor metadata, nor allowed by an enclosing enum).
    pub fn unknown_keys(&mut self, map: MapRef<'_>, known: &[&str]) {
        for key in map.keys() {
            if known.contains(&key) || is_metadata_key(key) || self.allowed.contains(&key) {
                continue;
            }
            let hint = closest(key, known).map_or_else(
                || {
                    if known.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " (expected {})",
                            known
                                .iter()
                                .map(|k| format!("`{k}`"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                },
                |k| format!(" — did you mean `{k}`?"),
            );
            let mut found = Diagnostic::error(
                Problem::UnknownField,
                format!("unknown field `{key}`{hint}"),
            )
            .at_path(self.path().key(key));
            found.on_key = true;
            self.problems.push(found);
        }
    }

    /// Reports a name that isn't one of `known` (an enum's variant).
    pub fn unknown_name(&mut self, name: &str, known: &[&str]) {
        let hint = match closest(name, known) {
            Some(k) => format!(" — did you mean `{k}`?"),
            None => format!(
                " (expected {})",
                known
                    .iter()
                    .map(|k| format!("`{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        self.problem(format!("unknown `{name}`{hint}"));
    }

    /// Field `key` of `map`: read, or its absent value, or a problem.
    pub fn field<T: Schema>(&mut self, map: MapRef<'_>, key: &'static str) -> Option<T> {
        match map.get(key) {
            Some(v) => self.at(Seg::Key(key.to_string()), |cx| T::read(v, cx)),
            None => T::absent().or_else(|| {
                self.problem_of(Problem::MissingField, format!("missing `{key}`"));
                None
            }),
        }
    }

    /// Field `key` of `map`, `default()` when it isn't there.
    pub fn field_or<T: Schema>(
        &mut self,
        map: MapRef<'_>,
        key: &'static str,
        default: impl FnOnce() -> T,
    ) -> Option<T> {
        match map.get(key) {
            Some(v) => self.at(Seg::Key(key.to_string()), |cx| T::read(v, cx)),
            None => Some(default()),
        }
    }

    /// Field `key` of `map` through codec `W`.
    pub fn field_with<W: crate::with::With<F>, F>(
        &mut self,
        map: MapRef<'_>,
        key: &'static str,
    ) -> Option<F> {
        match map.get(key) {
            Some(v) => self.in_field(key, |cx| W::read(v, cx)),
            None => W::absent().or_else(|| {
                self.problem_of(Problem::MissingField, format!("missing `{key}`"));
                None
            }),
        }
    }

    /// Field `key` of `map` through codec `W`, `default()` when it isn't there.
    pub fn field_or_with<W: crate::with::With<F>, F>(
        &mut self,
        map: MapRef<'_>,
        key: &'static str,
        default: impl FnOnce() -> F,
    ) -> Option<F> {
        match map.get(key) {
            Some(v) => self.in_field(key, |cx| W::read(v, cx)),
            None => Some(default()),
        }
    }

    /// Reads `v` as `T` with `key` allowed in it — the tag of the enum variant `T` is.
    pub fn with_allowed<T>(&mut self, key: &'static str, f: impl FnOnce(&mut Self) -> T) -> T {
        self.allowed.push(key);
        let r = f(self);
        self.allowed.pop();
        r
    }

    /// Reads with every one of `keys` allowed (the fields of a struct read
    /// from the same object — see [`read_both`](crate::schema::read_both)).
    pub fn with_allowed_all<T>(
        &mut self,
        keys: &[&'static str],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let before = self.allowed.len();
        self.allowed.extend_from_slice(keys);
        let r = f(self);
        self.allowed.truncate(before);
        r
    }

    /// Reads a struct's own keys with no allowance inherited from outside.
    pub fn own_keys<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::take(&mut self.allowed);
        let r = f(self);
        self.allowed = saved;
        r
    }
}
