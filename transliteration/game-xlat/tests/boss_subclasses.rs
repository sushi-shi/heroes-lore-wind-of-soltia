//! Unit gate for the CONCRETE BOSS SUBCLASSES that extend the abstract `Boss` (`av`):
//! the solo `RockyBoss` (`cc`) and the three-part `Geb` family — `GebCore` (`bv`),
//! `GebHead` (`cg`), `GebHandLeft` (`ba`), `GebHandRight` (`ak`). Each is modelled as a
//! `Boss` node carrying a `boss::BossSubclass` tag (no new `EntityData` variant); the
//! `boss` dispatchers route the virtual overrides to the subclass functions.
//!
//! * **Construction records the tag + subclass fields.** Each subclass constructor runs
//!   `super(...)` ([`boss::new_boss`]) then rewrites the tag: `RockyBoss` selects its
//!   first attack pattern (`attackPattern = 1`); `GebHead` records its two owned hands;
//!   the hands anchor five/four rows down (`super(tileY+5)` / `super(tileY+4)`), arm the
//!   short `stats.attackDelay = 2`, and (right hand) start `swingSide = 2`. The overridden
//!   `layer` (the 5th `Boss`-ctor arg) is recorded over the size-derived default.
//! * **`onDeath` dispatches to the subclass hook, vs the base `Boss` no-op.** A base
//!   `Boss` node's `onDeath` leaves `deathTimer` untouched (the abstract no-op); each
//!   subclass's override arms it (`RockyBoss` 24, `GebCore` 16, `GebHead` 12, both hands
//!   0), and `GebHead.onDeath` additionally despawns both owned hands. Paired with the
//!   proven-red controls (the base no-op leaves the timer; the hands' reset-to-0 is proven
//!   by seeding a sentinel first).
//! * **A single AI tick runs through the virtual dispatch.** `boss::update` routes each
//!   node to its subclass tick (`RockyBoss`/`GebHead`/`GebHand*` have their own `update`;
//!   `GebCore` inherits `Boss.update`), advancing `animFrame` (-1 → 0). Paired with a
//!   proven-red control: an identically-built, un-ticked boss keeps `animFrame = -1`.
//!
//! DEFERRED per the subclass modules (never reached by these assertions): the
//! `EventScript.fire` story triggers, `GameMap.isWalkable`/`canOccupy`, `Hero.takeHit`/
//! `Hero.slide`, and the DEFERRED-loaded `AssetCache.bossExtraFrames`/`bossFrames`/
//! `attackEffectScripts` banks.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::boss::{self, BossSubclass};
use heroes_lore_wind_of_soltia_game_xlat::enemy_type::EnemyTypeData;
use heroes_lore_wind_of_soltia_game_xlat::entity::{EntityId, EntityKind};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, byte_util, enemy_type, font_manager, game_loop, game_map, game_midlet, game_state,
    geb_core, geb_hand_left, geb_hand_right, geb_head, rocky_boss, title_screen, Game,
};

// --- Shared New-Game → world drive (mirrors boss.rs / enemy.rs) ----------------------

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
/// would fill it is DEFERRED). Every subclass here is built with `statRow = 0`, so its
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

/// Each subclass constructor produces a `Boss` node tagged with its concrete variant,
/// records its subclass fields, and applies the overridden `layer` / anchor offset /
/// armed `stats.attackDelay`.
#[test]
fn subclass_construction_records_tag_and_fields() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // RockyBoss(tileX=6, tileY=6, kind=0, statRow=0) — layer 1, first pattern selected.
    let rocky = rocky_boss::new(&mut g, 6, 6, 0, 0);
    assert_eq!(g.entity_arena[rocky].kind(), EntityKind::Boss);
    assert_eq!(g.entity_arena[rocky].layer, 1, "RockyBoss super(...,1)");
    assert_eq!(
        g.entity_arena[rocky].as_enemy().unwrap().hp,
        100,
        "Enemy super hp = maxHp"
    );
    match &g.entity_arena[rocky].as_boss().unwrap().subclass {
        BossSubclass::RockyBoss(d) => {
            assert_eq!(d.pattern_index, 0, "patternIndex starts at 0");
            assert_eq!(
                d.attack_pattern, 1,
                "ctor selectAttackPattern(patternSequence[0]=1)"
            );
            assert_eq!(
                d.attack_frame_count, 0,
                "attackFrameCount DEFERRED (bossExtraFrames)"
            );
        }
        _ => panic!("expected a RockyBoss tag"),
    }

    // GebCore(tileX=6, tileY=8, kind=0, statRow=0) — layer 1, unit tag (no fields).
    let core = geb_core::new(&mut g, 6, 8, 0, 0);
    assert_eq!(g.entity_arena[core].layer, 1, "GebCore super(...,1)");
    assert!(
        matches!(
            g.entity_arena[core].as_boss().unwrap().subclass,
            BossSubclass::GebCore
        ),
        "GebCore is a unit tag"
    );

    // GebHandLeft(tileX=2, tileY=2, ...) — anchored at tileY+5 = 7, layer 2, attackDelay 2.
    let left = geb_hand_left::new(&mut g, 2, 2, 0, 0);
    assert_eq!(g.entity_arena[left].layer, 2, "GebHandLeft super(...,2)");
    assert_eq!(
        g.entity_arena[left].tile_y, 7,
        "super(tileX, (byte)(tileY+5), ...)"
    );
    assert_eq!(
        g.entity_arena[left].as_enemy().unwrap().stats.attack_delay,
        2,
        "ctor set stats.attackDelay = 2"
    );
    match &g.entity_arena[left].as_boss().unwrap().subclass {
        BossSubclass::GebHandLeft(d) => assert_eq!(d.ticks_since_hit, 0),
        _ => panic!("expected a GebHandLeft tag"),
    }

    // GebHandRight(tileX=12, tileY=2, ...) — anchored at tileY+4 = 6, layer 1, swingSide 2.
    let right = geb_hand_right::new(&mut g, 12, 2, 0, 0);
    assert_eq!(g.entity_arena[right].layer, 1, "GebHandRight super(...,1)");
    assert_eq!(
        g.entity_arena[right].tile_y, 6,
        "super(tileX, (byte)(tileY+4), ...)"
    );
    assert_eq!(
        g.entity_arena[right].as_enemy().unwrap().stats.attack_delay,
        2,
        "ctor set stats.attackDelay = 2"
    );
    match &g.entity_arena[right].as_boss().unwrap().subclass {
        BossSubclass::GebHandRight(d) => {
            assert_eq!(d.ticks_since_hit, 0);
            assert_eq!(d.swing_side, 2, "ctor set swingSide = 2");
        }
        _ => panic!("expected a GebHandRight tag"),
    }

    // GebHead(tileX=6, tileY=2, ..., leftHand, rightHand) — layer 2, owns both hands.
    let head = geb_head::new(&mut g, 6, 2, 0, 0, left, right);
    assert_eq!(g.entity_arena[head].layer, 2, "GebHead super(...,2)");
    match &g.entity_arena[head].as_boss().unwrap().subclass {
        BossSubclass::GebHead(d) => {
            assert_eq!(d.left_hand, left, "records its left hand");
            assert_eq!(d.right_hand, right, "records its right hand");
            assert!(!d.collision_sealed, "seal guard starts false");
            assert_eq!(d.attack_burst_count, 0);
        }
        _ => panic!("expected a GebHead tag"),
    }
}

// --- onDeath dispatch (subclass hook vs base no-op) ----------------------------------

/// `boss::on_death` dispatches to each subclass's override (arming its death timer, and
/// for `GebHead` despawning both hands), while a base `Boss` node's `onDeath` is the
/// abstract no-op. Proven-red controls anchor each contrast.
#[test]
fn on_death_dispatches_to_subclass_hook_control_base_noop() {
    let mut g = drive_to_world();
    install_template(&mut g);

    // Proven-red control: the abstract Boss.onDeath leaves deathTimer untouched.
    let base = boss::new_boss(&mut g, 6, 10, 0, 0, 1);
    assert_eq!(g.entity_arena[base].as_enemy().unwrap().death_timer, 0);
    boss::on_death(&mut g, base);
    assert_eq!(
        g.entity_arena[base].as_enemy().unwrap().death_timer,
        0,
        "base Boss.onDeath is the abstract no-op"
    );

    // RockyBoss.onDeath → deathTimer = 24.
    let rocky = rocky_boss::new(&mut g, 6, 6, 0, 0);
    boss::on_death(&mut g, rocky);
    assert_eq!(
        g.entity_arena[rocky].as_enemy().unwrap().death_timer,
        24,
        "RockyBoss.onDeath armed deathTimer = 24"
    );

    // GebCore.onDeath → deathTimer = 16.
    let core = geb_core::new(&mut g, 6, 8, 0, 0);
    boss::on_death(&mut g, core);
    assert_eq!(
        g.entity_arena[core].as_enemy().unwrap().death_timer,
        16,
        "GebCore.onDeath armed deathTimer = 16"
    );

    // GebHandLeft/Right.onDeath → deathTimer = 0 (seed a sentinel first so the reset is
    // observable vs the no-op, which would leave the sentinel).
    let hl = geb_hand_left::new(&mut g, 2, 2, 0, 0);
    g.entity_arena[hl].as_enemy_mut().unwrap().death_timer = 99;
    boss::on_death(&mut g, hl);
    assert_eq!(
        g.entity_arena[hl].as_enemy().unwrap().death_timer,
        0,
        "GebHandLeft.onDeath reset deathTimer to 0 (not the sentinel)"
    );
    let hr = geb_hand_right::new(&mut g, 12, 2, 0, 0);
    g.entity_arena[hr].as_enemy_mut().unwrap().death_timer = 99;
    boss::on_death(&mut g, hr);
    assert_eq!(
        g.entity_arena[hr].as_enemy().unwrap().death_timer,
        0,
        "GebHandRight.onDeath reset deathTimer to 0 (not the sentinel)"
    );

    // GebHead.onDeath → deathTimer = 12 AND despawns both owned hands.
    let ghl = geb_hand_left::new(&mut g, 2, 2, 0, 0);
    let ghr = geb_hand_right::new(&mut g, 12, 2, 0, 0);
    let head = geb_head::new(&mut g, 6, 2, 0, 0, ghl, ghr);
    game_map::add_entity(&mut g, ghl);
    game_map::add_entity(&mut g, ghr);
    game_map::add_entity(&mut g, head);
    assert!(
        list_contains(&g, ghl) && list_contains(&g, ghr),
        "both hands linked before onDeath"
    );
    boss::on_death(&mut g, head);
    assert_eq!(
        g.entity_arena[head].as_enemy().unwrap().death_timer,
        12,
        "GebHead.onDeath armed deathTimer = 12"
    );
    assert!(
        !list_contains(&g, ghl),
        "GebHead.onDeath despawned the left hand"
    );
    assert!(
        !list_contains(&g, ghr),
        "GebHead.onDeath despawned the right hand"
    );
}

// --- A single AI tick through the virtual dispatch -----------------------------------

/// `boss::update` routes each subclass node to its tick, advancing `animFrame` from the
/// -1 spawn default to 0. Paired with a proven-red control: an un-ticked boss stays -1.
#[test]
fn boss_update_dispatches_one_tick_control_stays() {
    let mut g = drive_to_world();
    install_template(&mut g);

    let hl = geb_hand_left::new(&mut g, 2, 2, 0, 0);
    let hr = geb_hand_right::new(&mut g, 12, 2, 0, 0);
    let ticked = [
        rocky_boss::new(&mut g, 6, 6, 0, 0),
        geb_core::new(&mut g, 6, 8, 0, 0),
        geb_head::new(&mut g, 6, 2, 0, 0, hl, hr),
        hl,
        hr,
    ];

    // A proven-red control: an identically-built RockyBoss that is NEVER ticked.
    let control = rocky_boss::new(&mut g, 10, 10, 0, 0);

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

    // Proven-red control: the un-ticked boss kept the spawn default.
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
