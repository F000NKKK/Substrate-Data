//! The schema of a `Result`: `{ ok }` or `{ err }`.

use crate::model::borrowed::ValueRef;
use crate::model::value::{Seg, Value};
use crate::schema::{Expected, Reader, Schema, Shape};

/// The keys of a [`Result`]'s two sides.
pub(crate) const RESULT_OK: &str = "ok";
pub(crate) const RESULT_ERR: &str = "err";

/// What may fail: `{ "ok": T }` or `{ "err": E }`.
impl<T: Schema, E: Schema> Schema for Result<T, E> {
    fn shape() -> Shape {
        Shape::Any
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Self> {
        let ValueRef::Map(m) = v else {
            cx.expected(Expected::OkOrErr, v);
            return None;
        };
        if let Some(ok) = m.get(RESULT_OK) {
            return cx
                .at(Seg::Key(RESULT_OK.into()), |cx| T::read(ok, cx))
                .map(Ok);
        }
        let Some(err) = m.get(RESULT_ERR) else {
            cx.expected(Expected::OkOrErr, v);
            return None;
        };
        cx.at(Seg::Key(RESULT_ERR.into()), |cx| E::read(err, cx))
            .map(Err)
    }
    fn write(&self) -> Value {
        let (key, value) = match self {
            Ok(t) => (RESULT_OK, t.write()),
            Err(e) => (RESULT_ERR, e.write()),
        };
        Value::Map([(key.to_owned(), value)].into_iter().collect())
    }
}
