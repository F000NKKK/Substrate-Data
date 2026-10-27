//! What a kernel needs of a row: [`LANES`](super::LANES) 32-bit words held in
//! the registers of one instruction set.
#![expect(
    unsafe_code,
    reason = "register loads and stores through pointers the kernels bounds-check"
)]

/// A row of a group as a processor holds it. Using one is `unsafe`: the
/// processor must have the instruction set it is made of.
/// Every method is `#[inline(always)]`
/// in its implementation so a kernel compiled for the instruction set takes it
/// in whole.
pub(super) trait Row: Copy {
    /// How many consecutive groups a row holds a row of (the wider vectors
    /// take two groups' rows side by side).
    const GROUPS: usize;

    /// All lanes zero.
    ///
    /// # Safety
    ///
    /// The processor supports the instructions the row is made of (every
    /// method has this requirement; the ones that take pointers add theirs).
    unsafe fn zero() -> Self;
    /// All lanes `v`.
    unsafe fn splat(v: u32) -> Self;
    /// The rows at `p` (eight words each, any alignment), the next group's `stride` words on.
    ///
    /// # Safety
    ///
    /// `p` is valid for reading eight words, and again at `p + stride` when
    /// the row holds more than one group.
    unsafe fn load_words(p: *const u32, stride: usize) -> Self;
    /// The rows from eight little-endian words of bytes at `p`, the next group's `stride` bytes on.
    ///
    /// # Safety
    ///
    /// `p` is valid for reading 32 bytes, and again at `p + stride` when the
    /// row holds more than one group.
    unsafe fn load_bytes(p: *const u8, stride: usize) -> Self;
    /// Writes the rows to `p` as eight words each, the next group's at `p + stride`.
    ///
    /// # Safety
    ///
    /// `p` is valid for writing eight words, and again at `p + stride` when
    /// the row holds more than one group.
    unsafe fn store_words(self, p: *mut u32, stride: usize);
    /// Writes the rows as 32 little-endian bytes each at `p`, the next group's at `p + stride`.
    ///
    /// # Safety
    ///
    /// `p` is valid for writing 32 bytes, and again at `p + stride` when the
    /// row holds more than one group.
    unsafe fn store_bytes(self, p: *mut u8, stride: usize);
    /// Lane-wise or.
    unsafe fn or(self, other: Self) -> Self;
    /// Lane-wise and.
    unsafe fn and(self, other: Self) -> Self;
    /// Every lane shifted left by `n < 32` bits.
    unsafe fn shl(self, n: u32) -> Self;
    /// Every lane shifted right by `n < 32` bits.
    unsafe fn shr(self, n: u32) -> Self;
}
