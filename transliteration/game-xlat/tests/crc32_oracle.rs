//! CRC-32 cross-check oracle ("two implementations, one truth").
//!
//! The strict transliteration [`heroes_lore_wind_of_soltia_game_xlat::crc32`] (`ca.class`, the 256-entry
//! table-driven zlib CRC-32) is diffed against an INDEPENDENT, structurally
//! different reference (the bit-at-a-time reflected CRC, no lookup table) plus the
//! published test vectors, over random inputs AND real asset bytes from
//! `_originals/…v207.jar`. Both must agree byte-for-byte (rulebook R1/R3). This is
//! the CRC PngMerger uses for every PNG chunk it emits.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::crc32::{self, Crc32State};

/// Independent reference: bit-at-a-time reflected CRC-32 (polynomial 0xEDB88320),
/// no lookup table — a different computation from the table-driven transliteration.
fn ref_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// The transliteration's finished CRC over the whole buffer, as unsigned 32-bit.
fn translit_crc32(data: &[u8]) -> u32 {
    let d = to_i8(data);
    let mut s = Crc32State::new();
    crc32::update(&mut s, &d, 0, d.len() as i32);
    crc32::get_value(&s) as u32
}

/// Tiny deterministic PRNG (xorshift64*), reproducible random corpus.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}

#[test]
fn crc32_agrees_with_reference_on_published_vectors() {
    // Canonical published zlib / ISO-3309 CRC-32 values.
    for (input, expected) in [
        ("".as_bytes(), 0x0000_0000u32),
        ("a".as_bytes(), 0xE8B7_BE43),
        ("abc".as_bytes(), 0x3524_41C2),
        ("123456789".as_bytes(), 0xCBF4_3926), // the standard CRC-32 "check" value
        (
            "The quick brown fox jumps over the lazy dog".as_bytes(),
            0x414F_A339,
        ),
    ] {
        assert_eq!(
            ref_crc32(input),
            expected,
            "reference disagrees with a published vector — the oracle's second \
             implementation is itself wrong"
        );
        assert_eq!(
            translit_crc32(input),
            expected,
            "transliteration disagrees with published CRC-32 for {input:?}"
        );
    }
}

#[test]
fn crc32_agrees_with_reference_on_random_and_real_inputs() {
    let mut rng = Rng(0xB5AD_4ECE_DA1E_1DDD);
    let mut random_checked = 0usize;
    for &len in &[0usize, 1, 2, 3, 7, 15, 64, 255, 1000, 4096, 20000, 65535] {
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_eq!(
            translit_crc32(&data),
            ref_crc32(&data),
            "transliteration vs reference disagree on a random {len}-byte input"
        );
        random_checked += 1;
    }
    assert!(
        random_checked >= 10,
        "only {random_checked} random inputs checked — below the liveness floor"
    );

    // Real asset bytes: every `.mph` and `.mpd` atlas blob.
    let blobs = jar().matching(|n| n.ends_with(".mpd") || n.ends_with(".mph"));
    let mut real_checked = 0usize;
    let mut nonzero = 0usize;
    for (name, bytes) in &blobs {
        let t = translit_crc32(bytes);
        assert_eq!(
            t,
            ref_crc32(bytes),
            "transliteration vs reference disagree on real blob {name}"
        );
        if t != 0 {
            nonzero += 1;
        }
        real_checked += 1;
    }
    eprintln!(
        "[crc32 oracle] agreed on {random_checked} random + {real_checked} real \
         atlas blobs ({nonzero} non-zero CRCs)"
    );
    assert!(
        real_checked >= 340,
        "only {real_checked} real atlas blobs processed, below floor 340 (170 .mpd + 170 .mph)"
    );
    assert!(
        nonzero >= 300,
        "suspiciously few non-zero CRCs ({nonzero}) — the CRC may be a no-op"
    );
}

/// Negative control (R3): the comparison has teeth — a one-byte change moves the
/// transliterated CRC (a CRC that ignored its input would pass this vacuously).
#[test]
fn crc32_oracle_negative_control() {
    let base = b"the quick brown fox".to_vec();
    let mut mutated = base.clone();
    mutated[3] = mutated[3].wrapping_add(1);
    assert_ne!(
        translit_crc32(&base),
        translit_crc32(&mutated),
        "a one-byte input change left the CRC unchanged — the oracle is blind"
    );
    assert_ne!(ref_crc32(&base), ref_crc32(&mutated));
}
