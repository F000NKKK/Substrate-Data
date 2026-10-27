//! [`Sink`]: where the writer's text goes.

/// Where text is written: a `String`, or the bytes of one being built.
pub(super) trait Sink {
    fn text(&mut self, s: &str);
}

impl Sink for String {
    fn text(&mut self, s: &str) {
        self.push_str(s);
    }
}

impl Sink for Vec<u8> {
    fn text(&mut self, s: &str) {
        self.extend_from_slice(s.as_bytes());
    }
}
