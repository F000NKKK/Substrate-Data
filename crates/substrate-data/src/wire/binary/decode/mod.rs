//! Reading the binary form: the decoder in its three shapes and its error.

mod decoder;
mod decoder_ref;
mod decoder_reference;
mod decoder_straight;
mod decoder_value;
mod error;
mod shape;

pub use decoder::Decoder;
pub use error::DecodeError;
