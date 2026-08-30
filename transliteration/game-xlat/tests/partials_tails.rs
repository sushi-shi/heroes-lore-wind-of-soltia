//! Behavioural gate for the three PARTIAL modules' newly-completed deferred tails:
//!
//!   * `BaseCanvas` loading-screen number/label kit — `drawLabelBox` and `drawNumber`
//!     must actually blit ink onto a framebuffer (liveness), and `drawLabelBox`'s
//!     returned right edge / `numberWidth`'s pixel width match their arithmetic;
//!   * `GameLoop` options persistence — `packOptions` ↔ `unpackOptions` round-trip a
//!     full settings blob (state), with a one-byte perturbation proven to decode
//!     differently (teeth);
//!   * `StringTable.resolveLocale` — an exact / case-insensitive / two-letter-prefix
//!     match resolves to the expected `locales` index (with the `0x8000` prefix flag).
//!
//! `drawLabelBox` needs the real shipped `.mf` fonts (from the baseline JAR, like
//! `wrapped_text.rs`), so this gate fails loudly when `_originals/` is absent. The
//! `drawNumber` liveness fixture is a synthetic opaque glyph strip: the module under
//! test is `drawNumber`'s clip/anchor/blit control flow (the real `numberFont0` bank
//! is unported and threaded in as a snapshot param), so an opaque sheet is the
//! faithful stand-in — ink appears only if the blit path actually runs.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    base_canvas, font_manager, game_loop, string_table, Game,
};
use j2me_me::{Graphics, Image};

/// The framebuffer the draws are measured against (the device resolution).
const CANVAS_W: i32 = 240;
const CANVAS_H: i32 = 320;

/// Build a `Game`, load every baseline-JAR resource, and run `initFonts` so the six
/// real `.mf` fonts (and `currentFont = smallBlack`) are ready — the setup the
/// `FontManager`-backed `drawLabelBox` needs.
fn game_with_fonts() -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    font_manager::init_fonts(&mut g);
    g
}

/// `&str` as a `char[]` (`Vec<u16>`), the port's string type.
fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Count pixels of exactly `argb` over the whole image.
fn count_color(img: &Image, argb: u32) -> usize {
    let mut n = 0;
    for y in 0..img.height() {
        for x in 0..img.width() {
            if img.get(x, y) == Some(argb) {
                n += 1;
            }
        }
    }
    n
}

/// Count pixels that are NOT the opaque-white default ground.
fn count_non_white(img: &Image) -> usize {
    let mut n = 0;
    for y in 0..img.height() {
        for x in 0..img.width() {
            if img.get(x, y) != Some(0xFFFF_FFFF) {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn draw_label_box_blits_ink_and_returns_the_right_edge() {
    let g = game_with_fonts();
    let mut fb = Image::create_mutable(CANVAS_W, CANVAS_H).unwrap();
    let text = u16s("HP 999");

    // The black box + white glyphs land somewhere well inside the canvas.
    let right = {
        let mut gr = Graphics::new(&mut fb);
        base_canvas::draw_label_box(&g.font_manager, &mut gr, &text, 40, 40)
    };

    // Liveness: the box fill (black) + text put ink on the white ground.
    let ink = count_non_white(&fb);
    assert!(ink > 0, "drawLabelBox must blit ink onto the framebuffer");
    // The black box specifically is present (setColor(0) + fillRect).
    let black = count_color(&fb, 0xFF00_0000);
    assert!(black > 0, "drawLabelBox must fill its black box");

    // State: the returned right edge is x + (stringWidth + 2).
    let box_width = font_manager::string_width(&g.font_manager, &text) + 2;
    assert_eq!(right, 40 + box_width, "right edge == x + boxWidth");
    assert!(
        box_width > 2,
        "a non-empty label measures wider than its padding"
    );

    // The `String` overload delegates to the `char[]` one — same right edge, same box.
    let mut fb2 = Image::create_mutable(CANVAS_W, CANVAS_H).unwrap();
    let right_str = {
        let mut gr = Graphics::new(&mut fb2);
        base_canvas::draw_label_box_string(&g.font_manager, &mut gr, "HP 999", 40, 40)
    };
    assert_eq!(
        right_str, right,
        "String overload matches the char[] overload"
    );
    assert!(
        count_color(&fb2, 0xFF00_0000) > 0,
        "String overload blits the box too"
    );
}

#[test]
fn draw_number_blits_a_glyph_per_digit() {
    // A synthetic opaque digit strip: 10 cells of 5px (style-0 glyphWidth) at 7px
    // tall (glyphHeight). A distinctive colour lets us count exactly what landed.
    const SHEET: u32 = 0xFF11_88CC;
    let sheet = Image::from_argb(50, 7, vec![SHEET; 50 * 7]).unwrap();

    let mut fb = Image::create_mutable(CANVAS_W, CANVAS_H).unwrap();
    {
        let mut gr = Graphics::new(&mut fb);
        // drawNumberAt(graphics, 123, 100, 100, 0) — style 0, anchor 0 (left-aligned).
        base_canvas::draw_number_at(&mut gr, 123, 100, 100, 0, &sheet);
    }

    // Liveness: the three digit glyphs blit the sheet colour into their clip windows.
    let ink = count_color(&fb, SHEET);
    assert!(ink > 0, "drawNumber must blit glyph ink");
    // Each digit's 5x7 window contributes; the run is at least one full glyph tall/wide.
    assert!(ink >= 5 * 7, "at least one full glyph cell was drawn");
    // The ink stays within the digit run's band (three overlapping 5px cells at 4px
    // advance ⇒ span ≤ 5 + 2*4 = 13 px wide, 7 px tall).
    assert!(
        ink <= 13 * 7,
        "glyph ink stays inside the number's bounding band"
    );

    // numberWidth is pure arithmetic: 1 + 4 per digit.
    assert_eq!(base_canvas::number_width(123), 1 + 4 * 3);
    assert_eq!(base_canvas::number_width(0), 1 + 4); // one digit
    assert_eq!(base_canvas::number_width(7), 1 + 4);
}

#[test]
fn pack_options_round_trips_through_unpack_options() {
    let mut g = Game::new();
    // A known, non-default settings state.
    g.game_loop.volume = 12; // 0..15
    g.game_loop.has_created_character = true;
    g.game_loop.auto_text_advance = true;
    g.game_loop.camera_follow = false;
    g.game_loop.difficulty = 3;
    g.game_loop.progress_flags = 5; // low nibble survives the pack
    g.game_loop.progress_data = 0x1234_5678;

    let packed = game_loop::pack_options(&g);
    assert_eq!(packed.len(), 6, "the options blob is exactly 6 bytes");

    // Clobber every field to a different value, then restore from the blob.
    g.game_loop.volume = 1;
    g.game_loop.has_created_character = false;
    g.game_loop.auto_text_advance = false;
    g.game_loop.camera_follow = true;
    g.game_loop.difficulty = 0;
    g.game_loop.progress_flags = 0;
    g.game_loop.progress_data = -1;

    game_loop::unpack_options(&mut g, &packed);

    assert_eq!(g.game_loop.volume, 12, "volume restored");
    assert!(
        g.game_loop.has_created_character,
        "hasCreatedCharacter restored"
    );
    assert!(g.game_loop.auto_text_advance, "autoTextAdvance restored");
    assert!(!g.game_loop.camera_follow, "cameraFollow restored");
    assert_eq!(g.game_loop.difficulty, 3, "difficulty restored");
    assert_eq!(g.game_loop.progress_flags, 5, "progressFlags restored");
    assert_eq!(
        g.game_loop.progress_data, 0x1234_5678,
        "progressData restored"
    );
    // setDifficulty side effect: frameDelay tracks the difficulty table.
    assert_eq!(
        g.game_loop.frame_delay, g.game_loop.frame_delay_table[3],
        "unpack re-applies setDifficulty(frameDelayTable[difficulty])"
    );

    // Teeth: a one-byte perturbation of the blob must not decode to the same volume.
    let mut g2 = Game::new();
    let mut mutated = packed.clone();
    mutated[0] = (mutated[0] as i32 ^ 0x10) as i8; // flip a volume-nibble bit
    game_loop::unpack_options(&mut g2, &mutated);
    assert_ne!(
        g2.game_loop.volume, 12,
        "a perturbed blob must decode to a different volume — the round-trip has teeth"
    );
}

#[test]
fn resolve_locale_picks_the_expected_index() {
    let g = Game::new();
    let st = &g.string_table;

    // Exact matches → the plain index (locales = en-GB, fr-FR, de-DE, it-IT, es-ES).
    assert_eq!(string_table::resolve_locale(st, Some("en-GB")), 0);
    assert_eq!(string_table::resolve_locale(st, Some("fr-FR")), 1);
    assert_eq!(string_table::resolve_locale(st, Some("de-DE")), 2);
    assert_eq!(string_table::resolve_locale(st, Some("es-ES")), 4);
    // Case-insensitive exact match.
    assert_eq!(string_table::resolve_locale(st, Some("IT-it")), 3);

    // Two-letter (language-code) prefix match → index OR'd with 0x8000 (32768).
    assert_eq!(string_table::resolve_locale(st, Some("de-CH")), 2 | 32768);
    assert_eq!(string_table::resolve_locale(st, Some("EN-us")), 32768); // 0 | 0x8000

    // No match at all → -1; a null locale (no device property, headless) → -1.
    assert_eq!(string_table::resolve_locale(st, Some("ja-JP")), -1);
    assert_eq!(string_table::resolve_locale(st, None), -1);
}
