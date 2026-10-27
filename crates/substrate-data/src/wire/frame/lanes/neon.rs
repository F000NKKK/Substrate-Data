//! A row as two 128-bit NEON vectors.
#![expect(
    clippy::inline_always,
    reason = "the row operations must inline into the kernel compiled for the instruction set, or they run without it"
)]
#![expect(
    unsafe_code,
    reason = "NEON kernels, chosen only where the processor has NEON"
)]

use std::arch::aarch64::{
    uint32x4_t, vandq_u32, vdupq_n_s32, vdupq_n_u32, vld1q_u8, vld1q_u32, vorrq_u32,
    vreinterpretq_u8_u32, vreinterpretq_u32_u8, vshlq_u32, vst1q_u8, vst1q_u32,
};

use super::dispatch::{by_width, simd_kernels};
use super::kernel;
use super::row::Row;

/// Lanes 0..4 and 4..8.
#[derive(Clone, Copy)]
struct Pair(uint32x4_t, uint32x4_t);

impl Row for Pair {
    const GROUPS: usize = 1;

    #[inline(always)]
    unsafe fn zero() -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(vdupq_n_u32(0), vdupq_n_u32(0)) }
    }

    #[inline(always)]
    unsafe fn splat(v: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            let x = vdupq_n_u32(v);
            Self(x, x)
        }
    }

    #[inline(always)]
    unsafe fn load_words(p: *const u32, _stride: usize) -> Self {
        // SAFETY: eight words readable; NEON loads take any alignment.
        unsafe { Self(vld1q_u32(p), vld1q_u32(p.add(4))) }
    }

    #[inline(always)]
    unsafe fn load_bytes(p: *const u8, _stride: usize) -> Self {
        // SAFETY: 32 bytes readable; bytes reinterpreted as little-endian
        // words (these kernels are built for little-endian targets only).
        unsafe {
            Self(
                vreinterpretq_u32_u8(vld1q_u8(p)),
                vreinterpretq_u32_u8(vld1q_u8(p.add(16))),
            )
        }
    }

    #[inline(always)]
    unsafe fn store_words(self, p: *mut u32, _stride: usize) {
        // SAFETY: eight words writable.
        unsafe {
            vst1q_u32(p, self.0);
            vst1q_u32(p.add(4), self.1);
        }
    }

    #[inline(always)]
    unsafe fn store_bytes(self, p: *mut u8, _stride: usize) {
        // SAFETY: 32 bytes writable.
        unsafe {
            vst1q_u8(p, vreinterpretq_u8_u32(self.0));
            vst1q_u8(p.add(16), vreinterpretq_u8_u32(self.1));
        }
    }

    #[inline(always)]
    unsafe fn or(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(vorrq_u32(self.0, o.0), vorrq_u32(self.1, o.1)) }
    }

    #[inline(always)]
    unsafe fn and(self, o: Self) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe { Self(vandq_u32(self.0, o.0), vandq_u32(self.1, o.1)) }
    }

    #[inline(always)]
    unsafe fn shl(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            let c = vdupq_n_s32(n.cast_signed());
            Self(vshlq_u32(self.0, c), vshlq_u32(self.1, c))
        }
    }

    #[inline(always)]
    unsafe fn shr(self, n: u32) -> Self {
        // SAFETY: the trait's contract: the processor has the instructions.
        unsafe {
            // A negative count shifts right.
            let c = vdupq_n_s32((n.cast_signed()).wrapping_neg());
            Self(vshlq_u32(self.0, c), vshlq_u32(self.1, c))
        }
    }
}

simd_kernels!(Pair, "neon");
