//! Transliterated from `java/src/main/java/defpackage/GameScreen.java`
//! (original `as.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The in-game `BaseCanvas`: routes each frame by `GameState.screen` (world, menus,
//! event scenes, game-over, credits, …), owns the HUD, and dispatches gameplay/menu
//! keys.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports **only** the fresh-install main-menu path (`GameState.screen
//! == 9`): the constructor's screen geometry, and the `case 9` branches of `paint`
//! (`MainMenu.invalidateDown` + `MainMenu.draw`) and `keyPressed`
//! (`MainMenu.handleKey`). Every other `screen` case — the world view + HUD
//! (`drawHud`, `drawHudFrame`), the character/shop/refine/blacksmith menus, the
//! minimap, game-over, credits, endings, the paused overlay — and the whole HUD /
//! ending / staff-roll machinery are DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `as.<init>:()V` (the
//! geometry `idiv`s — `width/2`, `worldHeight/2`, `(width-74)/6`),
//! `as.a:(…Graphics;)V => []` (paint — no arithmetic in the ported dispatch),
//! `as.keyPressed:(I)V => []` (keyPressed).

use crate::game::Game;
use crate::game_loop;
use crate::game_state;
use crate::main_menu;
use crate::menu;
use j2me_jvm::java_div;

/// Java `as` / `GameScreen` state — the class `static` geometry (obf `a`/`b`/`c`/`d`
/// = width/worldHeight/centerX/centerY, `n`/`o`/`p` = hudSlots/barWidth/expWidth,
/// `e` = fxTimer; see `java/reconstruction/ownership.tsv`) plus the constructor's
/// instance HUD flags. ANTI-BOG: none of these are read by the `case 9` main-menu
/// render — they are modelled for a faithful constructor and the deferred cases.
#[derive(Debug, Default)]
pub struct GameScreenState {
    /// `public static int width;` (obf `as.a`) — playfield width.
    pub width: i32,
    /// `public static int worldHeight;` (obf `as.b`) — screen height minus HUD.
    pub world_height: i32,
    /// `public static int centerX;` (obf `as.c`).
    pub center_x: i32,
    /// `public static int centerY;` (obf `as.d`).
    pub center_y: i32,
    /// `private static int hudSlots;` (obf `as.n`).
    pub hud_slots: i32,
    /// `private static int barWidth;` (obf `as.o`).
    pub bar_width: i32,
    /// `private static int expWidth;` (obf `as.p`).
    pub exp_width: i32,
    /// `public static int fxTimer;` (obf `as.e`) — cinematic fade/timer.
    pub fx_timer: i32,
    /// `private boolean worldVisible;` (instance).
    pub world_visible: bool,
    /// `private boolean redrawAll;` (instance).
    pub redraw_all: bool,
    /// `private int lowHpBlink;` (instance).
    pub low_hp_blink: i32,
    /// `private int messageTtl;` (instance).
    pub message_ttl: i32,
    /// `private int targetTtl;` (instance).
    pub target_ttl: i32,
}

/// `public GameScreen()` (`as.<init>:()V`): computes the screen geometry from the
/// `BaseCanvas` size and clears the transient HUD state. `System.out.println` is
/// dropped; the discarded `message = "".toCharArray()` field initializer is a
/// no-op here (not read on the main-menu path).
pub fn construct(g: &mut Game) {
    // width = BaseCanvas.width;
    g.game_screen.width = g.base_canvas.width;
    // worldHeight = BaseCanvas.height - 21;
    g.game_screen.world_height = g.base_canvas.height.wrapping_sub(21);
    // centerX = (width / 2) - 8;
    g.game_screen.center_x = java_div(g.game_screen.width, 2)
        .expect("width / 2")
        .wrapping_sub(8);
    // centerY = worldHeight / 2;
    g.game_screen.center_y = java_div(g.game_screen.world_height, 2).expect("worldHeight / 2");
    // hudSlots = (BaseCanvas.width - 74) / 6;
    g.game_screen.hud_slots =
        java_div(g.base_canvas.width.wrapping_sub(74), 6).expect("(width - 74) / 6");
    // barWidth = BaseCanvas.width - 67;
    g.game_screen.bar_width = g.base_canvas.width.wrapping_sub(67);
    // expWidth = BaseCanvas.width - 6;
    g.game_screen.exp_width = g.base_canvas.width.wrapping_sub(6);
    // this.redrawAll = true;
    g.game_screen.redraw_all = true;
    // this.lowHpBlink = 0; this.messageTtl = 0; this.targetTtl = 0;
    g.game_screen.low_hp_blink = 0;
    g.game_screen.message_ttl = 0;
    g.game_screen.target_ttl = 0;
}

/// `public final void paint(Graphics graphics)` (`as.a:(…Graphics;)V`): the ported
/// `case 9` (main menu) dispatch. `synchronized (GameLoop.lock)` is a no-op in the
/// single-threaded transliteration.
pub fn paint(g: &mut Game) {
    // GameState.processStateRequest();
    game_state::process_state_request(g);
    // switch (GameState.screen)  — a 15-case switch; only `case 9` (main menu) is
    // ported, the rest are DEFERRED to the default arm (hence single_match).
    #[allow(clippy::single_match)]
    match g.game_state.screen {
        9 => {
            // MainMenu.instance().invalidateDown();
            debug_assert!(
                main_menu::instance(g),
                "MainMenu singleton null at screen 9"
            );
            menu::invalidate_down(&mut g.main_menu.base);
            // MainMenu.instance().draw(graphics);
            main_menu::draw(g);
        }
        // (DEFERRED: cases 1,2,4,5,6,7,8,10,11,12,13,14,15 — loading overlay, world +
        // HUD, event scenes, character/shop/refine/blacksmith menus, game-over,
        // minimap, credits, endings, paused overlay. Not reached by the boot→menu route.)
        _ => {}
    }
    // graphics.setColor(16777215);  — a trailing pen-colour set with no draw (the
    //   MainMenu paint's Graphics already flushed); a dead store, modelled as a no-op.
    // GameLoop.instance.throttle();
    game_loop::throttle(g);
}

/// `public final void keyPressed(int keyCode)` (`as.keyPressed:(I)V`): the ported
/// `case 9` (main menu) key dispatch. The soft-key remaps (`-6 → 53`, `-7 → -8`)
/// and the `getGameAction` decode are preserved; `synchronized` is a no-op.
pub fn key_pressed(g: &mut Game, key_code: i32) {
    let mut key_code = key_code;
    // if (keyCode == -6) keyCode = 53;
    if key_code == -6 {
        key_code = 53;
    }
    // if (keyCode == -7) keyCode = -8;
    if key_code == -7 {
        key_code = -8;
    }
    // if (GameLoop.instance == null || GameLoop.instance.stopped) return;
    if !g.game_loop.instance || g.game_loop.stopped {
        return;
    }
    // ((BaseCanvas) this).keyDown = true;
    g.base_canvas.key_down = true;
    // int gameAction = getGameAction(keyCode);
    let game_action = j2me_me::Canvas::common_game_action(key_code);
    // switch (GameState.screen)  — only `case 9` (main menu) is ported; the rest are
    // DEFERRED to the default arm (hence single_match).
    #[allow(clippy::single_match)]
    match g.game_state.screen {
        9 => {
            // MainMenu.instance().handleKey(gameAction, keyCode);
            main_menu::handle_key(g, game_action, key_code);
        }
        // (DEFERRED: the play / event / character / shop / refine / blacksmith /
        // minimap / credits / clear / paused key handling.)
        _ => {}
    }
}
