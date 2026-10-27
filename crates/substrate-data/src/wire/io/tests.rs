//! Frames over a byte stream: round trips, cut and oversized frames.

use super::*;

#[test]
fn frames_follow_each_other_and_end_cleanly() {
    let mut pipe = Vec::new();
    write_frame(&mut pipe, &"hello".to_string()).unwrap();
    write_frame(&mut pipe, &vec![1u32, 2, 3]).unwrap();
    let mut r = pipe.as_slice();
    assert_eq!(
        read_frame::<String>(&mut r).unwrap().as_deref(),
        Some("hello")
    );
    assert_eq!(read_frame::<Vec<u32>>(&mut r).unwrap(), Some(vec![1, 2, 3]));
    assert_eq!(read_frame::<String>(&mut r).unwrap(), None);
}

#[test]
fn a_cut_or_oversized_frame_is_an_error() {
    let mut pipe = Vec::new();
    write_frame(&mut pipe, &"hello".to_string()).unwrap();
    let cut = &pipe[..pipe.len() - 1];
    assert!(read_frame::<String>(&mut &cut[..]).is_err());
    let huge = (u32::try_from(MAX_FRAME).unwrap() + 1).to_le_bytes();
    assert!(read_frame_bytes(&mut &huge[..]).is_err());
}

#[test]
fn a_stream_cut_inside_the_length_is_an_error_not_a_clean_end() {
    for cut in 1..size_of::<Len>() {
        let header = vec![7u8; cut];
        let err = read_frame_bytes(&mut header.as_slice()).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof, "{cut} bytes");
    }
    assert!(read_frame_bytes(&mut [].as_slice()).unwrap().is_none());
}

/// Fails each `read` with the next error of its script, then reads from `rest`.
struct Scripted<'a> {
    errors: Vec<std::io::ErrorKind>,
    rest: &'a [u8],
}

impl std::io::Read for Scripted<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.errors.pop() {
            Some(kind) => Err(kind.into()),
            None => self.rest.read(buf),
        }
    }
}

#[test]
fn an_interrupted_read_of_the_length_is_retried_and_other_errors_surface() {
    let mut bytes = Vec::new();
    write_frame(&mut bytes, &7_u32).unwrap();
    let mut retried = Scripted {
        errors: vec![std::io::ErrorKind::Interrupted],
        rest: &bytes,
    };
    assert_eq!(read_frame::<u32>(&mut retried).unwrap(), Some(7));

    let mut broken = Scripted {
        errors: vec![std::io::ErrorKind::BrokenPipe],
        rest: &bytes,
    };
    let err = read_frame::<u32>(&mut broken).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
}
