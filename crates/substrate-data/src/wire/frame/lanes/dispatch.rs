//! The width of a group is a number from 1 to 32; the kernels want it as a
//! constant, so one `match` picks the instance, and a backend's two entry
//! points are made from its row type and its instruction set.

/// `$f::<width>($args)` for a run-time `$width` in `1..=32`.
macro_rules! by_width {
    ($width:expr, $f:ident($($arg:expr),*)) => {
        match $width {
            1 => $f::<1>($($arg),*),
            2 => $f::<2>($($arg),*),
            3 => $f::<3>($($arg),*),
            4 => $f::<4>($($arg),*),
            5 => $f::<5>($($arg),*),
            6 => $f::<6>($($arg),*),
            7 => $f::<7>($($arg),*),
            8 => $f::<8>($($arg),*),
            9 => $f::<9>($($arg),*),
            10 => $f::<10>($($arg),*),
            11 => $f::<11>($($arg),*),
            12 => $f::<12>($($arg),*),
            13 => $f::<13>($($arg),*),
            14 => $f::<14>($($arg),*),
            15 => $f::<15>($($arg),*),
            16 => $f::<16>($($arg),*),
            17 => $f::<17>($($arg),*),
            18 => $f::<18>($($arg),*),
            19 => $f::<19>($($arg),*),
            20 => $f::<20>($($arg),*),
            21 => $f::<21>($($arg),*),
            22 => $f::<22>($($arg),*),
            23 => $f::<23>($($arg),*),
            24 => $f::<24>($($arg),*),
            25 => $f::<25>($($arg),*),
            26 => $f::<26>($($arg),*),
            27 => $f::<27>($($arg),*),
            28 => $f::<28>($($arg),*),
            29 => $f::<29>($($arg),*),
            30 => $f::<30>($($arg),*),
            31 => $f::<31>($($arg),*),
            32 => $f::<32>($($arg),*),
            other => unreachable!("a group is 1 to 32 bits wide, not {other}"),
        }
    };
}

/// `pack` and `unpack` for a backend: the generic kernels, compiled with
/// `$feature` enabled, the row type `$row`.
macro_rules! simd_kernels {
    ($row:ty, $feature:literal) => {
        #[target_feature(enable = $feature)]
        unsafe fn pack_width<const W: u32>(src: &[u32], out: &mut [u8]) {
            // SAFETY: only called through `pack`, which `Backend` chooses
            // where the processor has `$feature`.
            unsafe { kernel::pack::<$row, W>(src, out) }
        }

        #[target_feature(enable = $feature)]
        unsafe fn unpack_width<const W: u32>(src: &[u8], out: &mut [u32]) {
            // SAFETY: as `pack_width`.
            unsafe { kernel::unpack::<$row, W>(src, out) }
        }

        /// The groups one call takes, packed; callable only where the processor has the feature.
        pub(super) fn pack(src: &[u32], width: u32, out: &mut [u8]) {
            // SAFETY: `Backend` chooses this backend only where the
            // processor has the feature.
            unsafe { by_width!(width, pack_width(src, out)) }
        }

        /// The groups one call takes, unpacked; callable only where the processor has the feature.
        pub(super) fn unpack(src: &[u8], width: u32, out: &mut [u32]) {
            // SAFETY: as `pack`.
            unsafe { by_width!(width, unpack_width(src, out)) }
        }
    };
}

pub(super) use by_width;
pub(super) use simd_kernels;
