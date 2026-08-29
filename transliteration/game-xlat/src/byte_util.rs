//! Transliterated from `java/src/main/java/defpackage/ByteUtil.java`
//! (original `h.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Big-endian byte helpers plus the shared RNG. The byte helpers are pure static
//! utilities (free functions, no state). The one piece of mutable state is the
//! shared `static Random rng` backing [`rand_range`], modelled as
//! [`ByteUtilState`] holding a faithful [`JavaRandom`].
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `h.a:(II)I => isub,iadd,irem,iadd` (randRange),
//! `h.a:([BI)S => isub,iand,ishl,ior,i2s,iadd,iand,ior,i2s` (readU16),
//! `h.a:([BI)I => iadd,iand,ishl,iadd,iand,ishl,ior,iadd,iand,ishl,ior,iadd,iand,ior` (readS32),
//! `h.a:(I[BI)V => iand,ishr,iand,i2b,ishr,iand,i2b,ishr,iand,i2b,iand,i2b` (writeI32),
//! `h.a:([C[C)[C => iadd` (concat).

use j2me_jvm::{ishl, ishr, java_rem};

/// A faithful reimplementation of `java.util.Random` (the linear congruential
/// generator specified by the JDK), enough for [`rand_range`]'s `nextInt()`.
///
/// Reproduces the exact 48-bit LCG so a seeded run matches the JVM bit-for-bit;
/// verified against published `java.util.Random` vectors in the unit tests.
pub struct JavaRandom {
    seed: i64,
}

impl JavaRandom {
    const MULTIPLIER: i64 = 0x5DEECE66D;
    const ADDEND: i64 = 0xB;
    const MASK: i64 = (1i64 << 48) - 1;

    /// `new Random(seed)` — scrambles the seed the way the JDK constructor does.
    pub fn new(seed: i64) -> Self {
        JavaRandom {
            seed: (seed ^ Self::MULTIPLIER) & Self::MASK,
        }
    }

    /// `protected int next(int bits)`.
    fn next(&mut self, bits: i32) -> i32 {
        // seed = (seed * MULTIPLIER + ADDEND) & MASK;
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        // return (int) (seed >>> (48 - bits));   (lushr, then l2i truncation)
        ((self.seed as u64) >> ((48 - bits) & 63)) as i64 as i32
    }

    /// `public int nextInt()`.
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
}

/// Java `ByteUtil` mutable state: the shared `static Random rng = new Random();`.
///
/// The default (`new Random()`) is time-seeded and therefore non-deterministic;
/// [`ByteUtilState::seeded`] constructs a reproducible instance for tests/traces.
pub struct ByteUtilState {
    /// Shared RNG backing [`rand_range`].
    pub rng: JavaRandom,
}

impl ByteUtilState {
    /// A reproducible state seeded like `new Random(seed)`.
    pub fn seeded(seed: i64) -> Self {
        ByteUtilState {
            rng: JavaRandom::new(seed),
        }
    }
}

/// `Math.abs(int)` — note `Math.abs(i32::MIN)` returns `i32::MIN` (overflow),
/// unlike Rust's panicking `i32::abs`.
#[inline]
fn java_math_abs(x: i32) -> i32 {
    if x < 0 {
        x.wrapping_neg()
    } else {
        x
    }
}

/// Uniform random integer in the inclusive range `[low, high]`; returns 0 when
/// the span wraps to 0. Asserts `low <= high` (`Debug.assertTrue`, `Debug.java:19`
/// — a false assertion throws `RuntimeException`, here a panic).
pub fn rand_range(s: &mut ByteUtilState, low: i32, high: i32) -> i32 {
    // Debug.assertTrue(low <= high);
    if !(low <= high) {
        panic!("ASSERT FAILED"); // Debug.java:21
    }
    // int span = (high - low) + 1;
    let span: i32 = high.wrapping_sub(low).wrapping_add(1);
    if span == 0 {
        return 0;
    }
    // return low + (Math.abs(rng.nextInt()) % span);
    low.wrapping_add(java_rem(java_math_abs(s.rng.next_int()), span).expect("randRange mod span"))
}

/// Reads a big-endian unsigned 16-bit value at `offset`, returned as a signed
/// `short`. Throws (panics) on an out-of-range `offset` (`ByteUtil.java:37`).
// The `0 | ...` is faithful to the Java source; not simplified away.
#[allow(clippy::identity_op)]
pub fn read_u16(buffer: &[i8], offset: i32) -> i16 {
    // if (buffer.length - 2 < offset) throw new ArrayIndexOutOfBoundsException();
    if (buffer.len() as i32).wrapping_sub(2) < offset {
        panic!("ArrayIndexOutOfBoundsException"); // ByteUtil.java:37
    }
    // (short) (((short) (0 | ((buffer[offset] & 255) << 8))) | (buffer[offset + 1] & 255))
    let hi: i16 = (0 | ishl((buffer[offset as usize] as i32) & 255, 8)) as i16;
    ((hi as i32) | ((buffer[offset.wrapping_add(1) as usize] as i32) & 255)) as i16
}

/// Reads a big-endian signed 32-bit value at `offset`, or `-1` if out of range.
pub fn read_s32(buffer: &[i8], offset: i32) -> i32 {
    // if (buffer.length < offset + 4) return -1;
    if (buffer.len() as i32) < offset.wrapping_add(4) {
        return -1;
    }
    // ((buffer[offset] & 255) << 24) | ((buffer[offset+1] & 255) << 16)
    //   | ((buffer[offset+2] & 255) << 8) | (buffer[offset+3] & 255)
    ishl((buffer[offset as usize] as i32) & 255, 24)
        | ishl((buffer[offset.wrapping_add(1) as usize] as i32) & 255, 16)
        | ishl((buffer[offset.wrapping_add(2) as usize] as i32) & 255, 8)
        | ((buffer[offset.wrapping_add(3) as usize] as i32) & 255)
}

/// Writes `value` as a big-endian 32-bit integer into `buffer` at `offset`.
// The `value & -1` redundant mask is a preserved defect; not simplified away.
#[allow(clippy::identity_op)]
pub fn write_i32(value: i32, buffer: &mut [i8], offset: i32) {
    // byte[] tmp = {0, 0, 0, 0};
    let mut tmp: [i8; 4] = [0, 0, 0, 0];
    // int v = value & (-1);   (redundant identity mask, preserved — ByteUtil.java:64)
    let v: i32 = value & (-1);
    // tmp[k] = (byte) ((v >> shift) & 255);
    tmp[0] = (ishr(v, 24) & 255) as i8;
    tmp[1] = (ishr(v, 16) & 255) as i8;
    tmp[2] = (ishr(v, 8) & 255) as i8;
    tmp[3] = (v & 255) as i8;
    // System.arraycopy(tmp, 0, buffer, offset, 4);
    let dst: usize = offset as usize;
    buffer[dst..dst + 4].copy_from_slice(&tmp);
}

/// Returns a new array holding `first` followed by `second`.
pub fn concat(first: &[u16], second: &[u16]) -> Vec<u16> {
    // char[] result = new char[first.length + second.length];
    let mut result: Vec<u16> =
        vec![0u16; (first.len() as i32).wrapping_add(second.len() as i32) as usize];
    // System.arraycopy(first, 0, result, 0, first.length);
    result[..first.len()].copy_from_slice(first);
    // System.arraycopy(second, 0, result, first.length, second.length);
    let fl: usize = first.len();
    result[fl..fl + second.len()].copy_from_slice(second);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Published `java.util.Random` vectors: the first three `nextInt()` values
    // for a few seeds. If the LCG diverges from the JVM, these break.
    #[test]
    fn java_random_matches_published_vectors() {
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int(), -1155484576);
        assert_eq!(r.next_int(), -723955400);
        assert_eq!(r.next_int(), 1033096058);

        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(), -1170105035);
    }

    #[test]
    fn rand_range_stays_in_bounds_and_is_reproducible() {
        let mut s = ByteUtilState::seeded(12345);
        for _ in 0..10000 {
            let v = rand_range(&mut s, -7, 42);
            assert!((-7..=42).contains(&v), "rand_range out of [low,high]: {v}");
        }
        // Empty span (high - low + 1 wraps to 0) returns 0.
        let mut s = ByteUtilState::seeded(1);
        assert_eq!(rand_range(&mut s, i32::MIN, i32::MAX), 0);
    }

    #[test]
    fn byte_helpers_round_trip_big_endian() {
        // writeI32 then readS32 is identity for representable values.
        let mut buf = vec![0i8; 8];
        write_i32(0x12345678, &mut buf, 2);
        assert_eq!(read_s32(&buf, 2), 0x12345678);
        // The written bytes are big-endian.
        assert_eq!(buf[2] as u8, 0x12);
        assert_eq!(buf[3] as u8, 0x34);
        assert_eq!(buf[4] as u8, 0x56);
        assert_eq!(buf[5] as u8, 0x78);

        // readU16 is big-endian unsigned; 0xFFFE reads as the signed short -2.
        let b = vec![0i8, 0i8, -1i8, -2i8]; // 0x00 0x00 0xFF 0xFE
        assert_eq!(read_u16(&b, 2), -2i16); // (short) 0xFFFE
        assert_eq!(read_u16(&b, 0), 0i16);

        // readS32 out of range yields -1.
        assert_eq!(read_s32(&b, 3), -1);
    }

    #[test]
    fn concat_joins_char_arrays() {
        let a: Vec<u16> = vec![1, 2, 3];
        let b: Vec<u16> = vec![4, 5];
        assert_eq!(concat(&a, &b), vec![1, 2, 3, 4, 5]);
    }
}
