//! A row as two 128-bit vectors.
#![expect(
    clippy::inline_always,
    reason = "the row operations must inline into the kernel compiled for the instruction set, or they run without it"
)]
#![expect(
    unsafe_code,
    reason = "SSE kernels, chosen only where the processor has SSE4.2"
)]

use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cvtsi32_si128, _mm_loadu_si128, _mm_or_si128, _mm_set1_epi32,
    _mm_setzero_si128, _mm_sll_epi32, _mm_srl_epi32, _mm_storeu_si128,
};

use super::dispatch::{by_width, simd_kernels};
use super::kernel;
use super::row::Row;

/// Lanes 0..4 and 4..8.
#[derive(Clone, Copy)]
struct Pair(__m128i, __m128i);

#[inline(always)]
fn count(n: u32) -> __m128i {
    // SAFETY: SSE2 is part of x86_64.
    unsafe { _mm_cvtsi32_si128(n.cast_signed()) }
}

impl Row for Pair {
    const GROUPS: usize = 1;

    #[inline(always)]
    unsafe fn zero() -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm_setzero_si128(), _mm_setzero_si128()) }
    }

    #[inline(always)]
    unsafe fn splat(v: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            let x = _mm_set1_epi32(v.cast_signed());
            Self(x, x)
        }
    }

    #[inline(always)]
    unsafe fn load_words(p: *const u32, _stride: usize) -> Self {
        // SAFETY: eight words readable; `loadu` takes any alignment.
        unsafe { Self::load_bytes(p.cast(), 0) }
    }

    #[inline(always)]
    unsafe fn load_bytes(p: *const u8, _stride: usize) -> Self {
        // SAFETY: 32 bytes readable, two unaligned 16-byte loads.
        unsafe { Self(_mm_loadu_si128(p.cast()), _mm_loadu_si128(p.add(16).cast())) }
    }

    #[inline(always)]
    unsafe fn store_words(self, p: *mut u32, _stride: usize) {
        // SAFETY: as `load_words`.
        unsafe { self.store_bytes(p.cast(), 0) }
    }

    #[inline(always)]
    unsafe fn store_bytes(self, p: *mut u8, _stride: usize) {
        // SAFETY: 32 bytes writable, two unaligned 16-byte stores.
        unsafe {
            _mm_storeu_si128(p.cast(), self.0);
            _mm_storeu_si128(p.add(16).cast(), self.1);
        }
    }

    #[inline(always)]
    unsafe fn or(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm_or_si128(self.0, o.0), _mm_or_si128(self.1, o.1)) }
    }

    #[inline(always)]
    unsafe fn and(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm_and_si128(self.0, o.0), _mm_and_si128(self.1, o.1)) }
    }

    #[inline(always)]
    unsafe fn shl(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            let c = count(n);
            Self(_mm_sll_epi32(self.0, c), _mm_sll_epi32(self.1, c))
        }
    }

    #[inline(always)]
    unsafe fn shr(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            let c = count(n);
            Self(_mm_srl_epi32(self.0, c), _mm_srl_epi32(self.1, c))
        }
    }
}

simd_kernels!(Pair, "sse4.2");
