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
    /// `mainMenuLabels` (`char[7][]`, obf `bh.a:[[C`) — the main-menu item labels
    /// (lang 3920..3926: New Game / Load Game / Options / Help / About / (buy) /
    /// Exit), filled by [`load_title_labels`] (the boot's `loadLabels` subset). Each
    /// entry is `None` until filled. `mainMenuLabels[5]` is overwritten by
    /// `AppConfig.apply` (see [`load_title_labels`]).
    pub main_menu_labels: Vec<Option<Vec<u16>>>,
    /// `labelOk` (lang 3908 = "Ok") — the main-menu left soft-key label.
    pub label_ok: Option<Vec<u16>>,
    /// `labelExit` (lang 3907 = "Exit") — the main-menu right soft-key label.
    pub label_exit: Option<Vec<u16>>,
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

    // --- The main-menu label subset of `loadLabels(StringTable)` (bh.a:(Lcj;)V) ---
    // The boot's `loadLanguage` runs the full `loadLabels(3902..3950)`; ANTI-BOG,
    // only the labels the main-menu render reads are filled here:
    //   labelExit = getString(3907); labelOk = getString(3908);
    //   mainMenuLabels[0..6] = getString(3920,3921,3922,3923,3924,3924,3926)
    // (the `[4]` and `[5]` are BOTH `getString(3924)` in the shipped bytecode —
    // verified by `javap -c` on `bh.class`, aastore indices 4 and 5).
    g.font_manager.label_exit = Some(get_string(g, 3907));
    g.font_manager.label_ok = Some(get_string(g, 3908));
    let l0 = get_string(g, 3920);
    let l1 = get_string(g, 3921);
    let l2 = get_string(g, 3922);
    let l3 = get_string(g, 3923);
    let l4 = get_string(g, 3924);
    let l5 = get_string(g, 3924);
    let l6 = get_string(g, 3926);
    g.font_manager.main_menu_labels = vec![
        Some(l0),
        Some(l1),
        Some(l2),
        Some(l3),
        Some(l4),
        Some(l5),
        Some(l6),
    ];

    // --- `AppConfig.apply()` (deferred; see game_loop's cross-owner snapshot
    // convention): when `menuBuyEnabled` is false it overwrites
    //   mainMenuLabels[5] = mainMenuLabels[6];
    // so the 6-item fresh-install menu's last row is "Exit" (getString(3926)),
    // NOT the second `getString(3924)`. On the EN v207 build `menuBuyEnabled` is
    // false (the reference main-menu has 6 rows ending in Exit, no buy row).
    if !APP_CONFIG_MENU_BUY_ENABLED {
        g.font_manager.main_menu_labels[5] = g.font_manager.main_menu_labels[6].clone();
    }
    // (menuBuyEnabled == true would set mainMenuLabels[5] = resolveBuyLabel() — DEFERRED.)
}

/// Deferred cross-class static `AppConfig.menuBuyEnabled` (obf `...:Z`), read by
/// `AppConfig.apply` and `MainMenu`. On the EN v207 build it is `false` (`HO-Demo`
/// marks a demo, but no in-menu buy row is configured — the captured main-menu has
/// exactly six rows ending in "Exit"). AppConfig is not ported in this increment;
/// snapshotted per the contract's cross-owner-read convention.
pub const APP_CONFIG_MENU_BUY_ENABLED: bool = false;

/// `public static final void setBigFont(boolean active)` (`bh.a:(Z)V => []`):
/// selects the small/big family as `currentFont`.
pub fn set_big_font(s: &mut FontManagerState, active: bool) {
    // bigFontActive = active;
    s.big_font_active = active;
    // currentFont = bigFontActive ? bigBlack : smallBlack;
    s.current_font = if s.big_font_active {
        CurrentFont::BigBlack
    } else {
        CurrentFont::SmallBlack
    };
}

/// `public static final int lineHeight()` (`bh.a:()I => []`):
/// `((BitmapFont) currentFont).lineHeight`.
pub fn line_height(s: &FontManagerState) -> i32 {
    current_font(s).line_height
}

/// `public static final void drawMenuItem(Graphics graphics, int itemState, int unused, int y)`
/// (`bh.a:(…Graphics;III)V => [ishr, ishr, irem, iadd]`): draws one centred
/// main-menu item in the big font. An even `itemState` renders it white
/// (highlighted), an odd one black; the item is `mainMenuLabels[itemState >> 1]`.
pub fn draw_menu_item(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &crate::base_canvas::BaseCanvasState,
    item_state: i32,
    _unused: i32,
    y: i32,
) {
    // int centerX = BaseCanvas.width >> 1;
    let center_x: i32 = j2me_jvm::ishr(base_canvas.width, 1);
    // setBigFont(true);
    set_big_font(s, true);
    // int index = itemState >> 1;
    let index: i32 = j2me_jvm::ishr(item_state, 1);
    // if (itemState % 2 == 0) setColor(16777215); else setColor(0);
    if j2me_jvm::java_rem(item_state, 2).expect("itemState % 2") == 0 {
        graphics.set_color(16777215);
    } else {
        graphics.set_color(0);
    }
    // drawCharsCentered(graphics, centerX, y + 4, mainMenuLabels[index], 1);
    let label = s.main_menu_labels[index as usize]
        .clone()
        .expect("mainMenuLabels[index] null");
    draw_chars_centered(s, graphics, center_x, y.wrapping_add(4), &label, 1);
    // setBigFont(false);
    set_big_font(s, false);
}

/// `public static final void drawSoftKeys(Graphics graphics, char[] leftLabel, char[] rightLabel)`
/// (`bh.a:(…Graphics;[C[C)V => [iadd,iadd,isub,iadd,iadd,iadd,isub,isub,iadd,iadd,iadd]`):
/// the bottom soft-key command bar — a black box + white label bottom-left for
/// `leftLabel`, bottom-right for `rightLabel`. Either may be `null`.
pub fn draw_soft_keys(
    s: &FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &crate::base_canvas::BaseCanvasState,
    left_label: Option<&[u16]>,
    right_label: Option<&[u16]>,
) {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    // int barHeight = lineHeight() + 5;
    let bar_height: i32 = line_height(s).wrapping_add(5);
    // if (leftLabel != null) { ... }
    if let Some(left) = left_label {
        // int boxWidth = stringWidth(leftLabel) + 2;
        let box_width: i32 = string_width(s, left).wrapping_add(2);
        // int boxY = (BaseCanvas.height - barHeight) + 3;
        let box_y: i32 = base_canvas.height.wrapping_sub(bar_height).wrapping_add(3);
        // graphics.setColor(0); graphics.fillRect(0, boxY, boxWidth, barHeight);
        graphics.set_color(0);
        graphics.fill_rect(0, box_y, box_width, bar_height);
        // graphics.setColor(16777215); drawChars(graphics, 1, boxY + 1, leftLabel, 1);
        graphics.set_color(16777215);
        draw_chars(s, graphics, 1, box_y.wrapping_add(1), left, 1);
    }
    // if (rightLabel != null) { ... }
    if let Some(right) = right_label {
        // int boxWidth = stringWidth(rightLabel) + 2;
        let box_width: i32 = string_width(s, right).wrapping_add(2);
        // int boxX = BaseCanvas.width - boxWidth;
        let box_x: i32 = base_canvas.width.wrapping_sub(box_width);
        // int boxY = (BaseCanvas.height - barHeight) + 3;
        let box_y: i32 = base_canvas.height.wrapping_sub(bar_height).wrapping_add(3);
        // graphics.setColor(0); graphics.fillRect(boxX, boxY, boxWidth, barHeight);
        graphics.set_color(0);
        graphics.fill_rect(box_x, box_y, box_width, bar_height);
        // graphics.setColor(16777215); drawChars(graphics, boxX + 1, boxY + 1, rightLabel, 1);
        graphics.set_color(16777215);
        draw_chars(
            s,
            graphics,
            box_x.wrapping_add(1),
            box_y.wrapping_add(1),
            right,
            1,
        );
    }
}
