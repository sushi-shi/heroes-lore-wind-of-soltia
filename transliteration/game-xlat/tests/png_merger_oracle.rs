//! PngMerger reconstruction oracle ("two implementations, one truth").
//!
//! The strict transliteration [`heroes_lore_wind_of_soltia_game_xlat::png_merger`] (`br.class`)
//! reassembles a full, decodable PNG for every atlas frame. It is cross-checked
//! against the INDEPENDENT, separately-reversed `.mph`/`.mpd` parsers in
//! [`heroes_lore_wind_of_soltia_formats`], over EVERY frame of EVERY atlas in `_originals/…v207.jar`:
//!
//!  1. `heroes_lore_wind_of_soltia_formats::mph::parse` decodes each frame record `offset`; the
//!     transliteration's reconstruction must pull the frame from *that same
//!     offset* — proven by the reassembled IHDR chunk being byte-identical to the
//!     `.mpd` bytes at `offset` (which the reconstruction reaches via its own
//!     independent `readU32(mphData, 8 + i*8 + 2)`).
//!  2. That `offset` must land on a real `IHDR` start, per the independent
//!     `heroes_lore_wind_of_soltia_formats::mpd::ihdr_offsets` — the structural offset agreement.
//!  3. Every reassembled frame PNG must actually **decode** (the `png` crate is a
//!     third independent implementation) — the liveness proof — and its decoded
//!     width/height must equal the `.mpd`'s IHDR at `offset`.
//!
//! Non-vacuity floors (GATES.md R3): >= 170 `.mpd`, >= 1800 frames.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_formats::{mpd, mph};
use heroes_lore_wind_of_soltia_game_xlat::png_merger::{self, PngMergerState};
use std::collections::HashSet;

/// The 8-byte PNG signature the transliteration prepends to every frame.
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// Byte length of an `IHDR` chunk: `len(4) + "IHDR"(4) + data(13) + crc(4)`.
const IHDR_CHUNK_LEN: usize = 25;

fn be_u32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Reinterpret the transliteration's `Vec<i8>` output as host bytes for decoding.
fn to_u8(bytes: &[i8]) -> Vec<u8> {
    bytes.iter().map(|&b| b as u8).collect()
}

/// Build a fully-loaded merger for one atlas: inject the `.mph` blob, parse its
/// header, then load every paired `<stem>_<k>.mpd`. Stands in for the deferred
/// `AssetCache` load path (see the module's transliteration notes).
fn load_atlas(stem: &str, mph_bytes: &[u8]) -> PngMergerState {
    let mut s = PngMergerState::new();
    s.mph_data = to_i8(mph_bytes);
    png_merger::parse_header(&mut s);
    for k in 0..s.mpd_data.len() {
        let name = format!("{stem}_{k}.mpd");
        if let Some(bytes) = jar().get(&name) {
            s.mpd_data[k] = Some(to_i8(bytes));
        }
    }
    s
}

/// Decode a reconstructed PNG, returning `(width, height)`. This both proves the
/// bytes are a valid PNG and decompresses the IDAT (validating zlib + CRCs).
fn decode_png(bytes: &[u8]) -> (u32, u32) {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|e| panic!("reconstructed PNG failed to read_info: {e}"));
    let (w, h) = {
        let info = reader.info();
        (info.width, info.height)
    };
    let mut buf = vec![0u8; reader.output_buffer_size()];
    reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("reconstructed PNG failed to decode IDAT: {e}"));
    (w, h)
}

#[test]
fn every_frame_reconstructs_and_agrees_with_the_format_parsers() {
    let mphs = jar().matching(|n| n.ends_with(".mph"));
    assert!(
        !mphs.is_empty(),
        "no .mph atlases found — extraction is broken"
    );

    let mut total_frames = 0usize;
    let mut decoded_ok = 0usize;
    let mut mpd_files_seen: HashSet<String> = HashSet::new();
    // Cache IHDR offsets per paired .mpd file (usually only `_0.mpd`).
    let mut ihdr_cache: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();

    for (name, mph_bytes) in &mphs {
        let stem = name.strip_suffix(".mph").expect(".mph name");
        // Independent parse of the index.
        let m = mph::parse(mph_bytes).unwrap_or_else(|e| panic!("{name}: mph parse: {e}"));

        // The transliteration's own header read must agree on the frame count.
        let s = load_atlas(stem, mph_bytes);
        assert_eq!(
            png_merger::frame_count(&s) as u64,
            m.count as u64,
            "{name}: transliteration frame_count != mph count"
        );

        for (i, rec) in m.frames.iter().enumerate() {
            // The paired .mpd for this frame.
            let mpd_name = format!("{stem}_{}.mpd", rec.mpd_index);
            let mpd_bytes = jar()
                .get(&mpd_name)
                .unwrap_or_else(|| panic!("{name} frame {i}: missing paired {mpd_name}"));
            mpd_files_seen.insert(mpd_name.clone());
            let ihdrs = ihdr_cache.entry(mpd_name.clone()).or_insert_with(|| {
                mpd::ihdr_offsets(mpd_bytes)
                    .unwrap_or_else(|e| panic!("{mpd_name}: ihdr_offsets: {e}"))
            });

            // (2) mph's offset lands on a real IHDR (independent structural check).
            assert!(
                ihdrs.contains(&(rec.offset as usize)),
                "{name} frame {i}: mph offset {} is not an IHDR start in {mpd_name}",
                rec.offset
            );

            // Reconstruct the frame through the transliteration.
            let recon_i8 = png_merger::assemble_frame(&s, i as i32)
                .unwrap_or_else(|| panic!("{name} frame {i}: assemble_frame returned null"));
            let recon = to_u8(&recon_i8);

            // The signature must be the PNG magic.
            assert_eq!(
                &recon[0..8],
                &PNG_SIGNATURE,
                "{name} frame {i}: reconstructed signature is not the PNG magic"
            );

            // (1) The reassembled IHDR chunk is byte-identical to the .mpd bytes at
            // the mph-reported offset — proving the transliteration pulled the
            // frame from exactly `rec.offset` (its own readU32 vs mph::parse).
            let off = rec.offset as usize;
            assert_eq!(
                &recon[8..8 + IHDR_CHUNK_LEN],
                &mpd_bytes[off..off + IHDR_CHUNK_LEN],
                "{name} frame {i}: reassembled IHDR != .mpd IHDR at offset {off} \
                 (offset disagreement between transliteration and mph::parse)"
            );

            // (3) Liveness: the reconstruction is a valid, decodable PNG, and its
            // dimensions equal the .mpd's IHDR at `offset`.
            let (w, h) = decode_png(&recon);
            assert_eq!(
                w,
                be_u32(mpd_bytes, off + 8),
                "{name} frame {i}: decoded width != .mpd IHDR width"
            );
            assert_eq!(
                h,
                be_u32(mpd_bytes, off + 12),
                "{name} frame {i}: decoded height != .mpd IHDR height"
            );
            decoded_ok += 1;
            total_frames += 1;
        }
    }

    eprintln!(
        "[png_merger oracle] reconstructed + decoded {decoded_ok} frames across {} atlases, \
         referencing {} distinct .mpd files (floors: 1800 frames / 170 mpd)",
        mphs.len(),
        mpd_files_seen.len()
    );
    // Liveness / non-vacuity floors (R3): real work happened.
    assert_eq!(decoded_ok, total_frames, "some frames did not decode");
    assert!(
        total_frames >= 1800,
        "only {total_frames} frames reconstructed, below the corpus floor 1800"
    );
    assert!(
        mpd_files_seen.len() >= 170,
        "only {} distinct .mpd files referenced, below the corpus floor 170",
        mpd_files_seen.len()
    );
}

/// Negative control A (R3): the reconstruction is content-sensitive. In an atlas
/// with frames at distinct offsets, two frames must reconstruct to *different*
/// bytes — an agreement that survived identical output would be vacuous.
#[test]
fn png_merger_negative_control_distinct_frames_differ() {
    let mphs = jar().matching(|n| n.ends_with(".mph"));
    let mut proven = false;
    for (name, mph_bytes) in &mphs {
        let stem = name.strip_suffix(".mph").unwrap();
        let m = mph::parse(mph_bytes).unwrap();
        // Find two frames with different offsets in the same mpd.
        if m.frames.len() < 2 {
            continue;
        }
        let s = load_atlas(stem, mph_bytes);
        for j in 1..m.frames.len() {
            if m.frames[j].offset != m.frames[0].offset
                && m.frames[j].mpd_index == m.frames[0].mpd_index
            {
                let a = png_merger::assemble_frame(&s, 0).unwrap();
                let b = png_merger::assemble_frame(&s, j as i32).unwrap();
                assert_ne!(
                    a, b,
                    "{name}: frames 0 and {j} (distinct offsets) reconstructed identically — \
                     the reconstruction is not content-sensitive"
                );
                proven = true;
                break;
            }
        }
        if proven {
            break;
        }
    }
    assert!(
        proven,
        "found no atlas with two distinct-offset frames to compare (vacuous)"
    );
}

/// Negative control B (R3): the reconstruction is offset-sensitive. Corrupting one
/// byte of a frame's `.mph` offset field must change what gets assembled — either
/// it no longer reads the true IHDR at the original offset, or it fails outright.
#[test]
fn png_merger_negative_control_corrupt_offset_desyncs() {
    let mphs = jar().matching(|n| n.ends_with(".mph"));
    // Pick the first atlas whose first frame is a merge-or-simple frame we can
    // reconstruct cleanly, then corrupt its offset.
    let (name, mph_bytes) = mphs.first().expect("at least one .mph");
    let stem = name.strip_suffix(".mph").unwrap();
    let m = mph::parse(mph_bytes).unwrap();
    assert!(!m.frames.is_empty(), "{name}: no frames");

    // Baseline: the true reconstruction reads the true IHDR at frame 0's offset.
    let s0 = load_atlas(stem, mph_bytes);
    let base = to_u8(&png_merger::assemble_frame(&s0, 0).unwrap());

    // Corrupt the low byte of frame 0's offset field (mph record at byte 8; the
    // u32 offset is at record bytes 2..6, so byte 8+5 is its low byte).
    let mut corrupt = mph_bytes.clone();
    let low_byte = 8 + 5;
    corrupt[low_byte] = corrupt[low_byte].wrapping_add(1);

    let s1 = load_atlas(stem, &corrupt);
    // Assembling from a corrupted offset may panic (out-of-range chunk walk) — that
    // is itself a desync. If it returns, the bytes must differ from the baseline.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        png_merger::assemble_frame(&s1, 0)
    }));
    match outcome {
        Err(_) => { /* panicked: a genuine desync */ }
        Ok(None) => { /* null: a genuine desync */ }
        Ok(Some(bytes)) => {
            let changed = to_u8(&bytes);
            assert_ne!(
                changed, base,
                "{name}: corrupting frame 0's offset produced an identical reconstruction — \
                 the assembly is not offset-sensitive"
            );
        }
    }
}
