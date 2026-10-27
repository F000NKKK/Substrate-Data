//! [`patch`]: a document changed to hold a new value, the rest kept as it was.

use super::format::{INDENT, format, quoted, same};
use crate::model::value::{Map, Value};
use crate::text::Spans;

/// `text` changed to hold `next`, the rest as it was. Maps and lists are
/// edited entry by entry (a kept entry keeps the comments before it; a
/// removed one takes them along). Text that doesn't parse is written anew.
pub fn patch(text: &str, next: &Value) -> String {
    let Ok(doc) = crate::text::parse(text) else {
        return format!("{}\n", format(next, 0));
    };
    let (start, end) = range(&doc.spans);
    format!(
        "{}{}{}",
        &text[..start],
        rewrite(text, &doc.value, &doc.spans, next),
        &text[end..]
    )
}

fn range(s: &Spans) -> (usize, usize) {
    (s.span.start as usize, s.span.end as usize)
}

/// The indent of the line `at` is on.
fn line_indent(text: &str, at: usize) -> &str {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[start..at];
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

fn level_of(indent: &str) -> usize {
    indent
        .chars()
        .map(|c| if c == '\t' { INDENT.len() } else { 1 })
        .sum::<usize>()
        .div_ceil(INDENT.len())
}

/// Past whitespace and comments from `i`.
fn skip_blank(text: &str, mut i: usize) -> usize {
    let b = text.as_bytes();
    loop {
        match (b.get(i), b.get(i + 1)) {
            (Some(b' ' | b'\t' | b'\n' | b'\r'), _) => i += 1,
            (Some(b'/'), Some(b'/')) => i = text[i..].find('\n').map_or(text.len(), |n| i + n),
            (Some(b'/'), Some(b'*')) => {
                i = text[i + 2..]
                    .find("*/")
                    .map_or(text.len(), |n| i + 2 + n + 2);
            }
            _ => return i,
        }
    }
}

/// A member as rewritten: its text, then what followed it (a comment) — the comma goes between.
struct Member {
    body: String,
    trail: String,
}

fn join(out: &[Member], trailing_comma: bool) -> String {
    let mut s = String::new();
    for (i, m) in out.iter().enumerate() {
        s.push_str(&m.body);
        if i + 1 < out.len() || trailing_comma {
            s.push(',');
        }
        s.push_str(&m.trail);
    }
    s
}

/// A container's members as text: each one's lead (the comments above it)
/// and trail (after it, up to its comma); whether the last has a comma;
/// what's left before the bracket.
struct Members {
    segments: Vec<(String, String)>,
    comma: bool,
    tail: String,
}

fn members(text: &str, node: (usize, usize), spans: &[(usize, usize)]) -> Members {
    let mut segments = Vec::with_capacity(spans.len());
    let mut at = node.0 + 1;
    let mut comma = false;
    for &(start, end) in spans {
        let lead = text[at..start].to_string();
        let after = skip_blank(text, end);
        comma = text.as_bytes().get(after) == Some(&b',');
        // A comment on the member's own line stays with it; the rest goes to the next one.
        let eol = text[end..].find('\n').map(|n| end + n);
        let stop = if comma {
            after
        } else {
            eol.map_or(after, |e| after.min(e))
        };
        let mut trail = text[end..stop].to_string();
        at = if comma { after + 1 } else { stop };
        if comma {
            let line_end = text[at..].find('\n').map_or(text.len(), |n| at + n);
            let rest = &text[at..line_end];
            let t = rest.trim();
            let comment = t.starts_with("//")
                || (t.starts_with("/*") && t.ends_with("*/") && t.matches("*/").count() == 1);
            if comment && line_end < node.1 {
                trail.push_str(rest);
                at = line_end;
            }
        }
        segments.push((lead, trail));
    }
    Members {
        segments,
        comma,
        tail: text[at..node.1 - 1].to_string(),
    }
}

/// The new text of the value `old` (at `spans`) holding `next`.
fn rewrite(text: &str, old: &Value, spans: &Spans, next: &Value) -> String {
    let node = range(spans);
    if same(old, next) {
        return text[node.0..node.1].to_string();
    }
    let level = level_of(line_indent(text, node.0));
    let fresh = || format(next, level);
    if !text[node.0..node.1].contains('\n') {
        return fresh();
    }
    match (old, next) {
        (Value::Map(old), Value::Map(next))
            if !old.is_empty() && spans.children.len() == old.len() =>
        {
            rewrite_map(text, node, old, spans, next)
        }
        (Value::List(old), Value::List(next))
            if !old.is_empty() && spans.children.len() == old.len() =>
        {
            rewrite_list(text, node, old, spans, next)
        }
        _ => fresh(),
    }
}

fn rewrite_map(text: &str, node: (usize, usize), old: &Map, spans: &Spans, next: &Map) -> String {
    let keys: Vec<&str> = old.keys().collect();
    let key_start = |c: &Spans| c.key.map_or(c.span.start, |k| k.start) as usize;
    let ranges: Vec<(usize, usize)> = spans
        .children
        .iter()
        .map(|c| (key_start(c), c.span.end as usize))
        .collect();
    let parts = members(text, node, &ranges);
    let pad = line_indent(text, ranges[0].0);
    let gone = |i: usize| i < keys.len() && next.get(keys[i]).is_none();
    let mut out = Vec::with_capacity(next.len());
    // The old entry last written: a new key right after it, in the place of one that's gone, is that one renamed.
    let mut last: Option<usize> = None;
    for (k, v) in next.iter() {
        let after = last.map_or(0, |l| l + 1);
        let found = keys.iter().position(|o| *o == k);
        let renamed = found.is_none() && gone(after);
        let Some(i) = found.or(renamed.then_some(after)) else {
            out.push(Member {
                body: format!("\n{pad}{}: {}", quoted(k), format(v, level_of(pad))),
                trail: String::new(),
            });
            continue;
        };
        last = Some(i);
        let child = &spans.children[i];
        let value_start = child.span.start as usize;
        let key = if renamed {
            let key_end = child.key.map_or(value_start, |s| s.end as usize);
            format!("{}{}", quoted(k), &text[key_end..value_start])
        } else {
            text[ranges[i].0..value_start].to_string()
        };
        let old_value = old.value_at(i).unwrap_or(&Value::Null);
        let (lead, trail) = &parts.segments[i];
        out.push(Member {
            body: format!("{lead}{key}{}", rewrite(text, old_value, child, v)),
            trail: trail.clone(),
        });
    }
    format!("{{{}{}}}", join(&out, parts.comma), parts.tail)
}

fn rewrite_list(
    text: &str,
    node: (usize, usize),
    old: &[Value],
    spans: &Spans,
    next: &[Value],
) -> String {
    let ranges: Vec<(usize, usize)> = spans.children.iter().map(range).collect();
    let parts = members(text, node, &ranges);
    let pad = line_indent(text, ranges[0].0);
    let item = |i: usize, v: &Value| {
        let (lead, trail) = &parts.segments[i];
        Member {
            body: format!("{lead}{}", rewrite(text, &old[i], &spans.children[i], v)),
            trail: trail.clone(),
        }
    };
    let out: Vec<Member> = if next.len() == old.len() {
        next.iter().enumerate().map(|(i, v)| item(i, v)).collect()
    } else {
        // Added or removed items: the others keep their text.
        let mut from = 0;
        next.iter()
            .map(|v| match (from..old.len()).find(|&k| same(&old[k], v)) {
                Some(i) => {
                    from = i + 1;
                    item(i, v)
                }
                None => Member {
                    body: format!("\n{pad}{}", format(v, level_of(pad))),
                    trail: String::new(),
                },
            })
            .collect()
    };
    format!("[{}{}]", join(&out, parts.comma), parts.tail)
}
