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
//! This increment ports the constructor's screen geometry, the `case 9`
//! (main-menu) branches of `paint`/`keyPressed`, and the world-render entry —
//! `paint` `case 1` (loading overlay) and `case 2` (`GameMap.paint` → the tiles).
//! Inside `case 2` the pre-render `GameState.update()` (world sim + the unported
//! Guardian), the follow-camera easing (`scrollCamera`) and `drawHud` (which derefs
//! the Guardian) are DEFERRED; so are every other `screen` case (character/shop/
//! refine/blacksmith menus, minimap, game-over, credits, endings, paused overlay)
//! and the whole HUD / ending / staff-roll machinery.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `as.<init>:()V` (the
//! geometry `idiv`s — `width/2`, `worldHeight/2`, `(width-74)/6`),
//! `as.a:(…Graphics;)V => []` (paint — no arithmetic in the ported dispatch),
//! `as.keyPressed:(I)V => []` (keyPressed).

use crate::asset_cache::AssetCacheState;
use crate::asset_loader;
use crate::game::Game;
use crate::game_loop;
use crate::game_map;
use crate::game_state;
use crate::item_bag;
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

/// `public final void markRedraw()` (`as`): requests a full HUD redraw next frame.
/// Called by `GameState.warpMap` and `Hero.recomputeStats`.
pub fn mark_redraw(g: &mut Game) {
    // this.redrawAll = true;
    g.game_screen.redraw_all = true;
}

/// `public final void paint(Graphics graphics)` (`as.a:(…Graphics;)V`): the ported
/// `case 1`/`case 2`/`case 9` dispatch. `synchronized (GameLoop.lock)` is a no-op in
/// the single-threaded transliteration.
pub fn paint(g: &mut Game) {
    // GameState.processStateRequest();
    game_state::process_state_request(g);
    // switch (GameState.screen)  — a 15-case switch; this slice ports `case 1`
    // (loading overlay), `case 2` (world tiles) and `case 9` (main menu); the rest
    // are DEFERRED to the default arm.
    match g.game_state.screen {
        // case 1: AssetLoader.drawLoadingOverlay(graphics);
        1 => {
            asset_loader::draw_loading_overlay(g);
        }
        // case 2: the world view. Run the world simulation (`GameState.update` — hero
        // FSM + world) with the follow-camera easing, then render the tiles. The hero
        // sprite / HUD draw layered ON TOP of the tiles is owned by the render lane and
        // stays DEFERRED here.
        2 => {
            // if (cameraFollow) { centerCamera(); update(); } else { update(); centerCamera(); }
            if g.game_loop.camera_follow {
                game_state::center_camera(g);
                game_state::update(g);
            } else {
                game_state::update(g);
                game_state::center_camera(g);
            }
            // if (GameState.screen == 2) {
            if g.game_state.screen == 2 {
                // if (!map.lockedCamera && cameraFollow) scrollCamera(true, true);
                let locked = g
                    .game_state
                    .map
                    .as_ref()
                    .expect("GameState.map null at screen 2")
                    .locked_camera;
                if !locked && g.game_loop.camera_follow {
                    game_state::scroll_camera(g, true, true);
                }
                // map.paint(graphics);
                game_map::paint(g);
                // drawHud(graphics);  — DEFERRED (derefs the unported Guardian via
                //   getActiveGuardian; the whole HUD/target/message machinery — render lane).
                // (DEFERRED: the Debug.fullVersion level-8 requestState(13,1) escape.)
            }
        }
        9 => {
            // MainMenu.instance().invalidateDown();
            debug_assert!(
                main_menu::instance(g),
                "MainMenu singleton null at screen 9"
            );
            menu::invalidate_down(g, menu::MenuNode::Main);
            // MainMenu.instance().draw(graphics);
            main_menu::draw(g);
        }
        // (DEFERRED: cases 4,5,6,7,8,10,11,12,13,14,15 — event scenes,
        // character/shop/refine/blacksmith menus, game-over, minimap, credits,
        // endings, paused overlay.)
        _ => {}
    }
    // graphics.setColor(16777215);  — a trailing pen-colour set with no draw (the
    //   MainMenu paint's Graphics already flushed); a dead store, modelled as a no-op.
    // GameLoop.instance.throttle();
    game_loop::throttle(g);
}

/// `public static final void drawFrame(Graphics graphics, byte[] frames, byte
/// frameIndex, int x, int y)` (`as.a:(…Graphics;[BBII)V => [imul, iadd×6]`): draws
/// one animation frame `frameIndex` of the per-frame draw script `frames` at
/// (`x`,`y`), anchored `20` (TOP|LEFT).
///
/// `frames` is one `AssetCache.heroFrames[…]` element — a `byte[]` or Java null; the
/// null (unloaded-layer) case is the `Option::None` early return. Layout: `frames[0]`
/// = frame count, then 4 bytes per frame `[dx, dy, spriteBankByte, imageIndex]`. The
/// bank byte indexes [`AssetCacheState::sprite_banks`]; a `-1` image index or an
/// absent atlas frame draws nothing. `spriteBanks[bank] == null` with a live image
/// index is an unguarded NPE in Java — reproduced as an `.expect`.
pub fn draw_frame(
    graphics: &mut j2me_me::Graphics,
    asset_cache: &AssetCacheState,
    frames: Option<&[i8]>,
    frame_index: i8,
    x: i32,
    y: i32,
) {
    // if (frames == null || frameIndex >= frames[0]) return;
    let frames = match frames {
        None => return,
        Some(f) => f,
    };
    if frame_index as i32 >= frames[0] as i32 {
        return;
    }
    // int base = 1 + (frameIndex * 4);
    let base = 1i32.wrapping_add((frame_index as i32).wrapping_mul(4));
    // Image[] images = spriteBanks[frames[base + 2]];  byte imgIdx = frames[base + 3];
    let bank = frames[base.wrapping_add(2) as usize] as i32;
    let img_idx = frames[base.wrapping_add(3) as usize];
    // if (imgIdx == -1 || images[imgIdx] == null) return;  (images==null → Java NPE)
    if img_idx == -1 {
        return;
    }
    let images = asset_cache.sprite_banks[bank as usize]
        .as_ref()
        .expect("NullPointerException: spriteBanks[bank] null in drawFrame");
    let image = match &images[img_idx as usize] {
        None => return,
        Some(im) => im,
    };
    // graphics.drawImage(images[imgIdx], x + frames[base], y + frames[base + 1], 20);
    graphics
        .draw_image(
            image,
            x.wrapping_add(frames[base as usize] as i32),
            y.wrapping_add(frames[base.wrapping_add(1) as usize] as i32),
            20,
        )
        .expect("drawImage(spriteBank frame)");
}

/// `public static final void drawFrameGroup(Graphics graphics, byte[] frames, byte
/// groupIndex, int x, int y)` (`as.b:(…Graphics;[BBII)V`): draws every part of
/// animation group `groupIndex` of `frames` at (`x`,`y`). Used only for the aura
/// layer (7), which is gated on `map.combatEnabled` (false on the class-6 start map),
/// so it is not exercised in this milestone but is ported faithfully.
///
/// The script packs each group as `[partCount][partCount × (dx, dy, bank, imageIndex)]`;
/// `cursor` walks past `groupIndex` earlier groups, then each part is blitted like
/// [`draw_frame`].
pub fn draw_frame_group(
    graphics: &mut j2me_me::Graphics,
    asset_cache: &AssetCacheState,
    frames: Option<&[i8]>,
    group_index: i8,
    x: i32,
    y: i32,
) {
    // if (frames == null || groupIndex >= frames[0]) return;
    let frames = match frames {
        None => return,
        Some(f) => f,
    };
    if group_index as i32 >= frames[0] as i32 {
        return;
    }
    // int cursor = 1; for (group = 0; group < groupIndex; group++) cursor += 1 + (frames[cursor]*4);
    let mut cursor: i32 = 1;
    let mut group: i32 = 0;
    while group < group_index as i32 {
        cursor = cursor
            .wrapping_add(1)
            .wrapping_add((frames[cursor as usize] as i32).wrapping_mul(4));
        group = group.wrapping_add(1);
    }
    // int countPos = cursor; int part = cursor + 1; byte partCount = frames[countPos];
    let count_pos = cursor;
    let mut part = cursor.wrapping_add(1);
    let part_count = frames[count_pos as usize];
    // for (int p = 0; p < partCount; p++) { ... part += 4; }
    let mut p: i32 = 0;
    while p < part_count as i32 {
        let bank = frames[part.wrapping_add(2) as usize] as i32;
        let img_idx = frames[part.wrapping_add(3) as usize];
        // if (imgIdx != -1 && images[imgIdx] != null) drawImage(...);
        if img_idx != -1 {
            let images = asset_cache.sprite_banks[bank as usize]
                .as_ref()
                .expect("NullPointerException: spriteBanks[bank] null in drawFrameGroup");
            if let Some(image) = &images[img_idx as usize] {
                graphics
                    .draw_image(
                        image,
                        x.wrapping_add(frames[part as usize] as i32),
                        y.wrapping_add(frames[part.wrapping_add(1) as usize] as i32),
                        20,
                    )
                    .expect("drawImage(spriteBank group part)");
            }
        }
        part = part.wrapping_add(4);
        p = p.wrapping_add(1);
    }
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
    // switch (GameState.screen)  — `case 2` (world/play) and `case 9` (main menu) are
    // ported; the rest are DEFERRED to the default arm.
    match g.game_state.screen {
        // case 2: handlePlayKey(gameAction, keyCode);
        2 => {
            handle_play_key(g, game_action, key_code);
        }
        9 => {
            // MainMenu.instance().handleKey(gameAction, keyCode);
            main_menu::handle_key(g, game_action, key_code);
        }
        // (DEFERRED: the event / character / shop / refine / blacksmith /
        // minimap / credits / clear / paused key handling.)
        _ => {}
    }
}

/// `private final void handlePlayKey(int gameAction, int keyCode)`
/// (`as.handlePlayKey`, GameScreen.java:303): the world (screen 2) key dispatch.
///
/// **This slice ports the MOVEMENT keys** — the numeric d-pad (`2`/`4`/`6`/`8`),
/// their `gameAction` fallbacks (UP/LEFT/RIGHT/DOWN), the `#` quick-type cycle, the
/// `0` map-menu request, and the back-key character-menu request — each driving the
/// ported [`game_state`] entry points. The combat / guardian / pickup / quick-item
/// keys (`1`/`3`/`5`/`7`/`9` and the FIRE fallback) reach `castGuardianSkill` /
/// `tryPickup` / `requestHeroAttack` / `useQuickItem` / `EventScript`, which are not
/// on the movement path and are DEFERRED (clearly marked).
fn handle_play_key(g: &mut Game, game_action: i32, key_code: i32) {
    // switch (keyCode)
    match key_code {
        // case -8: if (((Battler) hero()).state == 1) requestState(13);
        -8 => {
            if game_state::hero_state(g) == 1 {
                game_state::request_state(g, 13);
            }
        }
        // case 35 (#): hero().bag.cycleQuickType(); markRedraw();
        35 => {
            let id = g
                .game_state
                .hero
                .expect("GameState.hero null in handlePlayKey");
            let bag = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").bag;
            item_bag::cycle_quick_type(bag);
            mark_redraw(g);
        }
        // case 48 (0): if (state == 1 && map.tilesetId <= 14) requestState(2, 11, 3);
        48 => {
            if game_state::hero_state(g) == 1 {
                let tileset_id = g
                    .game_state
                    .map
                    .as_ref()
                    .expect("GameState.map null in handlePlayKey")
                    .tileset_id;
                if tileset_id <= 14 {
                    game_state::request_state_a0_a1(g, 2, 11, 3);
                }
            }
        }
        // case 49 (1): hero().castGuardianSkill(true);  — DEFERRED (guardian skill).
        49 => {}
        // case 50 (2): walkHero((byte) 1);  [up]
        50 => game_state::walk_hero(g, 1),
        // case 51 (3): hero().castGuardianSkill(false);  — DEFERRED (guardian skill).
        51 => {}
        // case 52 (4): walkHero((byte) 3);  [left]
        52 => game_state::walk_hero(g, 3),
        // case 53 (5): if (!tryPickup() && !checkActionTrigger()) requestHeroAttack(false);
        //   — DEFERRED (pickup / EventScript / combat).
        53 => {}
        // case 54 (6): walkHero((byte) 4);  [right]
        54 => game_state::walk_hero(g, 4),
        // case 55 (7): requestHeroAttack(true);  — DEFERRED (combat).
        55 => {}
        // case 56 (8): walkHero((byte) 2);  [down]
        56 => game_state::walk_hero(g, 2),
        // case 57 (9): hero().useQuickItem();  — DEFERRED (quick item).
        57 => {}
        // default: switch (gameAction)
        _ => match game_action {
            // case 1 (UP): walkHero((byte) 1);
            1 => game_state::walk_hero(g, 1),
            // case 2 (LEFT): walkHero((byte) 3);
            2 => game_state::walk_hero(g, 3),
            // case 5 (RIGHT): walkHero((byte) 4);
            5 => game_state::walk_hero(g, 4),
            // case 6 (DOWN): walkHero((byte) 2);
            6 => game_state::walk_hero(g, 2),
            // case 8 (FIRE): tryPickup / checkActionTrigger / requestHeroAttack
            //   — DEFERRED (pickup / EventScript / combat).
            8 => {}
            _ => {}
        },
    }
}
