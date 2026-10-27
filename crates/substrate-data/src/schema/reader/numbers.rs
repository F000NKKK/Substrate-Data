//! [`Numbers`]: the numbers a value holds, checked against a field's range.

use super::Reader;
use crate::diagnostic::Problem;
use crate::model::number::whole_to_i64;
use crate::model::value::Seg;

/// The numbers a value holds — what `#[data(range = …)]` checks.
pub trait Numbers {
    /// Puts its numbers on `out`.
    fn numbers(&self, out: &mut Vec<f64>);
}

macro_rules! numbers {
    ($($t:ty),*) => {$(
        impl Numbers for $t {
            fn numbers(&self, out: &mut Vec<f64>) {
                out.push(f64::from(*self));
            }
        }
    )*};
}
numbers!(i8, i16, i32, u8, u16, u32, f32, f64);

macro_rules! wide_numbers {
    ($($t:ty),*) => {$(
        impl Numbers for $t {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a range is checked on the nearest float"
            )]
            fn numbers(&self, out: &mut Vec<f64>) {
                out.push(*self as f64);
            }
        }
    )*};
}
wide_numbers!(i64, u64, usize);

impl<T: Numbers> Numbers for Option<T> {
    fn numbers(&self, out: &mut Vec<f64>) {
        if let Some(x) = self {
            x.numbers(out);
        }
    }
}

impl<T: Numbers> Numbers for Vec<T> {
    fn numbers(&self, out: &mut Vec<f64>) {
        for x in self {
            x.numbers(out);
        }
    }
}

impl<T: Numbers, const N: usize> Numbers for [T; N] {
    fn numbers(&self, out: &mut Vec<f64>) {
        for x in self {
            x.numbers(out);
        }
    }
}

impl Reader<'_> {
    /// `value` (field `key`) checked against `lo..=hi`; `None` (a problem) if it's out.
    pub fn in_range<T: Numbers>(
        &mut self,
        key: &str,
        value: Option<T>,
        lo: f64,
        hi: f64,
    ) -> Option<T> {
        let v = value?;
        let mut ns = Vec::new();
        v.numbers(&mut ns);
        match ns.into_iter().find(|n| !(lo..=hi).contains(n)) {
            None => Some(v),
            Some(n) => {
                let show = |x: f64| {
                    whole_to_i64(x).map_or_else(|| x.to_string(), |whole| whole.to_string())
                };
                let bad = show(n);
                self.at(Seg::Key(key.to_string()), |cx| {
                    cx.problem_of(
                        Problem::OutOfRange,
                        format!("{bad} is out of range ({}..={})", show(lo), show(hi)),
                    );
                });
                None
            }
        }
    }
}
