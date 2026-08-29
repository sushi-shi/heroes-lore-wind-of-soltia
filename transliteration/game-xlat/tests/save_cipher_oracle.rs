//! SaveCipher cross-check oracle ("two implementations, one truth").
//!
//! The strict transliteration [`heroes_lore_wind_of_soltia_game_xlat::save_cipher`]
//! (`bq.class`, the rolling XOR + additive-checksum save scrambler) is diffed
//! against an INDEPENDENT reference of the same algorithm — one that derives the
//! per-byte key index in closed form `(i + 1) % key.length` instead of the
//! original's pre-increment/wrap loop — over random inputs AND real asset bytes
//! from `_originals/…v207.jar` used as plaintext, under the game's real save key
//! and random keys. Both must agree byte-for-byte (rulebook R1/R3). Plus:
//! `decrypt(encrypt(p))` recovers `p`, and a corrupted trailing checksum is
//! rejected.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::save_cipher::{decrypt, encrypt};

/// Independent reference for `encrypt`: a structurally-different derivation of the
/// same keystream (closed-form index `(i + 1) % key.len()`), never the original's
/// stateful pre-increment loop.
fn ref_encrypt(plain: &[i8], key: &[i8]) -> Vec<i8> {
    assert!(!key.is_empty(), "reference requires a non-empty key");
    let mut out = vec![0i8; plain.len() + 1];
    let mut checksum: i32 = 0;
    for (i, &p) in plain.iter().enumerate() {
        let ki = (i + 1) % key.len();
        let kb = key[ki];
        out[i] = ((p as i32) ^ (kb as i32)) as i8;
        checksum = checksum.wrapping_add((kb as i32) & 0xFF);
    }
    out[plain.len()] = checksum as i8;
    out
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
    fn byte(&mut self) -> i8 {
        (self.next_u64() >> 33) as u8 as i8
    }
}

/// The game's real save key (`GameState.saveKey = {5, 11, 8, 81, 3, 20}`).
const SAVE_KEY: [i8; 6] = [5, 11, 8, 81, 3, 20];

#[test]
fn encrypt_agrees_with_independent_reference_and_round_trips() {
    let mut rng = Rng(0x51A5_7E5A_1234_9876);
    let mut checked = 0usize;

    // Random plaintexts under random non-empty keys.
    for &len in &[0usize, 1, 2, 3, 5, 6, 7, 16, 64, 255, 1000] {
        for &klen in &[1usize, 2, 6, 17] {
            let plain: Vec<i8> = (0..len).map(|_| rng.byte()).collect();
            let key: Vec<i8> = (0..klen).map(|_| rng.byte()).collect();

            // Two implementations, one truth.
            let t = encrypt(&plain, &key);
            let r = ref_encrypt(&plain, &key);
            assert_eq!(
                t, r,
                "transliteration vs reference disagree on len {len}, klen {klen}"
            );

            // Round-trip: decrypt recovers the plaintext in the leading bytes.
            // (decrypt returns cipher.len()+1 == plain.len()+2 bytes; only the
            // first plain.len() are meaningful — a preserved quirk.)
            let back = decrypt(&t, &key).expect("valid checksum must decrypt");
            assert_eq!(&back[0..plain.len()], &plain[..], "round-trip lost data");

            checked += 1;
        }
    }
    assert!(
        checked >= 30,
        "only {checked} cases — below the liveness floor"
    );
}

#[test]
fn encrypt_agrees_on_real_asset_bytes_as_plaintext() {
    // Real bytes: every atlas blob, ciphered under the real save key and a fixed
    // alternate key. Not what the game ciphers (it ciphers save records), but real
    // non-trivial bytes exercising the algorithm at scale.
    let blobs = jar().matching(|n| n.ends_with(".mpd") || n.ends_with(".mph"));
    let alt_key: [i8; 4] = [-1, 0, 127, -128];
    let mut real_checked = 0usize;

    for (name, bytes) in &blobs {
        let plain = to_i8(bytes);
        for key in [SAVE_KEY.as_slice(), alt_key.as_slice()] {
            let t = encrypt(&plain, key);
            let r = ref_encrypt(&plain, key);
            assert_eq!(t, r, "transliteration vs reference disagree on {name}");
            let back = decrypt(&t, key).expect("real-bytes round-trip");
            assert_eq!(&back[0..plain.len()], &plain[..]);
        }
        real_checked += 1;
    }
    eprintln!("[save_cipher oracle] agreed on {real_checked} real atlas blobs (×2 keys)");
    assert!(
        real_checked >= 170,
        "only {real_checked} real blobs processed, below floor 170"
    );
}

#[test]
fn decrypt_rejects_a_corrupted_checksum() {
    let plain: Vec<i8> = vec![10, 20, 30, 40, 50];
    let key: Vec<i8> = SAVE_KEY.to_vec();
    let mut cipher = encrypt(&plain, &key);

    // Intact → decrypts.
    assert!(decrypt(&cipher, &key).is_some());

    // Corrupt the trailing checksum byte → rejected (returns null → None).
    let last = cipher.len() - 1;
    cipher[last] = cipher[last].wrapping_add(1);
    assert!(
        decrypt(&cipher, &key).is_none(),
        "a corrupted checksum was accepted — the guard is dead"
    );

    // Empty input → null.
    assert!(decrypt(&[], &key).is_none());
    // Empty key → the try/catch path returns null (the key access throws).
    assert!(decrypt(&[1, 2, 3], &[]).is_none());
}

#[test]
fn negative_control_a_plaintext_change_moves_the_ciphertext() {
    // The oracle has teeth: a one-byte plaintext change changes the ciphertext in
    // both implementations (a cipher that ignored its input would pass vacuously).
    let key: Vec<i8> = SAVE_KEY.to_vec();
    let base: Vec<i8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut mutated = base.clone();
    mutated[3] = mutated[3].wrapping_add(1);

    assert_ne!(encrypt(&base, &key), encrypt(&mutated, &key));
    assert_ne!(ref_encrypt(&base, &key), ref_encrypt(&mutated, &key));
}

#[test]
#[should_panic]
fn encrypt_with_empty_key_panics_unguarded() {
    // encrypt has no try/catch: keyIndex++ → 1, key[1] on an empty key is an
    // uncaught ArrayIndexOutOfBoundsException (faithful panic).
    let _ = encrypt(&[1, 2, 3], &[]);
}
