//! Transliterated from `java/src/main/java/defpackage/SaveCipher.java`
//! (original `bq.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The obfuscation cipher applied to every RMS save blob: a rolling XOR against a
//! repeating key with a trailing one-byte additive checksum. The key index is
//! advanced *before* each byte (so the first plaintext byte is XORed with
//! `key[1]`, not `key[0]`) and wraps to 0 when it reaches `key.length`; the
//! checksum accumulates `key[keyIndex] & 0xFF` and is appended as the final byte.
//!
//! A purely-static utility class (no fields) → free functions with no state
//! (`docs/TRANSLITERATION.md`, *Statics and ownership*).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bq.a:([B[B)[B => iadd,iinc,ixor,iand,iadd,i2b,iinc,i2b` (encrypt),
//! `bq.b:([B[B)[B => iadd,isub,iinc,ixor,iand,iadd,i2b,iinc,iand,iand` (decrypt).

/// `public static final byte[] encrypt(byte[] plaintext, byte[] key)` (`bq.a`).
///
/// Output is `plaintext.length + 1` bytes: the rolling-XOR ciphertext followed by
/// the additive checksum byte that [`decrypt`] verifies. Every array access is
/// unguarded in the original (no `try`/`catch`), so an out-of-range index — e.g.
/// an empty `key` — is an uncaught `ArrayIndexOutOfBoundsException`, faithfully a
/// Rust panic here.
pub fn encrypt(plaintext: &[i8], key: &[i8]) -> Vec<i8> {
    // byte[] out = new byte[plaintext.length + 1];
    let mut out: Vec<i8> = vec![0i8; (plaintext.len() as i32).wrapping_add(1) as usize];
    let mut checksum: i32 = 0;
    let mut key_index: i32 = 0;
    // for (int i = 0; i < plaintext.length; i++)
    let mut i: i32 = 0;
    while i < plaintext.len() as i32 {
        // byte plain = plaintext[i];
        let plain: i8 = plaintext[i as usize];
        // keyIndex++;
        key_index = key_index.wrapping_add(1);
        // if (keyIndex == key.length) keyIndex = 0;
        if key_index == key.len() as i32 {
            key_index = 0;
        }
        // int cipher = plain ^ key[keyIndex];  (both baload sign-extend to int)
        let cipher: i32 = (plain as i32) ^ (key[key_index as usize] as i32);
        // checksum += key[keyIndex] & 255;
        checksum = checksum.wrapping_add((key[key_index as usize] as i32) & 255);
        // out[i] = (byte) cipher;
        out[i as usize] = cipher as i8;
        // i++
        i = i.wrapping_add(1);
    }
    // out[plaintext.length] = (byte) checksum;
    out[plaintext.len()] = checksum as i8;
    out
}

/// `public static final byte[] decrypt(byte[] cipher, byte[] key)` (`bq.b`).
///
/// Recreates the keystream over `cipher[0 .. length-2]`, accumulates the checksum,
/// and returns the plaintext only if the recomputed checksum matches the trailing
/// byte; returns `null` (→ [`None`]) on mismatch, on an empty input, or on any
/// exception. The per-byte loop body is wrapped in `try { … } catch (Exception) {
/// return null; }` in the original: `cipher[i]` and `out[i]` are provably
/// in-range (`i < cipher.length - 1`), so the only access that can realistically
/// throw is `key[keyIndex]` (an empty `key`), routed here through a checked read
/// that reproduces the catch's `return null`.
///
/// Quirk preserved verbatim: `out` is allocated `cipher.length + 1` bytes but only
/// `cipher.length - 1` are ever written, so the returned array is two bytes longer
/// than the recovered plaintext (trailing zeros). `SaveCipher` callers read only
/// the leading bytes.
pub fn decrypt(cipher: &[i8], key: &[i8]) -> Option<Vec<i8>> {
    // byte[] out = new byte[cipher.length + 1];
    let mut out: Vec<i8> = vec![0i8; (cipher.len() as i32).wrapping_add(1) as usize];
    let mut checksum: i32 = 0;
    // if (cipher.length < 1) return null;
    if (cipher.len() as i32) < 1 {
        return None;
    }
    let mut key_index: i32 = 0;
    let mut i: i32 = 0;
    // while (i < cipher.length - 1)
    while i < (cipher.len() as i32).wrapping_sub(1) {
        // try {
        // byte enc = cipher[i];   (i < cipher.length - 1 → in range)
        let enc: i8 = cipher[i as usize];
        // keyIndex++;
        key_index = key_index.wrapping_add(1);
        // if (keyIndex == key.length) keyIndex = 0;
        if key_index == key.len() as i32 {
            key_index = 0;
        }
        // key[keyIndex] — the one access that can throw; the catch returns null.
        let key_byte: i8 = match key.get(key_index as usize) {
            Some(&b) => b,
            None => return None, // catch (Exception unused) { return null; }
        };
        // int plain = enc ^ key[keyIndex];
        let plain: i32 = (enc as i32) ^ (key_byte as i32);
        // checksum += key[keyIndex] & 255;
        checksum = checksum.wrapping_add((key_byte as i32) & 255);
        // out[i] = (byte) plain;
        out[i as usize] = plain as i8;
        // i++;
        i = i.wrapping_add(1);
        // } catch (Exception unused) { return null; }
    }
    // if ((checksum & 255) != (cipher[i] & 255)) return null;   (outside the try)
    if (checksum & 255) != ((cipher[i as usize] as i32) & 255) {
        return None;
    }
    Some(out)
}
