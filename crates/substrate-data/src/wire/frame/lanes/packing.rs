//! Packing and unpacking: groups through the processor's kernels, the rest one by one.

#[cfg(target_arch = "x86_64")]
use super::avx2;
use super::portable;
use crate::sync::OnceLock;

/// Lanes of a group: a 256-bit vector of 32-bit words.
pub const LANES: usize = 8;
/// Numbers a lane holds in a group: as many as its word has bits.
pub(super) const ROWS: usize = u32::BITS as usize;
/// Numbers in a group.
pub const GROUP: usize = LANES * ROWS;
/// The widest numbers packed in groups.
pub const GROUP_MAX_BITS: u32 = u32::BITS;
pub(super) const WORD_BYTES: usize = 4;
const BYTE_BITS: usize = 8;

/// A group's bytes, its numbers `width` bits each.
#[must_use]
pub const fn group_bytes(width: u32) -> usize {
    LANES * width as usize * WORD_BYTES
}

/// The bytes `n` numbers `width` bits each take.
#[must_use]
pub fn packed_bytes(n: usize, width: u32) -> Option<usize> {
    let (groups, rest) = split(n, width);
    groups
        .checked_mul(group_bytes(width))?
        .checked_add(rest.checked_mul(width as usize)?.div_ceil(BYTE_BITS))
}

/// How many whole groups `n` numbers make, and how many go after them.
fn split(n: usize, width: u32) -> (usize, usize) {
    if width > GROUP_MAX_BITS {
        (0, n)
    } else {
        (n / GROUP, n % GROUP)
    }
}

/// One group packed: `src`'s numbers (each fitting `width` bits) onto
/// `out` ([`group_bytes`] long).
type PackFn = fn(&[u32; GROUP], u32, &mut [u8]);
/// One group unpacked.
type UnpackFn = fn(&[u8], u32, &mut [u32; GROUP]);

/// This processor's kernels: AVX2 if it has it, chosen once.
fn kernels() -> &'static (PackFn, UnpackFn) {
    static KERNELS: OnceLock<(PackFn, UnpackFn)> = OnceLock::new();
    KERNELS.get_or_init(|| {
        #[cfg(target_arch = "x86_64")]
        if std::arch::is_x86_feature_detected!("avx2") {
            return (avx2::pack, avx2::unpack);
        }
        (portable::pack, portable::unpack)
    })
}

/// The low 32 bits of `v` (a grouped number fits them: groups are only
/// packed up to 32 bits wide).
fn low_word(v: u64) -> u32 {
    u32::try_from(v & u64::from(u32::MAX)).unwrap_or(u32::MAX)
}

/// The low 64 bits of `v`.
fn low_u64(v: u128) -> u64 {
    u64::try_from(v & u128::from(u64::MAX)).unwrap_or(u64::MAX)
}

/// A word's mask of `width` low bits.
pub(super) fn mask(width: u32) -> u32 {
    u32::MAX >> (u32::BITS - width)
}

/// Packs `items` (`n` of them, each fitting `width` bits) onto `out`.
pub fn pack(out: &mut Vec<u8>, n: usize, width: u32, items: impl Iterator<Item = u64>) {
    let (groups, rest) = split(n, width);
    let start = out.len();
    out.resize(start + packed_bytes(n, width).unwrap_or(0), 0);
    let mut items = items;
    let (pack_group, _) = kernels();
    let mut group = [0u32; GROUP];
    let mut at = start;
    for _ in 0..groups {
        for (slot, v) in group.iter_mut().zip(items.by_ref()) {
            *slot = low_word(v);
        }
        let bytes = group_bytes(width);
        pack_group(&group, width, &mut out[at..at + bytes]);
        at += bytes;
    }
    pack_sequential(&mut out[at..], rest, width, items);
}

/// Numbers one after another, little-endian bits.
fn pack_sequential(out: &mut [u8], n: usize, width: u32, items: impl Iterator<Item = u64>) {
    let mut acc: u128 = 0;
    let mut have = 0u32;
    let mut at = 0usize;
    for v in items.take(n) {
        acc |= u128::from(v) << have;
        have += width;
        while have >= u8::BITS {
            out[at] = acc.to_le_bytes()[0];
            at += 1;
            acc >>= u8::BITS;
            have -= u8::BITS;
        }
    }
    if have > 0 {
        out[at] = acc.to_le_bytes()[0];
    }
}

/// Each of the `n` numbers in `bytes` (`width` bits each), in order, to `f`.
pub fn unpack(bytes: &[u8], n: usize, width: u32, mut f: impl FnMut(u64)) {
    let (groups, rest) = split(n, width);
    let (_, unpack_group) = kernels();
    let mut group = [0u32; GROUP];
    let size = group_bytes(width);
    for b in 0..groups {
        unpack_group(&bytes[b * size..(b + 1) * size], width, &mut group);
        for &v in &group {
            f(u64::from(v));
        }
    }
    let tail = &bytes[groups * size..];
    for i in 0..rest {
        f(sequential(tail, i, width));
    }
}

/// Number `i` of `bytes` (`n` numbers, `width` bits each) — `i < n`.
#[must_use]
pub fn get(bytes: &[u8], n: usize, width: u32, i: usize) -> u64 {
    let (groups, _) = split(n, width);
    if i >= groups * GROUP {
        let tail = &bytes[groups * group_bytes(width)..];
        return sequential(tail, i - groups * GROUP, width);
    }
    let (group, j) = (i / GROUP, i % GROUP);
    let (lane, row) = (j % LANES, j / LANES);
    let bit = row * width as usize;
    let (word, shift) = (bit / u32::BITS as usize, bit % u32::BITS as usize);
    let at = group * group_bytes(width);
    let word_at = |k: usize| {
        let o = at + (k * LANES + lane) * WORD_BYTES;
        bytes.get(o..o + WORD_BYTES).map_or(0, |b| {
            u64::from(u32::from_le_bytes(b.try_into().unwrap_or_default()))
        })
    };
    let two = word_at(word)
        | if shift + width as usize > u32::BITS as usize {
            word_at(word + 1) << u32::BITS
        } else {
            0
        };
    (two >> shift) & (u64::MAX >> (u64::BITS - width))
}

/// Number `i` of numbers packed one after another.
fn sequential(bytes: &[u8], i: usize, width: u32) -> u64 {
    // A number spans at most 9 bytes (64 bits starting anywhere in a byte).
    const SPAN: usize = 9;
    let bit = i * width as usize;
    let (byte, shift) = (bit / BYTE_BITS, bit % BYTE_BITS);
    let mut window = [0u8; 16];
    let end = (byte + SPAN).min(bytes.len());
    if byte < end {
        window[..end - byte].copy_from_slice(&bytes[byte..end]);
    }
    let mask = u128::from(u64::MAX) >> (u64::BITS - width);
    low_u64((u128::from_le_bytes(window) >> shift) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(n: usize, width: u32) {
        let max = u64::MAX >> (u64::BITS - width);
        let items: Vec<u64> = (0..n as u64)
            .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15) & max)
            .collect();
        let mut out = Vec::new();
        pack(&mut out, n, width, items.iter().copied());
        assert_eq!(Some(out.len()), packed_bytes(n, width));
        let mut back = Vec::new();
        unpack(&out, n, width, |v| back.push(v));
        assert_eq!(back, items, "n {n}, width {width}");
        for (i, v) in items.iter().enumerate() {
            assert_eq!(
                get(&out, n, width, i),
                *v,
                "get {i} of n {n}, width {width}"
            );
        }
    }

    #[test]
    fn every_width_round_trips_in_groups_and_tail() {
        for width in 1..=u64::BITS {
            for n in [0, 1, 7, GROUP - 1, GROUP, GROUP + 3, 3 * GROUP + 100] {
                roundtrip(n, width);
            }
        }
    }

    #[test]
    fn the_kernels_agree() {
        let (pack_group, unpack_group) = *kernels();
        let src: [u32; GROUP] =
            std::array::from_fn(|i| u32::try_from(i).unwrap().wrapping_mul(2_654_435_761));
        for width in 1..=GROUP_MAX_BITS {
            let src = src.map(|v| v & mask(width));
            let (mut a, mut b) = (vec![0; group_bytes(width)], vec![0; group_bytes(width)]);
            portable::pack(&src, width, &mut a);
            pack_group(&src, width, &mut b);
            assert_eq!(a, b, "width {width}");
            let (mut x, mut y) = ([0; GROUP], [0; GROUP]);
            portable::unpack(&a, width, &mut x);
            unpack_group(&a, width, &mut y);
            assert_eq!((x, y), (src, src), "width {width}");
        }
    }
}
