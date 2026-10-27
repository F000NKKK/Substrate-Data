//! Two groups at once: a row of each, side by side in one 512-bit vector.
#![expect(
    clippy::inline_always,
    reason = "the row operations must inline into the kernel compiled for the instruction set, or they run without it"
)]
#![expect(
    unsafe_code,
    reason = "AVX-512 kernels, chosen only where the processor has AVX-512F"
)]

use std::arch::x86_64::{
    __m128i, __m512i, _mm_cvtsi32_si128, _mm256_loadu_si256, _mm256_storeu_si256, _mm512_and_si512,
    _mm512_castsi256_si512, _mm512_castsi512_si256, _mm512_extracti64x4_epi64, _mm512_inserti64x4,
    _mm512_or_si512, _mm512_set1_epi32, _mm512_setzero_si512, _mm512_sll_epi32, _mm512_srl_epi32,
};

use super::dispatch::{by_width, simd_kernels};
use super::kernel;
use super::row::Row;

/// A row of one group in the low half, the same row of the next group in
/// the high half.
#[derive(Clone, Copy)]
struct Sixteen(__m512i);

#[inline(always)]
fn count(n: u32) -> __m128i {
    // SAFETY: SSE2 is part of x86_64.
    unsafe { _mm_cvtsi32_si128(n.cast_signed()) }
}

impl Row for Sixteen {
    const GROUPS: usize = 2;

    #[inline(always)]
    unsafe fn zero() -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm512_setzero_si512()) }
    }

    #[inline(always)]
    unsafe fn splat(v: u32) -> Self {
        // SAFETY: as `zero`.
        unsafe { Self(_mm512_set1_epi32(v.cast_signed())) }
    }

    #[inline(always)]
    unsafe fn load_words(p: *const u32, stride: usize) -> Self {
        // SAFETY: two rows readable, `stride` words apart.
        unsafe { Self::load_bytes(p.cast(), stride * size_of::<u32>()) }
    }

    #[inline(always)]
    unsafe fn load_bytes(p: *const u8, stride: usize) -> Self {
        // SAFETY: two 32-byte rows readable, `stride` bytes apart; the loads
        // are unaligned.
        unsafe {
            let low = _mm256_loadu_si256(p.cast());
            let high = _mm256_loadu_si256(p.add(stride).cast());
            Self(_mm512_inserti64x4::<1>(_mm512_castsi256_si512(low), high))
        }
    }

    #[inline(always)]
    unsafe fn store_words(self, p: *mut u32, stride: usize) {
        // SAFETY: two rows writable, `stride` words apart.
        unsafe { self.store_bytes(p.cast(), stride * size_of::<u32>()) }
    }

    #[inline(always)]
    unsafe fn store_bytes(self, p: *mut u8, stride: usize) {
        // SAFETY: two 32-byte rows writable, `stride` bytes apart; the
        // stores are unaligned.
        unsafe {
            _mm256_storeu_si256(p.cast(), _mm512_castsi512_si256(self.0));
            _mm256_storeu_si256(p.add(stride).cast(), _mm512_extracti64x4_epi64::<1>(self.0));
        }
    }

    #[inline(always)]
    unsafe fn or(self, o: Self) -> Self {
        // SAFETY: as `zero`.
        unsafe { Self(_mm512_or_si512(self.0, o.0)) }
    }

    #[inline(always)]
    unsafe fn and(self, o: Self) -> Self {
        // SAFETY: as `zero`.
        unsafe { Self(_mm512_and_si512(self.0, o.0)) }
    }

    #[inline(always)]
    unsafe fn shl(self, n: u32) -> Self {
        // SAFETY: as `zero`.
        unsafe { Self(_mm512_sll_epi32(self.0, count(n))) }
    }

    #[inline(always)]
    unsafe fn shr(self, n: u32) -> Self {
        // SAFETY: as `zero`.
        unsafe { Self(_mm512_srl_epi32(self.0, count(n))) }
    }
}

simd_kernels!(Sixteen, "avx512f");
