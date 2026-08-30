//! Unit gate for the BOSS + GUARDIAN-CAST-FX lane: the `Boss` (`av`) multi-tile boss
//! base (an `Enemy`) and the `GuardianCastFx` (`bj`) guardian summon/cast overlay.
//!
//! * **Boss construction + tick.** A `Boss` built from an installed `EnemyType`
//!   records its `Enemy`-derived stats (hp = maxHp) AND its boss-specific state: the
//!   overridden `layer` (the constructor's fifth arg, over the Enemy size-derived one),
//!   and the two cached hero-offset fields (`heroDistX`/`heroDistY`) that `Enemy`
//!   lacks, at their zero default. `boss::update` (Boss.update — the overriding tick)
//!   advances `animFrame` and recomputes `heroDistX`/`heroDistY` from the hero tile.
//!   Paired with a proven-red control: an identically-built boss that is NOT ticked
//!   keeps `animFrame = -1` and `heroDistX/Y = 0`.
//! * **Paint dispatch.** `game_map::draw_entities` routes a `Boss` node to `boss::paint`
//!   (the virtual override, distinct from `enemy::paint`); the boss sprite bank is
//!   DEFERRED-loaded so it draws nothing, but the dispatch + null-fallback run without
//!   panicking.
//! * **GuardianCastFx lifetime.** A `GuardianCastFx` overlay carried in a battler's
//!   floater list advances its `frame` each `draw_floaters` paint and is reaped exactly
//!   when `frame` reaches its `lifetime`. Paired with a proven-red control: an
//!   un-painted `GuardianCastFx` stays at frame 0, not finished.
//!
//! The concrete boss subclasses (Geb*/Nord*/RockyBoss), the `EventScript` spawn
//! machinery, `Guardian` (which spawns the cast FX), and the `Weapon`/`Guardian`-gated
//! `takeHeroHit` body are DEFERRED (see `boss.rs`/`guardian_cast_fx.rs`), so what is
//! asserted is the ported, guardian-independent boss + overlay state.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::battler::{self, BattlerData};
use heroes_lore_wind_of_soltia_game_xlat::enemy_type::EnemyTypeData;
use heroes_lore_wind_of_soltia_game_xlat::entity::EntityKind;
use heroes_lore_wind_of_soltia_game_xlat::overlay::OverlayData;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, boss, byte_util, enemy_type, font_manager, game_loop, game_map, game_midlet,
    game_state, guardian_cast_fx, title_screen, Game,
};
use j2me_me::{Graphics, Image};

// --- Shared New-Game → world drive (mirrors enemy.rs) --------------------------------

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

/// A plain melee (aiType 0) template with generous cooldowns/sight, installed as
/// `EnemyType.types[0]` (the `.evt` boss parse that would fill it is DEFERRED). A boss
/// passes its `statRow` as the Enemy constructor's fourth arg, so a boss built with
/// `halfHeight = 0` reads this template.
fn install_template(g: &mut Game) {
    let template = EnemyTypeData {
        name: Vec::new(),
        size: 0,
        elem_color: 0,
        element: 0,
        ai_type: 0, // melee chaser
        relentless: false,
        armored: false,
        summons_allies: false,
        ambush: false,
        summon_ward_element: 0,
        level: 3,
        max_hp: 100,
        attack: 10,
        defense: 2,
        evasion: 5,
        sight_range: 8,
        attack_delay: 20,
        hurt_delay: 20,
        exp_reward: 50,
        drop_table: Vec::new(),
        walk_frames: 4,
        attack_frames: 3,
        cast_frames: 3,
        die_frames: 3,
    };
    enemy_type::alloc(&mut g.enemy_type, 1);
    g.enemy_type.types.as_mut().expect("types allocated")[0] = Some(template);
}

// --- Boss construction + AI tick ------------------------------------------------------

/// A constructed `Boss` records its `Enemy` stats, the overridden `layer`, and its
/// boss-specific `heroDistX`/`heroDistY` at zero; `boss::update` advances `animFrame`
/// and caches the hero tile-offset. An identical un-ticked boss (proven-red control)
/// keeps the defaults.
#[test]
fn boss_caches_hero_offset_when_ticked_control_stays() {
    let mut g = drive_to_world();
    install_template(&mut g);
    let hero = g.game_state.hero.expect("hero");
    let (hero_tx, hero_ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x, n.tile_y)
    };

    // Boss(tileX = hero_tx+3, tileY = hero_ty+2, halfWidth=0 (→ Enemy kind),
    //      halfHeight=0 (→ Enemy statRow), layer=2).
    let ticked = boss::new_boss(&mut g, hero_tx + 3, hero_ty + 2, 0, 0, 2);

    // A distinct node kind (Boss), not Enemy.
    assert_eq!(g.entity_arena[ticked].kind(), EntityKind::Boss);
    // Enemy "super" stats: hp = stats.maxHp, kind/statRow from the ctor args.
    {
        let e = g.entity_arena[ticked].as_enemy().expect("Boss enemy");
        assert_eq!(e.hp, 100, "hp = stats.maxHp");
        assert_eq!(e.kind, 0, "halfWidth passed as Enemy kind");
        assert_eq!(e.stat_row, 0, "halfHeight passed as Enemy statRow");
        assert_eq!(e.battler.state, 1, "idle Battler base");
        assert_eq!(e.battler.anim_frame, -1);
    }
    // Boss-specific: layer override (2, over the size-0 → 1 Enemy default) + zero offsets.
    assert_eq!(
        g.entity_arena[ticked].layer, 2,
        "Boss ctor overrode layer to its fifth arg"
    );
    {
        let b = g.entity_arena[ticked].as_boss().expect("Boss");
        assert_eq!(
            (b.hero_dist_x, b.hero_dist_y),
            (0, 0),
            "offsets zero at spawn"
        );
    }

    // Control boss at a distinct tile, NEVER ticked.
    let control = boss::new_boss(&mut g, hero_tx - 3, hero_ty - 2, 0, 0, 2);

    // Tick ONLY the first boss (Boss.update).
    boss::update(&mut g, ticked);

    // heroDistX/Y now cache the absolute tile distance to the hero (|±3|, |±2|).
    {
        let b = g.entity_arena[ticked].as_boss().expect("Boss");
        assert_eq!(b.hero_dist_x, 3, "heroDistX = |hero.tileX - boss.tileX|");
        assert_eq!(b.hero_dist_y, 2, "heroDistY = |hero.tileY - boss.tileY|");
    }
    assert_eq!(
        g.entity_arena[ticked]
            .as_enemy()
            .unwrap()
            .battler
            .anim_frame,
        0,
        "Boss.update advanced animFrame (-1 → 0)"
    );

    // Proven-red control: the un-ticked boss kept its spawn defaults.
    {
        let b = g.entity_arena[control].as_boss().expect("Boss");
        assert_eq!(
            (b.hero_dist_x, b.hero_dist_y),
            (0, 0),
            "un-ticked boss never cached an offset"
        );
    }
    assert_eq!(
        g.entity_arena[control]
            .as_enemy()
            .unwrap()
            .battler
            .anim_frame,
        -1,
        "un-ticked boss kept animFrame -1"
    );
}

/// `game_map::draw_entities` dispatches a `Boss` node to `boss::paint` (the virtual
/// override). The boss sprite bank is DEFERRED-loaded, so it draws nothing, but the
/// dispatch + `bossFrames`-null fallback run without panicking.
#[test]
fn boss_paint_dispatch_runs() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // An on-screen boss linked into the map's draw list.
    let id = boss::new_boss(&mut g, 5, 5, 0, 0, 1); // tile (5,5)
    game_map::add_entity(&mut g, id);

    // drawEntities walks the z-list; the Boss arm routes to boss::paint (no panic).
    game_map::draw_entities(&mut g, 0, 0);

    // The boss is still linked (paint did not remove it).
    assert_eq!(g.entity_arena[id].kind(), EntityKind::Boss);
}

// --- GuardianCastFx lifetime ---------------------------------------------------------

/// A `GuardianCastFx` overlay ticks its `frame` each `draw_floaters` paint and is
/// reaped when `frame` reaches its `lifetime`; an un-painted one (proven-red control)
/// stays at frame 0, not finished.
#[test]
fn guardian_cast_fx_ticks_to_expiry_control_stays() {
    let mut g = Game::new();
    // Inject the element atlas (spriteBanks[12]) the ctor captures + the base pose
    // draws (slots 0/1 for the beam ctor read, 7/8/9 for the base-pose blits). The
    // DEFERRED loadElementAtlas would fill this in real play. guardianFrames is already
    // the <clinit> Object[3] from AssetCacheState::new.
    let sprite = Image::from_argb(4, 4, vec![0xff00_0000u32; 16]).unwrap();
    g.asset_cache.sprite_banks[12] = Some((0..13).map(|_| Some(sprite.clone())).collect());

    // GuardianCastFx(startDelay=0, lifetime=3, guardianType=0, skillSlot=0) — the base
    // summon pose (elementSprites[7..9]).
    let fx = guardian_cast_fx::new(&g, 0, 3, 0, 0);
    assert_eq!(fx.lifetime, 3, "lifetime from the constructor");
    assert_eq!(fx.frame, 0);
    assert!(!fx.finished);
    assert!(matches!(fx.data, OverlayData::GuardianCastFx(_)));

    // Carry it in a battler's floater list (as Guardian does), and drive it via
    // draw_floaters (the overlay-union paint path).
    let mut b = BattlerData::new();
    battler::add_floater(&mut b, fx);
    // A control FX that is NEVER painted.
    let control = guardian_cast_fx::new(&g, 0, 3, 0, 0);

    let mut fb = Image::create_mutable(64, 64).unwrap();

    // Two paints: frame 2, still short of the lifetime → not reaped.
    {
        let mut gr = Graphics::new(&mut fb);
        for _ in 0..2 {
            battler::draw_floaters(&mut b, &mut gr, 32, 40);
        }
    }
    assert_eq!(b.floaters.len(), 1, "not reaped before its lifetime");
    assert_eq!(b.floaters[0].frame, 2, "paint advanced the frame counter");
    assert!(!b.floaters[0].finished);

    // The third paint reaches the lifetime (frame 3 >= 3) → finished → reaped in place.
    {
        let mut gr = Graphics::new(&mut fb);
        battler::draw_floaters(&mut b, &mut gr, 32, 40);
    }
    assert!(
        b.floaters.is_empty(),
        "draw_floaters reaped the finished GuardianCastFx at expiry"
    );

    // Proven-red control: the un-painted FX never advanced or finished.
    assert_eq!(
        control.frame, 0,
        "un-painted GuardianCastFx stays at frame 0"
    );
    assert!(
        !control.finished,
        "un-painted GuardianCastFx never finishes"
    );
}
