//! Bytes as lower-case hex: how the manifest spells them.

/// `bytes` as lower-case hex, no separators.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02x}").expect("writing to a String");
        out
    })
}

/// The bytes `hex` spells; `None` for odd length or a non-hex digit.
pub(crate) fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || !hex.is_ascii() {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}
