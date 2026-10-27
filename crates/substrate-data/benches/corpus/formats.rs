//! The formats a typed document is encoded in: this crate's three and the
//! serde ones it is measured against.

use std::fmt;

use super::typed::Typed;

/// A format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fmt {
    /// This crate's binary form (`to_bytes`).
    SdBinary,
    /// This crate's frame (`to_frame`).
    SdFrame,
    /// This crate's text (`to_text`).
    SdText,
    /// JSON (`serde_json`).
    Json,
    /// CBOR (`ciborium`).
    Cbor,
    /// `MessagePack` with field names (`rmp-serde`, `to_vec_named`).
    MsgpackNamed,
    /// `MessagePack` with structs as arrays (`rmp-serde`, `to_vec`).
    Msgpack,
    /// `postcard`.
    Postcard,
    /// `bincode` 2 with its standard configuration.
    Bincode,
}

impl Fmt {
    /// Every format.
    pub(crate) const ALL: [Fmt; 9] = [
        Fmt::SdBinary,
        Fmt::SdFrame,
        Fmt::SdText,
        Fmt::Json,
        Fmt::Cbor,
        Fmt::MsgpackNamed,
        Fmt::Msgpack,
        Fmt::Postcard,
        Fmt::Bincode,
    ];

    /// `value` in this format.
    pub(crate) fn encode<T: Typed>(self, value: &T) -> Vec<u8> {
        match self {
            Fmt::SdBinary => substrate_data::to_bytes(value),
            Fmt::SdFrame => substrate_data::frame::to_frame(value),
            Fmt::SdText => substrate_data::to_text(value).into_bytes(),
            Fmt::Json => serde_json::to_vec(value).expect("JSON writes the corpus"),
            Fmt::Cbor => {
                let mut out = Vec::new();
                ciborium::into_writer(value, &mut out).expect("CBOR writes the corpus");
                out
            }
            Fmt::MsgpackNamed => {
                rmp_serde::to_vec_named(value).expect("MessagePack writes the corpus")
            }
            Fmt::Msgpack => rmp_serde::to_vec(value).expect("MessagePack writes the corpus"),
            Fmt::Postcard => postcard::to_stdvec(value).expect("postcard writes the corpus"),
            Fmt::Bincode => bincode::serde::encode_to_vec(value, bincode::config::standard())
                .expect("bincode writes the corpus"),
        }
    }

    /// The document `bytes` hold, if they are one.
    pub(crate) fn decode<T: Typed>(self, bytes: &[u8]) -> Option<T> {
        match self {
            Fmt::SdBinary => substrate_data::read_bytes(bytes).ok(),
            Fmt::SdFrame => substrate_data::frame::read_frame(bytes),
            Fmt::SdText => {
                let text = std::str::from_utf8(bytes).ok()?;
                substrate_data::from_text(text).ok().map(|r| r.value)
            }
            Fmt::Json => serde_json::from_slice(bytes).ok(),
            Fmt::Cbor => ciborium::from_reader(bytes).ok(),
            Fmt::MsgpackNamed | Fmt::Msgpack => rmp_serde::from_slice(bytes).ok(),
            Fmt::Postcard => postcard::from_bytes(bytes).ok(),
            Fmt::Bincode => bincode::serde::decode_from_slice(bytes, bincode::config::standard())
                .ok()
                .map(|(v, _)| v),
        }
    }
}

impl fmt::Display for Fmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Fmt::SdBinary => "sd_binary",
            Fmt::SdFrame => "sd_frame",
            Fmt::SdText => "sd_text",
            Fmt::Json => "json",
            Fmt::Cbor => "cbor",
            Fmt::MsgpackNamed => "msgpack_named",
            Fmt::Msgpack => "msgpack",
            Fmt::Postcard => "postcard",
            Fmt::Bincode => "bincode",
        })
    }
}

/// Asserts that `bytes`, `value` in format `f`, read back as `value` (`name`
/// is for the message). `serde_json` without its `float_roundtrip` feature
/// can be one bit off on an `f64`, so only that it reads is asserted of it.
pub(crate) fn check<T: Typed>(f: Fmt, value: &T, bytes: &[u8], name: &str) {
    let back = f
        .decode::<T>(bytes)
        .expect("the format reads what it wrote");
    if f != Fmt::Json {
        assert_eq!(&back, value, "{f} on {name}");
    }
}
