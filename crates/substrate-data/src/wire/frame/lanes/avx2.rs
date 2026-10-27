//! A row as one 256-bit vector.
#![expect(
    clippy::inline_always,
    reason = "the row operations must inline into the kernel compiled for the instruction set, or they run without it"
)]
#![expect(
    unsafe_code,
    reason = "AVX2 kernels, chosen only where the processor has AVX2"
)]

use std::arch::x86_64::{
    __m128i, __m256i, _mm_cvtsi32_si128, _mm256_and_si256, _mm256_loadu_si256, _mm256_or_si256,
    _mm256_set1_epi32, _mm256_setzero_si256, _mm256_sll_epi32, _mm256_srl_epi32,
    _mm256_storeu_si256,
};

use super::dispatch::{by_width, simd_kernels};
use super::kernel;
use super::row::Row;

/// The eight lanes.
#[derive(Clone, Copy)]
struct Eight(__m256i);

#[inline(always)]
fn count(n: u32) -> __m128i {
    // SAFETY: SSE2 is part of x86_64.
    unsafe { _mm_cvtsi32_si128(n.cast_signed()) }
}

impl Row for Eight {
    const GROUPS: usize = 1;

    #[inline(always)]
    unsafe fn zero() -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_setzero_si256()) }
    }

    #[inline(always)]
    unsafe fn splat(v: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_set1_epi32(v.cast_signed())) }
    }

    #[inline(always)]
    unsafe fn load_words(p: *const u32, _stride: usize) -> Self {
        // SAFETY: eight words readable; `loadu` takes any alignment.
        unsafe { Self::load_bytes(p.cast(), 0) }
    }

    #[inline(always)]
    unsafe fn load_bytes(p: *const u8, _stride: usize) -> Self {
        // SAFETY: 32 bytes readable.
        unsafe { Self(_mm256_loadu_si256(p.cast())) }
    }

    #[inline(always)]
    unsafe fn store_words(self, p: *mut u32, _stride: usize) {
        // SAFETY: as `load_words`.
        unsafe { self.store_bytes(p.cast(), 0) }
    }

    #[inline(always)]
    unsafe fn store_bytes(self, p: *mut u8, _stride: usize) {
        // SAFETY: 32 bytes writable.
        unsafe { _mm256_storeu_si256(p.cast(), self.0) }
    }

    #[inline(always)]
    unsafe fn or(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_or_si256(self.0, o.0)) }
    }

    #[inline(always)]
    unsafe fn and(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_and_si256(self.0, o.0)) }
    }

    #[inline(always)]
    unsafe fn shl(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_sll_epi32(self.0, count(n))) }
    }

    #[inline(always)]
    unsafe fn shr(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(_mm256_srl_epi32(self.0, count(n))) }
    }
}

simd_kernels!(Eight, "avx2");
