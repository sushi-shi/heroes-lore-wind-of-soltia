//! Adler-32 cross-check oracle ("two implementations, one truth").
//!
//! The strict transliteration [`heroes_lore_wind_of_soltia_game_xlat::adler32`] (`an.class`) is diffed
//! against INDEPENDENT references over random inputs AND real asset bytes from
//! `_originals/…v207.jar`, plus the published RFC-1950 vectors (rulebook R1/R3).
//!
//! Two preserved defects of the shipped code are pinned here:
//!  * `Adler32.java:57` — `getValue()` sign-extends (`& -1L`), returning a
//!    negative `long` when bit 31 of the packed sum is set.
//!  * `Adler32.java:20` — the running sums accumulate into a **signed** `int`
//!    with the classic NMAX=5552 blocking that was designed for an *unsigned*
//!    accumulator; on large / high-byte inputs `sumB` overflows `i32::MAX` and
//!    Java's signed `%` diverges from true RFC Adler-32. The transliteration
//!    reproduces this exactly. The game only ever folds small per-frame IDATs
//!    (single block, no overflow), where Java == RFC.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::adler32::{self, Adler32State};

/// Reference #1 — textbook RFC-1950 Adler-32: reduce modulo 65521 after every
/// byte (a different loop from the transliteration's 5552-byte blocking, and
/// immune to the signed-overflow defect). This is the ground truth for the
/// no-overflow regime the game actually uses.
fn ref_adler32_rfc(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Reference #2 — an independent re-derivation of the *Java algorithm* itself:
/// signed `i32` accumulators, 5552-byte blocks (a flat block loop, structurally
/// unlike the transliteration's nested `remaining/block` loops). It reproduces
/// the signed-overflow behavior, so it must equal the transliteration on *every*
/// input, while equalling RFC only in the no-overflow regime.
fn ref_adler32_java(data: &[u8]) -> u32 {
    let mut sum_a: i32 = 1;
    let mut sum_b: i32 = 0;
    let mut idx = 0usize;
    let mut length = data.len() as i32;
    while length > 0 {
        let block = if length < 5552 { length } else { 5552 };
        length -= block;
        for _ in 0..block {
            sum_a = sum_a.wrapping_add(data[idx] as i32 & 255);
            sum_b = sum_b.wrapping_add(sum_a);
            idx += 1;
        }
        sum_a = sum_a.wrapping_rem(65521);
        sum_b = sum_b.wrapping_rem(65521);
    }
    (sum_b.wrapping_shl(16) | sum_a) as u32
}

/// The transliteration's checksum over the whole buffer, as unsigned 32-bit (the
/// low 32 bits of the possibly sign-extended `long` `getValue`).
fn translit_adler32(data: &[u8]) -> u32 {
    let d = to_i8(data);
    let mut s = Adler32State::new();
    adler32::update(&mut s, &d, 0, d.len() as i32);
    adler32::get_value(&s) as u32
}

/// Tiny deterministic PRNG (xorshift64*) so the random corpus is reproducible.
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

/// Below this length, a single-block Adler cannot overflow a signed `i32` for any
/// byte content, so Java == RFC. (Worst case all-`0xFF`: `sumB ≈ 255·L²/2`;
/// `< 2^31` for `L ≤ ~4095`.)
const NO_OVERFLOW_MAX: usize = 4000;

#[test]
fn adler32_agrees_with_rfc_on_published_vectors() {
    for (input, expected) in [
        ("".as_bytes(), 0x0000_0001u32),
        ("a".as_bytes(), 0x0062_0062),
        ("abc".as_bytes(), 0x024D_0127),
        ("Wikipedia".as_bytes(), 0x11E6_0398),
        ("123456789".as_bytes(), 0x091E_01DE),
    ] {
        assert_eq!(
            ref_adler32_rfc(input),
            expected,
            "RFC reference is itself wrong"
        );
        assert_eq!(
            ref_adler32_java(input),
            expected,
            "Java reference is itself wrong"
        );
        assert_eq!(
            translit_adler32(input),
            expected,
            "transliteration disagrees with published Adler-32 for {input:?}"
        );
    }
}

#[test]
fn adler32_agrees_with_rfc_in_the_no_overflow_regime() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut random_checked = 0usize;
    for &len in &[0usize, 1, 2, 3, 7, 15, 64, 255, 1000, 2048, NO_OVERFLOW_MAX] {
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_eq!(
            translit_adler32(&data),
            ref_adler32_rfc(&data),
            "transliteration vs RFC disagree on a random {len}-byte input (no-overflow regime)"
        );
        random_checked += 1;
    }
    assert!(
        random_checked >= 10,
        "only {random_checked} random inputs checked"
    );

    // Real asset bytes small enough to stay in the no-overflow regime (the game's
    // per-frame IDATs are this size). Every `.mph` plus small `.mpd` blobs.
    let blobs = jar().matching(|n| n.ends_with(".mpd") || n.ends_with(".mph"));
    let mut real_checked = 0usize;
    let mut nonzero = 0usize;
    for (name, bytes) in &blobs {
        if bytes.len() > NO_OVERFLOW_MAX {
            continue;
        }
        let t = translit_adler32(bytes);
        assert_eq!(
            t,
            ref_adler32_rfc(bytes),
            "transliteration vs RFC disagree on real blob {name} ({} bytes)",
            bytes.len()
        );
        if t != 0 && t != 1 {
            nonzero += 1;
        }
        real_checked += 1;
    }
    eprintln!(
        "[adler32 oracle] RFC agreement over {random_checked} random + {real_checked} real \
         blobs (<= {NO_OVERFLOW_MAX} B; {nonzero} non-trivial checksums)"
    );
    assert!(
        real_checked >= 170,
        "only {real_checked} real blobs in the no-overflow regime, below the corpus floor 170"
    );
    assert!(
        nonzero >= 100,
        "suspiciously few non-trivial checksums ({nonzero})"
    );
}

#[test]
fn adler32_reproduces_the_java_algorithm_including_signed_overflow() {
    let mut rng = Rng(0xF00D_CAFE_1234_5678);
    let mut overflow_seen = false;

    // Over inputs of every size, the transliteration must equal the independent
    // Java-faithful reference (structurally different, same signed-overflow).
    for &len in &[0usize, 100, 5551, 5552, 5553, 11104, 20000, 65535, 131072] {
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        assert_eq!(
            translit_adler32(&data),
            ref_adler32_java(&data),
            "transliteration diverges from the independent Java-algorithm reference at len {len}"
        );
        // Where the signed accumulator overflows, the transliteration must follow
        // Java (the defect), NOT silently compute the clean RFC value.
        if translit_adler32(&data) != ref_adler32_rfc(&data) {
            overflow_seen = true;
        }
    }
    assert!(
        overflow_seen,
        "no input triggered the signed-overflow divergence — the defect check is vacuous"
    );

    // Concrete pin: a large all-0xFF buffer overflows Java's signed block sum, so
    // the transliteration's result differs from true RFC Adler-32.
    let big = vec![0xFFu8; 20000];
    let t = translit_adler32(&big);
    assert_eq!(
        t,
        ref_adler32_java(&big),
        "translit != Java-reference on all-0xFF"
    );
    assert_ne!(
        t,
        ref_adler32_rfc(&big),
        "the signed-overflow defect was silently 'fixed' to RFC Adler-32"
    );
}

#[test]
fn adler32_preserves_the_sign_extension_defect() {
    // In the no-overflow regime, find an input whose true checksum has bit 31 set.
    // `getValue` must then return a NEGATIVE long, while its low 32 bits still
    // equal the true unsigned checksum (Adler32.java:57).
    let mut rng = Rng(0xDEAD_BEEF_0BAD_F00D);
    let mut found = false;
    for _ in 0..4000 {
        let len = 1000 + (rng.next_u64() % (NO_OVERFLOW_MAX as u64 - 1000)) as usize;
        let data: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let r = ref_adler32_rfc(&data);
        if r & 0x8000_0000 != 0 {
            let d = to_i8(&data);
            let mut s = Adler32State::new();
            adler32::update(&mut s, &d, 0, d.len() as i32);
            let raw = adler32::get_value(&s); // i64
            assert!(raw < 0, "bit 31 set but getValue returned {raw:#x} (>= 0)");
            assert_eq!(
                raw as u32, r,
                "sign-extended low 32 bits must equal the checksum"
            );
            assert_ne!(
                raw, r as i64,
                "defect vacuous: value equals the zero-extended form"
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "no bit-31 input found — the defect check never ran (vacuous)"
    );
}

/// Negative control (R3): a one-byte change must move the checksum.
#[test]
fn adler32_oracle_negative_control() {
    let base = b"the quick brown fox".to_vec();
    let mut mutated = base.clone();
    mutated[0] = mutated[0].wrapping_add(1);
    assert_ne!(
        translit_adler32(&base),
        translit_adler32(&mutated),
        "a one-byte change left the checksum unchanged — the oracle is blind"
    );
    assert_ne!(ref_adler32_rfc(&base), ref_adler32_rfc(&mutated));
}
