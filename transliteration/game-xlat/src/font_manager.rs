//! Transliterated from `java/src/main/java/defpackage/FontManager.java`
//! (original `bh.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The text/label/font hub. This increment ports the title-paint text path:
//! `initFonts` (builds the six fonts), `fontForColor` + `drawChars` +
//! `drawCharsCentered` + `stringWidth` (the version/footer draws), and `getString`
//! (the `StringTable` lookup that fills `titleFooter`).
//!
//! ## ANTI-BOG boundary
//!
//! `FontManager` owns ~30 label `static`s (soft-key labels, menu labels, prompts)
//! filled by `loadLabels(3902..3950)`, plus the wrapped-block draw/measure helpers
//! (`drawWrappedBlock`, `lineCount`, `advanceWord`, `loadLocaleImage`, …). Only the
//! statics the title paint reads are modelled here: the six fonts, `currentFont`,
//! `bigFontActive`, `versionText` and `titleFooter`. [`load_title_labels`] fills
//! the two the title needs (the rest of `loadLabels` is DEFERRED); the `versionText`
//! source (`AppConfig.apply` reading `MIDlet-Version`) is DEFERRED and injected
//! here from the reviewed manifest value.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bh.a:()V => []`
//! (initFonts), `bh.a:([C)I => []` (stringWidth), `bh.a:(…Graphics;II[CI)I => []`
//! (drawChars) and `bh.a:(…Graphics;II[CI)V => []` (drawCharsCentered) — all
//! arithmetic-free wrappers over the `az` font methods; `bh.a:(I)String` (getString)
//! is a `StringTable.get` + `String.replace` with no arithmetic opcodes.

use crate::bitmap_font::{self, BitmapFontState};
use crate::game::Game;
use crate::string_table;
use crate::wrap_font;

/// Which font `currentFont` points at (a reference field in Java). `Unset` models
/// the pre-`initFonts` null.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CurrentFont {
    #[default]
    Unset,
    SmallBlack,
    SmallWhite,
    SmallOrange,
    BigBlack,
    BigWhite,
}

/// Java `bh` / `FontManager` state — **partial** (anti-bog). Only the statics the
/// title paint reads are modelled (see the module header).
#[derive(Debug, Default)]
pub struct FontManagerState {
    /// `smallBlack` (fonts/small, ink 0).
    pub small_black: Option<BitmapFontState>,
    /// `smallWhite` (fonts/small, ink 0xFFFFFF).
    pub small_white: Option<BitmapFontState>,
    /// `smallOrange` (fonts/small, ink 0xFF8800 = 16746496).
    pub small_orange: Option<BitmapFontState>,
    /// `bigBlack` (fonts/big, black fill / white outline).
    pub big_black: Option<BitmapFontState>,
    /// `bigWhite` (fonts/big, white fill / black outline).
    pub big_white: Option<BitmapFontState>,
    // `bigDefault` aliases `bigBlack` (same object); `fontForColor` returns
    // `bigBlack` for the default case, so no separate field is needed.
    /// `currentFont` — the small/big family selected by `setBigFont`.
    pub current_font: CurrentFont,
    /// `bigFontActive`.
    pub big_font_active: bool,
    /// `versionText` ("2.0.7"), filled at runtime by `AppConfig` (deferred).
    pub version_text: Option<Vec<u16>>,
    /// `titleFooter` (lang 3950 = "PRESS ANY KEY"), filled by `loadLabels`.
    pub title_footer: Option<Vec<u16>>,
}

/// `public static final void initFonts()` (`bh.a:()V => []`): builds the six fonts,
/// disables their control glyphs, and selects small-black as `currentFont`.
pub fn init_fonts(g: &mut Game) {
    // smallBlack = WrapFont.create("fonts/small", 0, false);
    let f = wrap_font::create_single(g, "fonts/small", 0, false);
    g.font_manager.small_black = Some(f);
    // smallWhite = WrapFont.create("fonts/small", 16777215, false);
    let f = wrap_font::create_single(g, "fonts/small", 16777215, false);
    g.font_manager.small_white = Some(f);
    // smallOrange = WrapFont.create("fonts/small", 16746496, false);
    let f = wrap_font::create_single(g, "fonts/small", 16746496, false);
    g.font_manager.small_orange = Some(f);
    // bigBlack = WrapFont.create("fonts/big", 0, 16777215, true);
    let f = wrap_font::create(g, "fonts/big", 0, 16777215, true);
    g.font_manager.big_black = Some(f);
    // bigWhite = WrapFont.create("fonts/big", 16777215, 0, true);
    let f = wrap_font::create(g, "fonts/big", 16777215, 0, true);
    g.font_manager.big_white = Some(f);
    // bigDefault = bigBlack;   (alias — modelled implicitly)
    // ((BitmapFont) *).hideControls = true;  x6 (bigDefault aliases bigBlack)
    g.font_manager.small_black.as_mut().unwrap().hide_controls = true;
    g.font_manager.small_white.as_mut().unwrap().hide_controls = true;
    g.font_manager.small_orange.as_mut().unwrap().hide_controls = true;
    g.font_manager.big_black.as_mut().unwrap().hide_controls = true;
    g.font_manager.big_white.as_mut().unwrap().hide_controls = true;
    // currentFont = smallBlack;
    g.font_manager.current_font = CurrentFont::SmallBlack;
}

/// `private static BitmapFont fontForColor(int color)` — picks the font matching
/// the current pen colour and size. `bigDefault` aliases `bigBlack`.
fn font_for_color(s: &FontManagerState, color: i32) -> &BitmapFontState {
    if s.big_font_active {
        if color == 0 {
            return s.big_black.as_ref().expect("bigBlack null");
        }
        return if color == 16777215 {
            s.big_white.as_ref().expect("bigWhite null")
        } else {
            // bigDefault == bigBlack
            s.big_black.as_ref().expect("bigDefault null")
        };
    }
    if color == 0 {
        return s.small_black.as_ref().expect("smallBlack null");
    }
    if color == 16777215 {
        s.small_white.as_ref().expect("smallWhite null")
    } else {
        s.small_orange.as_ref().expect("smallOrange null")
    }
}

/// `currentFont` resolved to its `BitmapFontState`.
fn current_font(s: &FontManagerState) -> &BitmapFontState {
    match s.current_font {
        CurrentFont::SmallBlack => s.small_black.as_ref().expect("smallBlack null"),
        CurrentFont::SmallWhite => s.small_white.as_ref().expect("smallWhite null"),
        CurrentFont::SmallOrange => s.small_orange.as_ref().expect("smallOrange null"),
        CurrentFont::BigBlack => s.big_black.as_ref().expect("bigBlack null"),
        CurrentFont::BigWhite => s.big_white.as_ref().expect("bigWhite null"),
        CurrentFont::Unset => panic!("currentFont read before initFonts"),
    }
}

/// `public static final int drawChars(Graphics graphics, int x, int y, char[] chars, int flags)`
/// (`bh.a:(…Graphics;II[CI)I => []`): draw anchored top-left (anchor 20) in the
/// font matching the current pen colour.
pub fn draw_chars(
    s: &FontManagerState,
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    chars: &[u16],
    _flags: i32,
) -> i32 {
    // return fontForColor(graphics.getColor()).drawChars(graphics, chars, x, y, 20);
    let color = graphics.color();
    let font = font_for_color(s, color);
    bitmap_font::draw_chars(font, graphics, chars, x, y, 20)
}

/// `public static final void drawCharsCentered(Graphics graphics, int x, int y, char[] chars, int flags)`
/// (`bh.a:(…Graphics;II[CI)V => []`): horizontally centred (anchor 17).
pub fn draw_chars_centered(
    s: &FontManagerState,
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    chars: &[u16],
    _flags: i32,
) {
    // fontForColor(graphics.getColor()).drawChars(graphics, chars, x, y, 17);
    let color = graphics.color();
    let font = font_for_color(s, color);
    bitmap_font::draw_chars(font, graphics, chars, x, y, 17);
}

/// `public static final int stringWidth(char[] chars)` (`bh.a:([C)I => []`):
/// `currentFont.stringWidth(charsToString(chars))`. `charsToString` is a
/// `new String(chars)` round-trip (identity on the char sequence).
pub fn string_width(s: &FontManagerState, chars: &[u16]) -> i32 {
    let font = current_font(s);
    bitmap_font::string_width(font, Some(chars))
}

/// `public static final String getString(int id)`: `StringTable.instance.get(id)
/// .replace(';', '\n')`.
pub fn get_string(g: &Game, id: i32) -> Vec<u16> {
    // StringTable.instance.get(id)
    let raw = string_table::get(&g.string_table, id);
    // .replace(';', '\n')
    raw.iter()
        .map(|&c| if c == (b';' as u16) { b'\n' as u16 } else { c })
        .collect()
}

/// The title paint's label prerequisites, at the anti-bog boundary — the two
/// `FontManager` labels the state-1 paint reads, filled the way the deferred boot
/// (`loadLanguage` → `StringTable.load` + a subset of `loadLabels`; `AppConfig.apply`)
/// would.
///
/// - `StringTable.instance.load("/lang/language", "", 1)` — `loadLanguage(1)`
///   (langChoice = -1 as no `HO-LangList` ⇒ boot calls `loadLanguage(1)`; index 1
///   is `fr-FR`, the EN baseline's mislabeled English file).
/// - `titleFooter = getString(3950).toCharArray()` — the one `loadLabels` entry
///   the title reads ("PRESS ANY KEY").
/// - `versionText = "2.0.7".toCharArray()` — `AppConfig.apply` reads
///   `MIDlet-Version` = "2.0.7"; `fullVersion` is false (`HO-Demo = BEJ8K52N7A`),
///   so no suffix is appended.
pub fn load_title_labels(g: &mut Game) {
    // StringTable.instance.load("/lang/language", "", 1);
    string_table::load(g, "/lang/language", "", 1);
    // titleFooter = getString(3950).toCharArray();
    let footer = get_string(g, 3950);
    g.font_manager.title_footer = Some(footer);
    // versionText = "2.0.7".toCharArray();  (from MIDlet-Version; fullVersion=false)
    g.font_manager.version_text = Some("2.0.7".encode_utf16().collect());
}
