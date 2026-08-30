//! Unit + AI gate for the COMBAT FOUNDATION: the `EnemyType` (`j`) stat template and
//! the `Enemy` (`al`) hostile monster actor (a `Battler`).
//!
//! Two levels of drive, both over a live New-Game world (an `Enemy`'s constructor
//! registers occupancy, so `GameState.map` must exist):
//!
//! * **Construction** — an `Enemy` built from an installed `EnemyType` records its
//!   template stats (hp/kind/statRow/cooldowns), its idle `Battler` base (state 1,
//!   facing 2, moveDir 2, animFrame -1), its home tile and registered occupancy.
//! * **AI tick** — `enemy::update` runs the AI FSM. Two guardian-free demonstrations
//!   that the enemy *acts*, each paired with a proven-red control (an identically-set
//!   enemy that is NOT ticked stays put):
//!     - a KNOCKBACK enemy (state 4) advances its position 8px (updateAi case 4 +
//!       stepIfMoving + Battler.move);
//!     - an IDLE enemy with the hero in sight and its attack cooldown ready ACQUIRES a
//!       target (updateAi case 1 → tryAttack → pickTarget).
//!
//! The pursuit *step* (`Battler.approach`), the guardian-target branches and
//! `Hero.takeHit` are DEFERRED (see `enemy.rs`), so what is asserted is the ported,
//! guardian-independent AI: knockback movement and target acquisition.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::battler::STATE_KNOCKBACK;
use heroes_lore_wind_of_soltia_game_xlat::enemy_type::EnemyTypeData;
use heroes_lore_wind_of_soltia_game_xlat::entity::{EntityId, EntityKind};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, enemy, enemy_type, font_manager, game_loop, game_map, game_midlet,
    game_state, title_screen, Game,
};

// --- Shared New-Game → world drive (mirrors effect_projectile_npc.rs) --------------

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
/// `EnemyType.types[0]` (the `.evt` enemy parse that would fill it is DEFERRED).
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

/// The occupant handle in the map occupancy grid at (`tile_x`,`tile_y`).
fn occupant(g: &Game, tile_x: usize, tile_y: usize) -> Option<EntityId> {
    g.game_state
        .map
        .as_ref()
        .unwrap()
        .occupancy
        .as_ref()
        .unwrap()[tile_y][tile_x]
}

// --- Construction -----------------------------------------------------------------

/// A constructed `Enemy` records its template stats, idle `Battler` base, home tile,
/// and registered occupancy.
#[test]
fn enemy_has_initial_state() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // Enemy(short pixelX=160, short pixelY=160, byte kind=0, byte statRow=0)  → tile (10,10).
    let id = enemy::new_enemy(&mut g, 160, 160, 0, 0);

    assert_eq!(g.entity_arena[id].kind(), EntityKind::Enemy);
    let e = g.entity_arena[id].as_enemy().expect("Enemy");
    // template-derived stats.
    assert_eq!(e.kind, 0);
    assert_eq!(e.stat_row, 0);
    assert_eq!(e.hp, 100, "hp = stats.maxHp");
    assert_eq!(e.stats.max_hp, 100);
    assert_eq!(e.attack_cooldown, 20, "attackCooldown = stats.attackDelay");
    assert_eq!(e.hurt_cooldown, 20, "hurtCooldown = stats.hurtDelay");
    assert_eq!(e.summon_timer, -10);
    assert!(!e.aggroed);
    assert!(!e.hidden, "not an ambusher → not hidden");
    assert!(e.on_screen, "constructed onScreen");
    assert_eq!(e.target, None, "no target at spawn");
    assert_eq!(
        (e.home_tile_x, e.home_tile_y),
        (10, 10),
        "home tile = spawn tile"
    );
    // idle Battler base: init() → state 1, facing 2, moveDir 2, animFrame -1.
    assert_eq!(e.battler.state, 1, "idle");
    assert_eq!(e.battler.facing, 2);
    assert_eq!(e.battler.move_dir, 2);
    assert_eq!(e.battler.anim_frame, -1);
    // Entity base: pixel/tile/half/layer.
    let n = &g.entity_arena[id];
    assert_eq!((n.pixel_x, n.pixel_y), (160, 160));
    assert_eq!((n.tile_x, n.tile_y), (10, 10));
    assert_eq!((n.half_w, n.half_h), (8, 8), "Enemy half size 8×8");
    assert_eq!(n.layer, 1, "size != 2 → layer 1");
    // The final setPixelPos registered occupancy.
    assert_eq!(
        occupant(&g, 10, 10),
        Some(id),
        "enemy registered its footprint"
    );
}

// --- AI tick: knockback movement --------------------------------------------------

/// Pixel coords of a walkable tile whose downward neighbor is also open ground (both
/// unoccupied), scanned from the live collision grid. `Battler.tryStepForward` now
/// consults the real `/m/6/00.evt` collision (parsed by `game_map::load`, wired
/// through `battler::try_step_forward`), so a knockback step only lands on open
/// ground — the former fixed (10,10)→(10,11) is a wall in this map. Faithful, not a
/// weakened assertion: the enemy still steps; it just has to step onto open ground.
fn open_down_pair(g: &Game) -> (i16, i16) {
    let (w, h) = {
        let m = g.game_state.map.as_ref().expect("GameState.map");
        (m.width_tiles, m.height_tiles)
    };
    for ty in 1..(h - 1) {
        for tx in 1..(w - 1) {
            if game_map::is_walkable(g, tx, ty) && game_map::is_walkable(g, tx, ty + 1) {
                return ((tx * 16) as i16, (ty * 16) as i16);
            }
        }
    }
    panic!("no open vertical tile-pair found in the loaded map");
}

/// A KNOCKBACK enemy (state 4) advances 8px when its AI is ticked; an identical enemy
/// left un-ticked (the proven-red control) does not move.
#[test]
fn enemy_knockback_tick_moves_control_stays() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // Control enemy at a distinct tile (14,14), same knockback setup, NEVER ticked.
    // (It never steps, so its downward tile need not be open.) Spawned first so its
    // occupancy is excluded from the open-ground scan below.
    let control = enemy::new_enemy(&mut g, 224, 224, 0, 0);
    {
        let e = g.entity_arena[control].as_enemy_mut().unwrap();
        e.battler.state = STATE_KNOCKBACK;
        e.battler.knockback_timer = 2;
    }
    // Ticked enemy on open ground whose DOWN neighbor is also open, pushed into
    // knockback (facing 2 = down by default), so the collision-governed step lands.
    let (ex, ey) = open_down_pair(&g);
    let ticked = enemy::new_enemy(&mut g, ex, ey, 0, 0);
    {
        let e = g.entity_arena[ticked].as_enemy_mut().unwrap();
        e.battler.state = STATE_KNOCKBACK;
        e.battler.knockback_timer = 2;
    }

    let before_ticked = g.entity_arena[ticked].pixel_y;
    let before_control = g.entity_arena[control].pixel_y;

    // Tick ONLY the first enemy's AI.
    enemy::update(&mut g, ticked);

    // updateAi case 4 (kt 2 → 1, still state 4) + stepIfMoving → Battler.move(8) down.
    assert_eq!(
        g.entity_arena[ticked].pixel_y,
        before_ticked + 8,
        "ticked knockback enemy stepped 8px down"
    );
    assert_eq!(
        g.entity_arena[ticked]
            .as_enemy()
            .unwrap()
            .battler
            .knockback_timer,
        1,
        "knockbackTimer decremented"
    );
    // Proven-red control: the un-ticked enemy is exactly where it spawned.
    assert_eq!(
        g.entity_arena[control].pixel_y, before_control,
        "the un-ticked enemy stayed put"
    );
}

// --- AI tick: target acquisition --------------------------------------------------

/// An IDLE enemy with the hero in sight range and its attack cooldown ready ACQUIRES a
/// target when ticked (updateAi case 1 → tryAttack → pickTarget → hero); an identical
/// enemy left un-ticked (the proven-red control) has no target.
#[test]
fn enemy_targets_hero_when_ticked_control_untargeted() {
    let mut g = drive_to_world();
    install_template(&mut g);
    let hero = g.game_state.hero.expect("hero");

    // Spawn near the hero's actual start tile (the .evt setHeroTile placed it), a couple
    // of tiles apart along each axis (well within the template's sight range of 8).
    let (hero_tx, hero_ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x as i16, n.tile_y as i16)
    };
    // Ticked enemy two tiles right + two down from the hero; ready to attack this frame.
    let ticked = enemy::new_enemy(&mut g, (hero_tx + 2) * 16, (hero_ty + 2) * 16, 0, 0);
    g.entity_arena[ticked]
        .as_enemy_mut()
        .unwrap()
        .attack_cooldown = 0;
    // Control enemy, same setup, at a distinct tile, NEVER ticked.
    let control = enemy::new_enemy(&mut g, (hero_tx + 4) * 16, (hero_ty + 4) * 16, 0, 0);
    g.entity_arena[control]
        .as_enemy_mut()
        .unwrap()
        .attack_cooldown = 0;

    assert_eq!(
        g.entity_arena[ticked].as_enemy().unwrap().target,
        None,
        "no target before the tick"
    );

    // Tick ONLY the first enemy's AI.
    enemy::update(&mut g, ticked);

    // pickTarget: aggroed=false → reach = sightRange(8); hero within 2 tiles each axis → target = hero.
    assert_eq!(
        g.entity_arena[ticked].as_enemy().unwrap().target,
        Some(hero),
        "ticked enemy acquired the hero as its AI target"
    );
    // Proven-red control: the un-ticked enemy never targeted anyone.
    assert_eq!(
        g.entity_arena[control].as_enemy().unwrap().target,
        None,
        "the un-ticked enemy has no target"
    );
}

// --- Paint dispatch ---------------------------------------------------------------

/// `game_map::draw_entities` dispatches to `enemy::paint`, which blits the ground
/// shadow onto the framebuffer (the enemy sprite is DEFERRED, but the shadow is ported).
#[test]
fn enemy_paint_draws_the_ground_shadow() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // An on-screen enemy linked into the map's draw list.
    let id = enemy::new_enemy(&mut g, 80, 80, 0, 0); // tile (5,5)
    game_map::add_entity(&mut g, id);

    let before = g.screen.as_ref().expect("framebuffer").pixels().to_vec();
    // drawEntities walks the z-list and paints each entity at the camera offset (0,0).
    game_map::draw_entities(&mut g, 0, 0);
    let after = g.screen.as_ref().expect("framebuffer").pixels().to_vec();

    assert_ne!(
        before, after,
        "Enemy.paint blitted the entity shadow onto the framebuffer"
    );
}
