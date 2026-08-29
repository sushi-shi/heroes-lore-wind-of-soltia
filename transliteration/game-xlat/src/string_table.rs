//! Transliterated from `java/src/main/java/defpackage/StringTable.java`
//! (original `cj.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The localized-string table. A singleton holds the whole `lang/language.<suffix>`
//! payload in memory (`blob`) and hands out strings by numeric id (`get`) via a
//! per-id `u32` offset table followed by modified-UTF-8 records. This increment
//! ports `load` + `get` — the path the title footer (`getString(3950)`) needs.
//! `resolveLocale` is only reached when `load` is passed a negative index; the
//! title path calls `load("/lang/language","",1)` (fr-FR, the EN baseline's
//! mislabeled English file), so it is DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `cj.a:(Ljava/lang/String;Ljava/lang/String;I)V => [iand,i2b,isub,iadd]` (load),
//! `cj.a:(I)Ljava/lang/String; => [isub,ishl,i2l,i2l]` (get). The leading `isub`
//! of `get` is a redundant `id - 0` the bytecode emits (`iload_1; iconst_0; isub;
//! istore_1`) that the reconstructed Java elided; preserved here.

use crate::game::Game;
use j2me_jvm::ishl;

/// Java `cj` / `StringTable` singleton state. `blob` is the loaded payload (the
/// offset table + packed records); `locale_index` is the loaded language's index
/// into `LOCALES`. The `stream` field is a random-access re-reader of `blob`,
/// modelled here by indexing `blob` directly.
#[derive(Debug, Default)]
pub struct StringTableData {
    /// `public byte[] blob;`
    pub blob: Vec<i8>,
    /// `public byte localeIndex = 0;`
    pub locale_index: i8,
}

/// `public final String[] locales = {"en-GB","fr-FR","de-DE","it-IT","es-ES"};`
/// — only default-index filenames (carry no language meaning; the EN baseline's
/// `fr-FR` holds English).
pub const LOCALES: [&str; 5] = ["en-GB", "fr-FR", "de-DE", "it-IT", "es-ES"];

/// `public final void load(String basePath, String locale, int index)`
/// (`=> [iand,i2b,isub,iadd]`). Loads `basePath + "." + locales[index]` into
/// `blob`. The `IOException` catch (log "Couldn't load babble file.") is subsumed
/// — the resource is present on the title path.
pub fn load(g: &mut Game, base_path: &str, _locale: &str, index: i32) {
    // if (index < 0) { index = resolveLocale(locale); if (index == -1) index = 0; }
    //   DEFERRED: the title path always passes index >= 0 (loadLanguage(1)).
    let index: i32 = index;
    // this.localeIndex = (byte) (index & 32767);
    g.string_table.locale_index = (index & 32767) as i8;
    // getResourceAsStream(basePath + "." + locales[localeIndex])
    let path = format!(
        "{}.{}",
        base_path, LOCALES[g.string_table.locale_index as usize]
    );
    let src: Vec<i8> = g
        .resources
        .get(&path)
        .unwrap_or_else(|| panic!("StringTable.load: resource not found: {path}"))
        .to_vec();
    // int payloadLen = in.readInt();
    let payload_len: i32 = read_int_be(&src, 0);
    // this.blob = new byte[payloadLen];
    let mut blob: Vec<i8> = vec![0i8; payload_len as usize];
    // int filled = 0; do { total = filled + in.read(blob, filled, payloadLen - filled); filled = total; } while (total < payloadLen);
    //   The stream's first 4 bytes were consumed by readInt(); the read loop fills
    //   `blob` from the remaining bytes.
    let mut filled: i32 = 0;
    let body: &[i8] = &src[4..];
    while filled < payload_len {
        // in.read(blob, filled, payloadLen - filled) — copies the available bytes.
        let want: i32 = payload_len.wrapping_sub(filled);
        let avail: i32 = (body.len() as i32).wrapping_sub(filled);
        let n: i32 = core::cmp::min(want, avail);
        if n <= 0 {
            break;
        }
        blob[filled as usize..(filled.wrapping_add(n)) as usize]
            .copy_from_slice(&body[filled as usize..(filled.wrapping_add(n)) as usize]);
        // total = filled + read; filled = total;
        filled = filled.wrapping_add(n);
    }
    g.string_table.blob = blob;
}

/// `public final String get(int id)` (`=> [isub,ishl,i2l,i2l]`). Locates string
/// `id` via the offset table and reads its modified-UTF-8 record. On any failure
/// returns the diagnostic `"<id>.<exception>"` (here a fixed placeholder), never
/// throwing — matching the original's `catch (Exception)`.
#[allow(clippy::identity_op)]
pub fn get(s: &StringTableData, id: i32) -> Vec<u16> {
    // int id2 = id - 0;   (redundant bytecode identity: iload_1;iconst_0;isub;istore_1)
    let id: i32 = id - 0;
    // stream.reset(); stream.skip(id << 2);
    let mut pos: i32 = ishl(id, 2);
    // stream.skip(stream.readInt());   (readInt advances 4, its value is skipped)
    match try_get(s, &mut pos) {
        Some(v) => v,
        // catch (Exception e) -> "<id>.<exception>"
        None => diagnostic(id),
    }
}

/// The happy-path body of [`get`], returning `None` where the original would have
/// thrown (an out-of-range blob read), so the caller substitutes the diagnostic.
fn try_get(s: &StringTableData, pos: &mut i32) -> Option<Vec<u16>> {
    let blob = &s.blob;
    // readInt() at `pos`, then skip its value.
    let off: i32 = read_int_be_checked(blob, *pos)?;
    *pos = pos.wrapping_add(4).wrapping_add(off);
    // stream.skip(2L);
    *pos = pos.wrapping_add(2);
    // return stream.readUTF();
    read_utf(blob, *pos)
}

/// `DataInputStream.readInt()` — big-endian signed 32-bit (used for the payload
/// length header, where a well-formed file guarantees the 4 bytes).
fn read_int_be(b: &[i8], i: i32) -> i32 {
    ishl((b[i as usize] as i32) & 255, 24)
        | ishl((b[i.wrapping_add(1) as usize] as i32) & 255, 16)
        | ishl((b[i.wrapping_add(2) as usize] as i32) & 255, 8)
        | ((b[i.wrapping_add(3) as usize] as i32) & 255)
}

/// Bounds-checked `readInt` for the guarded `get` path (`None` == EOFException).
fn read_int_be_checked(b: &[i8], i: i32) -> Option<i32> {
    if i < 0 || (i as i64).wrapping_add(4) > b.len() as i64 {
        return None;
    }
    Some(read_int_be(b, i))
}

/// `DataInputStream.readUTF()` — a `u16` byte-length header then that many
/// modified-UTF-8 bytes, decoded to UTF-16 code units (Java `char`s). `None` on a
/// malformed/short record (the EOFException/UTFDataFormatException the catch eats).
fn read_utf(b: &[i8], i: i32) -> Option<Vec<u16>> {
    if i < 0 || (i as i64).wrapping_add(2) > b.len() as i64 {
        return None;
    }
    // int utflen = readUnsignedShort();
    let utflen: i32 =
        ishl((b[i as usize] as i32) & 255, 8) | ((b[i.wrapping_add(1) as usize] as i32) & 255);
    let base: i32 = i.wrapping_add(2);
    if (base as i64).wrapping_add(utflen as i64) > b.len() as i64 {
        return None;
    }
    let mut out: Vec<u16> = Vec::new();
    let mut k: i32 = 0;
    while k < utflen {
        let c: i32 = (b[(base.wrapping_add(k)) as usize] as i32) & 255;
        if c < 0x80 {
            out.push(c as u16);
            k = k.wrapping_add(1);
        } else if (c & 0xE0) == 0xC0 {
            if k.wrapping_add(2) > utflen {
                return None;
            }
            let c2: i32 = (b[(base.wrapping_add(k).wrapping_add(1)) as usize] as i32) & 0x3F;
            out.push((((c & 0x1F) << 6) | c2) as u16);
            k = k.wrapping_add(2);
        } else if (c & 0xF0) == 0xE0 {
            if k.wrapping_add(3) > utflen {
                return None;
            }
            let c2: i32 = (b[(base.wrapping_add(k).wrapping_add(1)) as usize] as i32) & 0x3F;
            let c3: i32 = (b[(base.wrapping_add(k).wrapping_add(2)) as usize] as i32) & 0x3F;
            out.push((((c & 0x0F) << 12) | (c2 << 6) | c3) as u16);
            k = k.wrapping_add(3);
        } else {
            return None;
        }
    }
    Some(out)
}

/// The `catch` diagnostic `"<id>.<exception>"` — a placeholder (never produced on
/// the title path, where id 3950 decodes cleanly).
fn diagnostic(id: i32) -> Vec<u16> {
    format!("{id}.error").encode_utf16().collect()
}
