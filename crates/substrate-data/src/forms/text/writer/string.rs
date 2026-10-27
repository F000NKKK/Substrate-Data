//! A string as a JSON string literal.

use super::super::scan;
use super::sink::Sink;

/// `s` as a JSON string literal.
pub fn write_string(s: &str, out: &mut String) {
    push_string(s, out);
}

/// `s` as a JSON string literal, into `out`.
pub(super) fn push_string<S: Sink>(s: &str, out: &mut S) {
    out.text("\"");
    write_body(s, out);
    out.text("\"");
}

/// `s` as the inside of a JSON string literal, into `out`.
pub(super) fn write_body<S: Sink>(s: &str, out: &mut S) {
    let bytes = s.as_bytes();
    let mut run = 0;
    while let Some(n) = scan::string_stop(&bytes[run..]) {
        let i = run + n;
        out.text(&s[run..i]);
        escape(bytes[i], out);
        run = i + 1;
    }
    out.text(&s[run..]);
}

/// The escape of `b`, a byte `needs_escape` is true of.
fn escape<S: Sink>(b: u8, out: &mut S) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    match b {
        b'"' => out.text("\\\""),
        b'\\' => out.text("\\\\"),
        b'\n' => out.text("\\n"),
        b'\r' => out.text("\\r"),
        b'\t' => out.text("\\t"),
        b => {
            let digits = [HEX[usize::from(b >> 4)], HEX[usize::from(b & 0xf)]];
            out.text("\\u00");
            out.text(std::str::from_utf8(&digits).unwrap_or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{push_string, write_string};

    fn text(s: &str) -> String {
        let mut out = String::new();
        write_string(s, &mut out);
        out
    }

    #[test]
    fn escapes_what_json_does() {
        assert_eq!(text("plain"), "\"plain\"");
        assert_eq!(text("a\"b\\c\nd\re\tf"), r#""a\"b\\c\nd\re\tf""#);
        assert_eq!(text("\u{0}\u{1f}\u{8}"), r#""\u0000\u001f\u0008""#);
        assert_eq!(text("日本語 naïve"), "\"日本語 naïve\"");
    }

    #[test]
    fn short_and_long_strings_escape_alike() {
        for len in [0, 1, 15, 16, 17, 40, 100] {
            for at in 0..len {
                let mut s = "x".repeat(len);
                s.replace_range(at..=at, "\"");
                let want = format!("\"{}\\\"{}\"", "x".repeat(at), "x".repeat(len - at - 1));
                assert_eq!(text(&s), want);
                let mut bytes = Vec::new();
                push_string(&s, &mut bytes);
                assert_eq!(bytes, want.as_bytes());
            }
        }
    }
}
