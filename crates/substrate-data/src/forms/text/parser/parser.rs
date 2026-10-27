//! [`Parser`]: values, lists, maps and keys; its strings and numbers are in
//! their own files.

use std::borrow::Cow;
use std::collections::HashSet;

use super::build::Build;
use super::entry::MAX_DEPTH;
use super::build::Failure;
use crate::forms::text::{LineCol, Span, Spans, SyntaxError, scan};
use crate::model::quick_hash::QuickBuild;

/// Maps up to this many keys are checked for a repeat by a scan; past it, a set.
const SCAN_MAX: usize = 16;

/// `B` makes the values; `SPANS`: whether to record where each value is.
#[derive(Debug)]
pub(super) struct Parser<'t, B, const SPANS: bool> {
    pub(super) src: &'t [u8],
    pub(super) text: &'t str,
    pub(super) i: usize,
    depth: usize,
    pub(super) build: B,
}

impl<'t, B: Build<'t>, const SPANS: bool> Parser<'t, B, SPANS> {
    pub(super) fn new(text: &'t str, build: B) -> Self {
        Self {
            src: text.as_bytes(),
            text,
            i: 0,
            depth: 0,
            build,
        }
    }

    pub(super) fn skip(&mut self, n: usize) {
        self.i += n;
    }

    pub(super) fn at_end(&self) -> bool {
        self.i >= self.src.len()
    }

    pub(super) fn error(&self, message: impl Into<String>) -> Failure {
        self.error_at(self.i, message)
    }

    pub(super) fn error_at(&self, offset: usize, message: impl Into<String>) -> Failure {
        let mut message = message.into();
        if offset >= self.src.len() {
            message.push_str(", found the end of the file");
        }
        Box::new(SyntaxError {
            message,
            at: LineCol::of(self.text, offset),
            offset,
        })
    }

    pub(super) fn peek(&self) -> Option<u8> {
        self.src.get(self.i).copied()
    }

    /// Skips spaces, tabs and line breaks; the runs of spaces that
    /// indentation is, eight at a time.
    fn skip_blanks(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' => {
                    self.i += 1;
                    while self.src[self.i..].first_chunk() == Some(&[b' '; 8]) {
                        self.i += 8;
                    }
                }
                b'\t' | b'\n' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }

    /// Skips whitespace and comments.
    pub(super) fn ws(&mut self) -> Result<(), Failure> {
        loop {
            self.skip_blanks();
            if self.peek() != Some(b'/') {
                return Ok(());
            }
            match self.src.get(self.i + 1) {
                Some(b'/') => {
                    self.i += scan::line_end(&self.src[self.i..]);
                }
                Some(b'*') => {
                    let start = self.i;
                    match scan::block_comment_end(&self.src[self.i + 2..]) {
                        Some(end) => self.i += 2 + end + 2,
                        None => return Err(self.error_at(start, "unclosed `/*` comment")),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// A value; its spans go to `spans` when they're being kept, `key` the
    /// span of its key if it's a map entry's.
    pub(super) fn value(
        &mut self,
        key: Option<Span>,
        spans: &mut Vec<Spans>,
    ) -> Result<B::Value, Failure> {
        let start = self.i;
        let mut children = Vec::new();
        let value = match self.peek() {
            Some(b'{') => self.nested(|p| p.map(&mut children))?,
            Some(b'[') => self.nested(|p| p.list(&mut children))?,
            Some(b'"') => {
                let s = self.string()?;
                self.build.string(s)
            }
            Some(b'-' | b'0'..=b'9') => self.number()?,
            Some(b't') if self.src[self.i..].starts_with(b"true") => {
                self.i += 4;
                self.build.boolean(true)
            }
            Some(b'f') if self.src[self.i..].starts_with(b"false") => {
                self.i += 5;
                self.build.boolean(false)
            }
            Some(b'n') if self.src[self.i..].starts_with(b"null") => {
                self.i += 4;
                self.build.null()
            }
            Some(b'\'') => return Err(self.error("strings use double quotes")),
            _ => return Err(self.error("expected a value")),
        };
        if SPANS {
            spans.push(Spans {
                span: Span::new(start, self.i),
                key,
                children,
            });
        }
        Ok(value)
    }

    fn nested(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<B::Value, Failure>,
    ) -> Result<B::Value, Failure> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error(format!("nested deeper than {MAX_DEPTH} levels")));
        }
        let r = f(self);
        self.depth -= 1;
        r
    }

    fn list(&mut self, spans: &mut Vec<Spans>) -> Result<B::Value, Failure> {
        self.i += 1;
        let mark = self.build.list_mark();
        loop {
            self.ws()?;
            if self.peek() == Some(b']') {
                self.i += 1;
                break;
            }
            let v = self.value(None, spans)?;
            self.build.push_item(v);
            self.ws()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.error("expected `,` or `]`")),
            }
        }
        Ok(self.build.end_list(mark))
    }

    fn map(&mut self, spans: &mut Vec<Spans>) -> Result<B::Value, Failure> {
        self.i += 1;
        let mark = self.build.map_mark();
        let mut keys = Keys::default();
        loop {
            self.ws()?;
            if self.peek() == Some(b'}') {
                self.i += 1;
                break;
            }
            let key_start = self.i;
            let key = self.key()?;
            let key_span = Span::new(key_start, self.i);
            if keys.repeats(&self.build, mark, &key) {
                return Err(
                    self.error_at(key_start, format!("`{key}` is already set in this object"))
                );
            }
            self.ws()?;
            if self.peek() != Some(b':') {
                return Err(self.error("expected `:` after the key"));
            }
            self.i += 1;
            self.ws()?;
            let v = self.value(Some(key_span), spans)?;
            self.build.push_entry(key, v);
            self.ws()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.error("expected `,` or `}`")),
            }
        }
        Ok(self.build.end_map(mark))
    }

    /// An object's key: a string, or a bare name.
    fn key(&mut self) -> Result<Cow<'t, str>, Failure> {
        let start = self.i;
        match self.peek() {
            Some(b'"') => self.string(),
            Some(first) if first.is_ascii_alphabetic() || first == b'_' || first == b'$' => {
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'$')
                {
                    self.i += 1;
                }
                Ok(Cow::Borrowed(&self.text[start..self.i]))
            }
            _ => Err(self.error("expected a key")),
        }
    }
}

/// The keys of the map being read, for refusing a repeat: the builder's
/// scan while it's small, a set once it grows.
#[derive(Default)]
struct Keys<'t> {
    count: usize,
    set: Option<HashSet<Cow<'t, str>, QuickBuild>>,
}

impl<'t> Keys<'t> {
    /// Whether the map begun at `mark` has `key` already; if not, notes it.
    fn repeats<B: Build<'t>>(&mut self, build: &B, mark: usize, key: &Cow<'t, str>) -> bool {
        self.count += 1;
        if let Some(set) = &mut self.set {
            return !set.insert(key.clone());
        }
        if build.has_key(mark, key) {
            return true;
        }
        if self.count > SCAN_MAX {
            let mut set = HashSet::with_hasher(QuickBuild::default());
            build.each_key(mark, &mut |k| {
                set.insert(Cow::Owned(k.to_owned()));
            });
            set.insert(key.clone());
            self.set = Some(set);
        }
        false
    }
}
