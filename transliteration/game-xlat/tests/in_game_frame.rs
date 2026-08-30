//! MILESTONE gate: the FIRST IN-GAME FRAME.
//!
//! `first_frame.rs` proved the boot → title (logo) render. `menu_chain.rs` proved
//! the menu chain records a `newGame(...)` launch. THIS gate drives that launch
//! DIRECTLY — bypassing the menu — and pumps the frame loop from a fresh main menu
//! through New Game to the world:
//!
//!   `GameState.newGame(false, 6, traits)` → `requestState(21)` → `startNewMap`
//!   (`Hero.initClass`, class-6 start triple `{0, 22, 4}`) → `setScreen(1)` +
//!   `AssetLoader.loadResources` → `requestMapWarp` → `loadMap` → `swapMap`
//!   (`new GameMap(0).load()` — reads `/m/00.map`, decodes the `/m/t/t01` tileset
//!   atlas) → `setHeroTile` → `requestState(15)` → `warpMap` (place hero, center +
//!   snap camera) → `requestState(2)` → `GameScreen.paint` case 2 →
//!   `GameMap.paint`/`drawTiles`.
//!
//! The worker-thread loader is collapsed into the frame loop (see `asset_loader`),
//! and the observable `screen` sequence `1 → 2` is preserved. DEFERRED past the
//! tiles: the hero sprite byte-script system, `drawHud`/Guardian, entities/NPCs,
//! movement, the loading-screen art.
//!
//! The gate proves a REAL WORLD frame (pixel-richness: distinct colours ≥ 8 AND the
//! dominant colour does not fill the frame AND it is not all-white) that also
//! DIFFERS from the main-menu frame it was reached from, backed by a proven-red
//! negative control (the same assertions on a blank framebuffer must FAIL).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_midlet, game_state, title_screen, Game,
};
use j2me_me::Image;
use std::collections::HashMap;

/// Matches `menu_chain.rs` / `menu_oracle.rs`: the route's game-RNG seed keeps the
/// pre-menu drive deterministic (title birds consume `ByteUtil.rng`).
const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
/// SOFT1 (any key) leaves the state-1 title.
const KEY_SOFT1: i32 = -6;
/// Warrior — the first selectable start class (`ClassSelectMenu` default).
const CLASS_WARRIOR: i8 = 6;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Drives boot → title → any-key → settled main menu (screen 9, `GameScreen`
/// current) with NEW GAME selected — the same path `menu_chain::drive_to_main_menu`
/// takes, replicated here from the public API.
fn drive_to_main_menu() -> Game {
    let mut g = Game::new();
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    load_resources(&mut g);

    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    title_screen::construct(&mut g);
    asset_cache::load_logo(&mut g);
    asset_cache::load_title_screen(&mut g);
    font_manager::init_fonts(&mut g);
    font_manager::load_title_labels(&mut g);
    title_screen::start_logo(&mut g);
    {
        let Game {
            display, canvas, ..
        } = &mut g;
        display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
    }
    let mut guard = 0u32;
    loop {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        if g.title_screen.state == 1 {
            break;
        }
        assert!(guard < 10_000, "state-10 never transitioned to the title");
    }
    for _ in 0..TITLE_FRAMES_BEFORE_KEY {
        game_loop::run_one_frame(&mut g);
    }
    key_press(&mut g, KEY_SOFT1);
    for _ in 0..MENU_SETTLE {
        game_loop::run_one_frame(&mut g);
    }
    assert_eq!(
        g.game_state.screen, 9,
        "settled on the main menu (screen 9)"
    );
    g
}

/// (distinct colour count, most-common colour's pixel count, total pixels).
fn frame_stats(img: &Image) -> (usize, usize, usize) {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for y in 0..img.height() {
        for x in 0..img.width() {
            if let Some(px) = img.get(x, y) {
                *counts.entry(px).or_insert(0) += 1;
            }
        }
    }
    let total = (img.width() * img.height()) as usize;
    let dominant = counts.values().copied().max().unwrap_or(0);
    (counts.len(), dominant, total)
}

/// The pixel-richness gate (shared by the real-frame test and the proven-red control).
fn assert_real_frame(img: &Image) {
    let (distinct, dominant, total) = frame_stats(img);
    let white = 0xffff_ffffu32;
    let all_white = img.pixels().iter().all(|&p| p == white);
    assert!(
        distinct >= 8,
        "a real frame has variety: distinct colours = {distinct} (< 8)"
    );
    assert!(
        dominant < total,
        "a real frame is not one flat colour: dominant {dominant} == total {total}"
    );
    assert!(!all_white, "a real frame is not blank (all-white)");
}

/// Drives New Game DIRECTLY and pumps loading → world frames until `screen == 2`.
/// Returns `(game, menu_frame_pixels)`.
fn drive_to_world() -> (Game, Vec<u32>) {
    let mut g = drive_to_main_menu();
    // Snapshot the main-menu frame we reached the world from.
    let menu_pixels = g.screen.as_ref().expect("framebuffer").pixels().to_vec();

    // GameState.newGame(false, classId=6, traits) — bypass the menu chain.
    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);
    assert_eq!(g.game_state.screen, 0, "newGame set screen 0");

    // Pump loading → world frames until the world screen (2) is reached and rendered.
    let mut guard = 0u32;
    while g.game_state.screen != 2 {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        assert!(
            guard < 100,
            "New Game drive never reached screen 2 (stuck at screen {})",
            g.game_state.screen
        );
    }
    (g, menu_pixels)
}

#[test]
fn direct_new_game_reaches_the_world_and_renders_tiles() {
    let (g, menu_pixels) = drive_to_world();

    // Reached the world screen (2), from a class-6 start on map 00 (tileset 1).
    assert_eq!(g.game_state.screen, 2, "screen == 2 (world)");
    assert_eq!(g.game_state.class_id, CLASS_WARRIOR, "class 6");
    {
        let map = g
            .game_state
            .map
            .as_ref()
            .expect("GameState.map materialised");
        assert_eq!(map.tileset_id, 1, "map 00 uses tileset 1");
        assert_eq!(map.width_tiles, 30, "map 00 is 30 tiles wide");
        assert_eq!(map.height_tiles, 30, "map 00 is 30 tiles tall");
        assert!(map.tile_grid.is_some(), "the tile grid was parsed");
    }
    assert!(
        g.asset_cache
            .map_tiles
            .as_ref()
            .is_some_and(|t| t.len() == 49),
        "the /m/t/t01 tileset decoded its 49 frames"
    );
    // The hero exists and initClass set the class-6 stats (maxHp = (5 + 1) * 12 = 72).
    let hero_id = g.game_state.hero.expect("hero materialised");
    let hero = g.entity_arena[hero_id].as_hero().expect("Hero data");
    assert_eq!(
        hero.max_hp, 72,
        "class-6 maxHp = (vitality 5 + level 1) * 12"
    );
    assert_eq!(hero.hp, 72, "hp filled to maxHp");

    let img = g.screen.as_ref().expect("framebuffer");
    assert_eq!((img.width(), img.height()), (240, 320));
    let (distinct, dominant, total) = frame_stats(img);
    eprintln!(
        "in_game_frame: {distinct} distinct colours, dominant {dominant}/{total} px, 240x320"
    );

    // A REAL world frame …
    assert_real_frame(img);
    // … and it DIFFERS from the main-menu frame it was reached from (the tiles
    // overwrote the menu parchment).
    assert_ne!(
        img.pixels(),
        menu_pixels.as_slice(),
        "the world frame is identical to the main-menu frame — no tiles were drawn"
    );
}

/// Liveness: the world paint actually WROTE tile pixels — the framebuffer differs
/// from a fresh blank surface, and a concrete non-white pixel exists.
#[test]
fn world_paint_actually_wrote_pixels() {
    let (g, _menu) = drive_to_world();
    let painted = g.screen.as_ref().expect("framebuffer");
    let blank = Image::create_mutable(painted.width(), painted.height()).unwrap();
    assert_ne!(
        painted.pixels(),
        blank.pixels(),
        "the world paint left the framebuffer identical to a blank surface"
    );
    let non_white = (0..painted.height())
        .any(|y| (0..painted.width()).any(|x| painted.get(x, y).is_some_and(|p| p != 0xffff_ffff)));
    assert!(non_white, "no non-white pixel — nothing was drawn");
}

/// Negative control (GATES.md R3): the SAME gate assertions on a fresh blank
/// (all-white) framebuffer must FAIL — proving the pixel-richness gate bites and a
/// blank frame cannot read as a rendered world.
#[test]
#[should_panic(expected = "distinct colours")]
fn negative_control_blank_framebuffer_is_rejected() {
    let blank = Image::create_mutable(240, 320).unwrap();
    assert_real_frame(&blank);
}
