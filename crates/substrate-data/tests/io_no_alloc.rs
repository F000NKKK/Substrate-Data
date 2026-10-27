//! A send allocates nothing once its buffers have grown to the messages.

#![expect(
    unsafe_code,
    reason = "a counting global allocator: `GlobalAlloc` is an unsafe trait, every call passes straight to `System`"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::{self, Write};

use substrate_data::Schema;
use substrate_data::io::{FrameReader, FrameWriter, write_frame};

struct Counting;

thread_local! {
    /// This thread's allocations (tests run in parallel; a `const` cell never allocates).
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    // A `const` cell without a destructor is usable for the thread's whole life.
    ALLOCATIONS.with(|n| n.set(n.get() + 1));
}

// SAFETY: every call is passed to the system allocator unchanged.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: `layout` is the caller's, unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` are the caller's, unchanged.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: the caller's arguments, unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[derive(Schema, Clone, PartialEq, Debug)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct Card {
    id: String,
    kind: String,
    home: Point,
    scores: Vec<u32>,
    name: String,
}

fn card() -> Card {
    Card {
        id: "docs/intro".into(),
        kind: "core:doc".into(),
        home: Point { x: 1.5, y: -2.0 },
        scores: (0..40).collect(),
        name: "Alice".into(),
    }
}

/// A writer that keeps nothing: only its own bookkeeping, no allocation.
struct Sink(usize);

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// How many allocations `f` made.
fn allocations(f: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.get();
    f();
    ALLOCATIONS.get() - before
}

#[test]
fn send_and_send_packed_allocate_nothing_once_warm() {
    let (message, mut sender, mut sink) = (card(), FrameWriter::new(), Sink(0));
    for _ in 0..3 {
        sender.send(&mut sink, &message).unwrap();
        sender.send_packed(&mut sink, &message).unwrap();
    }
    for n in 0..10 {
        let sent = allocations(|| sender.send(&mut sink, &message).unwrap());
        assert_eq!(sent, 0, "send #{n}");
        let packed = allocations(|| sender.send_packed(&mut sink, &message).unwrap());
        assert_eq!(packed, 0, "send_packed #{n}");
    }
    assert!(sink.0 > 0);
}

#[derive(Schema, Clone, PartialEq, Debug)]
struct Page {
    title: String,
    cells: Vec<u32>,
    cards: Vec<Card>,
}

#[test]
fn long_records_and_lists_allocate_nothing_once_warm_either() {
    // Records and lists over 127 bytes carry a longer length: the case that moves fields.
    let page = Page {
        title: "docs/long".into(),
        cells: (0..2000).map(|n| n * 1000).collect(),
        cards: vec![card(), card(), card()],
    };
    let (mut sender, mut sink) = (FrameWriter::new(), Sink(0));
    for _ in 0..3 {
        sender.send(&mut sink, &page).unwrap();
        sender.send_packed(&mut sink, &page).unwrap();
    }
    assert_eq!(allocations(|| sender.send(&mut sink, &page).unwrap()), 0);
    assert_eq!(
        allocations(|| sender.send_packed(&mut sink, &page).unwrap()),
        0
    );
}

#[test]
fn write_frame_allocates_every_time() {
    // The contrast: what the reusable sender avoids.
    let (message, mut sink) = (card(), Sink(0));
    write_frame(&mut sink, &message).unwrap();
    assert!(allocations(|| write_frame(&mut sink, &message).unwrap()) >= 2);
}

#[test]
fn the_reader_allocates_nothing_once_warm_to_read_bytes() {
    let mut pipe = Vec::new();
    for _ in 0..12 {
        write_frame(&mut pipe, &card()).unwrap();
    }
    let (mut r, mut frames) = (pipe.as_slice(), FrameReader::new());
    frames.read_bytes(&mut r).unwrap().unwrap();
    for n in 1..12 {
        let read = allocations(|| {
            frames.read_bytes(&mut r).unwrap().unwrap();
        });
        assert_eq!(read, 0, "frame #{n}");
    }
}
