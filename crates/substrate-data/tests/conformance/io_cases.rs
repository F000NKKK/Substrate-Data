//! Cases for the length-prefixed frame stream.

use substrate_data::io::write_frame_bytes;
use substrate_data::{Map, Value, binary};

use crate::case::{Case, Io};

/// The stream of `messages` as the reference writer writes it.
pub(crate) fn stream(messages: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for m in messages {
        write_frame_bytes(&mut out, m).expect("writing to a Vec");
    }
    out
}

fn message(op: &str, n: i64) -> Vec<u8> {
    let mut m = Map::new();
    m.insert("op", Value::from(op));
    m.insert("n", Value::Int(n));
    binary::encode(&Value::Map(m))
}

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let two = vec![message("ping", 1), message("pong", 1)];
    let long = vec![binary::encode(&Value::Bytes(vec![7; 300]))];
    let empty_frame = vec![Vec::new()];
    let make = |name: &str, why: &str, messages: Vec<Vec<u8>>| {
        Case::new(name, why).io(Io {
            bytes: stream(&messages),
            messages,
        })
    };
    vec![
        make(
            "io_two_messages",
            "Each message is a little-endian u32 length, then that many bytes (here a binary document). The stream ends cleanly after a whole frame, where a read of the next length finds no byte.",
            two,
        ),
        make(
            "io_no_messages",
            "An empty stream ends cleanly at once: no error, no frame.",
            vec![],
        ),
        make(
            "io_empty_frame",
            "A frame may be zero bytes long; it is still a frame, and the stream goes on.",
            empty_frame,
        ),
        make(
            "io_long_message",
            "A 300-byte message: the length is 0x0000012c, four bytes always (never a varint).",
            long,
        ),
    ]
}
