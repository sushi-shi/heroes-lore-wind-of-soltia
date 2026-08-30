//! Transliterated from `java/src/main/java/defpackage/AboutScreen.java`
//! (original `bl.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The credits/about screen (`AboutScreen extends Menu`), reached from the title
//! [`MainMenu`](crate::main_menu) FIRE-select case 4. The constructor assembles an
//! info blob from the MIDlet properties (name, vendor), the copyright/website/version
//! strings and two localized labels, wraps it into lines (start offsets in
//! [`line_offsets`](AboutScreenState::line_offsets)) and lets the player scroll
//! through [`body`](AboutScreenState::body). It un-hides the small fonts' control
//! glyphs while open and re-hides them on Back.
//!
//! ## ANTI-BOG boundary
//!
//! `handleKey` is ported **fully** — the Back path (`parent.close()` via the flat
//! model's [`parent_of`](crate::menu::parent_of) scan) and the control-glyph re-hide
//! are real. The constructor's `super(parent, 1)` + the three `smallBlack/smallWhite/
//! smallOrange.hideControls = false` writes are real; the **body assembly + line-offset
//! computation** are DEFERRED — they cross into unported statics
//! (`FontManager.charsInLine` (wrapped-text), `FontManager.websiteText`,
//! `GameMIDlet.getAppProperty`, `FontManager.charsToString`), so `body`/`lineOffsets`
//! stay empty and `itemCount` stays at the `super(parent, 1)` value. `paint` is
//! **PARTIAL** (the parchment fill + title plate + heading + menu panel); the scroll
//! arrows, the page fraction, the wrapped body block and the soft keys cross into
//! unported `AssetCache`-art / `BaseCanvas` / `FontManager` wrapped-text / the DEFERRED
//! `body`/`lineOffsets` and are DEFERRED.
//!
//! `AboutScreen`'s fields (`body`, `lineOffsets`) are all per-INSTANCE (the `Menu` base
//! fields likewise), so it contributes no `java/reconstruction/ownership.tsv` static
//! rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bl.<init>:(Lcb;Z)V => [iadd,i2b,iadd,i2b]` (constructor — the line-offset
//! `charPos += charsInLine(...)` accumulation + the `itemCount`/`lineCount` casts, all
//! in the DEFERRED wrapping block), `bl.a:(II)Z => []` (handleKey — pure branches),
//! `bl.a:(…Graphics;II)V => [ishr,iadd,iadd,ishr,iadd,isub,isub,isub,iadd,isub,ishr,iadd,isub]`
//! (paint — the ported prefix is `[ishr (x+155 >> 1), iadd (y+5), iadd (y+24)]`; the
//! remaining shifts/adds live in the DEFERRED fraction + wrapped-body draw).

use crate::font_manager;
use crate::game::Game;
use crate::main_menu;
use crate::menu::{self, MenuNode};
use j2me_jvm::ishr;

/// Java `bl` / `AboutScreen` instance state — the `Menu` (`cb`) base fields plus the
/// about screen's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct AboutScreenState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private char[] body;` (obf `a`) — the assembled, wrapped about-text
    /// (name/credits/website/version). DEFERRED (assembly crosses into unported
    /// `FontManager.websiteText` / `GameMIDlet.getAppProperty`) — stays empty.
    pub body: Vec<u16>,
    /// `private short[] lineOffsets;` (obf `a`, collision) — the start offset into
    /// [`body`](Self::body) of each wrapped line. DEFERRED (`FontManager.charsInLine`
    /// unported) — stays empty.
    pub line_offsets: Vec<i16>,
}

/// `public AboutScreen(Menu parent, boolean inGame)`
/// (`bl.<init>:(Lcb;Z)V => [iadd,i2b,iadd,i2b]`): `super(parent, (byte) 1)`, then the
/// body assembly + line-offset wrap (DEFERRED), then un-hides the three small fonts'
/// control glyphs.
pub fn construct(g: &mut Game, _in_game: bool) {
    // super(parent, (byte) 1);   (parent is the MainMenu → present)
    g.about_screen.base = menu::construct(true, 1);
    // GameMIDlet gameMIDlet = GameMIDlet.instance;
    // String developerLabel = FontManager.getString(3927);
    // String versionLabel = FontManager.getString(3928);
    // String appName = gameMIDlet.getAppProperty("MIDlet-Name").toUpperCase();
    // this.body = new StringBuffer().append(appName).append("\n\n")...toCharArray();
    // short[] offsets = new short[20]; int charPos = 0, lineCount = 0;
    // while (charPos < this.body.length) {
    //     offsets[lineCount++] = (short) charPos;
    //     charPos += FontManager.charsInLine(this.body, charPos, 130, 11);   // [iadd]
    // }
    // this.lineOffsets = new short[lineCount];
    // System.arraycopy(offsets, 0, this.lineOffsets, 0, this.lineOffsets.length);
    // ((Menu) this).itemCount = (byte) this.lineOffsets.length;                // [i2b]
    // (DEFERRED: the body assembly reads FontManager.websiteText + GameMIDlet.
    //  getAppProperty (both unported), and the line-offset loop drives on the unported
    //  wrapped-text FontManager.charsInLine — so `body`/`lineOffsets` stay empty and
    //  `itemCount` keeps the super(parent, 1) value.)
    // ((BitmapFont) FontManager.smallBlack).hideControls = false;
    g.font_manager
        .small_black
        .as_mut()
        .expect("smallBlack null")
        .hide_controls = false;
    // ((BitmapFont) FontManager.smallWhite).hideControls = false;
    g.font_manager
        .small_white
        .as_mut()
        .expect("smallWhite null")
        .hide_controls = false;
    // ((BitmapFont) FontManager.smallOrange).hideControls = false;
    g.font_manager
        .small_orange
        .as_mut()
        .expect("smallOrange null")
        .hide_controls = false;
}

/// `public final boolean handleKey(int action, int keyCode)` (`bl.a:(II)Z => []`):
/// forwards to the child, then vertical no-wrap navigation; on Back (`keyCode == -8`)
/// closes via `parent.close()` and re-hides the small fonts' control glyphs. Always
/// returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode) || keyCode != -8) return true;
    if menu::pass_key_to_child(g, MenuNode::About, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.about_screen.base, action, key_code)
        || key_code != -8
    {
        return true;
    }
    // ((Menu) this).parent.close();
    let parent =
        menu::parent_of(g, MenuNode::About).expect("NullPointerException: AboutScreen.parent");
    menu::close(g, parent);
    // ((BitmapFont) FontManager.smallBlack).hideControls = true;
    g.font_manager
        .small_black
        .as_mut()
        .expect("smallBlack null")
        .hide_controls = true;
    // ((BitmapFont) FontManager.smallWhite).hideControls = true;
    g.font_manager
        .small_white
        .as_mut()
        .expect("smallWhite null")
        .hide_controls = true;
    // ((BitmapFont) FontManager.smallOrange).hideControls = true;
    g.font_manager
        .small_orange
        .as_mut()
        .expect("smallOrange null")
        .hide_controls = true;
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int x, int y)` (`bl.a:(…Graphics;II)V`):
/// **PARTIAL** — the parchment fill, the shared title plate, the "About" heading
/// (`drawMenuItem(9)`) and the three-row menu panel. The scroll arrows
/// (`AssetCache.scrollUpArrow`/`scrollDownArrow`), the page fraction
/// (`BaseCanvas.drawFraction`), the wrapped body block
/// (`FontManager.drawWrappedBlockCentered` over the DEFERRED `body`/`lineOffsets`) and
/// the soft keys (`FontManager.labelBack` unported) are DEFERRED.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    let Game {
        screen,
        asset_cache,
        base_canvas,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // MainMenu.drawTitlePlate(graphics, x, y);
    main_menu::draw_title_plate(asset_cache, &mut graphics, x, y);
    // FontManager.drawMenuItem(graphics, 9, (x + 155) >> 1, y + 5);
    font_manager::draw_menu_item(
        font_manager,
        &mut graphics,
        base_canvas,
        9,
        ishr(x.wrapping_add(155), 1),
        y.wrapping_add(5),
    );
    // MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
    main_menu::draw_menu_panel(asset_cache, &mut graphics, x, y.wrapping_add(24), 3);
    // int contentX = x + 12; int contentY = y + 42;
    // if (itemCount > 1) { if (cursorIndex > 0) drawImage(scrollUpArrow, contentX+62, contentY-6, 20);
    //                      if (cursorIndex < itemCount-1) drawImage(scrollDownArrow, contentX+62, contentY+114, 20); }
    // BaseCanvas.drawFraction(graphics, (contentX+155)-25, contentY-8, cursorIndex+1, itemCount);
    // short lineStart = this.lineOffsets[cursorIndex];
    // short lineEnd = cursorIndex == itemCount-1 ? (short) body.length : lineOffsets[cursorIndex+1];
    // if (this.body[0] == '!' && lineStart == 0) lineStart = 1;
    // graphics.setColor(0);
    // FontManager.drawWrappedBlockCentered(graphics, (contentX+155)>>1, contentY+3, 130, 1, body, lineStart, 0, lineEnd-lineStart);
    // FontManager.drawSoftKeys(graphics, (char[]) null, FontManager.labelBack);
    // (DEFERRED: the scroll arrows read the unported AssetCache art bank; drawFraction
    //  is unported (BaseCanvas); the wrapped body block reads the DEFERRED `body`/
    //  `lineOffsets` through the unported FontManager.drawWrappedBlockCentered; and
    //  FontManager.labelBack is unported.)
}
