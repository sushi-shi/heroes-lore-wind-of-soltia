//! MILESTONE gate: THE HERO SPRITE APPEARS IN THE WORLD.
//!
//! `in_game_frame.rs` proved New Game reaches screen 2 and paints the map TILES.
//! THIS gate proves the next layer: `GameScreen.paint` case 2 → `GameMap.paint` →
//! `drawEntities` → `Hero.paint` now draws the CHARACTER into the world frame (at
//! least the base **body** layer of the 9-layer sprite), via the byte-script draw
//! system (`AssetLoader.loadSpriteBank` decoding the `/c1/s|i/b` script+atlas into
//! `AssetCache.heroFrames` / `AssetCache.spriteBanks`, blitted by
//! `GameScreen.drawFrame`).
//!
//! The gate is backed by:
//! - a **state** witness — `heroFrames` allocated, the hero's body-layer draw script
//!   present, the body sprite bank decoded, and `entityShadow` loaded;
//! - a **pixel** witness — the full world+hero frame DIFFERS from a tiles-only
//!   baseline (the same frame re-painted with the entity list emptied, so
//!   `drawEntities` draws nothing), proving the sprite wrote pixels;
//! - a **proven-red control** — asserting the full frame equals the tiles-only frame
//!   (i.e. that NO sprite was drawn) FAILS, so the pixel witness genuinely bites.
//!
//! DEFERRED (rendered as null layers / skipped): the armor/head-equip/weapon/aura/
//! shield layers beyond body+default-head (equipment slots are null in this slice),
//! and the pre-paint `GameState.update()` that ticks `animFrame` (owned by the
//! movement lane) — see `Hero.paint`'s milestone note.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, font_manager, game_loop, game_map, game_midlet, game_state,
    title_screen, Game,
};
use std::collections::HashMap;

const GAME_RNG_SEED: i64 = 305419896;
const TITLE_FRAMES_BEFORE_KEY: u32 = 3;
const MENU_SETTLE: u32 = 12;
const KEY_SOFT1: i32 = -6;
const CLASS_WARRIOR: i8 = 6;

fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

fn key_press(g: &mut Game, code: i32) {
    g.canvas.as_mut().expect("canvas").key_pressed(code);
}

/// Boot → title → any-key → settled main menu (screen 9), replicating
/// `in_game_frame`/`menu_chain`'s public-API drive.
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

/// Drives New Game directly and pumps loading → world frames until `screen == 2` is
/// reached and the world+hero frame has been rendered.
fn drive_to_world() -> Game {
    let mut g = drive_to_main_menu();
    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);
    assert_eq!(g.game_state.screen, 0, "newGame set screen 0");
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
    g
}

/// Distinct ARGB colours in a pixel buffer.
fn distinct_colours(pixels: &[u32]) -> usize {
    let mut set: HashMap<u32, ()> = HashMap::new();
    for &p in pixels {
        set.insert(p, ());
    }
    set.len()
}

/// Count of positions where two equal-length pixel buffers differ.
fn diff_count(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

/// Re-paints the world with the map's entity list emptied, so `drawEntities` draws
/// nothing — yielding the TILES-ONLY baseline (the frame with the hero sprite
/// skipped). Returns the resulting framebuffer pixels.
fn render_tiles_only(g: &mut Game) -> Vec<u32> {
    {
        let map = g.game_state.map.as_mut().expect("GameState.map");
        map.entities.head = None;
        map.entities.tail = None;
        map.entities.count = 0;
    }
    game_map::paint(g);
    g.screen.as_ref().expect("framebuffer").pixels().to_vec()
}

#[test]
fn hero_sprite_is_drawn_into_the_world_frame() {
    let mut g = drive_to_world();

    // --- STATE witness: the byte-script sprite system loaded the hero body. ---
    let hero_id = g.game_state.hero.expect("hero materialised");
    let facing = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero data")
        .battler
        .facing;
    // heroFrames is a live Object[396].
    let hero_frames = g
        .asset_cache
        .hero_frames
        .as_ref()
        .expect("AssetCache.heroFrames allocated (Object[396])");
    assert_eq!(hero_frames.len(), 396, "heroFrames is Object[396]");
    // The idle body layer for the hero's facing: baseIndex = (0*36)+((facing-1)*9), layer 0.
    let body_index = ((facing as i32 - 1) * 9) as usize;
    assert!(
        hero_frames[body_index].is_some(),
        "the hero's body-layer draw script (heroFrames[{body_index}], facing {facing}) was decoded"
    );
    // The body sprite bank (spriteBanks[1]) decoded at least one atlas frame; its
    // mirror twin (spriteBanks[7]) is allocated too.
    let body_bank = g.asset_cache.sprite_banks[1]
        .as_ref()
        .expect("spriteBanks[1] (body) decoded");
    assert!(
        body_bank.iter().any(|f| f.is_some()),
        "the body atlas decoded at least one frame image"
    );
    assert!(
        g.asset_cache.sprite_banks[7].is_some(),
        "spriteBanks[7] (mirrored body) allocated"
    );
    assert!(
        g.asset_cache.entity_shadow.is_some(),
        "entityShadow (the ground shadow Hero.paint draws) loaded"
    );

    // --- PIXEL witness: the world+hero frame differs from a tiles-only baseline. ---
    let full = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    assert_eq!(full.len(), 240 * 320, "240x320 framebuffer");
    let tiles_only = render_tiles_only(&mut g);

    let changed = diff_count(&full, &tiles_only);
    let (df, dt) = (distinct_colours(&full), distinct_colours(&tiles_only));
    eprintln!(
        "hero_sprite: {changed} px changed by the hero, distinct colours full={df} tiles-only={dt} (+{})",
        df.saturating_sub(dt)
    );
    assert!(
        changed >= 20,
        "the hero sprite must write pixels: only {changed} px differ from the tiles-only baseline"
    );
    assert!(
        df >= dt,
        "the hero sprite must not reduce the colour count (full={df}, tiles-only={dt})"
    );
}

#[test]
#[should_panic(expected = "NO sprite")]
fn negative_control_a_tiles_only_frame_lacks_the_hero() {
    let mut g = drive_to_world();
    let full = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    let tiles_only = render_tiles_only(&mut g);
    // Proven-red: asserting the world frame carries NO sprite (full == tiles-only)
    // FAILS, because Hero.paint genuinely altered the frame. This proves the pixel
    // witness above bites rather than passing on identical frames.
    assert_eq!(
        full, tiles_only,
        "NO sprite was drawn — the world frame equals the tiles-only baseline"
    );
}
