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
