//! Transliterated from `java/src/main/java/defpackage/Adler32.java`
//! (original `an.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Minimal Adler-32 running checksum (RFC 1950), used by [`crate::png_merger`]
//! to write the zlib check value of the raw-deflate stream it emits. The 32-bit
//! state packs two 16-bit sums `(sumB << 16) | sumA`; both are reduced modulo
//! `65521` and at most `5552` bytes are folded before each reduction.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`):
//! `an.a:([BII)V => iand,ishr,iand,isub,isub,iinc,iand,iadd,iadd,irem,irem,ishl,ior`
//! and `an.a:()J => i2l,land`.

use j2me_jvm::{ishl, ishr, java_rem};

/// Java `Adler32` instance state: `private int sum = 1;`.
pub struct Adler32State {
    /// Packed running checksum `(sumB << 16) | sumA`, seeded to 1.
    pub sum: i32,
}

impl Default for Adler32State {
    fn default() -> Self {
        // `private int sum = 1;`
        Adler32State { sum: 1 }
    }
}

impl Adler32State {
    /// `new Adler32()` — seeds `sum = 1`.
    pub fn new() -> Self {
        Adler32State::default()
    }
}

/// Folds `length` bytes of `data` starting at `offset` into the running checksum.
///
/// The array read `data[index]` is unguarded in the original — a bad index is an
/// uncaught `ArrayIndexOutOfBoundsException`, so a Rust panic here is faithful.
pub fn update(s: &mut Adler32State, data: &[i8], mut offset: i32, mut length: i32) {
    let mut sum_a: i32 = s.sum & 65535;
    let mut sum_b: i32 = ishr(s.sum, 16) & 65535;
    loop {
        if length <= 0 {
            s.sum = ishl(sum_b, 16) | sum_a;
            return;
        }
        let mut block: i32 = if length < 5552 { length } else { 5552 };
        length = length.wrapping_sub(block);
        loop {
            let remaining: i32 = block;
            block = remaining.wrapping_sub(1);
            if remaining <= 0 {
                break;
            }
            let index: i32 = offset;
            offset = offset.wrapping_add(1);
            // data[index] is a signed byte; `& 255` re-zero-extends (baload + iand).
            sum_a = sum_a.wrapping_add(data[index as usize] as i32 & 255);
            sum_b = sum_b.wrapping_add(sum_a);
        }
        // `% 65521` — bare `%` routed through java_rem; 65521 is a nonzero constant.
        sum_a = java_rem(sum_a, 65521).expect("adler mod 65521");
        sum_b = java_rem(sum_b, 65521).expect("adler mod 65521");
    }
}

/// Resets the checksum to its initial value (1).
pub fn reset(s: &mut Adler32State) {
    s.sum = 1;
}

/// Returns the current checksum.
///
/// Preserved defect (`Adler32.java:57`): the original `& -1L` mask **sign**-
/// extends the 32-bit state rather than zero-extending it, so a checksum with
/// bit 31 set comes back as a negative `long`. [`crate::png_merger`] casts the
/// result back to `int`, which undoes the extension.
// The `& -1L` mask is intentionally kept (preserved defect); not simplified away.
#[allow(clippy::identity_op)]
pub fn get_value(s: &Adler32State) -> i64 {
    // `((long) this.sum) & (-1)` — i2l then land with -1L (identity on the bits).
    (s.sum as i64) & (-1i64)
}
