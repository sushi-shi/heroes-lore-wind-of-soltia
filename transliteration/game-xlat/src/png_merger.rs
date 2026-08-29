//! Transliterated from `java/src/main/java/defpackage/PngMerger.java`
//! (original `br.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The atlas engine — the "PNGMerger" that reassembles individual sprite frames
//! from the headerless `.mpd` / `.mph` texture atlases. An `.mph` index holds a
//! header (flags + frame count) and, per frame, which `_<k>.mpd` file it lives
//! in, its byte offset there, and a bitmask of optional PNG chunks. Each `.mpd`
//! is a back-to-back run of sub-PNGs stripped of the 8-byte signature and IEND.
//! [`assemble_frame`] stitches a full PNG for a frame; [`mirror`] / [`apply_effect`]
//! / [`remap_palette`] edit the raw filter-0 IDAT and fix the zlib Adler-32 and
//! chunk CRC-32.
//!
//! ## Deferred boundary (see docs/TRANSLITERATION.md, accepted deviations)
//!
//! `load` / `readIndex` / `loadMpd` read bytes through `AssetCache.readResource`,
//! and `image` / `imageMirrored` / `imageGray` / `allImages` wrap the result in
//! `javax.microedition.lcdui.Image` (+ `BaseCanvas.yieldTick`). Those cross into
//! as-yet-unported classes and are **deferred**; the decoder core below (header
//! parse, assembly, transforms) is complete and driven from injected bytes. The
//! reload branch of `mpd_bytes` (which calls `loadMpd`) is part of that deferred
//! boundary and is not reached in the assembly-from-injected-bytes flow.
//!
//! The shared `static Crc32 crc` / `static Adler32 adler` engines are `.reset()`
//! before every use, so the transforms construct a fresh (== reset) local engine.

use crate::adler32;
use crate::asset_cache;
use crate::base_canvas;
use crate::crc32;
use crate::game::Game;
use j2me_jvm::{ishl, ishr, java_div, java_rem};

/// PNG chunk-type names, indexed the way `find_chunk` / `locate_chunk` use them.
const CHUNK_TYPES: [&str; 18] = [
    "IHDR", "cHRM", "gAMA", "iCCP", "sBIT", "sRGB", "tEXt", "zTXt", "iTXt", "pHYs", "sPLT", "tIME",
    "PLTE", "tRNS", "hIST", "bKGD", "IDAT", "IEND",
];

/// The 8-byte PNG signature prepended to every reassembled frame.
const PNG_SIGNATURE: [i8; 8] = [-119, 80, 78, 71, 13, 10, 26, 10];

/// A complete, pre-CRC'd IEND chunk appended to every reassembled frame.
const IEND_CHUNK: [i8; 12] = [0, 0, 0, 0, 73, 69, 78, 68, -82, 66, 96, -126];

/// Java `PngMerger` instance state. Fields are `pub` because the struct mirrors
/// the Java object's fields and the oracle injects `mph_data` / `mpd_data`.
#[derive(Default)]
pub struct PngMergerState {
    /// Base resource path (without extension) of the atlas pair.
    pub base_path: String,
    /// `.mph` flags bit `0x08`: frames carry an appended shared PLTE/tRNS palette.
    pub merge_palette: bool,
    /// `.mph` flags bit `0x04`: frames need runtime palette-remap.
    pub palette_remap: bool,
    /// Number of distinct `_<k>.mpd` files this atlas references.
    pub mpd_count: i32,
    /// Frame count per `_<k>.mpd` file (indexed by mpd number).
    pub frames_per_mpd: Vec<i32>,
    /// The whole `.mph` index blob.
    pub mph_data: Vec<i8>,
    /// Lazily-loaded `.mpd` payloads (bytes per mpd number, else `None`).
    pub mpd_data: Vec<Option<Vec<i8>>>,
    /// Per-frame optional-chunk bitmask (`char[]`).
    pub chunk_masks: Vec<u16>,
    /// Byte offset of the shared PLTE chunk within `mph_data`, or -1.
    pub plte_pos: i32,
    /// Byte offset of the shared tRNS chunk within `mph_data`, or -1.
    pub trns_pos: i32,
    /// True once every frame has been extracted.
    pub preload_all: bool,
}

impl PngMergerState {
    /// The `PngMerger()` no-arg constructor: all fields at their Java defaults.
    pub fn new() -> Self {
        // preloadAll defaults to false; every other field to its Java default.
        PngMergerState::default()
    }
}

/// Parses the `.mph` header into the state (Java `parseHeader`, private).
///
/// Exposed for the reconstruction oracle; private in Java.
pub fn parse_header(s: &mut PngMergerState) {
    // int iA = readU32(this.mphData, 0);
    let i_a: i32 = read_u32(&s.mph_data, 0);
    // this.mergePalette = (iA >> 27) % 2 == 1;
    s.merge_palette = java_rem(ishr(i_a, 27), 2).expect("mph flag mod") == 1;
    // this.paletteRemap = (iA >> 26) % 2 == 1;
    s.palette_remap = java_rem(ishr(i_a, 26), 2).expect("mph flag mod") == 1;
    // int iM45a = frameCount();
    let i_m45a: i32 = frame_count(s);
    // this.mpdCount = 0;
    s.mpd_count = 0;
    // for (int i = 0; i < iM45a; i++)
    let mut i: i32 = 0;
    while i < i_m45a {
        // if (this.mpdCount < readU16(this.mphData, 8 + (8 * i)) + 1)
        if s.mpd_count
            < (read_u16(&s.mph_data, 8i32.wrapping_add(8i32.wrapping_mul(i))) as i32)
                .wrapping_add(1)
        {
            // this.mpdCount = readU16(this.mphData, 8 + (8 * i)) + 1;
            s.mpd_count = (read_u16(&s.mph_data, 8i32.wrapping_add(8i32.wrapping_mul(i))) as i32)
                .wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    // this.framesPerMpd = new int[this.mpdCount];
    s.frames_per_mpd = vec![0i32; s.mpd_count as usize];
    // for (int i2 = 0; i2 < iM45a; i2++)
    let mut i2: i32 = 0;
    while i2 < i_m45a {
        // char cM54a = readU16(this.mphData, 8 + (8 * i2));
        let c_m54a: u16 = read_u16(&s.mph_data, 8i32.wrapping_add(8i32.wrapping_mul(i2)));
        // iArr[cM54a] = iArr[cM54a] + 1;
        s.frames_per_mpd[c_m54a as usize] = s.frames_per_mpd[c_m54a as usize].wrapping_add(1);
        i2 = i2.wrapping_add(1);
    }
    // this.mpdData = new Object[this.mpdCount];
    s.mpd_data = (0..s.mpd_count).map(|_| None).collect();
    // this.chunkMasks = new char[iM45a];
    s.chunk_masks = vec![0u16; i_m45a as usize];
    // for (int i3 = 0; i3 < iM45a; i3++)
    let mut i3: i32 = 0;
    while i3 < i_m45a {
        // this.chunkMasks[i3] = readU16(this.mphData, 8 + (8 * i3) + 6);
        s.chunk_masks[i3 as usize] = read_u16(
            &s.mph_data,
            8i32.wrapping_add(8i32.wrapping_mul(i3)).wrapping_add(6),
        );
        i3 = i3.wrapping_add(1);
    }
    // this.pltePos = locateChunk(this.mphData, 12);
    s.plte_pos = locate_chunk(&s.mph_data, 12);
    // this.trnsPos = locateChunk(this.mphData, 13);
    s.trns_pos = locate_chunk(&s.mph_data, 13);
}

/// Number of frames in the atlas (u32 at mph offset 4). Public in Java.
pub fn frame_count(s: &PngMergerState) -> i32 {
    // return readU32(this.mphData, 4);
    read_u32(&s.mph_data, 4)
}

/// Drops the cached bytes of `.mpd` number `i` (Java `unloadMpd`).
pub fn unload_mpd(s: &mut PngMergerState, i: i32) {
    // this.mpdData[i] = null;
    s.mpd_data[i as usize] = None;
}

/// Drops every cached `.mpd` and runs a GC (Java `unloadAllMpd`).
pub fn unload_all_mpd(s: &mut PngMergerState) {
    // for (int i = 0; i < this.mpdCount; i++) unloadMpd(i);
    let mut i: i32 = 0;
    while i < s.mpd_count {
        unload_mpd(s, i);
        i = i.wrapping_add(1);
    }
    // System.gc(); — no-op
}

/// In merge-palette atlases, rewrites two entries of the shared PLTE with colors
/// `i` / `i2` (Java `remapPalette`, public).
pub fn remap_palette(s: &mut PngMergerState, i: i32, i2: i32) {
    // if (this.mergePalette) transformPixels(this.mphData, this.pltePos, 4, i, i2);
    if s.merge_palette {
        let pos = s.plte_pos;
        transform_pixels(&mut s.mph_data, pos, 4, i, i2);
    }
}

/// Returns the `.mpd` bytes holding frame `i` (Java `mpdBytes`, private).
///
/// The reload branch (`preloadAll && mpdData[k] == null`) calls `loadMpd`, which
/// is the deferred `AssetCache` boundary; in the assembly-from-injected-bytes
/// flow `preloadAll` is false and the payloads are populated, so it is not
/// reached. If it ever were, this panics with a clear marker.
fn mpd_bytes(s: &PngMergerState, i: i32) -> &[i8] {
    // int iM50a = mpdIndexOf(i);
    let i_m50a: i32 = mpd_index_of(s, i);
    // if (this.preloadAll && this.mpdData[iM50a] == null) { unloadAllMpd(); loadMpd(iM50a); }
    //   -> deferred resource boundary (see module docs).
    // return (byte[]) this.mpdData[iM50a];
    s.mpd_data[i_m50a as usize]
        .as_deref()
        .expect("mpd payload not loaded (reload path is the deferred AssetCache boundary)")
}

/// Which `.mpd` file frame `i` lives in (Java `mpdIndexOf`, private).
fn mpd_index_of(s: &PngMergerState, i: i32) -> i32 {
    // return readU16(this.mphData, 8 + (8 * i));
    read_u16(&s.mph_data, 8i32.wrapping_add(8i32.wrapping_mul(i))) as i32
}

/// Reassembles a full PNG for frame `i` (Java `assembleFrame`, private).
///
/// Returns `None` where the Java returns `null` (merged path with no IHDR).
/// Exposed for the reconstruction oracle; private in Java.
pub fn assemble_frame(s: &PngMergerState, i: i32) -> Option<Vec<i8>> {
    // return this.mergePalette ? assembleMerged(i) : assembleSimple(i);
    if s.merge_palette {
        assemble_merged(s, i)
    } else {
        Some(assemble_simple(s, i))
    }
}

/// Assembles frame `i` for a non-merge atlas (Java `assembleSimple`, private).
fn assemble_simple(s: &PngMergerState, i: i32) -> Vec<i8> {
    // byte[] bArrM49a = mpdBytes(i);
    let mpd: &[i8] = mpd_bytes(s, i);
    // int iA = readU32(this.mphData, 8 + (i * 8) + 2);
    let i_a: i32 = read_u32(
        &s.mph_data,
        8i32.wrapping_add(i.wrapping_mul(8)).wrapping_add(2),
    );
    // int iM53b = frameLength(i);
    let i_m53b: i32 = frame_length(s, i);
    // byte[] bArr = new byte[8 + iM53b + 12];
    let mut b_arr: Vec<i8> = vec![0i8; 8i32.wrapping_add(i_m53b).wrapping_add(12) as usize];
    // System.arraycopy(PNG_SIGNATURE, 0, bArr, 0, 8);
    b_arr[0..8].copy_from_slice(&PNG_SIGNATURE);
    // System.arraycopy(bArrM49a, iA, bArr, 8, iM53b);
    let src: usize = i_a as usize;
    let n: usize = i_m53b as usize;
    b_arr[8..8 + n].copy_from_slice(&mpd[src..src + n]);
    // System.arraycopy(IEND_CHUNK, 0, bArr, 8 + iM53b, 12);
    let dst: usize = 8i32.wrapping_add(i_m53b) as usize;
    b_arr[dst..dst + 12].copy_from_slice(&IEND_CHUNK);
    b_arr
}

/// Assembles frame `i` for a merge-palette atlas (Java `assembleMerged`, private).
fn assemble_merged(s: &PngMergerState, i: i32) -> Option<Vec<i8>> {
    // byte[] bArrM49a = mpdBytes(i);
    let mpd: &[i8] = mpd_bytes(s, i);
    // int iA3 = readU32(this.mphData, 8 + (i * 8) + 2);
    let i_a3: i32 = read_u32(
        &s.mph_data,
        8i32.wrapping_add(i.wrapping_mul(8)).wrapping_add(2),
    );
    // int iM53b = frameLength(i);
    let i_m53b: i32 = frame_length(s, i);
    // byte[] bArr = new byte[8 + (this.mphData.length - ((readU32(this.mphData, 4) * 8) + 8)) + iM53b + 12];
    let size: i32 = 8i32
        .wrapping_add(
            (s.mph_data.len() as i32)
                .wrapping_sub(read_u32(&s.mph_data, 4).wrapping_mul(8).wrapping_add(8)),
        )
        .wrapping_add(i_m53b)
        .wrapping_add(12);
    let mut b_arr: Vec<i8> = vec![0i8; size as usize];
    // System.arraycopy(PNG_SIGNATURE, 0, bArr, 0, 8);
    b_arr[0..8].copy_from_slice(&PNG_SIGNATURE);
    // int iA4 = findChunk(bArrM49a, 0, iA3, iM53b);
    let i_a4: i32 = find_chunk(mpd, 0, i_a3, i_m53b);
    // if (iA4 == -1) return null;
    if i_a4 == -1 {
        return None;
    }
    // int iA5 = readU32(bArrM49a, iA4) + 12;
    let i_a5: i32 = read_u32(mpd, i_a4).wrapping_add(12);
    // System.arraycopy(bArrM49a, iA4, bArr, 8, iA5);
    b_arr[8..8 + i_a5 as usize].copy_from_slice(&mpd[i_a4 as usize..i_a4 as usize + i_a5 as usize]);
    // int i2 = 8 + iA5;
    let mut i2: i32 = 8i32.wrapping_add(i_a5);
    // for (int i3 = 0; i3 < 18; i3++)
    let mut i3: i32 = 0;
    while i3 < 18 {
        // if (frameHasChunk(i, i3))
        if frame_has_chunk(s, i, i3) {
            // switch (i3) { case 1..5,9,10: ... }
            if matches!(i3, 1 | 2 | 3 | 4 | 5 | 9 | 10) {
                // int iA6 = findChunk(bArrM49a, i3, iA3, iM53b);
                let i_a6: i32 = find_chunk(mpd, i3, i_a3, i_m53b);
                // if (iA6 != -1)
                if i_a6 != -1 {
                    // int iA7 = readU32(bArrM49a, iA6) + 12;
                    let i_a7: i32 = read_u32(mpd, i_a6).wrapping_add(12);
                    // System.arraycopy(bArrM49a, iA6, bArr, i2, iA7);
                    b_arr[i2 as usize..i2 as usize + i_a7 as usize]
                        .copy_from_slice(&mpd[i_a6 as usize..i_a6 as usize + i_a7 as usize]);
                    // i2 += iA7;
                    i2 = i2.wrapping_add(i_a7);
                }
            }
        }
        i3 = i3.wrapping_add(1);
    }
    // int i4 = this.pltePos;
    let i4: i32 = s.plte_pos;
    // int iA8 = readU32(this.mphData, i4) + 12;
    let i_a8: i32 = read_u32(&s.mph_data, i4).wrapping_add(12);
    // System.arraycopy(this.mphData, i4, bArr, i2, iA8);
    b_arr[i2 as usize..i2 as usize + i_a8 as usize]
        .copy_from_slice(&s.mph_data[i4 as usize..i4 as usize + i_a8 as usize]);
    // int i5 = i2 + iA8;
    let mut i5: i32 = i2.wrapping_add(i_a8);
    // int i6 = this.trnsPos;
    let i6: i32 = s.trns_pos;
    // if (i6 != -1) { ... tRNS ... }
    if i6 != -1 {
        // int iA9 = readU32(this.mphData, i6) + 12;
        let i_a9: i32 = read_u32(&s.mph_data, i6).wrapping_add(12);
        // System.arraycopy(this.mphData, i6, bArr, i5, iA9);
        b_arr[i5 as usize..i5 as usize + i_a9 as usize]
            .copy_from_slice(&s.mph_data[i6 as usize..i6 as usize + i_a9 as usize]);
        // i5 += iA9;
        i5 = i5.wrapping_add(i_a9);
    }
    // if (frameHasChunk(i, 14) && (iA2 = findChunk(bArrM49a, 14, iA3, iM53b)) != -1)
    if frame_has_chunk(s, i, 14) {
        let i_a2: i32 = find_chunk(mpd, 14, i_a3, i_m53b);
        if i_a2 != -1 {
            // int iA10 = readU32(bArrM49a, iA2) + 12;
            let i_a10: i32 = read_u32(mpd, i_a2).wrapping_add(12);
            // System.arraycopy(bArrM49a, iA2, bArr, i5, iA10);
            b_arr[i5 as usize..i5 as usize + i_a10 as usize]
                .copy_from_slice(&mpd[i_a2 as usize..i_a2 as usize + i_a10 as usize]);
            // i5 += iA10;
            i5 = i5.wrapping_add(i_a10);
        }
    }
    // if (frameHasChunk(i, 15) && (iA = findChunk(bArrM49a, 15, iA3, iM53b)) != -1)
    if frame_has_chunk(s, i, 15) {
        let i_a: i32 = find_chunk(mpd, 15, i_a3, i_m53b);
        if i_a != -1 {
            // int iA11 = readU32(bArrM49a, iA) + 12;
            let i_a11: i32 = read_u32(mpd, i_a).wrapping_add(12);
            // System.arraycopy(bArrM49a, iA, bArr, i5, iA11);
            b_arr[i5 as usize..i5 as usize + i_a11 as usize]
                .copy_from_slice(&mpd[i_a as usize..i_a as usize + i_a11 as usize]);
            // i5 += iA11;
            i5 = i5.wrapping_add(i_a11);
        }
    }
    // int iA12 = findChunk(bArrM49a, 16, iA3, iM53b);
    let i_a12: i32 = find_chunk(mpd, 16, i_a3, i_m53b);
    // int iA13 = readU32(bArrM49a, iA12) + 12;
    let i_a13: i32 = read_u32(mpd, i_a12).wrapping_add(12);
    // System.arraycopy(bArrM49a, iA12, bArr, i5, iA13);
    b_arr[i5 as usize..i5 as usize + i_a13 as usize]
        .copy_from_slice(&mpd[i_a12 as usize..i_a12 as usize + i_a13 as usize]);
    // System.arraycopy(IEND_CHUNK, 0, bArr, i5 + iA13, 12);
    let dst: usize = i5.wrapping_add(i_a13) as usize;
    b_arr[dst..dst + 12].copy_from_slice(&IEND_CHUNK);
    Some(b_arr)
}

/// Byte length of frame `i`'s data inside its `.mpd` (Java `frameLength`, private).
fn frame_length(s: &PngMergerState, i: i32) -> i32 {
    // byte[] bArrM49a = mpdBytes(i);
    let mpd: &[i8] = mpd_bytes(s, i);
    // ((i == frameCount() - 1 || readU16(mphData, 8+(i*8)) != readU16(mphData, 8+((i+1)*8)))
    //    ? bArrM49a.length : readU32(mphData, (8+((i+1)*8))+2)) - readU32(mphData, (8+(i*8))+2)
    let cond: bool = i == frame_count(s).wrapping_sub(1)
        || (read_u16(&s.mph_data, 8i32.wrapping_add(i.wrapping_mul(8))) as i32)
            != (read_u16(
                &s.mph_data,
                8i32.wrapping_add(i.wrapping_add(1).wrapping_mul(8)),
            ) as i32);
    let hi: i32 = if cond {
        mpd.len() as i32
    } else {
        read_u32(
            &s.mph_data,
            8i32.wrapping_add(i.wrapping_add(1).wrapping_mul(8))
                .wrapping_add(2),
        )
    };
    hi.wrapping_sub(read_u32(
        &s.mph_data,
        8i32.wrapping_add(i.wrapping_mul(8)).wrapping_add(2),
    ))
}

/// Finds the byte offset of the `CHUNK_TYPES[i]` chunk within `[i2, i2+i3)` (or
/// the whole buffer when `i3 == -1`); -1 if absent (Java `findChunk`, private).
fn find_chunk(b_arr: &[i8], i: i32, i2: i32, i3: i32) -> i32 {
    // String str = CHUNK_TYPES[i];
    let str_bytes: &[u8] = CHUNK_TYPES[i as usize].as_bytes();
    // int length = i3 == -1 ? bArr.length : i2 + i3;
    let length: i32 = if i3 == -1 {
        b_arr.len() as i32
    } else {
        i2.wrapping_add(i3)
    };
    // int iA = i2;
    let mut i_a: i32 = i2;
    loop {
        // int i4 = iA;
        let i4: i32 = i_a;
        // if (i4 >= length) return -1;
        if i4 >= length {
            return -1;
        }
        // if (bArr[i4+4]==charAt0 && ... bArr[i4+7]==charAt3) return i4;
        if (b_arr[i4.wrapping_add(4) as usize] as i32) == (str_bytes[0] as i32)
            && (b_arr[i4.wrapping_add(5) as usize] as i32) == (str_bytes[1] as i32)
            && (b_arr[i4.wrapping_add(6) as usize] as i32) == (str_bytes[2] as i32)
            && (b_arr[i4.wrapping_add(7) as usize] as i32) == (str_bytes[3] as i32)
        {
            return i4;
        }
        // iA = i4 + readU32(bArr, i4) + 12;
        i_a = i4.wrapping_add(read_u32(b_arr, i4)).wrapping_add(12);
    }
}

/// Reads a big-endian unsigned 32-bit value at `i` (Java `readU32`, private static).
fn read_u32(b_arr: &[i8], i: i32) -> i32 {
    // if (bArr.length - 4 < i) throw new ArrayIndexOutOfBoundsException();
    if (b_arr.len() as i32).wrapping_sub(4) < i {
        panic!("ArrayIndexOutOfBoundsException"); // PngMerger.java:356
    }
    // 0 + ((bArr[i] & 255) * 16777216) + ((bArr[i+1] & 255) * 65536)
    //   + ((bArr[i+2] & 255) * 256) + (bArr[i+3] & 255)
    (0i32)
        .wrapping_add(((b_arr[i as usize] as i32) & 255).wrapping_mul(16777216))
        .wrapping_add(((b_arr[i.wrapping_add(1) as usize] as i32) & 255).wrapping_mul(65536))
        .wrapping_add(((b_arr[i.wrapping_add(2) as usize] as i32) & 255).wrapping_mul(256))
        .wrapping_add((b_arr[i.wrapping_add(3) as usize] as i32) & 255)
}

/// Reads a big-endian unsigned 16-bit value at `i` (Java `readU16`, private static).
fn read_u16(b_arr: &[i8], i: i32) -> u16 {
    // if (bArr.length - 2 < i) throw new ArrayIndexOutOfBoundsException();
    if (b_arr.len() as i32).wrapping_sub(2) < i {
        panic!("ArrayIndexOutOfBoundsException"); // PngMerger.java:364
    }
    // (char) (((char) (0 + ((bArr[i] & 255) * 256))) + (bArr[i + 1] & 255))
    let hi: u16 = (0i32).wrapping_add(((b_arr[i as usize] as i32) & 255).wrapping_mul(256)) as u16;
    // char promotes (zero-extend) to int, add, then narrow back to char.
    (hi as i32).wrapping_add((b_arr[i.wrapping_add(1) as usize] as i32) & 255) as u16
}

/// True if frame `i`'s chunk bitmask has optional chunk `i2` set (Java
/// `frameHasChunk`, private).
// The two comparisons are faithful to Java `i2 >= 1 && i2 <= 16`; kept explicit.
#[allow(clippy::manual_range_contains)]
fn frame_has_chunk(s: &PngMergerState, i: i32, i2: i32) -> bool {
    // i2 >= 1 && i2 <= 16 && ((this.chunkMasks[i] >> (i2 - 1)) & 1) == 1
    i2 >= 1 && i2 <= 16 && (ishr(s.chunk_masks[i as usize] as i32, i2.wrapping_sub(1)) & 1) == 1
}

/// Scans the whole buffer for the `CHUNK_TYPES[i]` chunk, returning the offset of
/// its length field (-1 if absent) (Java `locateChunk`, private static).
fn locate_chunk(b_arr: &[i8], i: i32) -> i32 {
    // String str = CHUNK_TYPES[i];
    let str_bytes: &[u8] = CHUNK_TYPES[i as usize].as_bytes();
    // int length = bArr.length;
    let length: i32 = b_arr.len() as i32;
    // for (int i2 = 0; i2 < length - 3; i2++)
    let mut i2: i32 = 0;
    while i2 < length.wrapping_sub(3) {
        // if (bArr[i2]==charAt0 && ... bArr[i2+3]==charAt3) return i2 - 4;
        if (b_arr[i2 as usize] as i32) == (str_bytes[0] as i32)
            && (b_arr[i2.wrapping_add(1) as usize] as i32) == (str_bytes[1] as i32)
            && (b_arr[i2.wrapping_add(2) as usize] as i32) == (str_bytes[2] as i32)
            && (b_arr[i2.wrapping_add(3) as usize] as i32) == (str_bytes[3] as i32)
        {
            return i2.wrapping_sub(4);
        }
        i2 = i2.wrapping_add(1);
    }
    // return -1;
    -1
}

/// Horizontally mirrors a decoded frame in place, then fixes Adler-32 and CRC-32
/// (Java `mirror`, public static).
pub fn mirror(b_arr: &mut [i8]) {
    // int iA = findChunk(bArr, 16, 8, bArr.length);
    let i_a: i32 = find_chunk(b_arr, 16, 8, b_arr.len() as i32);
    // int iA2 = findChunk(bArr, 0, 8, bArr.length);
    let i_a2: i32 = find_chunk(b_arr, 0, 8, b_arr.len() as i32);
    // mirrorScanlines(bArr, iA, readU32(bArr, iA2+8), readU32(bArr, iA2+12), bArr[iA2+16]);
    let w: i32 = read_u32(b_arr, i_a2.wrapping_add(8));
    let h: i32 = read_u32(b_arr, i_a2.wrapping_add(12));
    let depth: i32 = b_arr[i_a2.wrapping_add(16) as usize] as i32; // byte -> int (sign-extend)
    mirror_scanlines(b_arr, i_a, w, h, depth);
}

/// The raw-scanline pixel-mirror worker (Java `mirrorScanlines`, private static).
fn mirror_scanlines(b_arr: &mut [i8], i: i32, i2: i32, i3: i32, i4: i32) {
    // int i5 = 8 / i4;
    let i5: i32 = java_div(8, i4).expect("mirror 8/depth");
    // int i6 = ((i2 - 1) / i5) + 1;
    let i6: i32 = java_div(i2.wrapping_sub(1), i5)
        .expect("mirror (w-1)/i5")
        .wrapping_add(1);
    // byte b2 = (byte) (255 >> (8 - i4));
    let b2: i8 = ishr(255, 8i32.wrapping_sub(i4)) as i8;
    // int i7 = i + 15;
    let i7: i32 = i.wrapping_add(15);
    // int i8 = (i6 + 1) * i3;
    let i8: i32 = i6.wrapping_add(1).wrapping_mul(i3);
    // int i9 = i2 / 2;
    let i9: i32 = java_div(i2, 2).expect("mirror w/2");
    // int i10 = i7 + i8;
    let i10: i32 = i7.wrapping_add(i8);
    // int i11 = i10 + 4;
    let i11: i32 = i10.wrapping_add(4);
    // int i12 = i + 4;
    let i12: i32 = i.wrapping_add(4);
    // for (int i13 = 0; i13 < i3; i13++) if (bArr[i7 + ((i6+1)*i13)] != 0) return;
    let mut i13: i32 = 0;
    while i13 < i3 {
        if b_arr[i7.wrapping_add(i6.wrapping_add(1).wrapping_mul(i13)) as usize] != 0 {
            return;
        }
        i13 = i13.wrapping_add(1);
    }
    // for (int i14 = 0; i14 < i3; i14++)
    let mut i14: i32 = 0;
    while i14 < i3 {
        // int i15 = i7 + ((i6+1)*i14) + 1;
        let i15: i32 = i7
            .wrapping_add(i6.wrapping_add(1).wrapping_mul(i14))
            .wrapping_add(1);
        // for (int i16 = 0; i16 < i9; i16++)
        let mut i16: i32 = 0;
        while i16 < i9 {
            // int i17 = (i2 - 1) - i16;
            let i17: i32 = i2.wrapping_sub(1).wrapping_sub(i16);
            // int i18 = i15 + (i16 / i5);
            let i18: i32 = i15.wrapping_add(java_div(i16, i5).expect("i16/i5"));
            // int i19 = i15 + (i17 / i5);
            let i19: i32 = i15.wrapping_add(java_div(i17, i5).expect("i17/i5"));
            // int i20 = i16 % i5;
            let i20: i32 = java_rem(i16, i5).expect("i16%i5");
            // int i21 = i17 % i5;
            let i21: i32 = java_rem(i17, i5).expect("i17%i5");
            // byte b3 = (byte) (((i5 - i20) - 1) * i4);
            let b3: i8 = i5.wrapping_sub(i20).wrapping_sub(1).wrapping_mul(i4) as i8;
            // byte b4 = (byte) (((i5 - i21) - 1) * i4);
            let b4: i8 = i5.wrapping_sub(i21).wrapping_sub(1).wrapping_mul(i4) as i8;
            // byte b5 = (byte) ((bArr[i18] >> b3) & b2);
            let b5: i8 = (ishr(b_arr[i18 as usize] as i32, b3 as i32) & (b2 as i32)) as i8;
            // bArr[i18] = (byte)((bArr[i18] & ((b2<<b3)^(-1))) | (((byte)((bArr[i19]>>b4)&b2)) << b3));
            let inner19: i8 = (ishr(b_arr[i19 as usize] as i32, b4 as i32) & (b2 as i32)) as i8;
            b_arr[i18 as usize] = ((b_arr[i18 as usize] as i32
                & (ishl(b2 as i32, b3 as i32) ^ (-1)))
                | ishl(inner19 as i32, b3 as i32)) as i8;
            // bArr[i19] = (byte)((bArr[i19] & ((b2<<b4)^(-1))) | (b5 << b4));
            b_arr[i19 as usize] = ((b_arr[i19 as usize] as i32
                & (ishl(b2 as i32, b4 as i32) ^ (-1)))
                | ishl(b5 as i32, b4 as i32)) as i8;
            i16 = i16.wrapping_add(1);
        }
        i14 = i14.wrapping_add(1);
    }
    // adler.reset(); adler.update(bArr, i7, i8);
    let mut adler = adler32::Adler32State::new();
    adler32::update(&mut adler, b_arr, i7, i8);
    // System.arraycopy(toBE32((int) adler.getValue()), 0, bArr, i10, 4);
    let be = to_be32(adler32::get_value(&adler) as i32);
    b_arr[i10 as usize..i10 as usize + 4].copy_from_slice(&be);
    // crc.reset(); crc.update(bArr, i12, i8 + 15);
    let mut crc = crc32::Crc32State::new();
    crc32::update(&mut crc, b_arr, i12, i8.wrapping_add(15));
    // System.arraycopy(toBE32(crc.getValue()), 0, bArr, i11, 4);
    let be = to_be32(crc32::get_value(&crc));
    b_arr[i11 as usize..i11 as usize + 4].copy_from_slice(&be);
}

/// `applyEffect(byte[], int)` — delegates to the three-arg form (arg 0).
pub fn apply_effect_default(b_arr: &mut [i8], i: i32) {
    // applyEffect(bArr, i, 0);
    apply_effect(b_arr, i, 0);
}

/// Applies recolor effect `i` (with arg `i2`) to the IDAT pixels and fixes the
/// CRC (Java `applyEffect(byte[],int,int)`, public static).
pub fn apply_effect(b_arr: &mut [i8], i: i32, i2: i32) {
    // transformPixels(bArr, findChunk(bArr, 12, 8, bArr.length), i, i2, 0);
    let pos: i32 = find_chunk(b_arr, 12, 8, b_arr.len() as i32);
    transform_pixels(b_arr, pos, i, i2, 0);
}

/// The pixel-recolor worker (Java `transformPixels`, private static). Mode `i2`
/// selects channel-swap (0), grayscale (1), brightness (2), invert (3) or
/// color-replace (4); recomputes the chunk CRC afterwards.
fn transform_pixels(b_arr: &mut [i8], i: i32, i2: i32, i3: i32, i4: i32) {
    // int iA = readU32(bArr, i);
    let i_a: i32 = read_u32(b_arr, i);
    // int i5 = i + 8;
    let i5: i32 = i.wrapping_add(8);
    // int i6 = i5 + iA;
    let i6: i32 = i5.wrapping_add(i_a);
    match i2 {
        0 => match i3 {
            0 => {
                // for (i7) swap [p] <-> [p+1]
                let mut i7: i32 = 0;
                while i7 < java_div(i_a, 3).expect("iA/3") {
                    let p: i32 = i5.wrapping_add(i7.wrapping_mul(3));
                    let b2: i8 = b_arr[p as usize];
                    b_arr[p as usize] = b_arr[p.wrapping_add(1) as usize];
                    b_arr[p.wrapping_add(1) as usize] = b2;
                    i7 = i7.wrapping_add(1);
                }
            }
            1 => {
                // for (i8) swap [p+1] <-> [p+2]
                let mut i8: i32 = 0;
                while i8 < java_div(i_a, 3).expect("iA/3") {
                    let p: i32 = i5.wrapping_add(i8.wrapping_mul(3));
                    let b3: i8 = b_arr[p.wrapping_add(1) as usize];
                    b_arr[p.wrapping_add(1) as usize] = b_arr[p.wrapping_add(2) as usize];
                    b_arr[p.wrapping_add(2) as usize] = b3;
                    i8 = i8.wrapping_add(1);
                }
            }
            2 => {
                // for (i9) swap [p] <-> [p+2]
                let mut i9: i32 = 0;
                while i9 < java_div(i_a, 3).expect("iA/3") {
                    let p: i32 = i5.wrapping_add(i9.wrapping_mul(3));
                    let b4: i8 = b_arr[p as usize];
                    b_arr[p as usize] = b_arr[p.wrapping_add(2) as usize];
                    b_arr[p.wrapping_add(2) as usize] = b4;
                    i9 = i9.wrapping_add(1);
                }
            }
            3 => {
                // for (i10) rotate: b5=[p]; [p]=[p+2]; [p+2]=[p+1]; [p+1]=b5
                let mut i10: i32 = 0;
                while i10 < java_div(i_a, 3).expect("iA/3") {
                    let p: i32 = i5.wrapping_add(i10.wrapping_mul(3));
                    let b5: i8 = b_arr[p as usize];
                    b_arr[p as usize] = b_arr[p.wrapping_add(2) as usize];
                    b_arr[p.wrapping_add(2) as usize] = b_arr[p.wrapping_add(1) as usize];
                    b_arr[p.wrapping_add(1) as usize] = b5;
                    i10 = i10.wrapping_add(1);
                }
            }
            4 => {
                // for (i11) rotate: b6=[p]; [p]=[p+1]; [p+1]=[p+2]; [p+2]=b6
                let mut i11: i32 = 0;
                while i11 < java_div(i_a, 3).expect("iA/3") {
                    let p: i32 = i5.wrapping_add(i11.wrapping_mul(3));
                    let b6: i8 = b_arr[p as usize];
                    b_arr[p as usize] = b_arr[p.wrapping_add(1) as usize];
                    b_arr[p.wrapping_add(1) as usize] = b_arr[p.wrapping_add(2) as usize];
                    b_arr[p.wrapping_add(2) as usize] = b6;
                    i11 = i11.wrapping_add(1);
                }
            }
            _ => {}
        },
        1 => {
            // grayscale: b7 = (byte)(((r&255)+(g&255)+(b&255)) / 3); r=g=b=b7
            let mut i12: i32 = 0;
            while i12 < java_div(i_a, 3).expect("iA/3") {
                let p: i32 = i5.wrapping_add(i12.wrapping_mul(3));
                let sum: i32 = ((b_arr[p as usize] as i32) & 255)
                    .wrapping_add((b_arr[p.wrapping_add(1) as usize] as i32) & 255)
                    .wrapping_add((b_arr[p.wrapping_add(2) as usize] as i32) & 255);
                let b7: i8 = java_div(sum, 3).expect("gray/3") as i8;
                b_arr[p as usize] = b7;
                b_arr[p.wrapping_add(1) as usize] = b7;
                b_arr[p.wrapping_add(2) as usize] = b7;
                i12 = i12.wrapping_add(1);
            }
        }
        2 => {
            // brightness: each channel = min(255, (c * (i3*10)) / 1000)
            let mut i13: i32 = 0;
            while i13 < java_div(i_a, 3).expect("iA/3") {
                let p: i32 = i5.wrapping_add(i13.wrapping_mul(3));
                let i14: i32 = (b_arr[p as usize] as i32) & 255;
                let i15: i32 = (b_arr[p.wrapping_add(1) as usize] as i32) & 255;
                let i16: i32 = (b_arr[p.wrapping_add(2) as usize] as i32) & 255;
                b_arr[p as usize] = scale_channel(i14, i3);
                b_arr[p.wrapping_add(1) as usize] = scale_channel(i15, i3);
                b_arr[p.wrapping_add(2) as usize] = scale_channel(i16, i3);
                i13 = i13.wrapping_add(1);
            }
        }
        3 => {
            // invert: each channel ^= -1
            let mut i17: i32 = 0;
            while i17 < java_div(i_a, 3).expect("iA/3") {
                let p: i32 = i5.wrapping_add(i17.wrapping_mul(3));
                b_arr[p as usize] = (b_arr[p as usize] as i32 ^ (-1)) as i8;
                b_arr[p.wrapping_add(1) as usize] =
                    (b_arr[p.wrapping_add(1) as usize] as i32 ^ (-1)) as i8;
                b_arr[p.wrapping_add(2) as usize] =
                    (b_arr[p.wrapping_add(2) as usize] as i32 ^ (-1)) as i8;
                i17 = i17.wrapping_add(1);
            }
        }
        4 => {
            // color-replace: pixels equal to (b8,b9,b10) become (b11,b12,b13)
            let b8: i8 = (ishr(i3, 16) & 255) as i8;
            let b9: i8 = (ishr(i3, 8) & 255) as i8;
            let b10: i8 = (i3 & 255) as i8;
            let b11: i8 = (ishr(i4, 16) & 255) as i8;
            let b12: i8 = (ishr(i4, 8) & 255) as i8;
            let b13: i8 = (i4 & 255) as i8;
            let mut i18: i32 = 0;
            while i18 < java_div(i_a, 3).expect("iA/3") {
                let p: i32 = i5.wrapping_add(i18.wrapping_mul(3));
                if b_arr[p as usize] == b8
                    && b_arr[p.wrapping_add(1) as usize] == b9
                    && b_arr[p.wrapping_add(2) as usize] == b10
                {
                    b_arr[p as usize] = b11;
                    b_arr[p.wrapping_add(1) as usize] = b12;
                    b_arr[p.wrapping_add(2) as usize] = b13;
                }
                i18 = i18.wrapping_add(1);
            }
        }
        _ => {}
    }
    // crc.reset(); crc.update(bArr, i + 4, iA + 4);
    let mut crc = crc32::Crc32State::new();
    crc32::update(&mut crc, b_arr, i.wrapping_add(4), i_a.wrapping_add(4));
    // System.arraycopy(toBE32(crc.getValue()), 0, bArr, i6, 4);
    let be = to_be32(crc32::get_value(&crc));
    b_arr[i6 as usize..i6 as usize + 4].copy_from_slice(&be);
}

/// `(byte) ((c * (i3*10)) / 1000 < 255 ? (c * (i3*10)) / 1000 : 255)` — the
/// brightness clamp used by [`transform_pixels`] case 2.
fn scale_channel(c: i32, i3: i32) -> i8 {
    let scaled: i32 = java_div(c.wrapping_mul(i3.wrapping_mul(10)), 1000).expect("brightness/1000");
    (if scaled < 255 { scaled } else { 255 }) as i8
}

/// Encodes `i` as 4 big-endian bytes (Java `toBE32`, private static).
fn to_be32(i: i32) -> [i8; 4] {
    // new byte[]{(byte)((i>>24)&255), (byte)((i>>16)&255), (byte)((i>>8)&255), (byte)(i&255)}
    [
        (ishr(i, 24) & 255) as i8,
        (ishr(i, 16) & 255) as i8,
        (ishr(i, 8) & 255) as i8,
        (i & 255) as i8,
    ]
}

// ===========================================================================
// The previously-DEFERRED resource / `Image` boundary (docs/TRANSLITERATION.md,
// accepted deviations). These wrap the oracle-tested decoder core above with the
// `AssetCache.readResource` reads and the `javax.microedition.lcdui.Image`
// decode + `BaseCanvas.yieldTick`, driving the title (logo) render path. They
// take `&mut Game` (for `readResource` + the device `Image` factory) alongside
// the `PngMergerState`, per the transliteration's free-function convention.
// ===========================================================================

/// `public PngMerger(String str) throws IOException`
/// (`br.<init>:(Ljava/lang/String;)V`) — constructs and loads the atlas.
pub fn construct(g: &mut Game, str: &str) -> PngMergerState {
    // this(); load(str);   — the no-arg <init> leaves every field at its default.
    let mut s = PngMergerState::new();
    load(g, &mut s, str);
    s
}

/// `public final void load(String str) throws IOException`
/// (`br.a:(Ljava/lang/String;)V => []`): resets state, records the base path,
/// reads the `.mph` index.
pub fn load(g: &mut Game, s: &mut PngMergerState, str: &str) {
    // this.framesPerMpd = null; this.mphData = null; this.mpdData = null; this.chunkMasks = null;
    s.frames_per_mpd = Vec::new();
    s.mph_data = Vec::new();
    s.mpd_data = Vec::new();
    s.chunk_masks = Vec::new();
    // this.basePath = str;
    s.base_path = str.to_string();
    // readIndex();
    read_index(g, s);
}

/// `private void readIndex() throws IOException` (`br.b:()V => []`): reads the
/// `.mph` blob into `mphData` and parses its header.
pub fn read_index(g: &mut Game, s: &mut PngMergerState) {
    // this.mphData = AssetCache.readResource(this.basePath + ".mph");
    let path = format!("{}.mph", s.base_path);
    // A null return here is a missing index (NPE in parseHeader on the real
    // device); the atlas exists on the classpath, so it is unwrapped.
    s.mph_data = asset_cache::read_resource(g, &path).expect("readResource(.mph) returned null");
    // parseHeader();
    parse_header(s);
}

/// `public final void loadMpd(int i) throws IOException` (`br.a:(I)V => []`):
/// loads `_<i>.mpd` into `mpdData[i]` (Java stores the possibly-null `byte[]`).
pub fn load_mpd(g: &mut Game, s: &mut PngMergerState, i: i32) {
    // this.mpdData[i] = AssetCache.readResource(this.basePath + "_" + i + ".mpd");
    let path = format!("{}_{}.mpd", s.base_path, i);
    s.mpd_data[i as usize] = asset_cache::read_resource(g, &path);
}

/// The reload decision Java performs lazily inside `mpdBytes` — hoisted to just
/// before assembly so the oracle-tested `mpd_bytes`/`assemble_frame` core stays
/// untouched. `if (preloadAll && mpdData[k] == null) { unloadAllMpd(); loadMpd(k); }`.
/// Net-identical to the in-`mpdBytes` reload (same `unloadAllMpd`+`loadMpd(k)`).
fn ensure_mpd(g: &mut Game, s: &mut PngMergerState, i: i32) {
    let k: i32 = read_u16(&s.mph_data, 8i32.wrapping_add(8i32.wrapping_mul(i))) as i32; // mpdIndexOf(i)
    if s.preload_all && s.mpd_data[k as usize].is_none() {
        unload_all_mpd(s);
        load_mpd(g, s, k);
    }
}

/// `public final Image image(int i)` (`br.a:(I)Ljavax/microedition/lcdui/Image;`):
/// assembles and decodes frame `i` (base bank).
pub fn image(g: &mut Game, s: &mut PngMergerState, i: i32) -> j2me_me::Image {
    // (reload hoisted from mpdBytes) then:
    ensure_mpd(g, s, i);
    // byte[] bArrM51b = assembleFrame(i);
    let b_arr = assemble_frame(s, i).expect("assembleFrame returned null (merged path, no IHDR)");
    // return Image.createImage(bArrM51b, 0, bArrM51b.length);
    let len = b_arr.len() as i32;
    j2me_me::create_image_region(&b_arr, 0, len).expect("Image.createImage(byte[],int,int)")
}

/// `public final Image[] allImages()`
/// (`br.a:()[Ljavax/microedition/lcdui/Image; => [iinc]`): extracts every frame,
/// enabling `preloadAll`, then frees the `.mpd` bytes.
pub fn all_images(g: &mut Game, s: &mut PngMergerState) -> Vec<j2me_me::Image> {
    // this.preloadAll = true;
    s.preload_all = true;
    // int iM45a = frameCount();
    let i_m45a: i32 = frame_count(s);
    // Image[] imageArr = new Image[iM45a];
    let mut image_arr: Vec<j2me_me::Image> = Vec::with_capacity(i_m45a as usize);
    // for (int i = 0; i < iM45a; i++) { imageArr[i] = image(i); BaseCanvas.yieldTick(); }
    let mut i: i32 = 0;
    while i < i_m45a {
        image_arr.push(image(g, s, i));
        base_canvas::yield_tick(g);
        i = i.wrapping_add(1);
    }
    // unloadAllMpd();
    unload_all_mpd(s);
    image_arr
}

/// `public final Image imageMirrored(int i)`
/// (`br.b:(I)Ljavax/microedition/lcdui/Image;`): frame `i` from the mirrored bank;
/// falls back to [`image`] when the atlas needs no remap.
pub fn image_mirrored(g: &mut Game, s: &mut PngMergerState, i: i32) -> j2me_me::Image {
    // if (!this.paletteRemap) return image(i);
    if !s.palette_remap {
        return image(g, s, i);
    }
    ensure_mpd(g, s, i);
    // byte[] bArrM51b = assembleFrame(i); mirror(bArrM51b);
    let mut b_arr = assemble_frame(s, i).expect("assembleFrame returned null");
    mirror(&mut b_arr);
    // return Image.createImage(bArrM51b, 0, bArrM51b.length);
    let len = b_arr.len() as i32;
    j2me_me::create_image_region(&b_arr, 0, len).expect("Image.createImage(byte[],int,int)")
}
