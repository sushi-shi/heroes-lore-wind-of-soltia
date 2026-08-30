//! Transliterated from `java/src/main/java/defpackage/ContinueMenu.java`
//! (original `a.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Load-game slot picker (`ContinueMenu extends Menu`) — the "Continue" screen
//! reached from [`MainMenu`](crate::main_menu) when saved games exist. It lists
//! each saved character with its level and progress %, and starts the highlighted
//! save through [`GameState::new_game`](crate::game_state::new_game). The
//! [`slot_data`](ContinueMenuState::slot_data) blob packs four bytes per slot
//! (`[classId, level, progress%, spare]`);
//! [`cursor_anim_frame`](ContinueMenuState::cursor_anim_frame) plays a short
//! three-frame highlight intro on the selected row.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor and `handleKey` fully — the FIRE-select
//! `GameState.newGame(true, slotData[cursorIndex*4], null)` launch is **real**
//! (`GameState.newGame` is ported), and Back's `parent.onPopupResult(-1, -1)` is
//! made real via the flat model's [`parent_of`](crate::menu::parent_of) scan. The
//! `paint` is **PARTIAL** (the parchment fill + title plate + heading + menu panel +
//! the row separators + the cursor-highlight sprite + its intro-frame advance); the
//! per-slot class name/level/progress/portrait rows and the soft keys cross into
//! unported statics (`AssetCache.heroText`/`classFaces`/`statLabel3`/`cursorArrow`,
//! `BaseCanvas.drawNumberAt`, `FontManager.progressLabel`/`drawChars`(commonText),
//! `FontManager.labelOk`/`labelBack`) and are DEFERRED. `ContinueMenu` is not on
//! the fresh-install path (Continue is skipped without a save), so its FIRE launch
//! is exercised only by an explicit drive.
//!
//! `ContinueMenu`'s fields are all per-INSTANCE (the `Menu` base fields likewise),
//! so it contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `a.<init>:(Lbf;[B)V => [idiv,i2b]` (constructor — `(byte)(slotData.length / 4)`),
//! `a.a:(II)Z => [imul,imul]` (handleKey — the two `slotData[cursorIndex * 4]`
//! reads; the ported subset keeps the `newGame` one, and drops the diagnostic
//! `System.out.println`'s identical read with the no-op println), and
//! `a.a:(…Graphics;II)V => [ishr,iadd,…]` (paint — the ported prefix; the rest is
//! DEFERRED).

use crate::font_manager;
use crate::game::Game;
use crate::game_state;
use crate::main_menu;
use crate::menu::{self, MenuNode};
use j2me_jvm::{ishr, java_div};

/// Java `a` / `ContinueMenu` instance state — the `Menu` (`cb`) base fields plus the
/// picker's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct ContinueMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte[] slotData;` (obf `h`) — 4 bytes per slot: class id, level,
    /// progress %, spare.
    pub slot_data: Vec<i8>,
    /// `private byte cursorAnimFrame;` (obf `c`) — selection-highlight intro frame
    /// (0..2), advanced each paint.
    pub cursor_anim_frame: i8,
}

/// `public ContinueMenu(MainMenu parent, byte[] slotData)`
/// (`a.<init>:(Lbf;[B)V => [idiv,i2b]`): `super(parent, (byte)(slotData.length / 4));
/// slotData = slotData; cursorAnimFrame = 0;`.
pub fn construct(g: &mut Game, slot_data: Vec<i8>) {
    // super(parent, (byte) (slotData.length / 4));   (parent is the MainMenu → present)
    let item_count = java_div(slot_data.len() as i32, 4).expect("slotData.length / 4") as i8;
    g.continue_menu.base = menu::construct(true, item_count);
    // this.slotData = slotData;
    g.continue_menu.slot_data = slot_data;
    // this.cursorAnimFrame = (byte) 0;
    g.continue_menu.cursor_anim_frame = 0;
}

/// `public final boolean handleKey(int action, int keyCode)` (`a.a:(II)Z => [imul,imul]`):
/// vertical no-wrap navigation (resetting the intro animation), FIRE → start the
/// highlighted save via `GameState.newGame(true, slotData[cursorIndex*4], null)`,
/// Back → `parent.onPopupResult(-1, -1)`. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Continue, action, key_code) {
        return true;
    }
    // if (itemCount > 1 && moveCursorVerticalNoWrap(action, keyCode)) { cursorAnimFrame = 0; return true; }
    if (g.continue_menu.base.item_count as i32) > 1
        && menu::move_cursor_vertical(&mut g.continue_menu.base, action, key_code, false)
    {
        g.continue_menu.cursor_anim_frame = 0;
        return true;
    }
    // if (keyCode == 53 || action == 8) {
    if key_code == 53 || action == 8 {
        // System.out.println("continue game with " + (int) slotData[cursorIndex * 4]);
        //   — diagnostic no-op; its `slotData[cursorIndex * 4]` read (one `imul`) is
        //     dropped with the println (the same index newGame reads immediately below).
        // GameState.newGame(true, this.slotData[cursorIndex * 4], (boolean[]) null);
        let idx = (g.continue_menu.base.cursor_index as i32).wrapping_mul(4); // imul
        let class_id = g.continue_menu.slot_data[idx as usize];
        // (boolean[]) null → resume == true, so newGame never dereferences `traits`.
        game_state::new_game(g, true, class_id, &[]);
        // return true;
        return true;
    }
    // if (keyCode != -8) return true;
    if key_code != -8 {
        return true;
    }
    // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1); return true;
    let parent =
        menu::parent_of(g, MenuNode::Continue).expect("NullPointerException: ContinueMenu.parent");
    menu::on_popup_result(g, parent, -1, -1);
    true
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`a.a:(…Graphics;II)V`): **PARTIAL** — the parchment fill, the shared title
/// plate + "Load Game" heading + menu panel, the five row separators, and the
/// selection-highlight sprite (with its three-frame intro advance). The per-slot
/// class-name/level/progress/portrait rows and the soft keys are DEFERRED (unported
/// statics).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    let cursor_index = g.continue_menu.base.cursor_index;
    let cursor_anim_frame = g.continue_menu.cursor_anim_frame;
    {
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
        // FontManager.drawMenuItem(graphics, 3, BaseCanvas.width >> 1, y + 5);
        font_manager::draw_menu_item(
            font_manager,
            &mut graphics,
            base_canvas,
            3,
            ishr(base_canvas.width, 1),
            y.wrapping_add(5),
        );
        // MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        main_menu::draw_menu_panel(asset_cache, &mut graphics, x, y.wrapping_add(24), 3);
        // int baseY = y + 5; int baseX = x + 10;
        let base_y = y.wrapping_add(5);
        let base_x = x.wrapping_add(10);
        // for (int row = 0; row < 5; row++) drawImage(menuFrames[19], baseX+13, baseY+49+(row*16), 20);
        let mf = asset_cache
            .menu_frames
            .as_ref()
            .expect("AssetCache.menuFrames null");
        let mut row: i32 = 0;
        while row < 5 {
            graphics
                .draw_image(
                    &mf[19],
                    base_x.wrapping_add(13),
                    base_y.wrapping_add(49).wrapping_add(row.wrapping_mul(16)),
                    20,
                )
                .expect("drawImage(menuFrames[19])");
            row = row.wrapping_add(1);
        }
        // switch (cursorAnimFrame) { case 0: menuFrames[14]; case 1: menuFrames[16]; default: menuFrames[18]; }
        //   at (baseX+5, baseY+31+(cursorIndex*16)).
        let sprite_idx: usize = match cursor_anim_frame {
            0 => 14,
            1 => 16,
            _ => 18,
        };
        graphics
            .draw_image(
                &mf[sprite_idx],
                base_x.wrapping_add(5),
                base_y
                    .wrapping_add(31)
                    .wrapping_add((cursor_index as i32).wrapping_mul(16)),
                20,
            )
            .expect("drawImage(menuFrames cursor)");
    }
    // if (this.cursorAnimFrame < 2) { ((Menu) this).needsRepaint = true; cursorAnimFrame = (byte)(cursorAnimFrame + 1); }
    if (g.continue_menu.cursor_anim_frame as i32) < 2 {
        g.continue_menu.base.needs_repaint = true;
        g.continue_menu.cursor_anim_frame =
            (g.continue_menu.cursor_anim_frame as i32).wrapping_add(1) as i8;
    }
    // (DEFERRED — the per-slot rows + the final stat/portrait/softkeys block cross into
    //  unported statics. Faithful full form: the `while` loop draws each saved hero's
    //  class name (AssetCache.heroText) in white/grey; when itemIndex >= itemCount it
    //  draws statLabel3 + BaseCanvas.drawNumberAt(level) + the progress% (FontManager.
    //  progressLabel), the class portrait (AssetCache.classFaces), and
    //  FontManager.drawSoftKeys(labelOk, labelBack).)
}
