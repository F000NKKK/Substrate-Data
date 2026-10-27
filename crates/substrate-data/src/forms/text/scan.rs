//! The parser's pure searches, vectorised by `memchr`: where a string's
//! plain run ends, where a line or a block comment ends.

use memchr::{memchr, memchr2, memmem};

/// Bytes a control-character check looks at at once.
const CHUNK: usize = 32;

/// Where a string's plain run ends in `bytes`: the first `"`, `\\` or
/// control byte (below 0x20), or `None` when there is none.
pub(super) fn string_stop(bytes: &[u8]) -> Option<usize> {
    let quote = memchr2(b'"', b'\\', bytes);
    let head = &bytes[..quote.unwrap_or(bytes.len())];
    control(head).or(quote)
}

/// The first control byte (below 0x20) of `bytes`, chunk by chunk so the
/// common "none in this chunk" answer is one vector compare.
fn control(bytes: &[u8]) -> Option<usize> {
    let (chunks, rest) = bytes.as_chunks::<CHUNK>();
    for (n, chunk) in chunks.iter().enumerate() {
        if chunk.iter().any(|&b| b < 0x20) {
            return chunk.iter().position(|&b| b < 0x20).map(|p| n * CHUNK + p);
        }
    }
    rest.iter()
        .position(|&b| b < 0x20)
        .map(|p| chunks.len() * CHUNK + p)
}

/// Where the line starting at `bytes` ends: its `\n`, or the end.
pub(super) fn line_end(bytes: &[u8]) -> usize {
    memchr(b'\n', bytes).unwrap_or(bytes.len())
}

/// Where the first `*/` starts in `bytes`.
pub(super) fn block_comment_end(bytes: &[u8]) -> Option<usize> {
    memmem::find(bytes, b"*/")
}

#[cfg(test)]
mod tests {
    use super::{block_comment_end, line_end, string_stop};

    /// The scalar scan the parser used before.
    fn string_stop_scalar(bytes: &[u8]) -> Option<usize> {
        bytes
            .iter()
            .position(|&b| b == b'"' || b == b'\\' || b < 0x20)
    }

    /// A small deterministic generator.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
            (0..len)
                .map(|_| alphabet[usize::try_from(self.next()).unwrap_or(0) % alphabet.len()])
                .collect()
        }
    }

    #[test]
    fn string_stop_matches_the_scalar_scan_on_generated_input() {
        let alphabets: [&[u8]; 4] = [
            b"abcdefgh ",
            b"ab\"\\\n\t\x00\x1f \x7f",
            &[b'a', 0xc3, 0xa9, 0xe6, 0x97, 0xa5, 0xff, 0x80, b'"'],
            &[b'a', b'b', 0x1f, 0x20, b'"', b'\\'],
        ];
        let mut rng = Lcg(7);
        for alphabet in alphabets {
            for len in (0..140).chain([255, 256, 257, 1000]) {
                for _ in 0..30 {
                    let bytes = rng.bytes(len, alphabet);
                    assert_eq!(string_stop(&bytes), string_stop_scalar(&bytes), "{bytes:?}");
                }
            }
        }
    }

    #[test]
    fn string_stop_edges() {
        for at in 0..100 {
            for stop in [b'"', b'\\', 0x00, 0x1f, b'\n'] {
                let mut bytes = vec![b'x'; 100];
                bytes[at] = stop;
                assert_eq!(string_stop(&bytes), Some(at));
                // A control byte before a quote wins, and the other way round.
                bytes[99] = b'"';
                assert_eq!(string_stop(&bytes), string_stop_scalar(&bytes));
            }
        }
        assert_eq!(string_stop(&[b'x'; 99]), None);
        assert_eq!(string_stop(b""), None);
        assert_eq!(string_stop("日本語 naïve".as_bytes()), None);
        assert_eq!(string_stop(&[b'x', 0x20, 0x7f, 0xff]), None);
        assert_eq!(string_stop("日\"".as_bytes()), Some(3));
    }

    #[test]
    fn line_end_is_the_newline_or_the_end() {
        assert_eq!(line_end(b"// a\nb"), 4);
        assert_eq!(line_end(b"// a"), 4);
        assert_eq!(line_end(b""), 0);
        assert_eq!(line_end(b"\n"), 0);
    }

    #[test]
    fn block_comment_end_matches_str_find() {
        let mut rng = Lcg(11);
        for len in 0..80 {
            for _ in 0..40 {
                let bytes = rng.bytes(len, b"*/ a\n");
                let text = String::from_utf8(bytes.clone()).unwrap_or_default();
                assert_eq!(block_comment_end(&bytes), text.find("*/"), "{text:?}");
            }
        }
        assert_eq!(block_comment_end(b" x **/"), Some(4));
        assert_eq!(block_comment_end(b"*"), None);
    }
}
