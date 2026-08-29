//! Transliterated from `java/src/main/java/defpackage/MainMenu.java`
//! (original `bf.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The front/start menu (`MainMenu extends Menu`): New Game / Load Game / Options
//! / Help / About / Exit, drawn on the parchment panel (`AssetCache.menuFrames`)
//! with the big-font item labels and the sliding selection sprite. This increment
//! ports the fresh-install render + vertical navigation path:
//!
//! - `create` / the private constructor / `instance` / `dispose`;
//! - `handleKey` — the cursor navigation (`moveCursorVertical` + the disabled-Load
//!   skip + `logoFrame` reset); the FIRE-select dispatch (into `ClassSelectMenu` /
//!   `ContinueMenu` / `OptionsMenu` / `HelpMenu` / `AboutScreen` / exit-buy popups)
//!   and the `keyCode == -8` exit-popup are DEFERRED (their target screens are not
//!   ported and the `01-menu` route never presses FIRE/Back);
//! - `draw` (the `demoExpiry <= 0` branch: `render` + the `logoFrame` intro
//!   animation) — the demo trial-splash branch is DEFERRED (`demoExpiry` is 0 on a
//!   fresh install);
//! - `paint` (the parchment fill + panel + selection sprite + item labels +
//!   soft keys) and the static `drawMenuPanel`.
//!
//! `onPopupResult`, `drawTitlePlate`, the demo splash, and the save-blob plumbing
//! are DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bf.a:(Z[B)V => [isub, isub]` (create — `halfW-77`, `halfH-85`),
//! `bf.<init>:(Z[B)V => [iadd, i2b, ladd]` (constructor — the menuBuy itemCount++
//! and the demo-expiry `+5000`, both guarded false on the fresh path),
//! `bf.a:(II)Z => [iadd, i2b, isub, i2b]` (handleKey — the ±1 cursor skip),
//! `bf.a:(…Graphics;)V => [ishr, isub, ishr, iadd, i2b]` (draw — the ported subset
//! is `logoFrame + 1`; the ishr/isub/ishr are the DEFERRED demo branch),
//! `bf.a:(…Graphics;II)V => [iinc,isub,isub,iadd,ishr,iadd,imul,iadd,iadd,imul,iadd,imul,i2b,iadd,i2b,iadd,ishr,iinc]` (paint),
//! `bf.b:(…Graphics;III)V => [iinc×…,iadd×…,imul×…]` (drawMenuPanel).

use crate::asset_cache::AssetCacheState;
use crate::font_manager::{self, APP_CONFIG_MENU_BUY_ENABLED};
use crate::game::Game;
use crate::menu;
use j2me_jvm::ishr;

/// Java `bf` / `MainMenu` state: the `Menu` instance fields (as [`menu::MenuBase`]),
/// the `MainMenu` instance fields, and its `static`s (see
/// `java/reconstruction/ownership.tsv`). Reference-typed statics/fields are
/// presence flags. The static initializers `aboutIndex = 5` / `exitIndex = 5`
/// (`bf.<clinit>:()V => []`) are reproduced by [`Default`].
#[derive(Debug)]
pub struct MainMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private boolean hasSave;` (obf `bf.e`) — governs New Game vs Continue.
    pub has_save: bool,
    /// `private byte[] saveBlob;` (obf `bf.h`) — save blob for the slot picker.
    pub save_blob: Vec<i8>,
    /// `private byte logoFrame;` (obf `bf.c`) — logo intro animation frame (0→1→2).
    pub logo_frame: i8,
    /// `private byte pendingAction;` (obf `bf.d`) — which confirm a pending popup
    /// belongs to.
    pub pending_action: i8,
    /// `private long demoExpiry;` (obf `bf.a`, long) — demo trial deadline (≤0 = not
    /// a demo splash). 0 on a fresh install.
    pub demo_expiry: i64,
    /// `public static int panelX;` (obf `bf.a`) — centered panel origin X.
    pub panel_x: i32,
    /// `public static int panelY;` — centered panel origin Y.
    pub panel_y: i32,
    /// `private static int aboutIndex = 5;` (obf `bf.c`).
    pub about_index: i32,
    /// `private static int exitIndex = 5;` (obf `bf.d`).
    pub exit_index: i32,
    /// `private static MainMenu singleton;` (obf `bf.a`) — presence flag.
    pub singleton: bool,
    /// `public static boolean pendingBuyPrompt;` (obf `bf.c`).
    pub pending_buy_prompt: bool,
    /// `public static boolean pendingExitPrompt;` (obf `bf.d`).
    pub pending_exit_prompt: bool,
}

impl Default for MainMenuState {
    fn default() -> Self {
        MainMenuState {
            base: menu::MenuBase::default(),
            has_save: false,
            save_blob: Vec::new(),
            logo_frame: 0,
            pending_action: 0,
            demo_expiry: 0,
            panel_x: 0,
            panel_y: 0,
            // static initializers (bf.<clinit>):
            about_index: 5,
            exit_index: 5,
            singleton: false,
            pending_buy_prompt: false,
            pending_exit_prompt: false,
        }
    }
}

/// `public static final MainMenu instance()` (`bf.a:()Lbf;`): the current
/// main-menu singleton (presence flag; `false` while null).
pub fn instance(g: &Game) -> bool {
    // return singleton;
    g.main_menu.singleton
}

/// `public static final void create(boolean hasSave, byte[] saveBlob)`
/// (`bf.a:(Z[B)V => [isub, isub]`): builds the singleton at the centered origin.
pub fn create(g: &mut Game, has_save: bool, save_blob: Vec<i8>) {
    // panelX = BaseCanvas.halfW - 77;
    g.main_menu.panel_x = g.base_canvas.half_w.wrapping_sub(77);
    // panelY = BaseCanvas.halfH - 85;
    g.main_menu.panel_y = g.base_canvas.half_h.wrapping_sub(85);
    // singleton = new MainMenu(hasSave, saveBlob);
    construct(g, has_save, save_blob);
    g.main_menu.singleton = true;
    // if (AppConfig.menuBuyEnabled) { aboutIndex = 6; exitIndex = 5; }
    if APP_CONFIG_MENU_BUY_ENABLED {
        g.main_menu.about_index = 6;
        g.main_menu.exit_index = 5;
    }
}

/// `private MainMenu(boolean hasSave, byte[] saveBlob)`
/// (`bf.<init>:(Z[B)V => [iadd, i2b, ladd]`).
pub fn construct(g: &mut Game, has_save: bool, save_blob: Vec<i8>) {
    // super(null, (byte) 6);
    g.main_menu.base = menu::construct(6);
    // if (AppConfig.menuBuyEnabled) itemCount = (byte) (itemCount + 1);
    if APP_CONFIG_MENU_BUY_ENABLED {
        g.main_menu.base.item_count = (g.main_menu.base.item_count as i32).wrapping_add(1) as i8;
    }
    // this.hasSave = hasSave;
    g.main_menu.has_save = has_save;
    // this.saveBlob = saveBlob;
    g.main_menu.save_blob = save_blob;
    // this.logoFrame = (byte) 0;
    g.main_menu.logo_frame = 0;
    // if (pendingExitPrompt || pendingBuyPrompt) { AssetCache.loadLogo(); demoExpiry = currentTimeMillis()+5000; ... }
    if g.main_menu.pending_exit_prompt || g.main_menu.pending_buy_prompt {
        // (DEFERRED: demo buy/exit splash arming — both prompts are false on the
        // fresh-install main-menu path, so this branch never executes.)
    }
}

/// `public static final void dispose()` (`bf.d:()V => []`): releases the singleton.
pub fn dispose(g: &mut Game) {
    // singleton = null;
    g.main_menu.singleton = false;
}

/// `public final boolean handleKey(int action, int keyCode)`
/// (`bf.a:(II)Z => [iadd, i2b, isub, i2b]`): the ported subset is the cursor
/// navigation. Returns whether the key was consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (this.demoExpiry > 0) { ...demo buy/exit... }
    if g.main_menu.demo_expiry > 0 {
        // (DEFERRED: demo trial buy/exit handling — demoExpiry is 0 on a fresh install.)
        return true;
    }
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(&mut g.main_menu.base, action, key_code) {
        return true;
    }
    // if (moveCursorVertical(action, keyCode, false)) { ... }
    if menu::move_cursor_vertical(&mut g.main_menu.base, action, key_code, false) {
        // if (!hasSave && cursorIndex == 1) { skip the disabled Load row }
        if !g.main_menu.has_save && (g.main_menu.base.cursor_index as i32) == 1 {
            if action == 6 || key_code == 56 {
                // cursorIndex = (byte)(cursorIndex + 1);
                g.main_menu.base.cursor_index =
                    (g.main_menu.base.cursor_index as i32).wrapping_add(1) as i8;
            } else {
                // cursorIndex = (byte)(cursorIndex - 1);
                g.main_menu.base.cursor_index =
                    (g.main_menu.base.cursor_index as i32).wrapping_sub(1) as i8;
            }
        }
        // this.logoFrame = (byte) 0;
        g.main_menu.logo_frame = 0;
        return true;
    }
    // if (keyCode == -8) { showPopup((byte)2,(byte)2,{confirmPrompt}); pendingAction = 2; }
    if key_code == -8 {
        // (DEFERRED: showPopup exit-confirm — PopupMenu not ported; not reached by the route.)
        g.main_menu.pending_action = 2;
    }
    // if (action != 8 && keyCode != 53) return false;
    if action != 8 && key_code != 53 {
        return false;
    }
    // switch (cursorIndex) { ...FIRE-select... }
    // (DEFERRED: FIRE-select dispatch into ClassSelectMenu / ContinueMenu /
    // OptionsMenu / HelpMenu / AboutScreen / exit+buy popups — their target screens
    // are not ported and the 01-menu nav route never presses FIRE.)
    false
}

/// `public final void draw(Graphics graphics)`
/// (`bf.a:(…Graphics;)V`): the ported `demoExpiry <= 0` branch — `render` then the
/// `logoFrame` intro animation. The `demoExpiry > 0` demo trial splash is DEFERRED.
pub fn draw(g: &mut Game) {
    // if (this.demoExpiry <= 0) { ... }
    if g.main_menu.demo_expiry <= 0 {
        // render(graphics, panelX, panelY);
        let px = g.main_menu.panel_x;
        let py = g.main_menu.panel_y;
        menu::render(g, px, py);
        // if (this.logoFrame >= 2 || this.child != null) return;
        if (g.main_menu.logo_frame as i32) >= 2 || g.main_menu.base.child {
            return;
        }
        // this.needsRepaint = true;
        g.main_menu.base.needs_repaint = true;
        // this.logoFrame = (byte) (this.logoFrame + 1);
        g.main_menu.logo_frame = (g.main_menu.logo_frame as i32).wrapping_add(1) as i8;
        // return;   — the Java `return` skips the demo splash below; that branch is
        //   DEFERRED (empty), so falling out of the `if` here is equivalent.
    }
    // (DEFERRED: demo trial splash — the `demoExpiry > 0` branch: logoFrames[4] +
    // website text + buy/exit soft keys + timeout exit; not reached on the
    // fresh-install main menu.)
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bf.a:(…Graphics;II)V`): the parchment fill + menu panel + selection sprite +
/// the six big-font item labels + the soft keys.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int menuY = y + 13;
    let menu_y: i32 = y.wrapping_add(13);
    // Snapshot the soft-key labels (read-only, passed to drawSoftKeys) and the
    // menu-state scalars before the disjoint field borrow.
    let label_ok = g.font_manager.label_ok.clone();
    let label_exit = g.font_manager.label_exit.clone();
    let logo_frame = g.main_menu.logo_frame;
    let cursor_index = g.main_menu.base.cursor_index;
    let item_count = g.main_menu.base.item_count;

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
    // drawMenuPanel(graphics, x, menuY - 12, 4);
    draw_menu_panel(asset_cache, &mut graphics, x, menu_y.wrapping_sub(12), 4);
    // char logoSprite = 18; if (logoFrame==0) logoSprite=14; else if (logoFrame==1) logoSprite=16;
    let logo_sprite: i32 = if (logo_frame as i32) == 0 {
        14
    } else if (logo_frame as i32) == 1 {
        16
    } else {
        18
    };
    {
        let mf = asset_cache
            .menu_frames
            .as_ref()
            .expect("AssetCache.menuFrames null");
        let sprite = &mf[logo_sprite as usize];
        // graphics.drawImage(menuFrames[logoSprite], (x + (155 - sprite.getWidth())) >> 1, menuY + 12 + (cursorIndex * 16), 20);
        let sprite_x: i32 = ishr(x.wrapping_add(155i32.wrapping_sub(sprite.width())), 1);
        let sprite_y: i32 = menu_y
            .wrapping_add(12)
            .wrapping_add((cursor_index as i32).wrapping_mul(16));
        graphics
            .draw_image(sprite, sprite_x, sprite_y, 20)
            .expect("drawImage(menuFrames[logoSprite])");
    }
    // for (int item = 0; item < itemCount; item++) { ... }
    let mut item: i32 = 0;
    while item < (item_count as i32) {
        // int itemY = menuY + 14 + (item * 16);
        let item_y: i32 = menu_y.wrapping_add(14).wrapping_add(item.wrapping_mul(16));
        // byte labelId = (byte) (item * 2);
        let mut label_id: i32 = (item.wrapping_mul(2) as i8) as i32;
        // if (cursorIndex != item || logoFrame < 2) labelId = (byte) (labelId + 1);
        if (cursor_index as i32) != item || (logo_frame as i32) < 2 {
            label_id = (label_id.wrapping_add(1) as i8) as i32;
        }
        // FontManager.drawMenuItem(graphics, labelId, (x + 155) >> 1, itemY);
        font_manager::draw_menu_item(
            font_manager,
            &mut graphics,
            base_canvas,
            label_id,
            ishr(x.wrapping_add(155), 1),
            item_y,
        );
        item = item.wrapping_add(1);
    }
    // FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelExit);
    font_manager::draw_soft_keys(
        font_manager,
        &mut graphics,
        base_canvas,
        label_ok.as_deref(),
        label_exit.as_deref(),
    );
}

/// `public static final void drawMenuPanel(Graphics graphics, int x, int y, int rows)`
/// (`bf.b:(…Graphics;III)V`): the bordered parchment panel from the UI atlas with
/// `rows`+1 content rows.
pub fn draw_menu_panel(
    asset_cache: &AssetCacheState,
    graphics: &mut j2me_me::Graphics,
    x: i32,
    y: i32,
    rows: i32,
) {
    let mf = asset_cache
        .menu_frames
        .as_ref()
        .expect("AssetCache.menuFrames null");
    let draw = |graphics: &mut j2me_me::Graphics, idx: usize, ix: i32, iy: i32| {
        graphics
            .draw_image(&mf[idx], ix, iy, 20)
            .expect("drawImage(menuFrames)");
    };
    // int contentRows = rows + 1;
    let content_rows: i32 = rows.wrapping_add(1);
    // graphics.drawImage(menuFrames[3], x, y, 20);
    draw(graphics, 3, x, y);
    // int topX = x + 12;
    let mut top_x: i32 = x.wrapping_add(12);
    // graphics.drawImage(menuFrames[4], topX, y, 20);
    draw(graphics, 4, top_x, y);
    // for (int col = 0; col < 3; col++) { topX += 32; drawImage(menuFrames[4], topX, y, 20); }
    let mut col: i32 = 0;
    while col < 3 {
        top_x = top_x.wrapping_add(32);
        draw(graphics, 4, top_x, y);
        col = col.wrapping_add(1);
    }
    // graphics.drawImage(menuFrames[5], topX + 32, y, 20);
    draw(graphics, 5, top_x.wrapping_add(32), y);
    // graphics.drawImage(menuFrames[6], x, y + 12, 20);
    draw(graphics, 6, x, y.wrapping_add(12));
    // int midX = x + 12;
    let mut mid_x: i32 = x.wrapping_add(12);
    // graphics.drawImage(menuFrames[7], midX, y + 12, 20);
    draw(graphics, 7, mid_x, y.wrapping_add(12));
    // for (int col2 = 0; col2 < 3; col2++) { midX += 32; drawImage(menuFrames[7], midX, y + 12, 20); }
    let mut col2: i32 = 0;
    while col2 < 3 {
        mid_x = mid_x.wrapping_add(32);
        draw(graphics, 7, mid_x, y.wrapping_add(12));
        col2 = col2.wrapping_add(1);
    }
    // graphics.drawImage(menuFrames[8], midX + 32, y + 12, 20);
    draw(graphics, 8, mid_x.wrapping_add(32), y.wrapping_add(12));
    // for (int row = 0; row < contentRows; row++) { drawImage(9, x, y+36+24*row); drawImage(10, x+12+128, y+36+24*row); }
    let mut row: i32 = 0;
    while row < content_rows {
        let ry: i32 = y.wrapping_add(36).wrapping_add(24i32.wrapping_mul(row));
        draw(graphics, 9, x, ry);
        draw(graphics, 10, x.wrapping_add(12).wrapping_add(128), ry);
        row = row.wrapping_add(1);
    }
    // graphics.setColor(16763769);
    graphics.set_color(16763769);
    // graphics.fillRect(x + 12, y + 36, 128, 24 * contentRows);
    graphics.fill_rect(
        x.wrapping_add(12),
        y.wrapping_add(36),
        128,
        24i32.wrapping_mul(content_rows),
    );
    // int bottomY = y + 36 + (24 * contentRows);
    let bottom_y: i32 = y
        .wrapping_add(36)
        .wrapping_add(24i32.wrapping_mul(content_rows));
    // graphics.drawImage(menuFrames[11], x, bottomY, 20);
    draw(graphics, 11, x, bottom_y);
    // int bottomX = x + 12;
    let mut bottom_x: i32 = x.wrapping_add(12);
    // graphics.drawImage(menuFrames[12], bottomX, bottomY, 20);
    draw(graphics, 12, bottom_x, bottom_y);
    // for (int col3 = 0; col3 < 3; col3++) { bottomX += 32; drawImage(menuFrames[12], bottomX, bottomY, 20); }
    let mut col3: i32 = 0;
    while col3 < 3 {
        bottom_x = bottom_x.wrapping_add(32);
        draw(graphics, 12, bottom_x, bottom_y);
        col3 = col3.wrapping_add(1);
    }
    // graphics.drawImage(menuFrames[13], bottomX + 32, bottomY, 20);
    draw(graphics, 13, bottom_x.wrapping_add(32), bottom_y);
}
