//! UTF-8 validation: the standard library's, or SIMD with `simd-utf8`.

use std::str::Utf8Error;

/// Validates `bytes` as UTF-8, with the standard library's error.
///
/// With the `simd-utf8` feature the check is vectorised
/// (runtime-dispatched); an invalid input is re-run through the standard
/// check, so the error is identical either way.
///
/// # Errors
///
/// [`Utf8Error`] when `bytes` isn't UTF-8.
#[cfg(feature = "simd-utf8")]
pub(crate) fn from_utf8(bytes: &[u8]) -> Result<&str, Utf8Error> {
    match simdutf8::basic::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(_) => std::str::from_utf8(bytes),
    }
}

/// Validates `bytes` as UTF-8, with the standard library's error.
///
/// # Errors
///
/// [`Utf8Error`] when `bytes` isn't UTF-8.
#[cfg(not(feature = "simd-utf8"))]
pub(crate) fn from_utf8(bytes: &[u8]) -> Result<&str, Utf8Error> {
    std::str::from_utf8(bytes)
}

#[cfg(test)]
mod tests {
    use super::from_utf8;

    /// Inputs around every UTF-8 boundary, valid and not.
    fn samples() -> Vec<Vec<u8>> {
        let mut v: Vec<Vec<u8>> = vec![
            vec![],
            b"plain ascii".to_vec(),
            "naïve — 日本語 😀".as_bytes().to_vec(),
            vec![0xff],
            vec![0xc0, 0x80],
            vec![0xe2, 0x82],
            vec![0xed, 0xa0, 0x80],
            vec![0xf4, 0x90, 0x80, 0x80],
            vec![b'a', 0x80],
        ];
        let long = "x".repeat(300) + "日本語";
        for cut in 0..long.len() {
            v.push(long.as_bytes()[..cut].to_vec());
        }
        let mut broken = long.into_bytes();
        for at in (0..broken.len()).step_by(7) {
            let saved = broken[at];
            broken[at] = 0xff;
            v.push(broken.clone());
            broken[at] = saved;
        }
        v
    }

    #[test]
    fn agrees_with_std_including_the_error() {
        for bytes in samples() {
            assert_eq!(from_utf8(&bytes), std::str::from_utf8(&bytes));
        }
    }
}
