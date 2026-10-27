//! One group packed or unpacked for a width known at compile time, whatever
//! a row is made of. With the width constant the compiler unrolls the 32
//! rows and turns every shift count into an immediate.
#![expect(
    clippy::inline_always,
    reason = "the row operations must inline into the kernel compiled for the instruction set, or they run without it"
)]
#![expect(
    unsafe_code,
    reason = "row operations and loads through pointers bounds-checked on entry"
)]

use super::row::Row;
use super::{GROUP, LANES, ROWS, WORD_BYTES, group_bytes, mask};

/// `R::GROUPS` groups from `src` packed onto `out`, `W` bits a number.
///
/// # Safety
///
/// The processor supports the instructions `R` is made of.
#[inline(always)]
pub(super) unsafe fn pack<R: Row, const W: u32>(src: &[u32], out: &mut [u8]) {
    assert!(src.len() >= R::GROUPS * GROUP, "a group's numbers");
    assert!(out.len() >= R::GROUPS * group_bytes(W), "a group's bytes");
    let rows = src.as_ptr();
    let dst = out.as_mut_ptr();
    let (src_stride, out_stride) = (GROUP, group_bytes(W));
    // SAFETY: the processor supports `R` (the caller's contract); every
    // pointer is in bounds: ROWS rows of LANES words are read from `src`, and
    // `W` rows of 32 bytes are written, per group, which the asserts above cover.
    unsafe {
        let mut acc = R::zero();
        let (mut filled, mut word) = (0u32, 0usize);
        for r in 0..ROWS {
            let row = R::load_words(rows.add(r * LANES), src_stride);
            acc = acc.or(row.shl(filled));
            filled += W;
            if filled >= u32::BITS {
                acc.store_bytes(dst.add(word * LANES * WORD_BYTES), out_stride);
                word += 1;
                filled -= u32::BITS;
                acc = if filled == 0 {
                    R::zero()
                } else {
                    row.shr(W - filled)
                };
            }
        }
    }
}

/// `R::GROUPS` groups from `src` unpacked onto `out`, `W` bits a number.
///
/// # Safety
///
/// The processor supports the instructions `R` is made of.
#[inline(always)]
pub(super) unsafe fn unpack<R: Row, const W: u32>(src: &[u8], out: &mut [u32]) {
    assert!(src.len() >= R::GROUPS * group_bytes(W), "a group's bytes");
    assert!(out.len() >= R::GROUPS * GROUP, "a group's numbers");
    let words = src.as_ptr();
    let rows = out.as_mut_ptr();
    let (src_stride, out_stride) = (group_bytes(W), GROUP);
    // SAFETY: the processor supports `R` (the caller's contract); at most `W`
    // rows of 32 bytes are read from `src`, which the assert above covers,
    // and ROWS rows of LANES words are written to `out`.
    unsafe {
        let m = R::splat(mask(W));
        let mut cur = R::load_bytes(words, src_stride);
        let (mut used, mut word) = (0u32, 1usize);
        for r in 0..ROWS {
            let end = used + W;
            let v = if end > u32::BITS {
                let next = R::load_bytes(words.add(word * LANES * WORD_BYTES), src_stride);
                word += 1;
                let v = cur.shr(used).or(next.shl(u32::BITS - used));
                cur = next;
                used = end - u32::BITS;
                v
            } else {
                let v = cur.shr(used);
                used = end;
                if used == u32::BITS && word < W as usize {
                    cur = R::load_bytes(words.add(word * LANES * WORD_BYTES), src_stride);
                    word += 1;
                    used = 0;
                }
                v
            };
            v.and(m).store_words(rows.add(r * LANES), out_stride);
        }
    }
}
