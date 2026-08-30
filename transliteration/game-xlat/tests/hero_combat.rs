//! Combat-FSM gate for the `Hero` (`ao`): the attack states of `Hero.update`
//! (`advanceCombo` → `performAttack` → `attackClass6`) and the incoming-damage
//! resolver `Hero.takeHit`, both over a live New-Game world (the hero + an `Enemy`
//! share the map occupancy grid, so `GameState.map` must exist).
//!
//! Two guardian-free drives, each paired with a proven-red control:
//!
//! * **The hero strikes** — a warrior faces an adjacent `Enemy`, queues a light combo
//!   step and enters state 3; ticking `Hero.update` to the warrior's strike frame runs
//!   `advanceCombo` → `performAttack`, which spends MP and (with an enemy directly
//!   ahead) floats a weapon-proc. The enemy-side damage application is
//!   `Enemy.takeHeroHit`, whose body is the *enemy lane's* own DEFERRED slice, so the
//!   observable hero-side evidence the strike resolved is the spawned floater + spent
//!   MP. Control: a no-attack idle tick spends no MP and floats nothing.
//! * **The hero is struck** — an `Enemy` hit is resolved into `Hero.takeHit` (exactly
//!   what `Enemy.resolveAttack` calls); a lethal blow drops the hero's HP to 0, spawns
//!   damage floaters, and drives the death state (6). A following tick with the death
//!   timer elapsed runs the case-6 `onDeath`, queuing the game-over state (16). Control:
//!   a no-attack idle tick leaves HP unchanged.
//!
//! The RNG is pinned by re-seeding `ByteUtil.rng` right before each drive to a seed
//! searched at test time (proc fires / dodge misses), so the gate is deterministic.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::enemy_type::EnemyTypeData;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, enemy, enemy_type, font_manager, game_loop, game_midlet, game_state,
    hero, title_screen, Game,
};

// --- Shared New-Game → world drive (mirrors tests/enemy.rs) -------------------------

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

/// A plain melee (aiType 0) template with generous stats, installed as
/// `EnemyType.types[0]` (the `.evt` enemy parse that would fill it is DEFERRED).
fn install_template(g: &mut Game) {
    let template = EnemyTypeData {
        name: Vec::new(),
        size: 0,
        elem_color: 0,
        element: 0,
        ai_type: 0, // melee chaser (no poison proc on takeHit)
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

/// Smallest non-negative seed whose FIRST `ByteUtil.randRange(0, 99)` is `< max` —
/// used to force `rollProc`'s proc roll under the proc chance.
fn seed_first_roll_below(max: i32) -> i64 {
    for s in 0..1_000_000i64 {
        let mut st = byte_util::ByteUtilState::seeded(s);
        if byte_util::rand_range(&mut st, 0, 99) < max {
            return s;
        }
    }
    panic!("no seed produced a first randRange(0,99) < {max}");
}

/// Smallest non-negative seed whose FIRST `ByteUtil.randRange(0, 99)` is `>= min` —
/// used to force `takeHit`'s dodge roll to miss the dodge window.
fn seed_first_roll_at_least(min: i32) -> i64 {
    for s in 0..1_000_000i64 {
        let mut st = byte_util::ByteUtilState::seeded(s);
        if byte_util::rand_range(&mut st, 0, 99) >= min {
            return s;
        }
    }
    panic!("no seed produced a first randRange(0,99) >= {min}");
}

// --- The hero strikes ---------------------------------------------------------------

/// A warrior with a proc weapon and an adjacent enemy, driven to the strike frame,
/// resolves `advanceCombo` → `performAttack`: it spends MP and floats the weapon proc.
/// Proven-red control: a no-attack idle tick spends no MP and floats nothing.
#[test]
fn hero_strike_spends_mp_and_floats_a_proc() {
    let mut g = drive_to_world();
    install_template(&mut g);
    let hero = g.game_state.hero.expect("hero");

    // Combat must be enabled for queueComboStep (the world clip may have disabled it).
    g.game_state.map.as_mut().expect("map").combat_enabled = true;

    // Face down and place an enemy on the tile directly ahead (enemyAhead's scan tile).
    {
        let h = g.entity_arena[hero].as_hero_mut().expect("hero");
        h.battler.facing = 2; // down
    }
    let (tx, ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x as i16, n.tile_y as i16)
    };
    let _enemy = enemy::new_enemy(&mut g, tx * 16, (ty + 1) * 16, 0, 0);

    // Give the weapon a proc attribute (3 → Armor.PROC_CHANCE 13) so a forced low proc
    // roll floats an emote (Floater kind 10) in performAttack.
    g.entity_arena[hero].as_hero().expect("hero").equipment[0]
        .as_ref()
        .expect("weapon slot")
        .borrow_mut()
        .attribute = 3;

    let max_mp = g.entity_arena[hero].as_hero().expect("hero").max_mp;

    // CONTROL (proven-red): an idle, no-attack tick spends no MP and floats nothing.
    {
        let h = g.entity_arena[hero].as_hero_mut().expect("hero");
        h.battler.state = 1;
        h.hp = h.max_hp;
        h.mp = h.max_mp;
    }
    hero::update(&mut g, hero);
    assert_eq!(
        g.entity_arena[hero].as_hero().expect("hero").mp,
        max_mp,
        "an idle (no-attack) tick spends no MP"
    );
    assert!(
        g.entity_arena[hero]
            .as_hero()
            .expect("hero")
            .battler
            .floaters
            .is_empty(),
        "an idle (no-attack) tick floats nothing"
    );

    // Queue a light combo step and enter the attack state (as the key handler would).
    assert!(
        hero::queue_combo_step(&mut g, hero, false),
        "queued a light combo step"
    );
    {
        let h = g.entity_arena[hero].as_hero_mut().expect("hero");
        h.battler.state = 3;
        h.battler.anim_frame = -1;
        h.combo_index = -1;
        h.mp = h.max_mp;
    }

    // Pin the proc: rollProc is the first ByteUtil consumer after this re-seed.
    g.byte_util = byte_util::ByteUtilState::seeded(seed_first_roll_below(13));

    // Warrior fires performAttack at animFrame == 2 → three update ticks (-1→0→1→2).
    for _ in 0..3 {
        hero::update(&mut g, hero);
    }

    let h = g.entity_arena[hero].as_hero().expect("hero");
    assert!(
        !h.battler.floaters.is_empty(),
        "the strike floated a weapon proc (enemy ahead → performAttack resolved)"
    );
    assert!(h.mp < max_mp, "the strike spent MP (advanceCombo → addMp)");
    assert_eq!(h.battler.state, 3, "the hero is still mid-combo (state 3)");
}

// --- The hero is struck -------------------------------------------------------------

/// An `Enemy` hit resolved into `Hero.takeHit` drops the hero's HP, floats the damage,
/// and — when lethal — enters death (state 6); a following elapsed-timer tick runs the
/// case-6 `onDeath` (game-over state 16). Proven-red control: a no-attack idle tick
/// leaves HP unchanged.
#[test]
fn hero_takehit_drops_hp_floats_damage_and_dies() {
    let mut g = drive_to_world();
    install_template(&mut g);
    let hero = g.game_state.hero.expect("hero");

    // A lethal, non-dodgeable, non-poison attacker placed clear of the hero.
    let (tx, ty) = {
        let n = &g.entity_arena[hero];
        (n.tile_x as i16, n.tile_y as i16)
    };
    let attacker = enemy::new_enemy(&mut g, tx * 16, (ty + 3) * 16, 0, 0);
    {
        let e = g.entity_arena[attacker].as_enemy_mut().expect("enemy");
        e.stats.attack = 9999; // one-shot lethal
        e.stats.evasion = 0;
        e.stats.ai_type = 0; // no poison proc
    }

    // Baseline: a living, full-HP hero.
    {
        let h = g.entity_arena[hero].as_hero_mut().expect("hero");
        h.battler.state = 1;
        h.hp = h.max_hp;
    }
    let hp_before = g.entity_arena[hero].as_hero().expect("hero").hp;
    let floaters_before = g.entity_arena[hero]
        .as_hero()
        .expect("hero")
        .battler
        .floaters
        .len();

    // CONTROL (proven-red): a no-attack idle tick leaves HP unchanged and alive.
    hero::update(&mut g, hero);
    assert_eq!(
        g.entity_arena[hero].as_hero().expect("hero").hp,
        hp_before,
        "a no-attack tick leaves the hero's HP unchanged"
    );
    assert_eq!(
        g.entity_arena[hero].as_hero().expect("hero").battler.state,
        1,
        "the hero is still alive after the idle tick"
    );

    // Resolve the enemy hit into Hero.takeHit — pin the dodge roll to a miss.
    g.byte_util = byte_util::ByteUtilState::seeded(seed_first_roll_at_least(60));
    hero::take_hit(&mut g, hero, attacker, 2);

    {
        let h = g.entity_arena[hero].as_hero().expect("hero");
        assert!(h.hp < hp_before, "the hit dropped the hero's HP");
        assert_eq!(h.hp, 0, "the lethal hit drove HP to 0");
        assert_eq!(h.battler.state, 6, "HP→0 set the death state (6)");
        assert_eq!(h.death_timer, 24, "death timer armed to 24");
        assert!(
            h.battler.floaters.len() > floaters_before,
            "the hit spawned damage floaters"
        );
        assert_eq!(h.recoil_dir, 2, "recoil direction recorded from the hit");
    }

    // Death FSM: with the timer elapsed, update case 6 runs onDeath → requestState(16).
    {
        let h = g.entity_arena[hero].as_hero_mut().expect("hero");
        h.death_timer = 0;
    }
    hero::update(&mut g, hero);
    assert_eq!(
        g.game_state.next_state, 16,
        "onDeath queued the game-over state (16)"
    );
}
