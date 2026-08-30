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
//! (main-menu) branches of `paint`/`keyPressed`, the world-render entry —
//! `paint` `case 1` (loading overlay) and `case 2` (`GameMap.paint` → the tiles) —
//! and the two in-game menu screens wired this lane: `case 5` (CharacterMenu) and
//! `case 6` (ShopMenu), each dispatching `paint`/`keyPressed` to the ported menu.
//! This slice adds the remaining PORTABLE overlays: `paint` `case 10` (game-over →
//! [`draw_game_over`] + the `fxTimer` fade-out), `case 14` (stage-cleared →
//! [`draw_clear_menu`]) and `case 15` (paused), their `keyPressed` arms (14/15),
//! [`activate`], the HUD dirty flags ([`mark_hp_dirty`]/[`mark_mp_dirty`]/
//! [`mark_exp_dirty`]/[`reset_hud_state`]/[`set_target`]), and the HUD itself —
//! [`draw_hud`] (the HP/MP/exp bars) + [`draw_hud_frame`].
//!
//! DEFERRED image banks (`AssetCache` models none of these — `load_in_game_ui`
//! DEFERS the `/img/uifrm` decode): `hudFrame` (so [`draw_hud_frame`] is a documented
//! stub), `statPointAlert`, `itemIcons`, `numberFont0` (so every `drawNumberAt` in
//! [`draw_hud`]), `guardianSkillIcons` / `skillChargeFill`, and `commonText` (so the
//! game-over caption and the four clear-menu labels). Also DEFERRED: `getActiveGuardian`
//! (Guardian is unported — the skill-charge icons, the cast banner, the target-panel
//! `castState` gate), `Hero.drawSummonPose` (game-over pose), the target-monster panel
//! and floating-message box (unported `Enemy` state / `Menu.drawSelectableBox` /
//! `Menu.drawTextField`), `FontManager.pausedLabel` (the `loadLabels` subset the
//! font hub leaves unfilled), and `AssetLoader.loadMainMenu` (game-over → screen 1).
//! Inside `case 2` the pre-render `GameState.update()` (world sim + the unported
//! Guardian), the follow-camera easing (`scrollCamera`) and `drawHud` stay DEFERRED
//! (the existing screen is left unchanged — [`draw_hud`] is ported and driven directly
//! by the `hud_screens` gate, not yet wired into the case-2 render);
//! `drawWorldBehindMenu`'s `drawHud` is likewise DEFERRED (and its body is unreached
//! while `worldVisible` is false unless [`activate`] set it). Still DEFERRED cases:
//! `4` (event scenes — EventScript), `7`/`8` (refine/blacksmith menus), `11` (minimap
//! — `GameMap.paintMinimap` unported), `12`/`13` (credits/ending — ScrollCaption +
//! `commonText` + `endingText`).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `as.<init>:()V` (the
//! geometry `idiv`s — `width/2`, `worldHeight/2`, `(width-74)/6`),
//! `as.a:(…Graphics;)V => []` (paint — no arithmetic in the ported dispatch),
//! `as.keyPressed:(I)V => []` (keyPressed).

use crate::asset_cache::AssetCacheState;
use crate::asset_loader;
use crate::audio_manager;
use crate::character_menu;
use crate::entity::EntityId;
use crate::font_manager;
use crate::game::Game;
use crate::game_loop;
use crate::game_map;
use crate::game_state;
use crate::item_bag;
use crate::main_menu;
use crate::menu;
use crate::shop_menu;
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
    /// `private boolean hpDirty;` (instance) — HP bar needs redraw.
    pub hp_dirty: bool,
    /// `private boolean mpDirty;` (instance) — MP bar needs redraw.
    pub mp_dirty: bool,
    /// `private boolean expDirty;` (instance) — exp bar needs redraw.
    pub exp_dirty: bool,
    /// `private boolean messageReplaced;` (instance) — set when a new message replaced
    /// one still showing (skip a frame). Written by [`reset_hud_state`] / the DEFERRED
    /// `showMessage`; read only by the DEFERRED drawHud message box.
    pub message_replaced: bool,
    /// `private Enemy targetMonster;` (instance, obf `as.a` : `Enemy`) — the monster
    /// currently shown in the (DEFERRED) target-monster panel. Modelled as an arena
    /// handle ([`EntityId`], the enemy's slot); Java reference identity becomes slot
    /// equality. Written by [`set_target`] / [`reset_hud_state`]; read only by the
    /// DEFERRED target-panel block of [`draw_hud`].
    pub target_monster: Option<EntityId>,
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

/// `public final void activate()` (`as`): activates the world view and forces a
/// redraw. Sets `worldVisible`, which gates [`draw_world_behind_menu`]'s body.
pub fn activate(g: &mut Game) {
    // this.worldVisible = true;
    g.game_screen.world_visible = true;
    // markRedraw();
    mark_redraw(g);
}

/// `public final void markHpDirty()` (`as`): marks the HP bar dirty.
pub fn mark_hp_dirty(g: &mut Game) {
    // this.hpDirty = true;
    g.game_screen.hp_dirty = true;
}

/// `public final void markMpDirty()` (`as`): marks the MP bar dirty.
pub fn mark_mp_dirty(g: &mut Game) {
    // this.mpDirty = true;
    g.game_screen.mp_dirty = true;
}

/// `public final void markExpDirty()` (`as`): marks the exp bar dirty.
pub fn mark_exp_dirty(g: &mut Game) {
    // this.expDirty = true;
    g.game_screen.exp_dirty = true;
}

/// `public final void resetHudState()` (`as.f`): clears the transient HUD state (the
/// floating message and the target-monster panel).
pub fn reset_hud_state(g: &mut Game) {
    // this.messageTtl = 0;
    g.game_screen.message_ttl = 0;
    // this.messageReplaced = false;
    g.game_screen.message_replaced = false;
    // this.targetTtl = 0;
    g.game_screen.target_ttl = 0;
    // this.targetMonster = null;
    g.game_screen.target_monster = None;
}

/// `public final void setTarget(Enemy enemy, boolean keepCurrent)` (`as`): sets the
/// target-monster panel to `enemy` (`keepCurrent` keeps a previously-set target).
/// `enemy` is the arena handle of the `Enemy` (see [`GameScreenState::target_monster`]);
/// Java's `targetMonster != enemy` reference test becomes slot equality. Only reached
/// from the (unported) combat/targeting code; the panel it feeds is DEFERRED.
pub fn set_target(g: &mut Game, enemy: EntityId, keep_current: bool) {
    // this.targetTtl = 24;
    g.game_screen.target_ttl = 24;
    // if ((!keepCurrent || this.targetMonster == null) && this.targetMonster != enemy) {
    if (!keep_current || g.game_screen.target_monster.is_none())
        && g.game_screen.target_monster != Some(enemy)
    {
        // this.targetMonster = enemy;
        g.game_screen.target_monster = Some(enemy);
    }
}

/// `public final void paint(Graphics graphics)` (`as.a:(…Graphics;)V`): the ported
/// `case 1`/`case 2`/`case 5`/`case 6`/`case 9` dispatch. `synchronized (GameLoop.lock)`
/// is a no-op in the single-threaded transliteration.
pub fn paint(g: &mut Game) {
    // GameState.processStateRequest();
    game_state::process_state_request(g);
    // switch (GameState.screen)  — a 15-case switch; this slice ports `case 1`
    // (loading overlay), `case 2` (world tiles), `case 5` (CharacterMenu), `case 6`
    // (ShopMenu) and `case 9` (main menu); the rest are DEFERRED to the default arm.
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
        // case 5: the character menu (six-tab panel). Invalidate up+down, then draw the
        // whole tree at its centred origin. The three `CharacterMenu.instance()` calls
        // are the singleton fetch — a no-op once open (case 13 created it).
        5 => {
            // CharacterMenu.instance().invalidateUp();
            character_menu::instance(g);
            menu::invalidate_up(g, menu::MenuNode::Character);
            // CharacterMenu.instance().invalidateDown();
            character_menu::instance(g);
            menu::invalidate_down(g, menu::MenuNode::Character);
            // CharacterMenu.instance().draw(graphics);
            character_menu::instance(g);
            character_menu::draw(g);
        }
        // case 6: the shop menu. Draw the world behind it (a no-op while worldVisible is
        // false — GameScreen.activate is unported this slice), then the shop panel.
        6 => {
            // drawWorldBehindMenu(graphics, ShopMenu.instance());
            shop_menu::instance(g);
            draw_world_behind_menu(g, Some(menu::MenuNode::ShopMenu));
            // ShopMenu.instance().draw(graphics);
            shop_menu::instance(g);
            shop_menu::draw(g);
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
        // case 10: the game-over screen — draw it, tick the fade-out, and on `fxTimer`
        // reaching 0 fall back to the main-menu load. `AssetLoader.loadMainMenu()` is
        // the one unported step (phase 5), so it is DEFERRED; `setScreen`/`unloadClip`
        // are ported, keeping the observable screen/clip transition faithful.
        10 => {
            // drawGameOver(graphics);
            draw_game_over(g);
            // if (fxTimer > 0) fxTimer--;
            if g.game_screen.fx_timer > 0 {
                g.game_screen.fx_timer = g.game_screen.fx_timer.wrapping_sub(1);
            }
            // if (fxTimer == 0) { setScreen(1); loadMainMenu(); unloadClip((byte) 12); }
            if g.game_screen.fx_timer == 0 {
                game_state::set_screen(g, 1);
                // AssetLoader.loadMainMenu();  — DEFERRED (loadMainMenu, phase 5, unported).
                audio_manager::unload_clip(g, 12);
            }
        }
        // case 14: the "stage cleared" menu box.
        14 => {
            // drawClearMenu(graphics);
            draw_clear_menu(g);
        }
        // case 15: the paused overlay (inline in Java — no helper method). Clears to
        // black, then draws the "OK" soft key. The centred `pausedText` draw is
        // DEFERRED: `FontManager.pausedLabel` is one of the `loadLabels(3902..3950)`
        // entries the font hub leaves unfilled (the boot fills only the title subset).
        15 => {
            // char[] pausedText = FontManager.pausedLabel;  — DEFERRED (unfilled label).
            // FontManager.labelOk — filled by the boot's label subset; snapshot it.
            let label_ok = g.font_manager.label_ok.clone();
            let Game {
                screen,
                base_canvas,
                font_manager,
                ..
            } = &mut *g;
            let target = screen.as_mut().expect("framebuffer");
            let mut graphics = j2me_me::Graphics::new(target);
            // FontManager.clearScreen(graphics);
            font_manager::clear_screen(&mut graphics, base_canvas);
            // graphics.setColor(16777215);
            graphics.set_color(16777215);
            // FontManager.drawCharsCentered(graphics, halfW, halfH - 15, pausedText, 1);
            //   — DEFERRED (pausedText / FontManager.pausedLabel unfilled).
            // FontManager.drawSoftKeys(graphics, FontManager.labelOk, (char[]) null);
            font_manager::draw_soft_keys(
                font_manager,
                &mut graphics,
                base_canvas,
                label_ok.as_deref(),
                None,
            );
        }
        // (DEFERRED: cases 4,7,8,11,12,13 — event scenes, refine/blacksmith menus,
        // minimap (GameMap.paintMinimap unported), credits, endings.)
        _ => {}
    }
    // graphics.setColor(16777215);  — a trailing pen-colour set with no draw (the
    //   MainMenu paint's Graphics already flushed); a dead store, modelled as a no-op.
    // GameLoop.instance.throttle();
    game_loop::throttle(g);
}

/// `private final void drawWorldBehindMenu(Graphics graphics, Menu menu)` (`as`):
/// when the world layer is active, repaints the world tiles + HUD behind an open
/// world-menu (shop/minimap) and marks the passed `menu` dirty. `worldVisible` is set
/// only by the unported `GameScreen.activate`, so it is `false` in this slice and the
/// body is skipped; ported faithfully so the shop's screen-6 dispatch is exact. The
/// `drawHud(graphics)` call derefs the unported Guardian (`getActiveGuardian`) and
/// stays DEFERRED inside the (unreached) guarded body. `menu` may be Java `null`
/// (screen-11 minimap passes null) → [`None`].
fn draw_world_behind_menu(g: &mut Game, node: Option<menu::MenuNode>) {
    // if (this.worldVisible) {
    if g.game_screen.world_visible {
        // GameState.map.paint(graphics);
        game_map::paint(g);
        // drawHud(graphics);  — DEFERRED (derefs the unported Guardian via getActiveGuardian).
        // if (menu != null) menu.invalidateDown();
        if let Some(n) = node {
            menu::invalidate_down(g, n);
        }
    }
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
/// `case 2` (world), `case 5` (CharacterMenu), `case 6` (ShopMenu) and `case 9`
/// (main menu) key dispatch. The soft-key remaps (`-6 → 53`, `-7 → -8`) and the
/// `getGameAction` decode are preserved; `synchronized` is a no-op.
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
    // switch (GameState.screen)  — `case 2` (world/play), `case 5` (CharacterMenu),
    // `case 6` (ShopMenu) and `case 9` (main menu) are ported; the rest are DEFERRED
    // to the default arm.
    match g.game_state.screen {
        // case 2: handlePlayKey(gameAction, keyCode);
        2 => {
            handle_play_key(g, game_action, key_code);
        }
        // case 5: CharacterMenu.instance().handleKey(gameAction, keyCode);
        5 => {
            character_menu::instance(g);
            character_menu::handle_key(g, game_action, key_code);
        }
        // case 6: ShopMenu.instance().handleKey(gameAction, keyCode);
        6 => {
            shop_menu::instance(g);
            shop_menu::handle_key(g, game_action, key_code);
        }
        9 => {
            // MainMenu.instance().handleKey(gameAction, keyCode);
            main_menu::handle_key(g, game_action, key_code);
        }
        // case 14: if (gameAction == 8 || keyCode == 53) requestState((byte) 21, (byte) 2);
        14 => {
            if game_action == 8 || key_code == 53 {
                game_state::request_state_a0(g, 21, 2);
            }
        }
        // case 15: if (keyCode == 53) setScreen(1);
        //   (kept as a `case`-body `if`, matching this file's other `case X: if (…)` arms
        //   — the Java structure, not a merged match guard.)
        #[allow(clippy::collapsible_match)]
        15 => {
            if key_code == 53 {
                game_state::set_screen(g, 1);
            }
        }
        // (DEFERRED: the event / refine / blacksmith / minimap / credits key handling
        // — cases 4/7/8/11/12. Case 11 sets advanceCredits/minimap-exit but is gated on
        // the unported paintMinimap; case 12 sets `advanceCredits` for the DEFERRED
        // credits scroll.)
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

/// `public final void drawHud(Graphics graphics)` (`as`): the in-game HUD.
///
/// This slice ports the PORTABLE, image-bank-free core — the HP/MP/exp bars (pure
/// `fillRect`/`drawLine` geometry over the ported [`crate::hero`] stats) and the
/// `statPoints` blink counter. Everything that binds an unmodeled `AssetCache` image
/// bank, or the unported Guardian, is DEFERRED with a named marker:
///
/// - `Guardian guardian = hero.getActiveGuardian();` and its skill-slot icons /
///   charge fills / cast banner — DEFERRED: Guardian (unported).
/// - [`draw_hud_frame`], the `statPointAlert` blink image, the quick-item icon + its
///   `drawNumberAt` quantity, and every bar `drawNumberAt` — DEFERRED: their
///   `AssetCache` banks (`hudFrame` / `statPointAlert` / `itemIcons` / `numberFont0`)
///   are unmodeled (`load_in_game_ui` DEFERS the `/img/uifrm` decode).
/// - the target-monster panel + floating-message box — DEFERRED (unported `Enemy`
///   state / `Menu.drawSelectableBox` / `Menu.drawTextField`).
///
/// The `synchronized`-free single-threaded caller (paint's case 2) does not yet wire
/// this in (the existing screen is left unchanged); the `hud_screens` gate drives it.
pub fn draw_hud(g: &mut Game) {
    // Hero hero = GameState.hero();
    let id = g.game_state.hero.expect("GameState.hero null in drawHud");
    let (hp, max_hp, mp, max_mp, exp, exp_to_next, stat_points) = {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        (
            hero.hp,
            hero.max_hp,
            hero.mp,
            hero.max_mp,
            hero.exp,
            hero.exp_to_next,
            hero.stat_points,
        )
    };
    // Guardian guardian = hero.getActiveGuardian();  — DEFERRED: Guardian (unported).
    //   Every `guardian`-gated block below is DEFERRED with it.
    // int hudY = (BaseCanvas.height - 31) - 5;
    let hud_y = g.base_canvas.height.wrapping_sub(31).wrapping_sub(5);

    let Game {
        screen,
        base_canvas,
        game_screen,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // if (hero.statPoints > 0) { lowHpBlink++; if (lowHpBlink < 5) drawImage(statPointAlert…);
    //   if (lowHpBlink >= 8) lowHpBlink = 0; }
    if (stat_points as i32) > 0 {
        // this.lowHpBlink++;
        game_screen.low_hp_blink = game_screen.low_hp_blink.wrapping_add(1);
        // if (lowHpBlink < 5) graphics.drawImage(AssetCache.statPointAlert, 5, hudY + 9, 36);
        //   — DEFERRED (statPointAlert bank unmodeled).
        // if (lowHpBlink >= 8) this.lowHpBlink = 0;
        if game_screen.low_hp_blink >= 8 {
            game_screen.low_hp_blink = 0;
        }
    }
    // if (redrawAll) setClip(0,0,width,height); else setClip(0,hudY,width,15);
    if game_screen.redraw_all {
        graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    } else {
        graphics.set_clip(0, hud_y, base_canvas.width, 15);
    }
    // drawHudFrame(graphics, 0, hudY);
    draw_hud_frame(&mut graphics, 0, hud_y);
    // Item activeItem = hero.bag.currentQuickItem();
    //   graphics.drawImage(AssetCache.itemIcons[hero.bag.currentQuickType()], width-10, hudY+19, 3);
    //   if (activeItem != null) drawNumberAt(graphics, activeItem.quantity, width-4, hudY+22, 24);
    //   else drawNumberAt(graphics, 0, width-4, hudY+22, 24);
    //   — DEFERRED (itemIcons / numberFont0 banks unmodeled).
    // if (guardian.skillSlotA != -1) { … skill-slot A icon + charge fill … }  — DEFERRED: Guardian.
    // if (guardian.skillSlotB != -1) { … skill-slot B icon + charge fill … }  — DEFERRED: Guardian.
    // graphics.setClip(0, 0, width, height);
    graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    // if (redrawAll || hpDirty) { … HP bar … hpDirty = false; }
    if game_screen.redraw_all || game_screen.hp_dirty {
        // int hpFill = (hero.hp * barWidth) / hero.maxHp;
        let hp_fill = java_div(hp.wrapping_mul(game_screen.bar_width), max_hp)
            .expect("(hp * barWidth) / maxHp");
        // graphics.setClip(47, hudY + 18, barWidth, 7);
        graphics.set_clip(47, hud_y.wrapping_add(18), game_screen.bar_width, 7);
        // drawHudFrame(graphics, 0, hudY);
        draw_hud_frame(&mut graphics, 0, hud_y);
        // if (hpFill > 0) { setColor(16711680); fillRect(47, hudY+20, hpFill, 4);
        //   setColor(16752447); fillRect(47, hudY+21, hpFill, 2); }
        if hp_fill > 0 {
            graphics.set_color(16711680);
            graphics.fill_rect(47, hud_y.wrapping_add(20), hp_fill, 4);
            graphics.set_color(16752447);
            graphics.fill_rect(47, hud_y.wrapping_add(21), hp_fill, 2);
        }
        // drawNumberAt(graphics, hero.hp, (46 + barWidth) - 2, hudY + 18, 8);
        //   — DEFERRED (numberFont0 bank unmodeled).
        // this.hpDirty = false;
        game_screen.hp_dirty = false;
        // graphics.setClip(0, 0, width, height);
        graphics.set_clip(0, 0, base_canvas.width, base_canvas.height);
    }
    // if (redrawAll || mpDirty) { … MP bar … mpDirty = false; }
    if game_screen.redraw_all || game_screen.mp_dirty {
        // int mpFill = (hero.mp * barWidth) / hero.maxMp;
        let mp_fill = java_div(mp.wrapping_mul(game_screen.bar_width), max_mp)
            .expect("(mp * barWidth) / maxMp");
        // graphics.setColor(4194239); graphics.fillRect(47, hudY + 27, mpFill, 2);
        graphics.set_color(4194239);
        graphics.fill_rect(47, hud_y.wrapping_add(27), mp_fill, 2);
        // graphics.setColor(0); graphics.fillRect(47 + mpFill, hudY + 27, barWidth - mpFill, 2);
        graphics.set_color(0);
        graphics.fill_rect(
            47i32.wrapping_add(mp_fill),
            hud_y.wrapping_add(27),
            game_screen.bar_width.wrapping_sub(mp_fill),
            2,
        );
        // this.mpDirty = false;
        game_screen.mp_dirty = false;
    }
    // if (redrawAll || expDirty) { … exp bar … expDirty = false; }
    if game_screen.redraw_all || game_screen.exp_dirty {
        // int expFill = (hero.exp * expWidth) / hero.expToNext;
        let exp_fill = java_div(exp.wrapping_mul(game_screen.exp_width), exp_to_next)
            .expect("(exp * expWidth) / expToNext");
        // graphics.setColor(10461055); graphics.fillRect(0, hudY + 31, width, 5);
        graphics.set_color(10461055);
        graphics.fill_rect(0, hud_y.wrapping_add(31), base_canvas.width, 5);
        // graphics.setColor(4144959); graphics.fillRect(2, hudY + 32, width - 4, 3);
        graphics.set_color(4144959);
        graphics.fill_rect(
            2,
            hud_y.wrapping_add(32),
            base_canvas.width.wrapping_sub(4),
            3,
        );
        // graphics.setColor(12566399); graphics.drawLine(3, hudY + 33, (3 + expFill) - 1, hudY + 33);
        graphics.set_color(12566399);
        graphics.draw_line(
            3,
            hud_y.wrapping_add(33),
            3i32.wrapping_add(exp_fill).wrapping_sub(1),
            hud_y.wrapping_add(33),
        );
        // this.expDirty = false;
        game_screen.exp_dirty = false;
    }
    // if (targetTtl <= 0 || targetMonster == null || targetMonster.state == 6) targetMonster = null;
    //   else { Menu.drawSelectableBox(…); Menu.drawTextField(… targetMonster.stats.name …);
    //          drawNumberAt(… targetMonster.stats.level …); … hp bar …; targetTtl--; }
    //   — DEFERRED (unported Enemy state / Menu.drawSelectableBox / Menu.drawTextField /
    //   the guardian.castState panel offset).
    // if (guardian != null && guardian.castState == 2) guardian.drawSkillBanner(graphics);
    //   — DEFERRED: Guardian.
    // graphics.setClip(0, 0, width, height);
    //   (dead here: the only draw that would follow — the message box — is DEFERRED.)
    // if (messageReplaced) { messageReplaced = false; return; }
    // if (messageTtl > 0) { Menu.drawSelectableBox(…); Menu.drawTextField(… this.message …);
    //   messageTtl--; }  — DEFERRED (this.message + Menu.drawSelectableBox / Menu.drawTextField).
}

/// `private static final void drawHudFrame(Graphics graphics, int x, int y)` (`as.a`):
/// the static HUD chrome.
///
/// **DEFERRED drawing.** Every operation binds the `AssetCache.hudFrame` image bank,
/// which is unmodeled (`load_in_game_ui` DEFERS the `/img/uifrm` HUD-frame decode —
/// see `asset_cache.rs`); the `hudSlots` filler loop's arithmetic is portable but its
/// body is a `hudFrame[4]` blit, so the whole method is a faithful no-op here. The
/// nine blits are recorded for when the bank lands:
/// `drawImage(hudFrame[1], x, y+12, 20)`, `drawImage(hudFrame[1], x+22, y+12, 20)`,
/// `drawImage(hudFrame[2], x+23, y+23, 20)`, `drawImage(hudFrame[3], x+44, y+12, 20)`,
/// `for slot in 0..hudSlots: drawImage(hudFrame[4], x+49+slot*6, y+14, 20)`,
/// `drawImage(hudFrame[0], x, y+9, 20)`, `drawImage(hudFrame[6], width-26, y, 20)`,
/// `drawImage(hudFrame[5], width-30, y+11, 20)`.
fn draw_hud_frame(_graphics: &mut j2me_me::Graphics, _x: i32, _y: i32) {
    // DEFERRED: AssetCache.hudFrame bank unmodeled (see the doc comment for the body).
}

/// `public static final void drawGameOver(Graphics graphics)` (`as`): the game-over
/// screen — a black fill, the dead-hero summon pose, and the fading caption.
///
/// The black fill is ported (a real frame). DEFERRED: `GameState.hero().drawSummonPose`
/// (unported `Hero` method) and the two `FontManager.drawWrappedBlock` caption draws
/// (their text is `AssetCache.commonText.get(32)`, and `commonText` is an unmodeled
/// bank). `System.out.println(...)` is a dropped no-op.
pub fn draw_game_over(g: &mut Game) {
    // char[] text = AssetCache.commonText.get(32);  — DEFERRED (commonText bank unmodeled).
    let Game {
        screen,
        base_canvas,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    // graphics.setColor(0);
    graphics.set_color(0);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // GameState.hero().drawSummonPose(graphics, halfW, halfH + 20);
    //   — DEFERRED (Hero.drawSummonPose unported).
    // char[] text = AssetCache.commonText.get(32); int textWidth = FontManager.stringWidth(text);
    // System.out.println(FontManager.charsToString(text));  — dropped no-op.
    // graphics.setColor(8355711); FontManager.drawWrappedBlock(graphics,
    //   (halfW - textWidth/2) + 1, (halfH - 20) + 1, 200, 1, text, 0, 0, (17 - fxTimer) * 2);
    // graphics.setColor(16777215); FontManager.drawWrappedBlock(graphics,
    //   halfW - textWidth/2, halfH - 20, 200, 1, text, 0, 0, (17 - fxTimer) * 2);
    //   — DEFERRED (caption text from the unmodeled commonText bank).
}

/// `private final void drawClearMenu(Graphics graphics)` (`as`): the four-line "stage
/// cleared" menu box.
///
/// The black fill + the bevelled panel (frame + blue interior) are ported. DEFERRED:
/// the four `FontManager.drawChars` label draws — their text is
/// `AssetCache.commonText.get(33..36)`, and `commonText` is an unmodeled bank.
fn draw_clear_menu(g: &mut Game) {
    // char[] line1 = commonText.get(33); … line4 = commonText.get(36);
    //   — DEFERRED (commonText bank unmodeled).
    let Game {
        screen,
        base_canvas,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    // graphics.setColor(0);
    graphics.set_color(0);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // int boxX = BaseCanvas.halfW - 55;
    let box_x = base_canvas.half_w.wrapping_sub(55);
    // int boxY = BaseCanvas.halfH - 36;
    let box_y = base_canvas.half_h.wrapping_sub(36);
    // Menu.drawPanelFrame(graphics, boxX, boxY, 110, 72);
    menu::draw_panel_frame(&mut graphics, box_x, box_y, 110, 72);
    // Menu.fillPanelInterior(graphics, boxX, boxY, 110, 72);
    menu::fill_panel_interior(&mut graphics, box_x, box_y, 110, 72);
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, boxX + 5, boxY + 5,  line1, 1);
    // FontManager.drawChars(graphics, boxX + 5, boxY + 21, line2, 1);
    // FontManager.drawChars(graphics, boxX + 5, boxY + 37, line3, 1);
    // FontManager.drawChars(graphics, boxX + 5, boxY + 53, line4, 1);
    //   — DEFERRED (label text from the unmodeled commonText bank).
}
