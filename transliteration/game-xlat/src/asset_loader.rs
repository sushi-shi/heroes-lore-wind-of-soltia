//! Transliterated from `java/src/main/java/defpackage/AssetLoader.java`
//! (original `bu.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The background asset loader. On device each entry point (`loadResources`,
//! `loadMap`, `loadGuardian`, `loadMainMenu`) sets [`AssetLoaderState::phase`],
//! shows a loading screen, and spawns a worker `Thread`; `run` then does the
//! phase's heavy work off the UI thread.
//!
//! ## ANTI-BOG boundary + the thread collapse
//!
//! This slice ports **only what the first in-game frame needs** — the phase-1
//! ("- RESOURCE") → phase-2 ("- MAP") chain that drives a New Game to the world.
//! The worker `Thread` is **collapsed into the frame loop**: `load_resources` /
//! `load_map` run their phase synchronously (each is followed immediately by the
//! next queued `GameState` request), which keeps the observable `screen` sequence
//! `1 → 2` faithful without modelling threads.
//!
//! DEFERRED on the phase-1 path: `loadCommonAssets`, `classSkillText`,
//! `loadHeroEquipSprites`, `loadGuardianSprites` (the sprite/UI/text banks + the
//! per-class byte-script sprite system) and `beginLoading`/`keepLoadingProgress`.
//! DEFERRED elsewhere: `loadGuardian` (phase 3), `loadMainMenu` (phase 5), the
//! sprite-bank assembler (`loadSpriteBank` and its `*Anim` tables), and
//! `drawLoadingOverlay`'s real loading art.
//!
//! `AssetLoader` has one mutable static this slice models — `phase` (`bu.a:B`);
//! `commonLoaded` (`bu.a:Z`) and the script/atlas/anim tables are DEFERRED.

use crate::game::Game;
use crate::game_map;
use crate::game_state;

/// Java `bu` / `AssetLoader` state — **partial** (anti-bog). Only the `phase`
/// static (which entry point a worker is running) is modelled; `commonLoaded` and
/// the sprite-script tables are DEFERRED (see the module header and
/// `java/reconstruction/ownership.tsv`).
#[derive(Debug, Default)]
pub struct AssetLoaderState {
    /// `private static byte phase = 0;` (obf `bu.a:B`) — which load phase a worker
    /// runs (1 resources, 2 map, 3 guardian, 5 menu). Read by [`draw_loading_overlay`].
    pub phase: i8,
}

/// `public static final void loadResources()` (`bu.a:()V`): starts the
/// "- RESOURCE" load. The worker `Thread` is collapsed into the frame loop, so the
/// phase-1 work ([`run`]) runs synchronously here.
pub fn load_resources(g: &mut Game) {
    // phase = (byte) 1;
    g.asset_loader.phase = 1;
    // BaseCanvas.beginLoading("- RESOURCE", 500);  — DEFERRED (loading overlay art).
    // new Thread(new AssetLoader()).start();  — collapsed: run the phase now.
    run(g);
}

/// `public static final void loadMap()` (`bu.b:()V`): starts the "- MAP" (map warp)
/// load. Collapsed like [`load_resources`].
pub fn load_map(g: &mut Game) {
    // GameLoop.gameScreen.resetHudState();  — DEFERRED (HUD state, not on the tile path).
    // phase = (byte) 2;
    g.asset_loader.phase = 2;
    // BaseCanvas.beginLoading("- MAP", 200);  — DEFERRED.
    // new Thread(new AssetLoader()).start();  — collapsed: run the phase now.
    run(g);
}

/// `public final void run()` (`bu.run:()V`) — the worker body, dispatched by
/// [`AssetLoaderState::phase`]. `Thread.sleep`/`yieldTick` are no-ops. Only phases 1
/// and 2 are ported (3 = guardian, 5 = main-menu are DEFERRED).
fn run(g: &mut Game) {
    // try { Thread.sleep(100L); } catch { }   — no-op.
    match g.asset_loader.phase {
        1 => run_phase_resources(g),
        2 => run_phase_map(g),
        // (DEFERRED: case 3 guardian summon, case 5 return-to-main-menu.)
        _ => {}
    }
}

/// `run`'s `case 1` (phase 1, "- RESOURCE"). The sprite/UI/text bank loads are
/// DEFERRED (see the module header); the observable effect that advances the New
/// Game sequence is the map-warp request.
fn run_phase_resources(g: &mut Game) {
    // (DEFERRED: Thread.sleep; yieldTick; if (!commonLoaded) loadCommonAssets();
    //  the classSkillText TextTable; loadHeroEquipSprites(); loadGuardianSprites().)
    // GameState.requestMapWarp(GameState.storyMapId, (byte) 1, GameState.arg0, GameState.arg1);
    let map_id = g.game_state.story_map_id;
    let a1 = g.game_state.arg0;
    let a2 = g.game_state.arg1;
    game_state::request_map_warp(g, map_id, 1, a1, a2);
    // BaseCanvas.keepLoadingProgress = true;  — DEFERRED.
}

/// `run`'s `case 2` (phase 2, "- MAP"): drops the old map, loads the destination
/// map, places the hero, seeds spawns, and requests the warp (state 15).
fn run_phase_map(g: &mut Game) {
    // swapMap();
    swap_map(g);
    // GameState.setHeroTile((int) GameState.arg1, (int) GameState.arg2);
    let tile_x = g.game_state.arg1 as i32;
    let tile_y = g.game_state.arg2 as i32;
    game_state::set_hero_tile(g, tile_x, tile_y);
    // GameState.map.fadeStep();
    game_map::fade_step(g);
    // GameState.requestState((byte) 15, GameState.arg0);
    let a0 = g.game_state.arg0;
    game_state::request_state_a0(g, 15, a0);
}

/// `private final GameMap swapMap()` (`bu`): unlinks the hero/guardian from the old
/// map, drops it, and loads the destination `new GameMap(storyMapId)`.
///
/// On a fresh New Game there is no prior map (`oldMap == null`), so the
/// `removeEntity`/`next`/`prev`/guardian cleanup is a DEFERRED no-op here.
fn swap_map(g: &mut Game) {
    // GameMap oldMap = GameState.map; Hero hero = GameState.hero();
    // if (oldMap != null) { oldMap.removeEntity(hero); ...; guardian cleanup }
    //   — DEFERRED (oldMap null on the first map; no guardian).
    // GameState.map = null; GameMap newMap = new GameMap(GameState.storyMapId);
    let story = g.game_state.story_map_id;
    let new_map = {
        let Game {
            game_map_class,
            base_canvas,
            ..
        } = &mut *g;
        game_map::new_game_map(game_map_class, base_canvas, story)
    };
    // GameState.setMap(newMap);
    g.game_state.map = Some(new_map);
    // newMap.load();
    game_map::load(g);
}

/// `public static final void drawLoadingOverlay(Graphics graphics)` (`bu`): draws
/// the phase-appropriate loading overlay. The real overlay art
/// (`BaseCanvas.drawLoadingScreen` / `GameScreen.drawLoadBox`) is DEFERRED; this
/// minimal body keeps the loading frame from panicking (it leaves the framebuffer
/// as-is — the world frame overwrites it once `screen` reaches 2).
pub fn draw_loading_overlay(g: &mut Game) {
    // switch (phase) { case 1/2: BaseCanvas.drawLoadingScreen(graphics); case 3: ... }
    //   — DEFERRED (loading overlay art not ported in this slice).
    let _ = g;
}
