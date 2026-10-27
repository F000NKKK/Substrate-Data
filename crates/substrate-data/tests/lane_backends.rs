//! Every instruction set that packs a group writes the bytes the portable
//! code writes and reads them back.

use substrate_data::frame::lanes::{Backend, GROUP, group_bytes};

fn mask(width: u32) -> u32 {
    u32::MAX >> (u32::BITS - width)
}

/// Groups of every flavour a width can see: noise, all ones, zeros, a ramp,
/// one bit set in each lane in turn.
fn groups(width: u32) -> Vec<[u32; GROUP]> {
    let m = mask(width);
    let noise = |seed: u32| -> [u32; GROUP] {
        let mut x = seed;
        std::array::from_fn(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x & m
        })
    };
    vec![
        noise(0x9E37_79B9),
        noise(1),
        [m; GROUP],
        [0; GROUP],
        std::array::from_fn(|i| u32::try_from(i).expect("a small index") & m),
        std::array::from_fn(|i| (1u32 << (i % width as usize)) & m),
    ]
}

#[test]
fn the_portable_backend_is_always_there_and_the_active_one_is_available() {
    assert!(Backend::Portable.is_available());
    assert!(Backend::active().is_available());
    assert_eq!(Backend::available().next(), Some(Backend::active()));
    assert_eq!(Backend::available().last(), Some(Backend::Portable));
}

#[test]
fn every_available_backend_packs_the_portable_bytes_and_unpacks_them() {
    for backend in Backend::available() {
        for width in 1..=32 {
            let size = group_bytes(width);
            for count in [1, 2, 3, 5] {
                let src: Vec<[u32; GROUP]> =
                    groups(width).into_iter().cycle().take(count).collect();
                let (mut want, mut got) = (vec![0; count * size], vec![0xAA; count * size]);
                Backend::Portable.pack_groups(&src, width, &mut want);
                backend.pack_groups(&src, width, &mut got);
                assert_eq!(got, want, "{backend} packs {count} groups of width {width}");
                let mut back = vec![[0xFFFF_FFFF; GROUP]; count];
                backend.unpack_groups(&want, width, &mut back);
                assert_eq!(
                    back, src,
                    "{backend} unpacks {count} groups of width {width}"
                );
            }
        }
    }
}

#[test]
fn a_backend_reads_groups_inside_a_longer_buffer() {
    for backend in Backend::available() {
        for width in [1, 5, 13, 32] {
            let src = groups(width).into_iter().take(3).collect::<Vec<_>>();
            let size = 3 * group_bytes(width);
            let mut buf = vec![0x55; size + 7];
            backend.pack_groups(&src, width, &mut buf[..size]);
            let mut back = vec![[0; GROUP]; 3];
            backend.unpack_groups(&buf, width, &mut back);
            assert_eq!(back, src, "{backend} width {width}");
            assert_eq!(&buf[size..], &[0x55; 7], "{backend} stays in bounds");
        }
    }
}

#[test]
#[should_panic(expected = "a group's bytes")]
fn a_short_output_is_refused() {
    Backend::active().pack_groups(&[[0; GROUP]], 8, &mut [0; 8]);
}

#[test]
#[should_panic(expected = "1 to 32 bits")]
fn a_width_out_of_range_is_refused() {
    Backend::active().pack_groups(&[[0; GROUP]], 33, &mut [0; 4096]);
}

#[test]
fn a_backend_names_itself() {
    let names: Vec<_> = [
        Backend::Portable,
        Backend::Sse42,
        Backend::Avx2,
        Backend::Avx512,
        Backend::Neon,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(names, ["portable", "SSE4.2", "AVX2", "AVX-512", "NEON"]);
}
