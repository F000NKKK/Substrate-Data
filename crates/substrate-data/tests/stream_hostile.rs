//! A hostile stream (a count or a length that lies, a cut, a flipped byte)
//! is an error or an unread item, never a panic and never a big allocation.

#![expect(
    unsafe_code,
    reason = "a counting global allocator: `GlobalAlloc` is an unsafe trait, every call passes straight to `System`"
)]

#[path = "support/stream_doc.rs"]
mod stream_doc;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use stream_doc::{Narrow, Words, forged};
use substrate_data::{Schema, read_bytes, to_bytes};

struct Largest;

thread_local! {
    /// The largest single allocation this thread has asked for.
    static LARGEST: Cell<usize> = const { Cell::new(0) };
}

fn note(size: usize) {
    LARGEST.with(|n| n.set(n.get().max(size)));
}

// SAFETY: every call is passed to the system allocator unchanged.
unsafe impl GlobalAlloc for Largest {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: `layout` is the caller's, unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` and `layout` are the caller's, unchanged.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note(new_size);
        // SAFETY: the caller's arguments, unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Largest = Largest;

/// The largest single allocation `f` asks for.
fn largest_allocation<R>(f: impl FnOnce() -> R) -> (R, usize) {
    LARGEST.with(|n| n.set(0));
    let result = f();
    (result, LARGEST.with(Cell::get))
}

#[test]
fn hostile_framing_is_an_error_and_allocates_nothing_big() {
    let huge_counts = [
        u64::MAX,
        u64::from(u32::MAX),
        1 << 40,
        1_000_000_000,
        // One more than the bytes it has can hold.
        3,
    ];
    for count in huge_counts {
        let doc = forged(count, &[145, 146]);
        let (result, largest) = largest_allocation(|| read_bytes::<Narrow>(&doc));
        assert!(result.is_err(), "count {count}");
        assert!(largest < 4096, "count {count} asked for {largest} bytes");
    }
    // A byte length past the document.
    let mut long = forged(1, &[145]);
    let at = long.len() - 1 - 1 - 4;
    long[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    let (result, largest) = largest_allocation(|| read_bytes::<Narrow>(&long));
    assert!(result.is_err());
    assert!(largest < 4096, "asked for {largest} bytes");
}

#[test]
fn a_big_count_the_bytes_hold_costs_the_bytes_not_the_count() {
    // As many items as bytes: legal framing, and nothing is read until asked.
    let items = vec![145u8; 100_000];
    let doc = forged(100_000, &items);
    let (narrow, largest) = largest_allocation(|| read_bytes::<Narrow>(&doc).unwrap());
    assert_eq!(narrow.items.len(), 100_000);
    assert!(largest < 2 * doc.len(), "asked for {largest} bytes");
    // A count of one more cannot be framed, and fails at decode.
    assert!(read_bytes::<Narrow>(&forged(100_001, &items)).is_err());
    // Iterating one byte at a time never allocates a list of them.
    let (count, iterating) = largest_allocation(|| narrow.items.iter().flatten().count());
    assert_eq!(count, 100_000);
    assert!(iterating < 4096, "iterating asked for {iterating} bytes");
}

#[test]
fn a_cut_or_corrupt_stream_never_panics() {
    let doc = to_bytes(&Words {
        items: ["alpha", "beta", "gamma"]
            .map(String::from)
            .into_iter()
            .collect(),
    });
    for cut in 0..doc.len() {
        if let Ok(words) = read_bytes::<Words>(&doc[..cut]) {
            std::hint::black_box(words.items.iter().count());
        }
    }
    for at in 0..doc.len() {
        for flip in [0x01, 0x80, 0xff] {
            let mut broken = doc.clone();
            broken[at] ^= flip;
            if let Ok(words) = read_bytes::<Words>(&broken) {
                std::hint::black_box(words.items.iter().count());
                std::hint::black_box(words.items.write());
            }
        }
    }
}
