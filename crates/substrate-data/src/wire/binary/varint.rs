//! Varints (LEB128) and zigzag: how numbers are written.

/// The low seven bits of a varint byte, and the bit saying more follow.
pub(crate) const VARINT_BITS: u64 = 0x7F;
pub(crate) const VARINT_MORE: u8 = 0x80;
pub(crate) const VARINT_SHIFT: u32 = 7;

/// `n` as a varint onto `out`.
pub(crate) fn varint(out: &mut Vec<u8>, mut n: u64) {
    while n > VARINT_BITS {
        out.push(low_byte(n) | VARINT_MORE);
        n >>= VARINT_SHIFT;
    }
    out.push(low_byte(n));
}

/// The low byte of `n`.
fn low_byte(n: u64) -> u8 {
    n.to_le_bytes()[0]
}

/// The bits a varint can hold.
pub(crate) const VARINT_MAX_SHIFT: u32 = 64;

/// The shift of a varint's tenth byte: the one bit left of 64.
pub(crate) const VARINT_LAST_SHIFT: u32 = 63;

/// The varint at `*i` in `b`, `*i` moved past it; `None` cut short or longer than 64 bits.
pub(crate) fn read_varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let mut n = 0u64;
    let mut shift = 0;
    while shift < VARINT_MAX_SHIFT {
        let byte = *b.get(*i)?;
        *i += 1;
        let bits = u64::from(byte) & VARINT_BITS;
        // The last byte has room for one bit: more would be lost, not read.
        if shift == VARINT_LAST_SHIFT && bits > 1 {
            return None;
        }
        n |= bits << shift;
        if byte & VARINT_MORE == 0 {
            return Some(n);
        }
        shift += VARINT_SHIFT;
    }
    None
}

/// `n` zigzagged: small negative numbers stay small varints.
pub(crate) fn zigzag(n: i64) -> u64 {
    ((n << 1) ^ (n >> 63)).cast_unsigned()
}

/// A zigzagged `z` back.
pub(crate) fn unzigzag(z: u64) -> i64 {
    (z >> 1).cast_signed() ^ -(z & 1).cast_signed()
}
