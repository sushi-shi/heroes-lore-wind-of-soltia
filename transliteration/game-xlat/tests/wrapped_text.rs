//! Behavioural gate for the wrapped-text family (`WrapFont.wrap`/`wrapInto`,
//! `BitmapFont.blockHeight`/`drawString`/`drawLines`, and `FontManager`'s
//! `wrapLines` + `lineCount` / `measureBlockHeight` / `drawWrappedText`).
//!
//! Unlike the pixel-exact oracles, there is no captured reference for an arbitrary
//! wrapped block, so this gate asserts the *algorithm's* invariants over the real
//! shipped `.mf` fonts (loaded from the baseline JAR — never a synthetic font):
//!
//!   * `wrapInto` breaks at word boundaries, loses no characters, and every line
//!     fits the pixel budget (greedy word-wrap);
//!   * `lineCount` and `measureBlockHeight` agree with that wrap
//!     (`height == (lineHeight + lineSpacing) * lines - lineSpacing`);
//!   * `drawWrappedText` actually blits ink (liveness — a real, decoded glyph sheet
//!     drawn onto a framebuffer), and its reported height matches the block.
//!
//! A negative control proves the multi-line assertion bites. Loud failure when
//! `_originals/` is absent (the font sheets come from the real JAR).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{bitmap_font, font_manager, wrap_font, Game};
use j2me_me::{Graphics, Image};

/// The canvas the wrapped block is measured/drawn against.
const CANVAS_W: i32 = 240;
const CANVAS_H: i32 = 320;

/// A multi-word, single-space, newline-free, already-uppercase sample. Greedy
/// wrapping of this at a width between one word and the whole string must break
/// only at spaces (no word here is itself too wide once the budget clears the
/// widest word), so the wrapped lines rejoined with a single space recover it.
const SAMPLE: &str = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG AND RAN AWAY";

/// Build a `Game`, load every baseline-JAR resource, and run `initFonts` so the six
/// real `.mf` fonts (and `currentFont = smallBlack`) are ready.
fn game_with_fonts() -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    font_manager::init_fonts(&mut g);
    g
}

/// `String::from(str)` as a `char[]` (`Vec<u16>`), the port's string type.
fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// The wrap's per-row advance of a word: the sum of `charWidth` over its chars
/// (no kerning subtracted — exactly what `wrap`/`wrapInto` accumulate). Choosing a
/// budget above the widest word's raw advance guarantees no mid-word break.
fn raw_advance(font: &bitmap_font::BitmapFontState, word: &[u16]) -> i32 {
    word.iter().map(|&c| bitmap_font::char_width(font, c)).sum()
}

/// Split on ASCII spaces (0x20) — the sample has no runs of spaces.
fn split_words(text: &[u16]) -> Vec<Vec<u16>> {
    text.split(|&c| c == b' ' as u16)
        .map(|w| w.to_vec())
        .collect()
}

/// Rejoin lines with a single space.
fn join_space(lines: &[Vec<u16>]) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push(b' ' as u16);
        }
        out.extend_from_slice(line);
    }
    out
}

/// A budget strictly above the widest word's raw advance but well below the whole
/// string, forcing several lines with no mid-word break.
fn wrap_budget(font: &bitmap_font::BitmapFontState, text: &[u16]) -> i32 {
    let words = split_words(text);
    let widest = words.iter().map(|w| raw_advance(font, w)).max().unwrap();
    widest + 8
}

#[test]
fn wrapinto_breaks_at_word_boundaries_and_fits_the_budget() {
    let g = game_with_fonts();
    let font = g
        .font_manager
        .small_black
        .as_ref()
        .expect("initFonts built smallBlack");
    // Liveness: the real glyph sheet decoded (this is not a metrics-only stub).
    assert!(
        font.glyph_sheet.is_some(),
        "smallBlack must own a decoded glyph sheet"
    );

    let text = u16s(SAMPLE);
    let width = wrap_budget(font, &text);

    let mut lines: Vec<Vec<u16>> = Vec::new();
    wrap_font::wrap_into(font, &mut lines, &text, width);

    // Liveness: wrapping actually happened (the budget is below the full string).
    let full = bitmap_font::string_width_range(font, &text, 0, text.len() as i32);
    assert!(full > width, "the sample must exceed the wrap budget");
    assert!(
        lines.len() >= 2,
        "a string wider than the budget must wrap onto >= 2 lines, got {}",
        lines.len()
    );

    // No character is lost and no word is split: rejoining with one space recovers
    // the source (greedy word-wrap consumes exactly the break space per line).
    assert_eq!(
        join_space(&lines),
        text,
        "wrapped lines rejoined with a space must equal the source"
    );

    // Every line fits the pixel budget, and none is empty.
    for (i, line) in lines.iter().enumerate() {
        assert!(!line.is_empty(), "line {i} is empty");
        let w = bitmap_font::string_width_range(font, line, 0, line.len() as i32);
        assert!(
            w <= width,
            "line {i} width {w} exceeds budget {width}: {:?}",
            String::from_utf16_lossy(line)
        );
    }
}

#[test]
fn wrap_char_count_matches_the_first_line() {
    // `WrapFont.wrap` (via `FontManager.charsInLine`) returns how many chars fit on
    // one line; that prefix must be the first `wrapInto` line (they share the greedy
    // core). `charsInLine` measures from `start` in `currentFont` (== smallBlack).
    let g = game_with_fonts();
    let font = g.font_manager.small_black.as_ref().unwrap();
    let text = u16s(SAMPLE);
    let width = wrap_budget(font, &text);

    let mut lines: Vec<Vec<u16>> = Vec::new();
    wrap_font::wrap_into(font, &mut lines, &text, width);
    let first_line_len = lines[0].len() as i32;

    // charsInLine(text, start=0, width, maxLines=1) — one line's worth of chars.
    let count = font_manager::chars_in_line(&g.font_manager, &text, 0, width, 1);
    assert!(count > 0, "wrap must consume at least one char");
    // The greedy wrap breaks after the space, so the char count reaches into (or
    // just past) the break; the first emitted line is a prefix of that many chars.
    assert!(
        count >= first_line_len,
        "wrap count {count} < first line length {first_line_len}"
    );
    assert!(
        count <= text.len() as i32,
        "wrap count {count} exceeds the text length"
    );
}

#[test]
fn linecount_and_measure_block_height_agree_with_the_wrap() {
    let mut g = game_with_fonts();
    let (line_height, line_spacing) = {
        let f = g.font_manager.small_black.as_ref().unwrap();
        (f.line_height, f.line_spacing)
    };
    let text = u16s(SAMPLE);
    let width = wrap_budget(g.font_manager.small_black.as_ref().unwrap(), &text);

    let n = font_manager::line_count(&mut g.font_manager, &text, width);
    assert!(n >= 2, "lineCount must see the wrap ({n} lines)");

    // measureBlockHeight == blockHeight(lines) - lineSpacing
    //                    == (lineHeight + lineSpacing) * n - lineSpacing
    let measured =
        font_manager::measure_block_height(&mut g.font_manager, width, 0, &text, 0, 0, 0);
    let expected = (line_height + line_spacing) * n - line_spacing;
    assert_eq!(
        measured, expected,
        "measureBlockHeight must equal (lineHeight+lineSpacing)*lines - lineSpacing"
    );
    assert!(measured > 0, "a non-empty block has positive height");
}

#[test]
fn draw_wrapped_text_blits_ink_and_reports_the_block_height() {
    let mut g = game_with_fonts();
    // Model the shown canvas the wrapped-text draws clip against.
    g.base_canvas.width = CANVAS_W;
    g.base_canvas.height = CANVAS_H;

    let text = u16s(SAMPLE);
    let width = wrap_budget(g.font_manager.small_black.as_ref().unwrap(), &text);
    let expected_lines = font_manager::line_count(&mut g.font_manager, &text, width);

    // A fresh mutable framebuffer starts all-white (0xFFFFFFFF); smallBlack draws
    // black ink, so any changed pixel is proof the block was rendered.
    let mut fb = Image::create_mutable(CANVAS_W, CANVAS_H).expect("framebuffer");
    let blank = 0xffff_ffffu32;

    let height = {
        let mut graphics = Graphics::new(&mut fb);
        // Black pen (0) selects smallBlack via fontForColor; top-left anchor.
        graphics.set_color(0);
        font_manager::draw_wrapped_text(
            &mut g.font_manager,
            &mut graphics,
            &g.base_canvas,
            8,
            8,
            width,
            0,
            &text,
        )
    };

    // The reported height is the whole stacked block (one step per wrapped line).
    let (line_height, line_spacing) = {
        let f = g.font_manager.small_black.as_ref().unwrap();
        (f.line_height, f.line_spacing)
    };
    assert_eq!(
        height,
        (line_height + line_spacing) * expected_lines,
        "drawLines returns lineY - y == step * lineCount"
    );

    // Liveness: real ink hit the framebuffer.
    let inked = fb.pixels().iter().filter(|&&p| p != blank).count();
    assert!(
        inked > 0,
        "drawWrappedText must blit at least one non-background pixel"
    );
}

/// Negative control (GATES.md R3): the multi-line assertion must bite. The sample
/// wraps onto several lines at the chosen budget; asserting it is a single line
/// must fail, proving the wrap gate is not vacuous.
#[test]
#[should_panic(expected = "negative control")]
fn negative_control_single_line_rejected() {
    let g = game_with_fonts();
    let font = g.font_manager.small_black.as_ref().unwrap();
    let text = u16s(SAMPLE);
    let width = wrap_budget(font, &text);
    let mut lines: Vec<Vec<u16>> = Vec::new();
    wrap_font::wrap_into(font, &mut lines, &text, width);
    assert_eq!(lines.len(), 1, "negative control");
}
