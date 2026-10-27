//! The same, a row a 256-bit vector.
#![expect(
    unsafe_code,
    reason = "AVX2 kernels, chosen only where the processor has AVX2"
)]

use std::arch::x86_64::{
    __m128i, __m256i, _mm_cvtsi32_si128, _mm256_and_si256, _mm256_loadu_si256, _mm256_or_si256,
    _mm256_set1_epi32, _mm256_setzero_si256, _mm256_sll_epi32, _mm256_srl_epi32,
    _mm256_storeu_si256,
};

use super::{GROUP, ROWS, group_bytes, mask};

pub(super) fn pack(src: &[u32; GROUP], width: u32, out: &mut [u8]) {
    assert!(out.len() >= group_bytes(width), "a group's bytes");
    // SAFETY: `kernels` chose this only where the processor has AVX2; the
    // buffers were checked above.
    unsafe { pack_avx2(src, width, out) }
}

pub(super) fn unpack(src: &[u8], width: u32, out: &mut [u32; GROUP]) {
    assert!(src.len() >= group_bytes(width), "a group's bytes");
    // SAFETY: `kernels` chose this only where the processor has AVX2; the
    // buffers were checked above.
    unsafe { unpack_avx2(src, width, out) }
}

fn count(n: u32) -> __m128i {
    // SAFETY: SSE2 is part of x86_64.
    unsafe { _mm_cvtsi32_si128(n.cast_signed()) }
}

#[target_feature(enable = "avx2")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "only the unaligned `loadu`/`storeu` read and write through it"
)]
unsafe fn pack_avx2(src: &[u32; GROUP], width: u32, out: &mut [u8]) {
    let rows = src.as_ptr().cast::<__m256i>();
    let dst = out.as_mut_ptr().cast::<__m256i>();
    let mut acc = _mm256_setzero_si256();
    let (mut filled, mut word) = (0u32, 0usize);
    for r in 0..ROWS {
        // SAFETY: in bounds: ROWS rows of LANES words; `width` words written.
        let row = unsafe { _mm256_loadu_si256(rows.add(r)) };
        acc = _mm256_or_si256(acc, _mm256_sll_epi32(row, count(filled)));
        filled += width;
        if filled >= u32::BITS {
            // SAFETY: in bounds: `width` words written in all.
            unsafe { _mm256_storeu_si256(dst.add(word), acc) };
            word += 1;
            filled -= u32::BITS;
            acc = if filled == 0 {
                _mm256_setzero_si256()
            } else {
                _mm256_srl_epi32(row, count(width - filled))
            };
        }
    }
}

#[target_feature(enable = "avx2")]
#[expect(
    clippy::cast_ptr_alignment,
    reason = "only the unaligned `loadu`/`storeu` read and write through it"
)]
unsafe fn unpack_avx2(src: &[u8], width: u32, out: &mut [u32; GROUP]) {
    let words = src.as_ptr().cast::<__m256i>();
    let rows = out.as_mut_ptr().cast::<__m256i>();
    let m = _mm256_set1_epi32(mask(width).cast_signed());
    // SAFETY: in bounds: `width` words read, ROWS rows written.
    let mut cur = unsafe { _mm256_loadu_si256(words) };
    let (mut used, mut word) = (0u32, 1usize);
    for r in 0..ROWS {
        let end = used + width;
        let v = if end > u32::BITS {
            // SAFETY: in bounds: `width` words read in all.
            let next = unsafe { _mm256_loadu_si256(words.add(word)) };
            word += 1;
            let v = _mm256_or_si256(
                _mm256_srl_epi32(cur, count(used)),
                _mm256_sll_epi32(next, count(u32::BITS - used)),
            );
            cur = next;
            used = end - u32::BITS;
            v
        } else {
            let v = _mm256_srl_epi32(cur, count(used));
            used = end;
            if used == u32::BITS && word < width as usize {
                // SAFETY: `word < width`: in bounds.
                cur = unsafe { _mm256_loadu_si256(words.add(word)) };
                word += 1;
                used = 0;
            }
            v
        };
        // SAFETY: in bounds: ROWS rows written.
        unsafe { _mm256_storeu_si256(rows.add(r), _mm256_and_si256(v, m)) };
    }
}
