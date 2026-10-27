//! What a problem says it found, and the known name a typo meant.

use crate::model::borrowed::ValueRef;

/// A found value as a problem shows it.
pub(super) fn found_text(v: &ValueRef<'_>) -> String {
    match v {
        ValueRef::Str(s) if s.chars().count() <= 40 => format!("{s:?}"),
        ValueRef::Int(n) => n.to_string(),
        ValueRef::Float(f) => f.to_string(),
        ValueRef::Bool(b) => b.to_string(),
        other => other.kind().to_string(),
    }
}

/// The `known` name closest to `key`, if it's close enough to be a typo
/// (`txt` → `text`, `img` → `image`, `colr` → `color`).
pub(super) fn closest<'k>(key: &str, known: &[&'k str]) -> Option<&'k str> {
    let lower = key.to_ascii_lowercase();
    known
        .iter()
        .map(|k| (edit_distance(&lower, k), *k))
        .filter(|(d, k)| *d <= if k.len() <= 3 { 1 } else { k.len() / 2 })
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

/// How many edits make `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != *cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}
