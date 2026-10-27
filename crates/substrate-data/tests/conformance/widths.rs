//! Frame cases for bit-packed int lists: widths and group boundaries.
//!
//! A group is 256 numbers: below that count the numbers are packed one
//! after another, little-endian bits; a full group is packed in 8 lanes of
//! 32-bit words, for widths up to 32. Wider numbers are never grouped.

use crate::case::Case;
use crate::typed::typed_case;

/// Numbers in a group.
const GROUP: usize = 256;

/// The `n` numbers of a list of exactly `width` bits (the first is all ones).
fn numbers(n: usize, width: u32) -> Vec<u64> {
    let mask = (1u64 << width) - 1;
    (0..n)
        .map(|i| {
            if i == 0 {
                mask
            } else {
                ((i as u64).wrapping_mul(2_654_435_761).wrapping_add(12_345)) & mask
            }
        })
        .collect()
}

/// Every case of this module.
pub(crate) fn cases() -> Vec<Case> {
    let mut all = Vec::new();
    for width in [1u32, 7, 8, 13] {
        for n in [GROUP - 1, GROUP, GROUP + 1] {
            all.push(width_case(width, n));
        }
    }
    for n in [3, GROUP, GROUP + 1] {
        all.push(width_case(32, n));
    }
    for (width, n) in [(33u32, 5), (33, GROUP + 1), (48, 5), (63, 5)] {
        all.push(wide_case(width, n));
    }
    all.extend(signed());
    all
}

fn width_case(width: u32, n: usize) -> Case {
    let items: Vec<u32> = numbers(n, width)
        .into_iter()
        .map(|v| u32::try_from(v).expect("fits"))
        .collect();
    let group = if n >= GROUP {
        "a full group in lanes, then the rest one by one"
    } else {
        "no full group: numbers one after another"
    };
    typed_case(
        &format!("frame_ints_w{width}_n{n}"),
        &format!("{n} unsigned ints of {width} bits: count, the width byte, then {group}."),
        "uint_list",
        &items,
    )
}

fn wide_case(width: u32, n: usize) -> Case {
    typed_case(
        &format!("frame_ints_w{width}_n{n}"),
        &format!(
            "{n} unsigned ints of {width} bits: wider than 32 bits never use lanes, the numbers follow one another."
        ),
        "ulong_list",
        &numbers(n, width),
    )
}

fn signed() -> Vec<Case> {
    vec![
        typed_case(
            "frame_ints_empty",
            "No ints: count 0, width 1, no bytes.",
            "uint_list",
            &Vec::<u32>::new(),
        ),
        typed_case(
            "frame_ints_one",
            "One int: count 1, width 1.",
            "uint_list",
            &vec![1u32],
        ),
        typed_case(
            "frame_ints_signed",
            "Signed ints are zigzagged before they are packed (-1 is 1, 1 is 2, -3 is 5, 70000 is 140000), and the width is that of the biggest.",
            "int_list",
            &vec![-3i32, 5, -70_000, 0, i32::MIN, i32::MAX],
        ),
    ]
}
