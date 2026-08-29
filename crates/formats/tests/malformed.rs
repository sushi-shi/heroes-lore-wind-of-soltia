//! Phase-1 gate, part 2: for every format, feed empty / truncated / garbage
//! bytes and assert the parser returns `Err` (never panics, never hangs).
//!
//! Each parser is also exercised against a *valid* minimal input where practical,
//! so these tests prove the `Err`s come from real validation rather than a
//! parser that rejects everything.

use heroes_lore_wind_of_soltia_formats::*;

/// PNG chunk helper: `[u32 len BE][type][data][u32 crc]`.
fn chunk(ty: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(data.len() as u32).to_be_bytes());
    v.extend_from_slice(ty);
    v.extend_from_slice(data);
    v.extend_from_slice(&[0, 0, 0, 0]); // crc (not validated in Phase 1)
    v
}

fn ihdr(w: u32, h: u32, depth: u8, color: u8) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&w.to_be_bytes());
    d.extend_from_slice(&h.to_be_bytes());
    d.extend_from_slice(&[depth, color, 0, 0, 0]);
    chunk(b"IHDR", &d)
}

// ----------------------------------------------------------------------- mpd

#[test]
fn mpd_rejects_malformed() {
    assert!(mpd::parse(&[]).is_err(), "empty");
    // Garbage: does not start at an IHDR signature.
    assert!(mpd::parse(&[1, 2, 3, 4, 5, 6, 7, 8]).is_err(), "garbage");
    // Truncated: valid IHDR start but the chunk data is cut off.
    let mut good = ihdr(8, 6, 4, 3);
    let cut = good.len() - 5;
    good.truncate(cut);
    assert!(mpd::parse(&good).is_err(), "truncated chunk");
    // A well-formed single sub-image (IHDR only) must parse.
    let ok = ihdr(56, 25, 4, 3);
    let m = mpd::parse(&ok).expect("valid mpd");
    assert_eq!(
        (m.width, m.height, m.bit_depth, m.color_type),
        (56, 25, 4, 3)
    );
}

// ----------------------------------------------------------------------- map

#[test]
fn map_rejects_malformed() {
    assert!(map::parse(&[]).is_err(), "empty");
    assert!(map::parse(&[1, 30]).is_err(), "short header");
    // Header claims 30x30 = 900 tiles but body is absent.
    assert!(map::parse(&[1, 30, 30]).is_err(), "length mismatch");
    // Valid 2x2 map.
    let m = map::parse(&[7, 2, 2, 10, 11, 12, 13]).expect("valid map");
    assert_eq!((m.ver, m.w, m.h, m.tiles.len()), (7, 2, 2, 4));
}

// ----------------------------------------------------------------------- mph

#[test]
fn mph_rejects_malformed() {
    assert!(mph::parse(&[]).is_err(), "empty");
    // Header declares a huge frame count that the body cannot satisfy.
    let mut hdr = Vec::new();
    hdr.extend_from_slice(&0u32.to_be_bytes());
    hdr.extend_from_slice(&1000u32.to_be_bytes());
    assert!(mph::parse(&hdr).is_err(), "count overruns body");
    // Records fit but the trailer is a malformed PNG chunk (overruns).
    let mut m = Vec::new();
    m.extend_from_slice(&0u32.to_be_bytes()); // flags
    m.extend_from_slice(&1u32.to_be_bytes()); // count = 1
    m.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0x98, 0]); // 1 frame record
    m.extend_from_slice(&[0, 0, 0, 100]); // trailer chunk len=100 (overruns)
    m.extend_from_slice(b"PLTE");
    assert!(mph::parse(&m).is_err(), "malformed trailer");
    // Valid: header + 1 record, no trailer. Record decodes big-endian as
    // [u16 mpd_index][u32 offset][u16 chunk_bitmask].
    let mut ok = Vec::new();
    ok.extend_from_slice(&4u32.to_be_bytes());
    ok.extend_from_slice(&1u32.to_be_bytes());
    ok.extend_from_slice(&[0, 0, 0, 0, 0x4e, 0, 0x98, 0]);
    let parsed = mph::parse(&ok).expect("valid mph");
    assert_eq!(parsed.count, 1);
    assert_eq!(parsed.frames.len(), 1);
    assert_eq!(parsed.frames[0].mpd_index, 0);
    assert_eq!(parsed.frames[0].offset, 0x0000_4e00);
    assert_eq!(parsed.frames[0].chunk_bitmask, 0x9800);
    assert_eq!(parsed.trailer_len, 0);
}

// ----------------------------------------------------------------------- tdf

#[test]
fn tdf_rejects_malformed() {
    assert!(tdf::parse(&[]).is_err(), "empty");
    // count=2 but only one record present.
    assert!(
        tdf::parse(&[2, 0, 3, b'6', b'8', b'5']).is_err(),
        "truncated"
    );
    // Bad prefix (high byte != 0).
    assert!(tdf::parse(&[1, 0xff, 1, b'5']).is_err(), "bad prefix");
    // Non-ascii id byte.
    assert!(tdf::parse(&[1, 0, 1, 0x00]).is_err(), "non-ascii");
    // Valid: count=2, "685","686".
    let ok = [2u8, 0, 3, b'6', b'8', b'5', 0, 3, b'6', b'8', b'6'];
    let t = tdf::parse(&ok).expect("valid tdf");
    assert_eq!(t.ids, vec!["685".to_string(), "686".to_string()]);
}

// ----------------------------------------------------------------------- eif

#[test]
fn eif_rejects_malformed() {
    // Bare-payload grammar: [u8 group_count] + group_count × ([u8 cell_count] + cells).
    assert!(eif::parse(&[]).is_err(), "empty");
    // group_count=9 but the groups do not follow -> truncated.
    assert!(eif::parse(&[9, 4, 5, 6]).is_err(), "truncated groups");
    // group_count=1, cell_count=200 but no cells -> truncated.
    assert!(eif::parse(&[1, 200, 1, 2, 3]).is_err(), "cells overrun");
    // Trailing byte after a complete group list -> inconsistent.
    assert!(
        eif::parse(&[1, 1, 0xf5, 0xf6, 0, 1, 0xff]).is_err(),
        "trailing byte"
    );
    // Valid: one group of two 4-byte cells (1 + 1 + 8 = 10 bytes).
    let e = eif::parse(&[1, 2, 0xf5, 0xf6, 0, 1, 0xfa, 0xeb, 1, 2]).expect("valid eif");
    assert_eq!(e.groups.len(), 1);
    assert_eq!(e.groups[0].cells.len(), 2);
    assert_eq!(e.cell_count(), 2);
    // Empty group list (group_count=0) consumes exactly 1 byte -> valid.
    assert!(eif::parse(&[0])
        .expect("empty group list")
        .groups
        .is_empty());
}

// ------------------------------------------------------------------- csprite

#[test]
fn csprite_rejects_malformed() {
    use csprite::Variant;
    // Suffix routing.
    assert_eq!(csprite::variant_for_name("c1/s/a"), Variant::Flat);
    assert_eq!(csprite::variant_for_name("c2/s/hB"), Variant::Flat);
    assert_eq!(csprite::variant_for_name("c3/s/s"), Variant::Flat);
    assert_eq!(csprite::variant_for_name("c1/s/e"), Variant::Nested);
    assert_eq!(csprite::variant_for_name("c3/s/ea2"), Variant::Bare);

    // Empty is rejected for every variant.
    for v in [Variant::Flat, Variant::Nested, Variant::Bare] {
        assert!(csprite::parse(&[], v).is_err(), "empty {v:?}");
    }
    // FLAT: header says n=4 cells but none follow -> truncated.
    assert!(
        csprite::parse(&[2, 3, 1, 4], Variant::Flat).is_err(),
        "flat truncated"
    );
    // FLAT valid: one record, header [2,3,1], n=1, one cell.
    let f = csprite::parse(&[2, 3, 1, 1, 0xf4, 0, 0, 4], Variant::Flat).expect("valid flat");
    match f {
        csprite::CharSprite::Flat(recs) => {
            assert_eq!(recs.len(), 1);
            assert_eq!(recs[0].header, [2, 3, 1]);
            assert_eq!(recs[0].cells.len(), 1);
        }
        _ => panic!("expected Flat"),
    }
    // NESTED: header n=1 group but the group's cell run overruns.
    assert!(
        csprite::parse(&[0, 0, 0, 1, 9, 1, 2], Variant::Nested).is_err(),
        "nested truncated"
    );
    // NESTED valid: header [0,0,0], n=1 group of 1 cell.
    let ns =
        csprite::parse(&[0, 0, 0, 1, 1, 0xf4, 0, 0, 4], Variant::Nested).expect("valid nested");
    match ns {
        csprite::CharSprite::Nested(recs) => {
            assert_eq!(recs.len(), 1);
            assert_eq!(recs[0].groups.len(), 1);
            assert_eq!(recs[0].groups[0].cells.len(), 1);
        }
        _ => panic!("expected Nested"),
    }
    // BARE reuses the eif grammar.
    assert!(
        csprite::parse(&[1, 9, 1], Variant::Bare).is_err(),
        "bare truncated"
    );
    let b = csprite::parse(&[1, 1, 0xf4, 0, 0, 4], Variant::Bare).expect("valid bare");
    assert!(matches!(b, csprite::CharSprite::Bare(_)));
}

// ---------------------------------------------------------------------- lang

#[test]
fn lang_rejects_malformed() {
    assert!(lang::parse(&[]).is_err(), "empty");
    // size word != len-4.
    assert!(lang::parse(&[0, 0, 0, 0, 0, 0, 0, 12]).is_err(), "bad size");
    // Build a tiny valid lang: 1 entry pointing at the string region.
    // layout: [size=len-4][off0=8]["hi"] -> len=10, off0=8 (table has 1 entry).
    let mut v = Vec::new();
    let body: &[u8] = &[0, 0, 0, 8, b'h', b'i']; // off0=8, then string "hi"
    v.extend_from_slice(&((body.len()) as u32).to_be_bytes());
    v.extend_from_slice(body);
    let l = lang::parse(&v).expect("valid lang");
    assert_eq!(l.len(), 1);
    assert_eq!(l.get(0).as_deref(), Some("hi"));
    // Non-ascending / out-of-range offsets: off0 points past EOF.
    let bad = {
        let body: &[u8] = &[0, 0, 0, 200];
        let mut b = Vec::new();
        b.extend_from_slice(&(body.len() as u32).to_be_bytes());
        b.extend_from_slice(body);
        b
    };
    assert!(lang::parse(&bad).is_err(), "offset out of range");
}

// -------------------------------------------------------------------- sprite

#[test]
fn sprite_rejects_malformed() {
    assert!(sprite::parse(&[]).is_err(), "empty");
    // Record len=16 but no payload follows.
    assert!(sprite::parse(&[0, 0, 16]).is_err(), "truncated payload");
    // Zero-length payload is invalid.
    assert!(sprite::parse(&[0, 0, 0]).is_err(), "zero-len record");
    // Outer framing fits, but the payload is not a valid group list
    // (group_count=5 with no groups) -> rejected by the inner decode.
    assert!(
        sprite::parse(&[0, 0, 3, 5, 1, 2]).is_err(),
        "bad inner payload"
    );
    // Valid: two records [grp,idx,len=6, payload=1 group of 1 cell].
    let ok = [
        0u8, 0, 6, 1, 1, 0xfa, 0xe9, 0, 1, // record 0
        0, 1, 6, 1, 1, 0xfa, 0xe9, 0, 0, // record 1
    ];
    let s = sprite::parse(&ok).expect("valid sprite");
    assert_eq!(s.records.len(), 2);
    assert_eq!(s.records[0].groups.len(), 1);
    assert_eq!(s.records[0].groups[0].cells.len(), 1);
}

// ---------------------------------------------------------------------- item

#[test]
fn item_rejects_malformed() {
    // `[u8 rec_len][rec_len bytes]` records tiling to EOF.
    assert!(item::parse(&[]).is_err(), "empty");
    // rec_len=13 but only 1 content byte follows -> truncated.
    assert!(item::parse(&[0x0d, 0x07]).is_err(), "record overruns");
    // Two records that tile exactly: [len=3 | 07 03 33] [len=1 | ff].
    let ok = [0x03u8, 0x07, 0x03, 0x33, 0x01, 0xff];
    let it = item::parse(&ok).expect("valid item table");
    assert_eq!(it.records.len(), 2);
    assert_eq!(it.records[0], vec![0x07, 0x03, 0x33]);
    assert_eq!(it.records[1], vec![0xff]);
}

// -------------------------------------------------------------------- mixtbl

#[test]
fn mixtbl_rejects_malformed() {
    // entry := [u8 ingredient_count] + count×[type,subtype] + [result_type,result_sub].
    assert!(item::parse_mixtbl(&[]).is_err(), "empty");
    // ingredient_count=2 needs 2*2 + 2 = 6 bytes, only some present -> truncated.
    assert!(
        item::parse_mixtbl(&[2, 1, 2, 3, 4]).is_err(),
        "truncated entry"
    );
    // Valid: one recipe with 2 ingredients then a 2-byte result (1+4+2 = 7).
    let m = item::parse_mixtbl(&[2, 1, 2, 3, 4, 9, 0]).expect("valid mixtbl");
    assert_eq!(m.entries.len(), 1);
    assert_eq!(m.entries[0].ingredients, vec![[1, 2], [3, 4]]);
    assert_eq!(m.entries[0].result, [9, 0]);
}

// --------------------------------------------------------------------- media

#[test]
fn media_rejects_malformed() {
    // PNG
    assert!(media::parse_png(&[]).is_err(), "png empty");
    assert!(
        media::parse_png(b"not a png xxxx").is_err(),
        "png bad magic"
    );
    let png = {
        let mut v = mpd::PNG_MAGIC.to_vec();
        v.extend_from_slice(&ihdr(1, 1, 8, 6));
        v
    };
    assert!(media::parse_png(&png).is_ok(), "valid png");
    // MIDI
    assert!(media::parse_mid(&[]).is_err(), "mid empty");
    assert!(
        media::parse_mid(b"XXXX\0\0\0\x06......").is_err(),
        "mid bad magic"
    );
    let mut mid = b"MThd".to_vec();
    mid.extend_from_slice(&6u32.to_be_bytes());
    mid.extend_from_slice(&[0, 0, 0, 1, 0, 96]);
    assert!(media::parse_mid(&mid).is_ok(), "valid mid");
    // WAV
    assert!(media::parse_wav(&[]).is_err(), "wav empty");
    assert!(media::parse_wav(b"RIFF____NOPE").is_err(), "wav bad WAVE");
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&36u32.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    assert!(media::parse_wav(&wav).is_ok(), "valid wav");
}

// ----------------------------------------------------------------------- evt

/// A minimal but fully-valid `.evt` container with `w=h=0` (empty collision
/// grid): all sections empty except one script holding a single `END_EVNT` row.
/// Byte map: obj[0,0] npc[0,0] enm[0,0] face[0] trig[0] scripts[1] len[1]
/// row[99,0,0] dlg[0] patches[0,0].
fn minimal_evt() -> Vec<u8> {
    vec![
        0, 0, // objects: image count, object count
        0, 0, // npcs: id count, npc count
        0, 0, // enemies: type count, enemy count
        0, // faces: count
        0, // triggers: count
        1, // scripts: count
        1, // script #0: instruction count
        99, 0, 0, // instruction: END_EVNT
        0, // dialogue: count
        0, 0, // patches: condition count, group count
    ]
}

#[test]
fn evt_rejects_malformed() {
    use heroes_lore_wind_of_soltia_formats::evt::OpCode;

    // Empty input.
    assert!(evt::parse(&[], 0, 0).is_err(), "empty");

    // A well-formed minimal container decodes, consuming exactly to EOF.
    let base = minimal_evt();
    let e = evt::parse(&base, 0, 0).expect("valid minimal evt");
    assert_eq!(e.scripts.len(), 1);
    assert_eq!(e.scripts[0].instructions.len(), 1);
    assert_eq!(e.scripts[0].instructions[0].op, OpCode::EndEvnt);

    // Collision grid underruns: claims 4*4=16 bytes but only 3 are present.
    assert!(
        matches!(
            evt::parse(&[1, 2, 3], 4, 4),
            Err(FormatError::Truncated { .. })
        ),
        "collision underrun"
    );

    // Unknown opcode: byte 55 is `null` in the game's opcode table.
    let mut bad_op = base.clone();
    bad_op[10] = 55;
    assert!(
        matches!(evt::parse(&bad_op, 0, 0), Err(FormatError::BadField { .. })),
        "unknown opcode"
    );

    // Truncated mid-operand: the single instruction row is cut after 2 of its
    // 3 bytes (script still claims one instruction).
    let cut = &base[..12];
    assert!(
        matches!(evt::parse(cut, 0, 0), Err(FormatError::Truncated { .. })),
        "truncated mid-operand"
    );

    // Trailing garbage after the final section.
    let mut trailing = base.clone();
    trailing.push(0xFF);
    assert!(
        matches!(
            evt::parse(&trailing, 0, 0),
            Err(FormatError::Inconsistent { .. })
        ),
        "trailing garbage"
    );
}
