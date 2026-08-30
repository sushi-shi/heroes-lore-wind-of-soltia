//! Unit + world gate for the TRANSIENT/WORLD ENTITY LEAVES: `Effect` (`y`, an
//! animated visual effect), its moving `Projectile` (`i`) subclass, and the town
//! `Npc` (`ac`).
//!
//! Two levels of drive:
//!
//! * **Pure arena** — an `Effect` built from a sprite script (`type == 100`) records
//!   its lifetime (`frameCount = spriteScript[0]`), frame `0`, and the tile-centred
//!   pixel position `tileX << 4`; a hero-owned `Projectile` records `range - 1` and
//!   its damage payload; an `Npc` records `visible`/`kind` and its idle `Battler`
//!   base (state 1, facing 2, animFrame -1). No boot/render needed.
//! * **Live world** (driven directly through New Game, as `in_game_frame.rs`) — an
//!   `Effect` linked into the map advances one frame per `paint`, finishes at its
//!   `frameCount`, and `removeEntity`s itself from the z-list on the finishing frame;
//!   a hero-owned `Projectile`'s `onFrame` chains a fresh segment one tile forward
//!   (its travel); an `Npc`'s `paint` blits the ground shadow onto the framebuffer.
//!
//! The projectile/npc sprite draws bottom out in DEFERRED `AssetCache` guardian/NPC
//! banks, so what is asserted is the ported, bank-independent state: the lifetime
//! machine, the tile-chaining, and the shadow blit.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::entity::{EntityId, EntityKind};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, effect, font_manager, game_loop, game_map, game_midlet, game_state,
    npc, projectile, title_screen, Game,
};

// --- Shared New-Game → world drive (mirrors in_game_frame.rs) ---------------------

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

fn drive_to_world() -> Game {
    let mut g = drive_to_main_menu();
    let traits = [false, false, false];
    game_state::new_game(&mut g, false, CLASS_WARRIOR, &traits);
    let mut guard = 0u32;
    while g.game_state.screen != 2 {
        game_loop::run_one_frame(&mut g);
        guard += 1;
        assert!(guard < 100, "New Game drive never reached screen 2");
    }
    assert!(g.game_state.map.is_some(), "GameState.map materialised");
    g
}

/// The map's z-list head→tail as a `Vec<EntityId>` (front to back).
fn map_entities(g: &Game) -> Vec<EntityId> {
    let mut out = Vec::new();
    let mut cursor = g.game_state.map.as_ref().unwrap().entities.head;
    while let Some(id) = cursor {
        out.push(id);
        cursor = g.entity_arena[id].next;
    }
    out
}

// --- Pure-arena construction ------------------------------------------------------

/// A script-fed `Effect` (`type == 100`) records `frameCount = spriteScript[0]`,
/// frame 0, and the tile-centred pixel position (`tileX << 4`).
#[test]
fn effect_from_script_has_initial_state() {
    let mut g = Game::new();
    // Effect(byte tileX=5, byte tileY=5, byte[] {3,0,0,0}) — a 3-frame script.
    let id = effect::new_effect_from_script(&mut g.entity_arena, 5, 5, vec![3, 0, 0, 0]);

    assert_eq!(g.entity_arena[id].kind(), EntityKind::Effect);
    let e = g.entity_arena[id].as_effect().expect("Effect base");
    assert_eq!(e.frame_count, 3, "frameCount = spriteScript[0]");
    assert_eq!(e.frame, 0);
    assert_eq!(e.type_, 100, "the script ctor sets type 100");
    assert_eq!(e.sprite_script.as_deref(), Some([3i8, 0, 0, 0].as_slice()));
    // super((short)(tileX << 4), (short)(tileY << 4), (byte) 8, (byte) 9);
    let n = &g.entity_arena[id];
    assert_eq!(
        (n.pixel_x, n.pixel_y),
        (80, 80),
        "tile 5 → pixel 80 (5 << 4)"
    );
    assert_eq!((n.tile_x, n.tile_y), (5, 5));
    assert_eq!((n.half_w, n.half_h), (8, 9), "Effect half size 8×9");
    // A plain Effect is not a Projectile, but `as_effect` sees it as an Effect base.
    assert!(g.entity_arena[id].as_projectile().is_none());
}

/// A hero-owned `Projectile` records `range - 1`, its damage payload, and exposes the
/// embedded `Effect` base through `as_effect`.
#[test]
fn projectile_hero_has_initial_state() {
    let mut g = Game::new();
    // A stand-in owner node (its concrete type is only read via instanceof).
    let owner = g.entity_arena.spawn(0, 0);
    let id = projectile::new_projectile_hero(
        &mut g.entity_arena,
        4,          // tileX
        7,          // tileY
        vec![2, 0], // spriteScript (frameCount 2)
        owner,
        false, // piercing
        2,     // dir (down)
        3,     // range
        0,     // chainFrame
        42,    // damage
        1,     // statusKind
        true,  // crit
    );

    assert_eq!(g.entity_arena[id].kind(), EntityKind::Projectile);
    // `((Effect) this)` base fields reach through as_effect.
    let e = g.entity_arena[id]
        .as_effect()
        .expect("Effect base of Projectile");
    assert_eq!(e.frame_count, 2, "frameCount = spriteScript[0]");
    assert_eq!(e.frame, 0);
    assert_eq!(e.type_, 100);
    let p = g.entity_arena[id].as_projectile().expect("Projectile");
    assert_eq!(p.range, 2, "range = ctor range - 1");
    assert_eq!(p.dir, 2);
    assert_eq!(p.chain_frame, 0);
    assert_eq!(p.damage, 42);
    assert_eq!(p.status_kind, 1);
    assert!(p.crit);
    assert!(!p.piercing);
    assert!(!p.has_hit, "hasHit defaults false");
    assert_eq!(p.owner, owner);
    let n = &g.entity_arena[id];
    assert_eq!((n.pixel_x, n.pixel_y), (64, 112), "tiles 4,7 → 64,112");
}

/// An `Npc` records `visible`/`kind`/`spriteSet` and its idle `Battler` base.
#[test]
fn npc_has_initial_state() {
    let mut g = Game::new();
    // Npc(short pixelX=48, short pixelY=32, byte kind=5, byte spriteSet=0)
    let id = npc::new_npc(&mut g.entity_arena, 48, 32, 5, 0);

    assert_eq!(g.entity_arena[id].kind(), EntityKind::Npc);
    let d = g.entity_arena[id].as_npc().expect("Npc");
    assert!(d.visible, "constructed visible");
    assert_eq!(d.kind, 5);
    assert_eq!(d.sprite_set, 0);
    // The Battler base: init() sets state 1, facing 2, moveDir 2, animFrame -1.
    let b = g.entity_arena[id].as_battler().expect("Npc is a Battler");
    assert_eq!(b.state, 1, "idle");
    assert_eq!(b.facing, 2);
    assert_eq!(b.move_dir, 2);
    assert_eq!(b.anim_frame, -1);
    let n = &g.entity_arena[id];
    assert_eq!((n.pixel_x, n.pixel_y), (48, 32));
    assert_eq!((n.half_w, n.half_h), (8, 8), "Npc half size 8×8");
}

// --- Live-world lifecycle ---------------------------------------------------------

/// An `Effect` linked into the map advances one frame per `paint` and reaps itself
/// from the z-list on the frame it reaches `frameCount`.
#[test]
fn effect_paint_ticks_and_removes_itself_at_lifetime() {
    let mut g = drive_to_world();

    // A 3-frame script effect (frameCount 3), placed at a mid-map tile.
    let id = effect::new_effect_from_script(&mut g.entity_arena, 8, 8, vec![3, 0, 0, 0]);
    game_map::add_entity(&mut g, id);
    assert!(
        map_entities(&g).contains(&id),
        "effect linked into the z-list"
    );
    assert!(!effect::is_finished(&g, id), "fresh effect is not finished");

    // paint 1 → frame 1, not finished, still linked.
    effect::paint(&mut g, id, 0, 0);
    assert_eq!(g.entity_arena[id].as_effect().unwrap().frame, 1);
    assert!(!effect::is_finished(&g, id));
    assert!(map_entities(&g).contains(&id), "not reaped before lifetime");

    // paint 2 → frame 2, not finished, still linked.
    effect::paint(&mut g, id, 0, 0);
    assert_eq!(g.entity_arena[id].as_effect().unwrap().frame, 2);
    assert!(!effect::is_finished(&g, id));
    assert!(map_entities(&g).contains(&id));

    // paint 3 → frame 3 (>= frameCount 3) → isFinished → removeEntity.
    effect::paint(&mut g, id, 0, 0);
    assert_eq!(g.entity_arena[id].as_effect().unwrap().frame, 3);
    assert!(
        effect::is_finished(&g, id),
        "finished once frame >= frameCount"
    );
    assert!(
        !map_entities(&g).contains(&id),
        "the finished effect reaped itself from the z-list"
    );
}

/// A hero-owned `Projectile`'s `onFrame` chains a fresh segment one tile forward
/// (its tile-by-tile travel), adding it to the map z-list.
#[test]
fn projectile_onframe_chains_a_forward_segment() {
    let mut g = drive_to_world();
    let owner = g.game_state.hero.expect("hero owner");

    // A hero bolt at tile (8,8) heading DOWN (dir 2), range 3, chaining at frame 0.
    let id = projectile::new_projectile_hero(
        &mut g.entity_arena,
        8,
        8,
        vec![2, 0], // frameCount 2 (no group parts)
        owner,
        false, // piercing
        2,     // dir (down)
        3,     // range
        0,     // chainFrame
        10,    // damage
        0,     // statusKind
        false, // crit
    );
    game_map::add_entity(&mut g, id);
    let before = map_entities(&g);
    assert!(before.contains(&id));

    // frame(0) == chainFrame(0), range(2) > 0, !hasHit → spawn the next segment at (8,9).
    projectile::on_frame(&mut g, id);

    let after = map_entities(&g);
    assert_eq!(
        after.len(),
        before.len() + 1,
        "onFrame chained exactly one forward segment"
    );
    let new_id = *after
        .iter()
        .find(|e| !before.contains(e))
        .expect("the chained segment");
    assert_eq!(g.entity_arena[new_id].kind(), EntityKind::Projectile);
    let n = &g.entity_arena[new_id];
    assert_eq!((n.tile_x, n.tile_y), (8, 9), "chained one tile DOWN");
    let np = g.entity_arena[new_id].as_projectile().unwrap();
    assert_eq!(np.range, 1, "the segment's range is (this.range) - 1");
    assert_eq!(np.owner, owner);
}

/// An `Npc`'s `paint` blits the ground shadow onto the framebuffer (the sprite draw
/// is DEFERRED, but the shadow is ported).
#[test]
fn npc_paint_draws_the_ground_shadow() {
    let mut g = drive_to_world();

    // An on-screen NPC (kind 5, animated) at pixel (0,0).
    let id = npc::new_npc(&mut g.entity_arena, 0, 0, 5, 0);
    game_map::add_entity(&mut g, id);

    let before = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    // screenX = 50 + 0 + 8 = 58; screenY = 50 + 0 + 8 = 58 — inside the world view.
    npc::paint(&mut g, id, 50, 50);
    let after = g.screen.as_ref().expect("framebuffer").pixels().to_vec();

    assert_ne!(
        before, after,
        "Npc.paint blitted the entity shadow onto the framebuffer"
    );
}

/// A hidden NPC (`visible == false`) paints nothing.
#[test]
fn npc_paint_skips_when_invisible() {
    let mut g = drive_to_world();
    let id = npc::new_npc(&mut g.entity_arena, 0, 0, 5, 0);
    g.entity_arena[id].as_npc_mut().unwrap().visible = false;

    let before = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    npc::paint(&mut g, id, 50, 50);
    let after = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    assert_eq!(before, after, "an invisible NPC paints nothing");
}
