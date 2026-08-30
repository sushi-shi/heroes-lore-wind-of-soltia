//! Unit gate for the **Nord** boss family that extends the abstract `Boss` (`av`):
//! `NordBody1` (`ar`), `NordBody2` (`ag`), `NordHealer` (`cd`), `NordTentacle` (`bd`).
//! Each is modelled as a `Boss` node carrying a `boss::BossSubclass` tag (no new
//! `EntityData` variant); the `boss` dispatchers route the virtual overrides to the
//! subclass functions.
//!
//! * **Construction records the tag + subclass fields + overridden layer.** `NordBody1`
//!   is a unit tag (no fields), layer 1; `NordBody2` (layer 3) and `NordHealer` (layer 2)
//!   start their companion handles `null` (assigned by `setParts`); `NordHealer` starts
//!   `healRotation = 0`; `NordTentacle` (layer 2) starts its marked tile at `(0, 0)`.
//! * **`onDeath` dispatches to the subclass hook, vs the base `Boss` no-op.** A base `Boss`
//!   node's `onDeath` leaves `deathTimer` untouched; `NordBody1`/`NordHealer`/`NordTentacle`
//!   reset it to 0 (proven vs a seeded sentinel), and `NordBody2` arms it to 24 AND
//!   despawns both linked companion parts (proven by a before/after list check).
//! * **A single AI tick runs through the virtual dispatch.** `boss::update` routes each
//!   node to its tick (`NordBody1` re-hosts the inherited `Boss.update`; the other three
//!   override it), advancing `animFrame` (-1 → 0) and staying idle under generous
//!   cooldowns. Paired with a proven-red control: an un-ticked Nord keeps `animFrame = -1`.
//!
//! DEFERRED per the subclass modules (never reached by these assertions): the
//! `GameMap.spawnNordBoss` phase-2 spawn (`NordBody1.die`), `EventScript.fire`
//! (`NordBody2.die`), `Hero.takeHit` (`NordTentacle.resolveAttack`), and the DEFERRED-loaded
//! `AssetCache.bossFrames`/`attackEffectScripts` banks.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::boss::{self, BossSubclass};
use heroes_lore_wind_of_soltia_game_xlat::enemy_type::EnemyTypeData;
use heroes_lore_wind_of_soltia_game_xlat::entity::{EntityId, EntityKind};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, enemy_type, font_manager, game_loop, game_map, game_midlet, game_state,
    nord_body1, nord_body2, nord_healer, nord_tentacle, title_screen, Game,
};

// --- Shared New-Game → world drive (mirrors boss_subclasses.rs) -----------------------

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

/// A plain melee template installed as `EnemyType.types[0]` (the `.evt` boss parse that
/// would fill it is DEFERRED). Every Nord part here is built with `statRow = 0`, so its
/// `Enemy` "super" reads this template. Generous cooldowns keep a single tick idle.
fn install_template(g: &mut Game) {
    let template = EnemyTypeData {
        name: Vec::new(),
        size: 0,
        elem_color: 0,
        element: 0,
        ai_type: 0,
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

/// Whether `target` is still linked in the map's z-sorted entity list.
fn list_contains(g: &Game, target: EntityId) -> bool {
    let mut cursor = g.game_state.map.as_ref().expect("map").entities.head;
    while let Some(id) = cursor {
        if id == target {
            return true;
        }
        cursor = g.entity_arena[id].next;
    }
    false
}

// --- Construction: tag + subclass fields ---------------------------------------------

/// Each Nord constructor produces a `Boss` node tagged with its concrete variant, records
/// its subclass fields (companion handles `null` until `setParts`), and applies the
/// overridden `layer` (the 5th `Boss`-ctor arg).
#[test]
fn subclass_construction_records_tag_and_fields() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // NordBody1(tileX=9, tileY=5, kind=0, statRow=0) — layer 1, unit tag (no fields).
    let body1 = nord_body1::new(&mut g, 9, 5, 0, 0);
    assert_eq!(g.entity_arena[body1].kind(), EntityKind::Boss);
    assert_eq!(g.entity_arena[body1].layer, 1, "NordBody1 super(...,1)");
    assert_eq!(
        g.entity_arena[body1].as_enemy().unwrap().hp,
        100,
        "Enemy super hp = maxHp"
    );
    assert!(
        matches!(
            g.entity_arena[body1].as_boss().unwrap().subclass,
            BossSubclass::NordBody1
        ),
        "NordBody1 is a unit tag"
    );

    // NordBody2(tileX=9, tileY=5, kind=0, statRow=0) — layer 3, companion handles null.
    let body2 = nord_body2::new(&mut g, 9, 5, 0, 0);
    assert_eq!(g.entity_arena[body2].layer, 3, "NordBody2 super(...,3)");
    match &g.entity_arena[body2].as_boss().unwrap().subclass {
        BossSubclass::NordBody2(d) => {
            assert!(d.healer.is_none(), "healer null before setParts");
            assert!(d.striker.is_none(), "striker null before setParts");
        }
        _ => panic!("expected a NordBody2 tag"),
    }

    // NordTentacle(tileX=6, tileY=5, kind=0, statRow=0) — layer 2, marked tile (0,0).
    let striker = nord_tentacle::new(&mut g, 6, 5, 0, 0);
    assert_eq!(
        g.entity_arena[striker].layer, 2,
        "NordTentacle super(...,2)"
    );
    match &g.entity_arena[striker].as_boss().unwrap().subclass {
        BossSubclass::NordTentacle(d) => {
            assert_eq!(d.marked_tile_x, 0, "markedTileX starts at 0");
            assert_eq!(d.marked_tile_y, 0, "markedTileY starts at 0");
        }
        _ => panic!("expected a NordTentacle tag"),
    }

    // NordHealer(tileX=13, tileY=5, kind=0, statRow=0) — layer 2, healRotation 0, handles null.
    let healer = nord_healer::new(&mut g, 13, 5, 0, 0);
    assert_eq!(g.entity_arena[healer].layer, 2, "NordHealer super(...,2)");
    match &g.entity_arena[healer].as_boss().unwrap().subclass {
        BossSubclass::NordHealer(d) => {
            assert!(d.body.is_none(), "body null before setParts");
            assert!(d.striker.is_none(), "striker null before setParts");
            assert_eq!(d.heal_rotation, 0, "healRotation starts at 0");
        }
        _ => panic!("expected a NordHealer tag"),
    }

    // setParts links the phase-2 trio (mirrors GameMap.spawnNordBoss(false)).
    nord_healer::set_parts(&mut g, healer, body2, striker);
    nord_body2::set_parts(&mut g, body2, healer, striker);
    match &g.entity_arena[body2].as_boss().unwrap().subclass {
        BossSubclass::NordBody2(d) => {
            assert_eq!(d.healer, Some(healer), "core records its healer");
            assert_eq!(d.striker, Some(striker), "core records its striker");
        }
        _ => panic!("expected a NordBody2 tag"),
    }
    match &g.entity_arena[healer].as_boss().unwrap().subclass {
        BossSubclass::NordHealer(d) => {
            assert_eq!(d.body, Some(body2), "healer records its core");
            assert_eq!(d.striker, Some(striker), "healer records its striker");
        }
        _ => panic!("expected a NordHealer tag"),
    }
}

// --- onDeath dispatch (subclass hook vs base no-op) ----------------------------------

/// `boss::on_death` dispatches to each Nord subclass's override: `NordBody1`/`NordHealer`/
/// `NordTentacle` reset `deathTimer` to 0 (proven vs a seeded sentinel), and `NordBody2`
/// arms it to 24 while despawning both companion parts. A base `Boss` node's `onDeath`
/// is the abstract no-op (proven-red control).
#[test]
fn on_death_dispatches_to_subclass_hook_control_base_noop() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // Proven-red control: the abstract Boss.onDeath leaves deathTimer untouched.
    let base = boss::new_boss(&mut g, 6, 10, 0, 0, 1);
    g.entity_arena[base].as_enemy_mut().unwrap().death_timer = 7;
    boss::on_death(&mut g, base);
    assert_eq!(
        g.entity_arena[base].as_enemy().unwrap().death_timer,
        7,
        "base Boss.onDeath is the abstract no-op (sentinel survives)"
    );

    // NordBody1.onDeath → deathTimer = 0 (seed a sentinel so the reset is observable).
    let body1 = nord_body1::new(&mut g, 9, 5, 0, 0);
    g.entity_arena[body1].as_enemy_mut().unwrap().death_timer = 99;
    boss::on_death(&mut g, body1);
    assert_eq!(
        g.entity_arena[body1].as_enemy().unwrap().death_timer,
        0,
        "NordBody1.onDeath reset deathTimer to 0 (not the sentinel)"
    );

    // NordHealer.onDeath → deathTimer = 0.
    let healer = nord_healer::new(&mut g, 13, 5, 0, 0);
    g.entity_arena[healer].as_enemy_mut().unwrap().death_timer = 99;
    boss::on_death(&mut g, healer);
    assert_eq!(
        g.entity_arena[healer].as_enemy().unwrap().death_timer,
        0,
        "NordHealer.onDeath reset deathTimer to 0 (not the sentinel)"
    );

    // NordTentacle.onDeath → deathTimer = 0.
    let striker = nord_tentacle::new(&mut g, 6, 5, 0, 0);
    g.entity_arena[striker].as_enemy_mut().unwrap().death_timer = 99;
    boss::on_death(&mut g, striker);
    assert_eq!(
        g.entity_arena[striker].as_enemy().unwrap().death_timer,
        0,
        "NordTentacle.onDeath reset deathTimer to 0 (not the sentinel)"
    );

    // NordBody2.onDeath → deathTimer = 24 AND despawns both linked companion parts.
    let core = nord_body2::new(&mut g, 9, 5, 0, 0);
    let hh = nord_healer::new(&mut g, 13, 5, 0, 0);
    let ss = nord_tentacle::new(&mut g, 6, 5, 0, 0);
    nord_body2::set_parts(&mut g, core, hh, ss);
    game_map::add_entity(&mut g, core);
    game_map::add_entity(&mut g, hh);
    game_map::add_entity(&mut g, ss);
    assert!(
        list_contains(&g, hh) && list_contains(&g, ss),
        "both companion parts linked before onDeath"
    );
    boss::on_death(&mut g, core);
    assert_eq!(
        g.entity_arena[core].as_enemy().unwrap().death_timer,
        24,
        "NordBody2.onDeath armed deathTimer = 24"
    );
    assert!(
        !list_contains(&g, hh),
        "NordBody2.onDeath despawned the healer"
    );
    assert!(
        !list_contains(&g, ss),
        "NordBody2.onDeath despawned the striker"
    );
}

// --- A single AI tick through the virtual dispatch -----------------------------------

/// `boss::update` routes each Nord node to its tick, advancing `animFrame` from the -1
/// spawn default to 0 and leaving it idle (generous cooldowns). Paired with a proven-red
/// control: an un-ticked Nord stays -1.
#[test]
fn nord_update_dispatches_one_tick_control_stays() {
    let mut g = drive_to_world();
    install_template(&mut g);

    let ticked = [
        nord_body1::new(&mut g, 9, 5, 0, 0),
        nord_body2::new(&mut g, 9, 5, 0, 0),
        nord_tentacle::new(&mut g, 6, 5, 0, 0),
        nord_healer::new(&mut g, 13, 5, 0, 0),
    ];

    // A proven-red control: an identically-built NordBody1 that is NEVER ticked.
    let control = nord_body1::new(&mut g, 10, 10, 0, 0);

    for &id in &ticked {
        assert_eq!(
            g.entity_arena[id].as_enemy().unwrap().battler.anim_frame,
            -1,
            "animFrame is the -1 spawn default before ticking"
        );
    }

    // One tick each, through the virtual boss::update dispatch (no panic).
    for &id in &ticked {
        boss::update(&mut g, id);
        assert_eq!(
            g.entity_arena[id].as_enemy().unwrap().battler.anim_frame,
            0,
            "boss::update advanced animFrame (-1 -> 0)"
        );
        // Each stayed idle (generous cooldowns) — the single tick did not enter combat.
        assert_eq!(
            g.entity_arena[id].as_enemy().unwrap().battler.state,
            1,
            "still idle after one tick"
        );
    }

    // Proven-red control: the un-ticked NordBody1 kept the spawn default.
    assert_eq!(
        g.entity_arena[control]
            .as_enemy()
            .unwrap()
            .battler
            .anim_frame,
        -1,
        "un-ticked Nord kept animFrame -1"
    );
}
