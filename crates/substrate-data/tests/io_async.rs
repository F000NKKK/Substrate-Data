//! Async frames over tokio streams: the sync wire format, the same limits and
//! end rules, partial reads, cut streams and cancellation.

#![cfg(feature = "tokio")]

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use substrate_data::Schema;
use substrate_data::io::asynchronous::{
    FrameReader, FrameWriter, read_frame, read_frame_bytes, write_frame, write_frame_bytes,
};
use substrate_data::io::{self as sync_io, MAX_FRAME};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};

#[derive(Schema, Clone, PartialEq, Debug)]
struct Note {
    title: String,
    tags: Vec<String>,
    stars: u32,
}

fn note(n: u32) -> Note {
    Note {
        title: format!("note {n}"),
        tags: vec!["core:docs".into(), format!("t{n}")],
        stars: n,
    }
}

/// Five notes, framed by the sync writer.
fn wire() -> Vec<u8> {
    let mut sender = sync_io::FrameWriter::new();
    let mut out = Vec::new();
    for n in 0..5 {
        sender
            .send(&mut out, &note(n))
            .expect("a Vec takes any write");
    }
    out
}

/// Hands out at most `step` bytes per read, and says `Pending` (waking itself)
/// between reads when `stall` is set.
struct Trickle<'a> {
    rest: &'a [u8],
    step: usize,
    stall: bool,
    ready: bool,
}

impl<'a> Trickle<'a> {
    fn new(rest: &'a [u8], step: usize, stall: bool) -> Self {
        Trickle {
            rest,
            step,
            stall,
            ready: true,
        }
    }
}

impl AsyncRead for Trickle<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.stall && !self.ready {
            self.ready = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.ready = false;
        let n = self.step.min(self.rest.len()).min(buf.remaining());
        let (now, later) = self.rest.split_at(n);
        buf.put_slice(now);
        self.rest = later;
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn frames_round_trip_over_a_small_duplex() {
    let (mut near, mut far) = tokio::io::duplex(7);
    let send = async move {
        let mut sender = FrameWriter::new();
        for n in 0..20 {
            sender.send(&mut near, &note(n)).await.unwrap();
        }
        sender.send_packed(&mut near, &note(99)).await.unwrap();
    };
    let receive = async move {
        let mut reader = FrameReader::new();
        for n in 0..20 {
            assert_eq!(reader.read::<Note>(&mut far).await.unwrap(), Some(note(n)));
        }
        let packed = reader.read_bytes(&mut far).await.unwrap().unwrap().to_vec();
        assert_eq!(
            substrate_data::frame::read_frame::<Note>(&packed).unwrap(),
            note(99)
        );
        assert!(reader.read::<Note>(&mut far).await.unwrap().is_none());
    };
    tokio::join!(send, receive);
}

#[tokio::test]
async fn the_async_writer_writes_the_sync_writers_bytes() {
    let (mut named, mut packed) = (Vec::new(), Vec::new());
    let mut sender = FrameWriter::new();
    for n in 0..5 {
        sender.send(&mut named, &note(n)).await.unwrap();
        sender.send_packed(&mut packed, &note(n)).await.unwrap();
    }
    assert_eq!(named, wire());

    let (mut sync_packed, mut sync_sender) = (Vec::new(), sync_io::FrameWriter::new());
    for n in 0..5 {
        sync_sender.send_packed(&mut sync_packed, &note(n)).unwrap();
    }
    assert_eq!(packed, sync_packed);

    let (mut async_frame, mut sync_frame) = (Vec::new(), Vec::new());
    write_frame(&mut async_frame, &note(3)).await.unwrap();
    sync_io::write_frame(&mut sync_frame, &note(3)).unwrap();
    assert_eq!(async_frame, sync_frame);
    let body = substrate_data::to_bytes(&note(3));
    let (mut async_bytes, mut sync_bytes) = (Vec::new(), Vec::new());
    write_frame_bytes(&mut async_bytes, &body).await.unwrap();
    sync_io::write_frame_bytes(&mut sync_bytes, &body).unwrap();
    assert_eq!(async_bytes, sync_bytes);
}

#[tokio::test]
async fn the_async_reader_reads_what_the_sync_reader_reads() {
    let bytes = wire();
    let mut sync_reader = sync_io::FrameReader::new();
    let mut sync_in = bytes.as_slice();
    let mut reader = FrameReader::new();
    let mut input = bytes.as_slice();
    loop {
        let expect = sync_reader
            .read_bytes(&mut sync_in)
            .unwrap()
            .map(<[u8]>::to_vec);
        let got = reader
            .read_bytes(&mut input)
            .await
            .unwrap()
            .map(<[u8]>::to_vec);
        assert_eq!(got, expect);
        if got.is_none() {
            break;
        }
    }
    let mut once = bytes.as_slice();
    assert_eq!(read_frame::<Note>(&mut once).await.unwrap(), Some(note(0)));
    let mut again = bytes.as_slice();
    let owned = read_frame_bytes(&mut again).await.unwrap().unwrap();
    assert_eq!(owned, substrate_data::to_bytes(&note(0)));
}

#[tokio::test]
async fn partial_reads_of_every_size_give_the_same_frames() {
    let bytes = wire();
    for step in [1, 2, 3, 5, 64] {
        for stall in [false, true] {
            let mut input = Trickle::new(&bytes, step, stall);
            let mut reader = FrameReader::new();
            for n in 0..5 {
                assert_eq!(
                    reader.read::<Note>(&mut input).await.unwrap(),
                    Some(note(n))
                );
            }
            assert!(reader.read::<Note>(&mut input).await.unwrap().is_none());
        }
    }
}

#[tokio::test]
async fn a_stream_cut_anywhere_ends_cleanly_only_between_frames() {
    let mut one = Vec::new();
    write_frame(&mut one, &note(1)).await.unwrap();
    for cut in 0..=one.len() {
        let mut input = &one[..cut];
        let got = read_frame_bytes(&mut input).await;
        match cut {
            0 => assert!(got.unwrap().is_none()),
            c if c == one.len() => assert!(got.unwrap().is_some()),
            _ => assert_eq!(
                got.unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "cut {cut}"
            ),
        }
    }
    // A clean end after a whole frame, an error for the next one cut short.
    let two = [one.clone(), one[..one.len() - 1].to_vec()].concat();
    let (mut input, mut reader) = (two.as_slice(), FrameReader::new());
    assert!(reader.read::<Note>(&mut input).await.unwrap().is_some());
    let err = reader.read::<Note>(&mut input).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
}

#[tokio::test]
async fn an_oversized_length_is_refused_before_any_body_is_read() {
    let huge = (u32::try_from(MAX_FRAME).unwrap() + 1).to_le_bytes();
    let mut reader = FrameReader::new();
    let mut input = huge.as_slice();
    let first = reader.read_bytes(&mut input).await.unwrap_err();
    assert_eq!(first.kind(), io::ErrorKind::InvalidData);
    // The refusal stays: the stream is out of step.
    let second = reader.read_bytes(&mut input).await.unwrap_err();
    assert_eq!(second.kind(), io::ErrorKind::InvalidData);
    let free = read_frame_bytes(&mut huge.as_slice()).await.unwrap_err();
    assert_eq!(free.kind(), io::ErrorKind::InvalidData);
    let max = u32::MAX.to_le_bytes();
    assert!(read_frame::<u32>(&mut max.as_slice()).await.is_err());
}

#[tokio::test]
async fn a_claimed_length_allocates_only_what_arrives() {
    // Claims the biggest frame allowed and sends ten bytes.
    let mut lie = u32::try_from(MAX_FRAME).unwrap().to_le_bytes().to_vec();
    lie.extend_from_slice(&[7; 10]);
    let mut reader = FrameReader::new();
    let err = reader.read_bytes(&mut lie.as_slice()).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
}

#[tokio::test]
async fn an_oversized_frame_is_not_written() {
    let huge = vec![0u8; MAX_FRAME + 1];
    let mut out = Vec::new();
    let raw = write_frame_bytes(&mut out, &huge).await.unwrap_err();
    assert_eq!(raw.kind(), io::ErrorKind::InvalidInput);
    let big = "a".repeat(MAX_FRAME);
    let named = FrameWriter::new().send(&mut out, &big).await.unwrap_err();
    assert_eq!(named.kind(), io::ErrorKind::InvalidInput);
    let packed = FrameWriter::new()
        .send_packed(&mut out, &big)
        .await
        .unwrap_err();
    assert_eq!(packed.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(out, Vec::<u8>::new());
}

#[tokio::test]
async fn a_frame_that_is_not_the_type_is_invalid_data_as_in_sync() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &"text".to_string()).await.unwrap();
    let a = read_frame::<Vec<Note>>(&mut bytes.as_slice())
        .await
        .unwrap_err();
    let b = sync_io::read_frame::<Vec<Note>>(&mut bytes.as_slice()).unwrap_err();
    assert_eq!(a.kind(), io::ErrorKind::InvalidData);
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.to_string(), b.to_string());
    // Garbage inside a well-framed body.
    let mut junk = 4u32.to_le_bytes().to_vec();
    junk.extend_from_slice(&[0xff; 4]);
    let err = read_frame::<u32>(&mut junk.as_slice()).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn big_frames_cross_many_growth_steps_and_the_buffer_is_reused() {
    let big = "x".repeat(300_000);
    let (mut near, mut far) = tokio::io::duplex(1000);
    let send = async move {
        let mut sender = FrameWriter::new();
        sender.send(&mut near, &big).await.unwrap();
        sender.send(&mut near, &"small".to_string()).await.unwrap();
        sender.send(&mut near, &String::new()).await.unwrap();
    };
    let receive = async move {
        let mut reader = FrameReader::new();
        let first = reader.read::<String>(&mut far).await.unwrap().unwrap();
        assert_eq!(
            (first.len(), first.bytes().all(|b| b == b'x')),
            (300_000, true)
        );
        let second = reader.read::<String>(&mut far).await.unwrap();
        assert_eq!(second.as_deref(), Some("small"));
        let third = reader.read::<String>(&mut far).await.unwrap();
        assert_eq!(third.as_deref(), Some(""));
        assert!(reader.read::<String>(&mut far).await.unwrap().is_none());
    };
    tokio::join!(send, receive);
}

#[tokio::test]
async fn a_dropped_read_loses_nothing() {
    let mut frame = Vec::new();
    write_frame(&mut frame, &note(5)).await.unwrap();
    let (mut near, mut far) = tokio::io::duplex(64);
    let mut reader = FrameReader::new();
    let mut cx = Context::from_waker(Waker::noop());
    // Feed the frame a byte at a time; each time, start a read, see it wait,
    // and drop it.
    for (i, byte) in frame.iter().enumerate() {
        near.write_all(&[*byte]).await.unwrap();
        let mut read = Box::pin(reader.read::<Note>(&mut far));
        match read.as_mut().poll(&mut cx) {
            Poll::Ready(got) => {
                assert_eq!(i, frame.len() - 1);
                assert_eq!(got.unwrap(), Some(note(5)));
                return;
            }
            Poll::Pending => drop(read),
        }
    }
    panic!("the last byte completes the frame");
}

#[tokio::test]
async fn a_read_error_surfaces_and_a_write_to_a_closed_peer_fails() {
    struct Broken;
    impl AsyncRead for Broken {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Err(io::ErrorKind::ConnectionReset.into()))
        }
    }
    let reset = read_frame::<u32>(&mut Broken).await.unwrap_err();
    assert_eq!(reset.kind(), io::ErrorKind::ConnectionReset);

    let (mut near, far) = tokio::io::duplex(64);
    drop(far);
    let err = FrameWriter::new()
        .send(&mut near, &note(1))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    // The reading half of the same pipe reports the end, not an error.
    let mut empty = Vec::new();
    assert_eq!(near.read_to_end(&mut empty).await.unwrap(), 0);
}
