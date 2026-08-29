//! Transliterated from `java/src/main/java/defpackage/Crc32.java`
//! (original `ca.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Minimal table-driven CRC-32 (ISO 3309 / zlib variant), used by
//! [`crate::png_merger`] to compute PNG chunk CRCs. Reversed polynomial
//! `0xEDB88320` (`-306674912`), initial value all-ones, final ones-complement —
//! matching `java.util.zip.CRC32`.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `ca.<clinit>:()V => iand,iushr,ixor,iushr,iadd,i2b,iadd,i2s`,
//! `ca.a:([BII)V => iadd,iushr,iand,ixor,iand,ixor,iinc`,
//! `ca.a:()I => ixor`.

use j2me_jvm::iushr;
use std::sync::OnceLock;

/// The `static final int[] TABLE = new int[256]` built by the class initializer.
///
/// Java builds it once at class load in `<clinit>`; here it is built once,
/// lazily, running the exact transliterated `<clinit>` loop.
fn table() -> &'static [i32; 256] {
    static TABLE: OnceLock<[i32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        // static { ... } — Crc32.java:40
        let mut t = [0i32; 256];
        // for (short n = 0; n < 256; n = (short) (n + 1))
        let mut n: i16 = 0;
        while (n as i32) < 256 {
            // int c = n;   (short widened to int)
            let mut c: i32 = n as i32;
            // for (byte bit = 1; bit < 9; bit = (byte) (bit + 1))
            let mut bit: i8 = 1;
            while (bit as i32) < 9 {
                // c = (c & 1) == 1 ? (c >>> 1) ^ (-306674912) : c >>> 1;
                c = if (c & 1) == 1 {
                    iushr(c, 1) ^ (-306674912)
                } else {
                    iushr(c, 1)
                };
                // bit = (byte) (bit + 1);
                bit = (bit as i32 + 1) as i8;
            }
            // TABLE[n] = c;
            t[n as usize] = c;
            // n = (short) (n + 1);
            n = (n as i32 + 1) as i16;
        }
        t
    })
}

/// Java `Crc32` instance state: `private int crc = -1;` (held pre-inverted).
pub struct Crc32State {
    /// Running CRC, seeded to `-1`.
    pub crc: i32,
}

impl Default for Crc32State {
    fn default() -> Self {
        // `private int crc = -1;`
        Crc32State { crc: -1 }
    }
}

impl Crc32State {
    /// `new Crc32()` — seeds `crc = -1`.
    pub fn new() -> Self {
        Crc32State::default()
    }
}

/// Resets the running CRC to its initial value.
pub fn reset(s: &mut Crc32State) {
    s.crc = -1;
}

/// Folds `length` bytes of `data` starting at `offset` into the running CRC.
///
/// `data[i]` is unguarded in the original; a Rust panic on a bad index is
/// faithful (uncaught `ArrayIndexOutOfBoundsException`).
pub fn update(s: &mut Crc32State, data: &[i8], offset: i32, length: i32) {
    // for (int i = offset; i < length + offset; i++)
    let mut i: i32 = offset;
    while i < length.wrapping_add(offset) {
        // this.crc = ((this.crc >>> 8) & 16777215) ^ TABLE[(this.crc ^ data[i]) & 255];
        let idx: i32 = (s.crc ^ (data[i as usize] as i32)) & 255;
        s.crc = (iushr(s.crc, 8) & 16777215) ^ table()[idx as usize];
        i = i.wrapping_add(1);
    }
}

/// Returns the finished CRC-32 value (the ones-complement of the state).
pub fn get_value(s: &Crc32State) -> i32 {
    // return this.crc ^ (-1);
    s.crc ^ (-1)
}
