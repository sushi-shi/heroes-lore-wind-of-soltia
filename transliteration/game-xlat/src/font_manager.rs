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
//! This increment also completes the **wrapped-text family** — the greedy
//! word-wrap layout/measure/draw hub (`wrapLines` + `wrapCache`, `charsInLine`,
//! `lineCount`, `measureBlockHeight`, `blockHeightPx`, `lineSpacing`, the whole
//! `drawWrapped*` set) built on [`wrap_font::wrap_into`] / [`bitmap_font::draw_lines`]
//! — plus the small text utilities (`clearScreen`, `percentOf`, `getStringChars`,
//! `isSeparator`, `advanceWord`, `charsToString`, `replaceAll`). This un-defers the
//! many menu/HUD paints that route text through `FontManager.drawWrappedText`.
//!
//! ## ANTI-BOG boundary
//!
//! `FontManager` owns ~30 label `static`s (soft-key labels, menu labels, prompts)
//! filled by `loadLabels(3902..3950)`. Only the statics the title paint + the
//! main-menu render read are modelled here: the six fonts, `currentFont`,
//! `bigFontActive`, `versionText`, `titleFooter`, the main-menu labels, the
//! `Ok`/`Exit` soft-key labels, plus the `wrapCache`/`wrapCacheKey` the wrapped-text
//! family needs. [`load_title_labels`] fills the labels the title/menu need (the
//! full `loadLabels(3902..3950)` remains DEFERRED); the `versionText` source
//! (`AppConfig.apply` reading `MIDlet-Version`) is DEFERRED and injected here from
//! the reviewed manifest value. `loadLocaleImage` (`Image.createImage(String)`
//! classpath-resource decode) and `requestBuyAndExit` (`GameMIDlet.platformRequest`,
//! `AppConfig.buyUrl`, `exit`) reach still-unported host/config seams and stay
//! DEFERRED (see their stubs below).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bh.a:()V => []`
//! (initFonts), `bh.a:([C)I => []` (stringWidth), `bh.a:(…Graphics;II[CI)I => []`
//! (drawChars) and `bh.a:(…Graphics;II[CI)V => []` (drawCharsCentered) — all
//! arithmetic-free wrappers over the `az` font methods; `bh.a:(I)String` (getString)
//! is a `StringTable.get` + `String.replace` with no arithmetic opcodes.

use crate::base_canvas::BaseCanvasState;
use crate::bitmap_font::{self, BitmapFontState};
use crate::game::Game;
use crate::string_table::{self, StringTableData};
use crate::wrap_font;
use j2me_jvm::java_div;

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
    /// `wrapCache` (`static Vector`, obf `a`) — scratch cache holding the lines of
    /// the most recent [`wrap_lines`] call. `new Vector()` ⇒ empty.
    pub wrap_cache: Vec<Vec<u16>>,
    /// `wrapCacheKey` (`static String`, obf `e`) — the cache guard, kept as the
    /// empty string, so any non-empty text re-wraps (and empty text reuses the
    /// stale cache — preserved verbatim).
    pub wrap_cache_key: Vec<u16>,
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

// ===========================================================================
// The wrapped-text family — the greedy word-wrap layout/measure/draw hub. Built
// on `WrapFont.wrapInto` (via `wrap_lines` → `wrapCache`) and `BitmapFont.drawLines`.
// ===========================================================================

/// `new String(char[] value, int offset, int count)` — the substring copy the
/// wrapped-block draws take before wrapping. Panics on a bad range exactly as the
/// JVM's `StringIndexOutOfBoundsException` would (the reached call sites are not
/// guarded — an uncaught throw is faithful).
fn substring(chars: &[u16], offset: i32, count: i32) -> Vec<u16> {
    chars[offset as usize..(offset.wrapping_add(count)) as usize].to_vec()
}

/// `public static final void clearScreen(Graphics graphics)`
/// (`bh.a:(…Graphics;)V => []`): clears the whole canvas to black.
pub fn clear_screen(graphics: &mut j2me_me::Graphics, base_canvas: &BaseCanvasState) {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    // graphics.setColor(0);
    graphics.set_color(0);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
}

/// `public static final int percentOf(int value, int percent)`
/// (`bh.a:(II)I => [imul, idiv]`): `(value * percent) / 100`. The `/ 100` routes
/// through `java_div` for uniformity (a nonzero constant divisor — `.expect`).
pub fn percent_of(value: i32, percent: i32) -> i32 {
    // return (value * percent) / 100;
    java_div(value.wrapping_mul(percent), 100).expect("percentOf / 100")
}

/// `String.trim()` — drops leading/trailing chars `<= ' '` (0x20). Used by
/// [`get_string_chars`] before `Integer.parseInt`.
fn trim(s: &[u16]) -> Vec<u16> {
    let mut start: usize = 0;
    let mut end: usize = s.len();
    while start < end && (s[start] as i32) <= 32 {
        start = start.wrapping_add(1);
    }
    while end > start && (s[end - 1] as i32) <= 32 {
        end -= 1;
    }
    s[start..end].to_vec()
}

/// `Integer.parseInt(String)` (decimal) — `None` reproduces the
/// `NumberFormatException` the caller's `catch (Exception)` eats.
fn parse_int(s: &[u16]) -> Option<i32> {
    let text = String::from_utf16(s).ok()?;
    text.parse::<i32>().ok()
}

/// `public static final char[] getStringChars(String id)` (`bh.a:(…String;)[C => []`):
/// resolve an ASCII-decimal lang id (as extracted from binary asset data) to glyph
/// `char[]`, replacing `';'` with `'\n'`. On a parse failure returns a `"2."`-prefixed
/// diagnostic (never throws), matching the original's `catch (Exception)`. Only
/// `Integer.parseInt` can throw here — `StringTable.get` already swallows its own.
pub fn get_string_chars(st: &StringTableData, id: &[u16]) -> Vec<u16> {
    // try { return StringTable.instance.get(Integer.parseInt(id.trim())).replace(';', '\n').toCharArray(); }
    match parse_int(&trim(id)) {
        Some(n) => {
            let raw = string_table::get(st, n);
            // .replace(';', '\n')
            raw.iter()
                .map(|&c| if c == (b';' as u16) { b'\n' as u16 } else { c })
                .collect()
        }
        // catch (Exception e2) { return ("2." + e2.toString()).toCharArray(); }
        //   The JVM exception `.toString()` is not reproducible; a fixed "2."-tagged
        //   placeholder stands in (never produced on the trusted, numeric-id paths),
        //   as with `StringTable.get`'s own diagnostic.
        None => "2.error".encode_utf16().collect(),
    }
}

/// `private static final boolean isSeparator(char c)` (`bh.a:(C)Z => []`):
/// `c == ';'`.
pub fn is_separator(c: u16) -> bool {
    // return c == ';';
    (c as i32) == 59
}

/// `public static final int advanceWord(char[] text, int start, int shown)`
/// (`bh.a:([CII)I => [iadd,iinc,iadd,isub,iinc,isub]`): advances a typewriter reveal
/// by one word — from `start + shown`, skip separators, then return the char count
/// (relative to `start`) up to and including the next word, or the remaining length.
pub fn advance_word(text: &[u16], start: i32, shown: i32) -> i32 {
    // int i = start + shown; boolean seenWord = false;
    let mut i: i32 = start.wrapping_add(shown);
    let mut seen_word: bool = false;
    // while (i < text.length)
    while i < text.len() as i32 {
        // if (isSeparator(text[i])) { i++; }
        if is_separator(text[i as usize]) {
            i = i.wrapping_add(1);
        } else {
            // if (seenWord) return (i + 1) - start;
            if seen_word {
                return i.wrapping_add(1).wrapping_sub(start);
            }
            // seenWord = true; i++;
            seen_word = true;
            i = i.wrapping_add(1);
        }
    }
    // return text.length - start;
    (text.len() as i32).wrapping_sub(start)
}

/// `public static final int charsInLine(char[] text, int start, int widthPx, int flags)`
/// (`bh.a:([CIII)I => [isub]`): the number of characters from `start` that fit on
/// one line of width `widthPx` in `currentFont`.
pub fn chars_in_line(
    s: &FontManagerState,
    text: &[u16],
    start: i32,
    width_px: i32,
    flags: i32,
) -> i32 {
    // return currentFont.wrap(new String(text, start, text.length - start), widthPx, flags);
    let sub = substring(text, start, (text.len() as i32).wrapping_sub(start));
    wrap_font::wrap(current_font(s), &sub, width_px, flags)
}

/// `public static final int lineCount(char[] chars, int widthPx)`
/// (`bh.a:([CI)I => []`): the number of wrapped lines `chars` occupies at `widthPx`.
pub fn line_count(s: &mut FontManagerState, chars: &[u16], width_px: i32) -> i32 {
    // return wrapLines(charsToString(chars), widthPx).size();
    let text = chars_to_string(chars);
    wrap_lines(s, &text, width_px);
    s.wrap_cache.len() as i32
}

/// `public static final int measureBlockHeight(int widthPx, int, char[] chars, int, int, int)`
/// (`bh.a:(II[CIII)I => []`): the pixel height of `chars` wrapped to `widthPx` (only
/// `widthPx` and `chars` are used).
#[allow(clippy::too_many_arguments)]
pub fn measure_block_height(
    s: &mut FontManagerState,
    width_px: i32,
    _arg2: i32,
    chars: &[u16],
    _arg4: i32,
    _arg5: i32,
    _arg6: i32,
) -> i32 {
    // return blockHeightPx(new String(chars), 0, 0, widthPx);
    block_height_px(s, &chars_to_string(chars), 0, 0, width_px)
}

/// `private static int lineSpacing()` (`bh.b:()I => []`):
/// `((BitmapFont) currentFont).lineSpacing`.
fn line_spacing(s: &FontManagerState) -> i32 {
    current_font(s).line_spacing
}

/// `private static int blockHeightPx(String text, int, int, int widthPx)`
/// (`bh.a:(…String;III)I => [isub]`): the pixel height of `text` wrapped to
/// `widthPx`, minus one line's spacing.
fn block_height_px(
    s: &mut FontManagerState,
    text: &[u16],
    _arg2: i32,
    _arg3: i32,
    width_px: i32,
) -> i32 {
    // return currentFont.blockHeight(wrapLines(text, widthPx)) - lineSpacing();
    //   `wrapLines` runs first (its side effect on wrapCache); `currentFont` is a
    //   pure field unaffected by it, so resolving it after is observationally
    //   identical and keeps `currentFont` + `wrapCache` as two immutable borrows.
    wrap_lines(s, text, width_px);
    let bh = bitmap_font::block_height(current_font(s), &s.wrap_cache);
    bh.wrapping_sub(line_spacing(s))
}

/// `private static Vector wrapLines(String text, int widthPx)` (`bh.a:(…String;I)Vector => []`):
/// wraps `text` to `widthPx` into `wrapCache` and returns it. Guarded by
/// `wrapCacheKey` (kept empty, so any non-empty text re-wraps). Java returns
/// `wrapCache`; callers read `s.wrap_cache` directly, so this returns nothing.
fn wrap_lines(s: &mut FontManagerState, text: &[u16], width_px: i32) {
    // if (!text.equals(wrapCacheKey)) { wrapCache.setSize(0); currentFont.wrapInto(wrapCache, text, widthPx); }
    if text != s.wrap_cache_key.as_slice() {
        // wrapCache.setSize(0);
        s.wrap_cache.clear();
        // currentFont.wrapInto(wrapCache, text, widthPx);
        //   `currentFont` resolves to one of the six font fields; borrow that field
        //   directly (a disjoint field borrow) so `wrapCache` stays mutably borrowable.
        let font: &BitmapFontState = match s.current_font {
            CurrentFont::SmallBlack => s.small_black.as_ref(),
            CurrentFont::SmallWhite => s.small_white.as_ref(),
            CurrentFont::SmallOrange => s.small_orange.as_ref(),
            CurrentFont::BigBlack => s.big_black.as_ref(),
            CurrentFont::BigWhite => s.big_white.as_ref(),
            CurrentFont::Unset => panic!("currentFont read before initFonts"),
        }
        .expect("currentFont null");
        wrap_font::wrap_into(font, &mut s.wrap_cache, text, width_px);
    }
}

/// `public static final void drawWrappedBlock(Graphics, int x, int y, int widthPx, int, char[] chars, int offset, int, int count)`
/// (`bh.a:(…Graphics;IIII[CIII)V => [iadd, isub]`): draws a wrapped block of `count`
/// chars (from `offset`), wrapped to `widthPx`, anchored top-left (anchor 20).
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_block(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &BaseCanvasState,
    x: i32,
    y: i32,
    width_px: i32,
    _arg5: i32,
    chars: &[u16],
    offset: i32,
    _arg7: i32,
    count: i32,
) {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    // BitmapFont font = fontForColor(graphics.getColor());  (pure lookup; resolved below)
    let color = graphics.color();
    // if (offset + count > chars.length) count = chars.length - offset;
    let mut count: i32 = count;
    if offset.wrapping_add(count) > chars.len() as i32 {
        count = (chars.len() as i32).wrapping_sub(offset);
    }
    // font.drawLines(graphics, wrapLines(new String(chars, offset, count), widthPx), x, y, BaseCanvas.height, 20);
    let sub = substring(chars, offset, count);
    wrap_lines(s, &sub, width_px);
    let font = font_for_color(s, color);
    bitmap_font::draw_lines(font, graphics, &s.wrap_cache, x, y, base_canvas.height, 20);
}

/// `public static final void drawWrappedBlockCentered(...)`
/// (`bh.b:(…Graphics;IIII[CIII)V => [iadd, isub]`): like [`draw_wrapped_block`] but
/// horizontally centred (anchor 17).
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_block_centered(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &BaseCanvasState,
    x: i32,
    y: i32,
    width_px: i32,
    _arg5: i32,
    chars: &[u16],
    offset: i32,
    _arg7: i32,
    count: i32,
) {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    let color = graphics.color();
    // if (offset + count > chars.length) count = chars.length - offset;
    let mut count: i32 = count;
    if offset.wrapping_add(count) > chars.len() as i32 {
        count = (chars.len() as i32).wrapping_sub(offset);
    }
    // font.drawLines(graphics, wrapLines(new String(chars, offset, count), widthPx), x, y, BaseCanvas.height, 17);
    let sub = substring(chars, offset, count);
    wrap_lines(s, &sub, width_px);
    let font = font_for_color(s, color);
    bitmap_font::draw_lines(font, graphics, &s.wrap_cache, x, y, base_canvas.height, 17);
}

/// `public static final void drawWrappedBlockPartial(...)`
/// (`bh.c:(…Graphics;IIII[CIII)V => [isub, iadd, isub, iadd, iadd, iinc]`): draws up
/// to the first three wrapped lines with a typewriter reveal — `shown` counts the
/// characters still to reveal, so the line that runs out is clipped mid-string.
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_block_partial(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &BaseCanvasState,
    x: i32,
    y: i32,
    width_px: i32,
    _arg5: i32,
    chars: &[u16],
    offset: i32,
    _arg7: i32,
    shown: i32,
) {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    let color = graphics.color();
    // Vector lines = wrapLines(new String(chars, offset, chars.length - offset), widthPx);
    let sub = substring(chars, offset, (chars.len() as i32).wrapping_sub(offset));
    wrap_lines(s, &sub, width_px);
    // int lineMax = Math.min(lines.size(), 3);
    let line_max: i32 = core::cmp::min(s.wrap_cache.len() as i32, 3);
    let font = font_for_color(s, color);
    // for (int li = 0; li < lineMax; li++)
    let mut shown: i32 = shown;
    let mut y: i32 = y;
    let mut li: i32 = 0;
    while li < line_max {
        // String line = (String) lines.elementAt(li);
        let line = &s.wrap_cache[li as usize];
        // if (shown <= line.length()) { font.drawString(graphics, line, 0, shown, x, y, 20); return; }
        if shown <= line.len() as i32 {
            bitmap_font::draw_string_range(font, graphics, line, 0, shown, x, y, 20);
            return;
        }
        // font.drawString(graphics, line, x, y, 20);
        bitmap_font::draw_string(font, graphics, line, x, y, 20);
        // shown -= line.length() + 1;
        shown = shown.wrapping_sub((line.len() as i32).wrapping_add(1));
        // y += ((BitmapFont) currentFont).lineHeight + 2;
        y = y.wrapping_add(current_font(s).line_height.wrapping_add(2));
        li = li.wrapping_add(1);
    }
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
}

/// `public static final int drawWrappedText(Graphics, int x, int y, int widthPx, int, char[] chars, int anchor)`
/// (`bh.a:(…Graphics;IIII[CI)I => []`): draws all wrapped lines of `chars` (to
/// `widthPx`) at (x,y) with the given anchor. Returns the underlying line-draw height.
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_text_anchor(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &BaseCanvasState,
    x: i32,
    y: i32,
    width_px: i32,
    _arg5: i32,
    chars: &[u16],
    anchor: i32,
) -> i32 {
    // graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    // return fontForColor(getColor()).drawLines(graphics, wrapLines(charsToString(chars), widthPx), x, y, BaseCanvas.height, anchor);
    //   fontForColor(getColor()) is a pure lookup unaffected by wrapLines; resolving
    //   it after keeps `wrapCache` + the font as two simultaneous immutable borrows.
    let color = graphics.color();
    let text = chars_to_string(chars);
    wrap_lines(s, &text, width_px);
    let font = font_for_color(s, color);
    bitmap_font::draw_lines(
        font,
        graphics,
        &s.wrap_cache,
        x,
        y,
        base_canvas.height,
        anchor,
    )
}

/// `public static final int drawWrappedText(Graphics, int x, int y, int widthPx, int, char[] chars)`
/// (`bh.a:(…Graphics;IIII[C)I => []`): the top-left-anchored (20) convenience overload.
#[allow(clippy::too_many_arguments)]
pub fn draw_wrapped_text(
    s: &mut FontManagerState,
    graphics: &mut j2me_me::Graphics,
    base_canvas: &BaseCanvasState,
    x: i32,
    y: i32,
    width_px: i32,
    arg5: i32,
    chars: &[u16],
) -> i32 {
    // return drawWrappedText(graphics, x, y, widthPx, arg5, chars, 20);
    draw_wrapped_text_anchor(s, graphics, base_canvas, x, y, width_px, arg5, chars, 20)
}

/// `public static final String charsToString(char[] chars)` (`bh.a:([C)String => []`):
/// `new String(chars)` — a copy of the char sequence (`String` == `Vec<u16>`).
pub fn chars_to_string(chars: &[u16]) -> Vec<u16> {
    chars.to_vec()
}

/// `String.indexOf(String)` for [`replace_all`] — the start index of the first
/// occurrence of `needle` in `haystack`, or `-1` (an empty `needle` returns 0, as
/// Java's does).
fn index_of_sub(haystack: &[u16], needle: &[u16]) -> i32 {
    if needle.is_empty() {
        return 0;
    }
    if needle.len() > haystack.len() {
        return -1;
    }
    let last: usize = haystack.len() - needle.len();
    let mut i: usize = 0;
    while i <= last {
        if &haystack[i..i + needle.len()] == needle {
            return i as i32;
        }
        i += 1;
    }
    -1
}

/// `public static final String replaceAll(String source, String find, String replacement)`
/// (`bh.a:(…String;String;String;)String => [iadd]`): replaces every occurrence of
/// `find` in `source` with `replacement`.
pub fn replace_all(source: &[u16], find: &[u16], replacement: &[u16]) -> Vec<u16> {
    let mut source: Vec<u16> = source.to_vec();
    // while (true)
    loop {
        // int index = source.indexOf(find);
        let index: i32 = index_of_sub(&source, find);
        // if (index < 0) return source;
        if index < 0 {
            return source;
        }
        // String head = source.substring(0, index);
        let head: Vec<u16> = source[0..index as usize].to_vec();
        // source = head + replacement + source.substring(index + find.length());
        let tail_start: usize = (index as usize).wrapping_add(find.len());
        let mut next: Vec<u16> = head;
        next.extend_from_slice(replacement);
        next.extend_from_slice(&source[tail_start..]);
        source = next;
    }
}

// --- DEFERRED (reach still-unported host/config seams) ---------------------
//
// `loadLocaleImage(String path)` (`bh.a:(…String;)Image => []`): returns
//   `Image.createImage("/" + StringTable.locales[localeIndex] + "/" + path)`.
//   The `StringTable` locale statics are available, but `Image.createImage(String)`
//   is the classpath-resource-load-and-decode seam (not the injected-bytes `.mf`
//   path), and this loader serves the (unported) locale-art menus, not the
//   wrapped-text path. DEFERRED.
//
// `requestBuyAndExit(String ignored)` (`bh.a:(…String;)V => []`): does
//   `GameMIDlet.instance.platformRequest(AppConfig.buyUrl); GameMIDlet.instance.exit();`.
//   Both `GameMIDlet.platformRequest`/`exit` (host ops) and `AppConfig.buyUrl` (an
//   unported `AppConfig` static) are still-unported cross-class seams. DEFERRED.
