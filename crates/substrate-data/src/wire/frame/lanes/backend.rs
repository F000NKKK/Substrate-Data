//! The instruction sets a group can be packed with, and which one this
//! processor uses.

use std::fmt;

use super::{GROUP, group_bytes, portable};
use crate::sync::OnceLock;

/// An instruction set that packs and unpacks groups. Every backend writes and
/// reads the same bytes; they differ only in speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Backend {
    /// Plain Rust, any processor.
    Portable,
    /// x86-64 SSE4.2: a row as two 128-bit vectors.
    Sse42,
    /// x86-64 AVX2: a row as one 256-bit vector.
    Avx2,
    /// x86-64 AVX-512: two rows of a group in one 512-bit vector.
    Avx512,
    /// `AArch64` NEON: a row as two 128-bit vectors.
    Neon,
}

/// Every backend, best first.
const BEST_FIRST: [Backend; 5] = [
    Backend::Avx512,
    Backend::Avx2,
    Backend::Neon,
    Backend::Sse42,
    Backend::Portable,
];

/// The most 32-bit groups AVX-512 unpacks faster than AVX2: a copy of this many
/// groups, in and out, still fits a first-level data cache (48 KiB).
const COPY_AVX512_GROUPS: usize = 24;

impl Backend {
    /// The backend this processor runs: the best of [`Backend::available`],
    /// found once.
    #[must_use]
    pub fn active() -> Self {
        static ACTIVE: OnceLock<Backend> = OnceLock::new();
        *ACTIVE.get_or_init(|| {
            BEST_FIRST
                .into_iter()
                .find(|b| b.is_available())
                .unwrap_or(Self::Portable)
        })
    }

    /// The backends this processor can run, best first.
    pub fn available() -> impl Iterator<Item = Self> {
        BEST_FIRST.into_iter().filter(|b| b.is_available())
    }

    /// Whether this processor has the instructions of the backend.
    #[must_use]
    pub fn is_available(self) -> bool {
        match self {
            Self::Portable => true,
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Sse42 => std::arch::is_x86_feature_detected!("sse4.2"),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx2 => std::arch::is_x86_feature_detected!("avx2"),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx512 => {
                std::arch::is_x86_feature_detected!("avx512f")
                    && std::arch::is_x86_feature_detected!("avx2")
            }
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            Self::Neon => std::arch::is_aarch64_feature_detected!("neon"),
            _ => false,
        }
    }

    /// Groups' numbers packed onto `out` ([`group_bytes`] of them each): `src`
    /// holds [`GROUP`] numbers per group, each fitting `width` bits (`1..=32`).
    ///
    /// # Panics
    ///
    /// If the backend is not [available](Self::is_available), `width` is out
    /// of range, or `out` is shorter than the groups' bytes.
    pub fn pack_groups(self, src: &[[u32; GROUP]], width: u32, out: &mut [u8]) {
        assert!(self.is_available(), "{self} is not available here");
        assert!((1..=32).contains(&width), "a group is 1 to 32 bits wide");
        let size = group_bytes(width);
        assert!(out.len() >= src.len() * size, "a group's bytes");
        let numbers = src.as_flattened();
        let mut group = 0;
        while group < src.len() {
            let backend = self.for_pack(width, src.len() - group);
            backend.pack_unchecked(&numbers[group * GROUP..], width, &mut out[group * size..]);
            group += backend.groups_per_call();
        }
    }

    /// Groups unpacked from `src` onto `out`.
    ///
    /// # Panics
    ///
    /// As [`pack_groups`](Self::pack_groups).
    pub fn unpack_groups(self, src: &[u8], width: u32, out: &mut [[u32; GROUP]]) {
        assert!(self.is_available(), "{self} is not available here");
        assert!((1..=32).contains(&width), "a group is 1 to 32 bits wide");
        let size = group_bytes(width);
        assert!(src.len() >= out.len() * size, "a group's bytes");
        let count = out.len();
        let numbers = out.as_flattened_mut();
        let mut group = 0;
        while group < count {
            let backend = self.for_unpack(width, count - group);
            backend.unpack_unchecked(&src[group * size..], width, &mut numbers[group * GROUP..]);
            group += backend.groups_per_call();
        }
    }

    /// The groups one call of the backend's kernels takes.
    fn groups_per_call(self) -> usize {
        match self {
            Self::Avx512 => 2,
            _ => 1,
        }
    }

    /// The backend to use for the next call of packing `width`-bit groups when
    /// `left` groups remain: this one, or a narrower one where it is faster
    /// (measured, `docs/performance.md`). The 512-bit kernels take two groups
    /// at a time, so a single group left goes to AVX2; and at 32 bits a group
    /// is a copy, which AVX2 does 1.2 to 1.5 times faster than shuffling two
    /// rows into one register and out again.
    fn for_pack(self, width: u32, left: usize) -> Self {
        match self {
            Self::Avx512 if left < self.groups_per_call() || width == u32::BITS => Self::Avx2,
            other => other,
        }
    }

    /// As [`for_pack`](Self::for_pack), for unpacking. At 32 bits AVX-512 is
    /// 1.6 to 2.2 times faster than AVX2 while the groups fit the first-level
    /// cache (up to [`COPY_AVX512_GROUPS`]), and no better beyond it, where
    /// the copy is bound by memory and the winner depends on how the buffers
    /// fall; AVX2 is the one that is never much worse.
    fn for_unpack(self, width: u32, left: usize) -> Self {
        match self {
            Self::Avx512
                if left < self.groups_per_call()
                    || (width == u32::BITS && left > COPY_AVX512_GROUPS) =>
            {
                Self::Avx2
            }
            other => other,
        }
    }

    /// The kernels' work for a backend known to be available and arguments
    /// known to fit: `groups_per_call` groups from the start of both.
    pub(super) fn pack_unchecked(self, src: &[u32], width: u32, out: &mut [u8]) {
        match self {
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Sse42 => super::sse42::pack(src, width, out),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx2 => super::avx2::pack(src, width, out),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx512 => super::avx512::pack(src, width, out),
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            Self::Neon => super::neon::pack(src, width, out),
            _ => portable::pack(src, width, out),
        }
    }

    /// The inverse of `pack_unchecked`.
    pub(super) fn unpack_unchecked(self, src: &[u8], width: u32, out: &mut [u32]) {
        match self {
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Sse42 => super::sse42::unpack(src, width, out),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx2 => super::avx2::unpack(src, width, out),
            #[cfg(all(target_arch = "x86_64", target_endian = "little"))]
            Self::Avx512 => super::avx512::unpack(src, width, out),
            #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
            Self::Neon => super::neon::unpack(src, width, out),
            _ => portable::unpack(src, width, out),
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Portable => "portable",
            Self::Sse42 => "SSE4.2",
            Self::Avx2 => "AVX2",
            Self::Avx512 => "AVX-512",
            Self::Neon => "NEON",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_512_bit_kernels_hand_a_lone_group_to_avx2() {
        assert_eq!(Backend::Avx512.for_pack(7, 1), Backend::Avx2);
        assert_eq!(Backend::Avx512.for_unpack(7, 1), Backend::Avx2);
        assert_eq!(Backend::Avx512.for_pack(7, 2), Backend::Avx512);
        assert_eq!(Backend::Avx512.for_unpack(7, 2), Backend::Avx512);
    }

    #[test]
    fn a_full_width_copy_is_packed_by_avx2_and_unpacked_by_avx512_while_it_fits_l1() {
        assert_eq!(Backend::Avx512.for_pack(32, 8), Backend::Avx2);
        assert_eq!(
            Backend::Avx512.for_unpack(32, COPY_AVX512_GROUPS),
            Backend::Avx512
        );
        assert_eq!(
            Backend::Avx512.for_unpack(32, COPY_AVX512_GROUPS + 1),
            Backend::Avx2
        );
    }

    #[test]
    fn narrower_backends_are_never_redirected() {
        for backend in [Backend::Portable, Backend::Avx2] {
            assert_eq!(backend.for_pack(32, 1), backend);
            assert_eq!(backend.for_unpack(32, 100), backend);
        }
    }
}
