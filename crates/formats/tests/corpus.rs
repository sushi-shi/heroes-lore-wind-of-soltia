//! Phase-1 gate, part 1: parse EVERY applicable blob in the baseline JAR for
//! each format, assert success, and assert the parsed count meets a per-kind
//! floor so a vacuous "0 blobs parsed, all passed" cannot happen.
//!
//! Floors are set to the full applicable set for each format (extraction is
//! deterministic), so partial coverage or a vacuous "0 blobs parsed" is caught:
//!   * `sprite`: the `*/spr/*` sprite-assembly set is 57; the `c*/s/*` character
//!     sprites (a separate, suffix-routed container) are covered by
//!     `corpus_char_sprite` (20). Together they are the full sprite domain (77).
//!   * `eif`: the bare-payload family is 39 — 18 `.eif` + 7 `boss/atef` +
//!     8 `enm/atef` + 3 `enm/die` + 3 `c3/s/ea`.
//!   * `item`: 25 = 24 numeric `itm/NN` + `itm/forshop` (same record framing);
//!     `itm/mixtbl` is a distinct crafting table, covered by `corpus_mixtbl` (1).
//!
//! `lang` had no recon floor; set to 1 (the single blob present).

mod common;

use common::{basename, fixtures, is_extensionless};
use heroes_lore_wind_of_soltia_formats::*;

/// Parse `blobs` with `f`, panicking with the blob name on the first failure;
/// assert the count meets `floor`; print the count for the run log.
fn run<T>(
    kind: &str,
    blobs: Vec<(String, Vec<u8>)>,
    floor: usize,
    f: impl Fn(&[u8]) -> Result<T, FormatError>,
) {
    let n = blobs.len();
    for (name, bytes) in &blobs {
        if let Err(e) = f(bytes) {
            panic!("{kind}: parsing {name} ({} bytes) failed: {e}", bytes.len());
        }
    }
    eprintln!("[corpus] {kind:8}: parsed {n} blobs (floor {floor})");
    assert!(
        n >= floor,
        "{kind}: only {n} blobs matched/parsed, below floor {floor} — extraction or matching is broken"
    );
}

#[test]
fn corpus_mpd() {
    let fx = fixtures();
    run("mpd", fx.matching(|n| n.ends_with(".mpd")), 170, mpd::parse);
}

#[test]
fn corpus_map() {
    let fx = fixtures();
    run("map", fx.matching(|n| n.ends_with(".map")), 80, map::parse);
}

#[test]
fn corpus_mph() {
    let fx = fixtures();
    run("mph", fx.matching(|n| n.ends_with(".mph")), 170, mph::parse);
}

#[test]
fn corpus_tdf() {
    let fx = fixtures();
    run("tdf", fx.matching(|n| n.ends_with(".tdf")), 18, tdf::parse);
}

/// True if `name` belongs to the bare-payload family (`.eif`, `*/atef/*`,
/// `enm/die/{0,1,2}`, `c3/s/ea*`).
fn is_bare_payload(n: &str) -> bool {
    n.ends_with(".eif")
        || (n.contains("/atef/") && is_extensionless(n))
        || (n.starts_with("enm/die/") && is_extensionless(n))
        || n.starts_with("c3/s/ea")
}

#[test]
fn corpus_eif() {
    let fx = fixtures();
    // The whole bare-payload family shares the `.eif` group-list grammar.
    run("eif", fx.matching(is_bare_payload), 39, eif::parse);
}

#[test]
fn corpus_lang() {
    let fx = fixtures();
    run(
        "lang",
        fx.matching(|n| n.starts_with("lang/language.")),
        1,
        lang::parse,
    );
}

#[test]
fn corpus_sprite() {
    let fx = fixtures();
    // `*/spr/*` extensionless sprite-assembly scripts (full applicable set: 57).
    run(
        "sprite",
        fx.matching(|n| n.contains("/spr/") && is_extensionless(n)),
        57,
        sprite::parse,
    );
}

/// True if `name` is a `c*/s/*` character-sprite blob (any of the 3 variants).
fn is_char_sprite(n: &str) -> bool {
    (n.starts_with("c1/s/") || n.starts_with("c2/s/") || n.starts_with("c3/s/"))
        && !n.ends_with('/')
        && n.matches('/').count() == 2
}

#[test]
fn corpus_char_sprite() {
    let fx = fixtures();
    // All 20 `c*/s/*` blobs, each parsed under the grammar its suffix routes to.
    let blobs = fx.matching(is_char_sprite);
    let n = blobs.len();
    let mut by_variant = [0usize; 3]; // [Flat, Nested, Bare]
    for (name, bytes) in &blobs {
        let v = csprite::variant_for_name(name);
        by_variant[match v {
            csprite::Variant::Flat => 0,
            csprite::Variant::Nested => 1,
            csprite::Variant::Bare => 2,
        }] += 1;
        csprite::parse(bytes, v).unwrap_or_else(|e| {
            panic!(
                "csprite: parsing {name} ({} bytes) as {v:?} failed: {e}",
                bytes.len()
            )
        });
    }
    eprintln!(
        "[corpus] c*/s/*  : parsed {n} blobs (FLAT {}, NESTED {}, BARE {}) (floor 20)",
        by_variant[0], by_variant[1], by_variant[2]
    );
    assert!(
        n >= 20,
        "c*/s/*: only {n} blobs matched/parsed, below floor 20"
    );
    assert!(
        by_variant[0] >= 14 && by_variant[1] >= 3 && by_variant[2] >= 3,
        "c*/s/*: variant split {by_variant:?} below expected (14 FLAT, 3 NESTED, 3 BARE)"
    );
}

#[test]
fn corpus_item() {
    let fx = fixtures();
    // itm/NN (00..23) plus itm/forshop — same `[u8 rec_len]` record framing.
    // Excludes itm/mixtbl (distinct grammar) and *.tdf.
    run(
        "item",
        fx.matching(|n| {
            if let Some(rest) = n.strip_prefix("itm/") {
                rest == "forshop" || (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))
            } else {
                false
            }
        }),
        25,
        item::parse,
    );
}

#[test]
fn corpus_mixtbl() {
    let fx = fixtures();
    run(
        "mixtbl",
        fx.matching(|n| n == "itm/mixtbl"),
        1,
        item::parse_mixtbl,
    );
}

#[test]
fn corpus_media_png() {
    let fx = fixtures();
    run(
        "png",
        fx.matching(|n| n.ends_with(".png")),
        12,
        media::parse_png,
    );
}

#[test]
fn corpus_media_mid() {
    let fx = fixtures();
    run(
        "mid",
        fx.matching(|n| n.ends_with(".mid")),
        20,
        media::parse_mid,
    );
}

#[test]
fn corpus_media_wav() {
    let fx = fixtures();
    run(
        "wav",
        fx.matching(|n| n.ends_with(".wav")),
        9,
        media::parse_wav,
    );
}

/// The `.evt` container's first section (the collision grid) is `w*h` bytes,
/// where `w`,`h` come from the paired `/m/NN.map` — so unlike the other formats,
/// `evt::parse` needs those dimensions. Each `m/{6,7,8}/NN.evt` pairs with
/// `m/NN.map`. Every blob is decoded through the real VM decoder and must consume
/// exactly to EOF; we also tally the decoded scripts/instructions as a stronger
/// signal than a bare success count.
#[test]
fn corpus_evt() {
    let fx = fixtures();
    let evts = fx.matching(|n| n.ends_with(".evt"));
    let floor = 200usize;
    let n = evts.len();
    let mut scripts = 0usize;
    let mut instrs = 0usize;
    for (name, bytes) in &evts {
        // "m/6/00.evt" -> paired map "m/00.map".
        let stem = basename(name)
            .strip_suffix(".evt")
            .unwrap_or_else(|| panic!("evt: {name} has no .evt suffix"));
        let map_name = format!("m/{stem}.map");
        let map_bytes = fx
            .get(&map_name)
            .unwrap_or_else(|| panic!("evt: {name} has no paired map {map_name}"));
        let m = map::parse(&map_bytes)
            .unwrap_or_else(|e| panic!("evt: paired map {map_name} failed to parse: {e}"));
        let evt = evt::parse(bytes, m.w, m.h)
            .unwrap_or_else(|e| panic!("evt: decoding {name} ({} bytes) failed: {e}", bytes.len()));
        scripts += evt.scripts.len();
        instrs += evt
            .scripts
            .iter()
            .map(|s| s.instructions.len())
            .sum::<usize>();
    }
    eprintln!(
        "[corpus] evt     : parsed {n} blobs (floor {floor}); decoded {scripts} scripts, {instrs} instructions"
    );
    assert!(
        n >= floor,
        "evt: only {n} blobs matched/parsed, below floor {floor} — extraction or matching is broken"
    );
}

/// Cross-format gate: every `.mph` record `offset` must land on a real `IHDR`
/// chunk inside the paired same-stem `<stem>_<mpd_index>.mpd`. Validated across
/// the whole corpus (1,831 frames) — a strong check that the record layout
/// (`[u16 mpd_index][u32 offset][u16 chunk_bitmask]`, big-endian) is correct.
#[test]
fn mph_offsets_point_at_mpd_ihdr() {
    let fx = fixtures();
    let mphs = fx.matching(|n| n.ends_with(".mph"));
    let mut frames_checked = 0usize;
    for (name, bytes) in &mphs {
        let m = mph::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let stem = name
            .strip_suffix(".mph")
            .unwrap_or_else(|| panic!("{name}: not a .mph name"));
        // Cache IHDR offsets per paired mpd file (usually only `_0.mpd`).
        for (i, rec) in m.frames.iter().enumerate() {
            let mpd_name = format!("{stem}_{}.mpd", rec.mpd_index);
            let mpd_bytes = fx.get(&mpd_name).unwrap_or_else(|| {
                panic!("{name} record {i}: paired atlas {mpd_name} is missing from the JAR")
            });
            let ihdrs = mpd::ihdr_offsets(&mpd_bytes)
                .unwrap_or_else(|e| panic!("{mpd_name}: enumerating IHDR offsets failed: {e}"));
            assert!(
                ihdrs.contains(&(rec.offset as usize)),
                "{name} record {i}: offset {} is not an IHDR chunk start in {mpd_name} \
                 (IHDR offsets: {ihdrs:?})",
                rec.offset
            );
            frames_checked += 1;
        }
    }
    eprintln!(
        "[corpus] mph->mpd: cross-checked {frames_checked} frame offsets across {} .mph files (floor 1800)",
        mphs.len()
    );
    assert!(
        frames_checked >= 1800,
        "only {frames_checked} frame offsets cross-checked, below floor 1800 — matching is broken"
    );
}

/// Sanity: the extensionless-sprite matcher must not swallow `.eif`/`.mpd`/`.mph`
/// blobs that also live under a `spr/` directory (e.g. `grd/spr/*.eif`).
#[test]
fn sprite_matcher_excludes_extensioned_spr_files() {
    let fx = fixtures();
    let sprite_named = fx.matching(|n| n.contains("/spr/") && is_extensionless(n));
    for (name, _) in &sprite_named {
        assert!(
            !basename(name).contains('.'),
            "sprite matcher picked up an extensioned file: {name}"
        );
    }
}
