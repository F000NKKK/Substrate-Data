//! Reusable frame buffers: `FrameWriter` and `FrameReader` over byte streams.

use std::io::{self, Write};

use substrate_data::Schema;
use substrate_data::io::{
    FrameReader, FrameWriter, MAX_FRAME, read_frame, read_frame_bytes, write_frame,
};

#[derive(Schema, Clone, PartialEq, Debug)]
struct Note {
    title: String,
    tags: Vec<String>,
    stars: u32,
}

fn note(n: u32) -> Note {
    Note {
        title: format!("note {n}"),
        tags: vec!["core:docs".into(), "core:docs".into(), format!("t{n}")],
        stars: n,
    }
}

/// Accepts `room` bytes, then fails every write.
struct Short {
    room: usize,
}

impl Write for Short {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.room == 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let n = buf.len().min(self.room);
        self.room -= n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Counts writes and flushes.
#[derive(Default)]
struct Counting {
    bytes: Vec<u8>,
    writes: usize,
    flushes: usize,
}

impl Write for Counting {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[test]
fn send_matches_write_frame_byte_for_byte() {
    let mut sender = FrameWriter::new();
    let (mut sent, mut written) = (Vec::new(), Vec::new());
    for n in 0..5 {
        sender.send(&mut sent, &note(n)).unwrap();
        write_frame(&mut written, &note(n)).unwrap();
    }
    assert_eq!(sent, written);
}

#[test]
fn each_frame_is_one_write_and_one_flush() {
    let mut sender = FrameWriter::new();
    let mut out = Counting::default();
    sender.send(&mut out, &note(1)).unwrap();
    sender.send_packed(&mut out, &note(2)).unwrap();
    assert_eq!((out.writes, out.flushes), (2, 2));
}

#[test]
fn many_messages_round_trip_in_one_buffer() {
    let mut sender = FrameWriter::new();
    let mut pipe = Vec::new();
    for n in 0..50 {
        if n % 2 == 0 {
            sender.send(&mut pipe, &note(n)).unwrap();
        } else {
            sender.send_packed(&mut pipe, &note(n)).unwrap();
        }
    }
    let mut r = pipe.as_slice();
    let mut frames = FrameReader::new();
    for n in (0..50).step_by(2) {
        assert_eq!(frames.read::<Note>(&mut r).unwrap(), Some(note(n)));
        // The packed frames are for `frame::read_frame`: step over them as bytes.
        let packed = frames.read_bytes(&mut r).unwrap().unwrap();
        let back: Note = substrate_data::frame::read_frame(packed).unwrap();
        assert_eq!(back, note(n + 1));
    }
    assert_eq!(frames.read::<Note>(&mut r).unwrap(), None);
}

#[test]
fn the_reader_reuses_one_buffer_for_frames_of_any_size() {
    let mut pipe = Vec::new();
    for len in [3, 300, 1, 0, 4000] {
        write_frame(&mut pipe, &"x".repeat(len)).unwrap();
    }
    let (mut r, mut frames) = (pipe.as_slice(), FrameReader::new());
    let mut plain = pipe.as_slice();
    while let Some(expected) = read_frame_bytes(&mut plain).unwrap() {
        assert_eq!(
            frames.read_bytes(&mut r).unwrap(),
            Some(expected.as_slice())
        );
    }
    assert!(frames.read_bytes(&mut r).unwrap().is_none());
}

#[test]
fn a_short_write_surfaces_as_its_error() {
    let mut sender = FrameWriter::new();
    for room in [0, 3, 8] {
        let sent = sender.send(&mut Short { room }, &note(1)).unwrap_err();
        assert!(matches!(
            sent.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::WriteZero
        ));
        let packed = sender
            .send_packed(&mut Short { room }, &note(1))
            .unwrap_err();
        assert!(matches!(
            packed.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::WriteZero
        ));
    }
    // The writer is fine afterwards.
    let mut pipe = Vec::new();
    sender.send(&mut pipe, &note(2)).unwrap();
    assert_eq!(
        read_frame::<Note>(&mut pipe.as_slice()).unwrap(),
        Some(note(2))
    );
}

/// A writer a refused frame must never reach: any write fails with another kind.
struct Never;

impl Write for Never {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::Other.into())
    }
}

#[test]
fn a_frame_over_max_frame_is_refused_before_anything_is_written() {
    let huge = "a".repeat(MAX_FRAME);
    let mut sender = FrameWriter::new();
    let sent = sender.send(&mut Never, &huge).unwrap_err();
    assert_eq!(sent.kind(), io::ErrorKind::InvalidInput);
    let packed = sender.send_packed(&mut Never, &huge).unwrap_err();
    assert_eq!(packed.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn the_reader_ends_cleanly_only_between_frames() {
    let mut frames = FrameReader::new();
    assert!(frames.read_bytes(&mut [].as_slice()).unwrap().is_none());
    for cut in 1..4 {
        let header = vec![7u8; cut];
        let err = frames.read_bytes(&mut header.as_slice()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof, "{cut} bytes");
    }
    let mut pipe = Vec::new();
    write_frame(&mut pipe, &note(1)).unwrap();
    let cut = &pipe[..pipe.len() - 1];
    let err = frames.read::<Note>(&mut &cut[..]).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn the_reader_refuses_an_oversized_or_foreign_frame() {
    let mut frames = FrameReader::new();
    let huge = (u32::try_from(MAX_FRAME).unwrap() + 1).to_le_bytes();
    let too_long = frames.read_bytes(&mut &huge[..]).unwrap_err();
    assert_eq!(too_long.kind(), io::ErrorKind::InvalidData);
    // A length that promises far more than arrives is cut, not allocated.
    let lie = [0xff, 0xff, 0xff, 0x0f, 1, 2, 3];
    let cut = frames.read_bytes(&mut &lie[..]).unwrap_err();
    assert_eq!(cut.kind(), io::ErrorKind::UnexpectedEof);

    let mut pipe = Vec::new();
    write_frame(&mut pipe, &"text".to_string()).unwrap();
    let foreign = frames.read::<Note>(&mut pipe.as_slice()).unwrap_err();
    assert_eq!(foreign.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn appending_encoders_keep_what_the_buffer_held() {
    let mut out = b"head".to_vec();
    substrate_data::to_bytes_into(&note(1), &mut out);
    assert_eq!(&out[..4], b"head");
    assert_eq!(&out[4..], substrate_data::to_bytes(&note(1)));
    let value = note(2).write();
    substrate_data::binary::encode_into(&value, &mut out);
    assert!(out.ends_with(&substrate_data::binary::encode(&value)));
}
