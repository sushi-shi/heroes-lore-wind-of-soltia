//! Transliterated from `java/src/main/java/defpackage/Hero.java`
//! (original `ao.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The player character: a [`crate::battler`] (embedded [`HeroData::battler`]) that
//! owns the four base stats, derived combat stats, HP/MP/exp and leveling, the
//! inventory and quick-item bags, equipment, guardian companions, the combo/attack
//! animation state, and the RMS save format.
//!
//! **This slice ports the FIELD LAYER + the constructor + New Game class setup.**
//! [`HeroData`] + [`new_hero`] (`new Hero(0,0,8,8,classId)`), plus [`init_class`]
//! (class base stats/level/gold), [`recompute_stats`] (derived attack/defense/
//! maxHp/maxMp/expToNext), [`init`] and [`reset_combo`] land here — enough to place
//! a viable hero on the map. The guardian setup and the five starting
//! `equipment` slots (`Item.create`) inside `init_class` are DEFERRED (see its doc),
//! as are the combat FSM ([`update`]) and rendering ([`paint`]).
//!
//! `Hero` has no mutable `static` fields; `COMBO_FRAMES_CLASS6/7/8` are
//! `static final` constant tables, reproduced as `const` (crcTable/QUICK_TYPES
//! precedent) — no `ownership.tsv` rows.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`):
//! `ao.<init>:(SSBBB)V => [ldiv,l2i]` — the whole ctor's only arithmetic is
//! `sessionStartSec = (int) (System.currentTimeMillis() / 1000)`.

use crate::battler::BattlerData;
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game::Game;
use crate::item_bag::{self, ItemBag, ItemRef};
use j2me_jvm::{java_div, java_ldiv, Clock, VirtualClock};

/// Warrior (class 6) combo frame table: `[attackType-1][comboStep]` durations.
/// `private static final byte[][] COMBO_FRAMES_CLASS6` (`ao.b`).
const COMBO_FRAMES_CLASS6: [[i8; 5]; 2] = [[4, 4, 4, 4, 0], [4, 0, 4, 4, 8]];
/// Rogue (class 7) combo frame table. `private static final byte[][]` (`ao.c`).
const COMBO_FRAMES_CLASS7: [[i8; 5]; 2] = [[3, 3, 3, 6, 0], [4, 0, 7, 9, 14]];
/// Mage (class 8) combo frame table. `private static final byte[][]` (`ao.d`).
const COMBO_FRAMES_CLASS8: [[i8; 5]; 2] = [[3, 3, 3, 3, 0], [3, 0, 3, 3, 6]];

/// DEFERRED placeholder for a `Guardian` (`bl`) heap object — that class is not
/// ported. The `guardians` array is all-`null` and `activeGuardian` is `null` after
/// construction (populated only by the DEFERRED `initClass`/`load`).
#[derive(Debug)]
pub struct GuardianRef;

/// `Hero` (`ao`) instance data — every field of the class, with the `Battler` base
/// embedded as [`HeroData::battler`]. Reference-typed slots hold arena/`Rc` handles
/// or the DEFERRED [`GuardianRef`] placeholder.
#[derive(Debug)]
pub struct HeroData {
    /// The `Battler` (`o`) base — the "super" of `Hero`.
    pub battler: BattlerData,

    // --- combo / attack animation ---
    /// `private byte[] comboSteps;` (`ao.h`) — attack type of each queued step.
    pub combo_steps: Vec<i8>,
    /// `private byte comboIndex;` (`ao.q`) — currently animating step (-1 = none).
    pub combo_index: i8,
    /// `private byte recoilTimer;` (`ao.r`) — hit-recoil display countdown.
    pub recoil_timer: i8,
    /// `private byte recoilDir;` (`ao.s`) — direction the recoil offset applies to.
    pub recoil_dir: i8,
    /// `private byte comboLockout;` (`ao.t`) — blocks queueing until it expires.
    pub combo_lockout: i8,
    /// `private byte deathTimer;` (`ao.u`) — death-animation countdown (state 6).
    pub death_timer: i8,
    /// `private byte hpRegenTimer;` (`ao.v`) — HP-regen countdown.
    pub hp_regen_timer: i8,
    /// `private byte mpRegenTimer;` (`ao.w`) — MP-regen countdown.
    pub mp_regen_timer: i8,
    /// `private byte queuedTurn;` (`ao.x`) — a turn queued mid-step.
    pub queued_turn: i8,
    /// `private byte lungeSteps;` (`ao.y`) — lunge distance for a dashing special.
    pub lunge_steps: i8,
    /// `private boolean triggerChecked;` (`ao.i`) — one tile-trigger per step guard.
    pub trigger_checked: bool,
    /// `private byte[][] comboFrames;` (`ao.e`) — the selected class's combo table.
    pub combo_frames: Vec<Vec<i8>>,

    // --- guardian buffs ---
    /// `public boolean attackUp;` (`ao.d`) — attack-up buff (×3/2 damage).
    pub attack_up: bool,
    /// `public boolean defenseUp;` (`ao.e`) — defense-up buff (×2 defense).
    pub defense_up: bool,
    /// `public boolean invincible;` (`ao.f`) — invincibility buff.
    pub invincible: bool,
    /// `public boolean reflectDamage;` (`ao.g`) — reflect buff.
    pub reflect_damage: bool,
    /// `public boolean regenBoost;` (`ao.h`) — regen-boost buff (×2 HP/MP regen).
    pub regen_boost: bool,

    // --- stats / progression ---
    /// `private byte maxCombo;` (`ao.z`) — max combo depth unlocked.
    pub max_combo: i8,
    /// `public short statPoints;`
    pub stat_points: i16,
    /// `public byte classId;` (`ao.f`) — the hero's class (NOT set by the ctor;
    /// assigned by the DEFERRED `load`). Distinct from `GameState.classId`.
    pub class_id: i8,
    /// `public byte level;` (`ao.g`)
    pub level: i8,
    /// `public short strength;` (`ao.b`)
    pub strength: i16,
    /// `public short vitality;` (`ao.e`)
    pub vitality: i16,
    /// `public short agility;` (`ao.f`)
    pub agility: i16,
    /// `public short spirit;` (`ao.g`)
    pub spirit: i16,
    /// `public byte strengthBonus;`
    pub strength_bonus: i8,
    /// `public byte vitalityBonus;`
    pub vitality_bonus: i8,
    /// `public byte agilityBonus;`
    pub agility_bonus: i8,
    /// `public byte spiritBonus;`
    pub spirit_bonus: i8,
    /// `public int hp;` (`ao.a`)
    pub hp: i32,
    /// `public int mp;` (`ao.b`)
    pub mp: i32,
    /// `public int exp;` (`ao.c`)
    pub exp: i32,
    /// `public short attack;` (`ao.h`)
    pub attack: i16,
    /// `public short defense;` (`ao.i`)
    pub defense: i16,
    /// `public int maxHp;` (`ao.d`)
    pub max_hp: i32,
    /// `public int maxMp;` (`ao.e`)
    pub max_mp: i32,
    /// `public int expToNext;` (`ao.f`)
    pub exp_to_next: i32,

    // --- inventory / companions ---
    /// `public ItemBag bag;` (`ao.a`) — the carried inventory (capacity 30).
    pub bag: ItemBag,
    /// `private Equipment[] equipment;` (`ao.a`) — the 5 equipment slots (shared
    /// `Item` refs, moved to/from `bag`).
    pub equipment: Vec<Option<ItemRef>>,
    /// `public Guardian[] guardians;` (`ao.a`) — the 5 guardian slots (DEFERRED).
    pub guardians: Vec<Option<GuardianRef>>,
    /// `private Guardian activeGuardian;` (`ao.a`) — the summoned guardian (DEFERRED).
    pub active_guardian: Option<GuardianRef>,
    /// `public ItemBag quickItems;` (`ao.b`) — the quick-item bar (capacity 15).
    pub quick_items: ItemBag,

    // --- per-strike scratch ---
    /// `private int rolledDamage;` (`ao.i`)
    pub rolled_damage: i32,
    /// `private boolean rolledCrit;` (`ao.j`)
    pub rolled_crit: bool,
    /// `private byte rolledProc;` (`ao.A`)
    pub rolled_proc: i8,

    // --- play-time bookkeeping ---
    /// `public int sessionStartSec;` (`ao.g`)
    pub session_start_sec: i32,
    /// `public int playSeconds;` (`ao.h`)
    pub play_seconds: i32,
}

/// `public Hero(short pixelX, short pixelY, byte halfWidth, byte halfHeight,
/// byte classId)`. Allocates the hero node in `arena` and returns its [`EntityId`].
///
/// The Java constructor threads through `super(Battler)` → `super(Entity)` and
/// invokes `init()` twice (once via `Battler`'s ctor by virtual dispatch, once
/// explicitly) plus `resetBuffs()`. All of those are pure field initialisation
/// with no side effects on this path (`activeGuardian` is null, so `dismiss()` is
/// skipped; no I/O), so the **net end-state** is assembled here as a single field
/// init, each field annotated with the assignment that produces it. The lone
/// arithmetic is `sessionStartSec = (int)(currentTimeMillis()/1000)` (`ldiv,l2i`).
pub fn new_hero(
    arena: &mut EntityArena,
    clock: &VirtualClock,
    pixel_x: i16,
    pixel_y: i16,
    half_width: i8,
    half_height: i8,
    class_id: i8,
) -> EntityId {
    // switch (classId) { case 6/7/8: this.comboFrames = COMBO_FRAMES_CLASS*; }
    // (default: no case matches → comboFrames stays null → empty here.)
    let combo_frames: Vec<Vec<i8>> = match class_id {
        6 => COMBO_FRAMES_CLASS6.iter().map(|r| r.to_vec()).collect(),
        7 => COMBO_FRAMES_CLASS7.iter().map(|r| r.to_vec()).collect(),
        8 => COMBO_FRAMES_CLASS8.iter().map(|r| r.to_vec()).collect(),
        _ => Vec::new(),
    };
    // this.sessionStartSec = (int) (System.currentTimeMillis() / 1000);
    let session_start_sec =
        java_ldiv(clock.current_time_millis(), 1000).expect("currentTimeMillis / 1000") as i32;

    let hero = HeroData {
        // super(Battler): knockbackTimer = 0; init() → base FSM fields.
        battler: BattlerData::new(),

        // init(): this.comboSteps = new byte[5]; this.comboIndex = (byte) -1;
        combo_steps: vec![0i8; 5],
        combo_index: -1,
        // ctor: this.recoilTimer = (byte) 0;
        recoil_timer: 0,
        recoil_dir: 0, // default (set on hit)
        combo_lockout: 0,
        death_timer: 0,
        // init(): hpRegenTimer = 67 + level < 100 ? (byte)(67+level) : 100; (level 0 → 67)
        hp_regen_timer: if 67i32.wrapping_add(0) < 100 {
            67i32.wrapping_add(0) as i8
        } else {
            100
        },
        // init(): this.mpRegenTimer = (byte) 21;
        mp_regen_timer: 21,
        // ctor: this.queuedTurn = (byte) 0;
        queued_turn: 0,
        // init(): this.lungeSteps = (byte) 0;
        lunge_steps: 0,
        // init(): this.triggerChecked = false;
        trigger_checked: false,
        combo_frames,

        // resetBuffs(): all guardian-buff flags cleared (init() also clears invincible).
        attack_up: false,
        defense_up: false,
        invincible: false,
        reflect_damage: false,
        regen_boost: false,

        // stats/progression: JVM field defaults (initClass fills these — DEFERRED).
        max_combo: 0,
        stat_points: 0,
        class_id: 0, // ctor does NOT assign this.classId (only selects comboFrames).
        level: 0,
        strength: 0,
        vitality: 0,
        agility: 0,
        spirit: 0,
        strength_bonus: 0,
        vitality_bonus: 0,
        agility_bonus: 0,
        spirit_bonus: 0,
        hp: 0,
        mp: 0,
        exp: 0,
        attack: 0,
        defense: 0,
        max_hp: 0,
        max_mp: 0,
        exp_to_next: 0,

        // ctor: this.bag = new ItemBag((byte) 30);
        bag: item_bag::new(30),
        // ctor: this.equipment = new Equipment[5];
        equipment: (0..5).map(|_| None).collect(),
        // ctor: this.guardians = new Guardian[5];
        guardians: (0..5).map(|_| None).collect(),
        // (activeGuardian null after construction)
        active_guardian: None,
        // ctor: this.quickItems = new ItemBag((byte) 15);
        quick_items: item_bag::new(15),

        rolled_damage: 0,
        rolled_crit: false,
        rolled_proc: 0,

        session_start_sec,
        play_seconds: 0,
    };

    // super(pixelX, pixelY, halfWidth, halfHeight);
    let mut node = EntityNode {
        data: EntityData::Hero(Box::new(hero)),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, half_width, half_height);
    arena.alloc(node)
}

/// `public final void initClass(byte classId)` (`ao.a:(B)V`) — sets the starting
/// stats/level/gold for the chosen class and recomputes the derived stats.
///
/// **Guardian + equipment DEFERRED.** The leading
/// `Debug.assertTrue(guardians[0/1] != null)` + `setActiveGuardian(guardians[0])`
/// and the five `equipment[i] = (Equipment) Item.create(...)` lines are the guardian
/// / item setup, which are not driven in this minimal New Game slice (the guardian
/// slots are populated by the DEFERRED guardian-summon path; `Item.create` is not
/// driven). Skipping them leaves `activeGuardian`/`equipment` null, so
/// [`recompute_stats`] adds no gear bonuses. The class base stats, `level`,
/// `maxCombo`, `gold`, `statPoints`, and `hp`/`mp`/`exp` are set faithfully.
pub fn init_class(g: &mut Game, id: EntityId, class_id: i8) {
    // Debug.assertTrue(guardians[0] != null); Debug.assertTrue(guardians[1] != null);
    // setActiveGuardian(guardians[0]);   — DEFERRED (guardian setup; see doc).
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // switch (classId) { case 6/7/8: strength/vitality/agility/spirit = ...; }
        match class_id {
            6 => {
                hero.strength = 8;
                hero.vitality = 5;
                hero.agility = 3;
                hero.spirit = 4;
            }
            7 => {
                hero.strength = 3;
                hero.vitality = 4;
                hero.agility = 8;
                hero.spirit = 5;
            }
            8 => {
                hero.strength = 5;
                hero.vitality = 8;
                hero.agility = 4;
                hero.spirit = 3;
            }
            _ => {}
        }
        // equipment[0]=class weapon; equipment[2..4]=armor/head/accessory (via
        //   Item.create).  — DEFERRED (item creation not driven; slots stay null).
        // this.level = 1; this.maxCombo = 1;
        hero.level = 1;
        hero.max_combo = 1;
        // this.bag.gold = 300;
        hero.bag.gold = 300;
        // this.statPoints = 0;
        hero.stat_points = 0;
    }
    // recomputeStats();
    recompute_stats(g, id);
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // this.hp = this.maxHp; this.mp = this.maxMp; this.exp = 0;
        hero.hp = hero.max_hp;
        hero.mp = hero.max_mp;
        hero.exp = 0;
    }
}

/// `public final void recomputeStats()` (`ao.a:()V`) — recomputes attack/defense/
/// maxHp/maxMp/expToNext from the base stats + equipment enchants. All five
/// `equipment` slots are null in this slice (item creation DEFERRED in
/// [`init_class`]), so every `equip[i] != null` branch takes the null side (adds
/// nothing); the branches are reproduced structurally. `GameLoop.gameScreen.markRedraw()`.
pub fn recompute_stats(g: &mut Game, id: EntityId) {
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // Equipment[] equip = this.equipment;
        // strengthBonus = vitalityBonus = agilityBonus = spiritBonus = 0;
        hero.strength_bonus = 0;
        hero.vitality_bonus = 0;
        hero.agility_bonus = 0;
        hero.spirit_bonus = 0;
        // for (i=0;i<5;i++) if (equip[i]!=null) bonuses += equip[i].enchant[..];
        for i in 0..5 {
            if hero.equipment[i].is_some() {
                // (DEFERRED: enchant-bonus accumulation — equipment null in this slice.)
            }
        }
        // maxHp = 0; maxMp = 0; expToNext = 0; attack = 0; defense = 0;
        hero.max_hp = 0;
        hero.max_mp = 0;
        hero.exp_to_next = 0;
        hero.attack = 0;
        hero.defense = 0;
        // maxHp = (vitality + vitalityBonus + level) * 12;
        hero.max_hp = (hero.vitality as i32)
            .wrapping_add(hero.vitality_bonus as i32)
            .wrapping_add(hero.level as i32)
            .wrapping_mul(12);
        // maxMp = (spirit + spiritBonus + level) * 12;
        hero.max_mp = (hero.spirit as i32)
            .wrapping_add(hero.spirit_bonus as i32)
            .wrapping_add(hero.level as i32)
            .wrapping_mul(12);
        // expToNext = ((level*level)*level) - (level*level) + (80*level);
        let level = hero.level as i32;
        hero.exp_to_next = level
            .wrapping_mul(level)
            .wrapping_mul(level)
            .wrapping_sub(level.wrapping_mul(level))
            .wrapping_add(80i32.wrapping_mul(level));
        // attack += (equip[0]!=null ? equip[0].value + (equip[0].refineLevel*5)/2 : 0);  → +0
        // attack = (short) (attack + ((strength + strengthBonus) * 4) / 5);
        let str_term = java_div(
            (hero.strength as i32)
                .wrapping_add(hero.strength_bonus as i32)
                .wrapping_mul(4),
            5,
        )
        .expect("((strength + strengthBonus) * 4) / 5");
        hero.attack = (hero.attack as i32).wrapping_add(str_term) as i16;
        // defense += the four equip[1..4] terms  → +0 (all null)
        // defense = (short) (defense + ((strength + strengthBonus) / 5));
        let def_str = java_div(
            (hero.strength as i32).wrapping_add(hero.strength_bonus as i32),
            5,
        )
        .expect("(strength + strengthBonus) / 5");
        hero.defense = (hero.defense as i32).wrapping_add(def_str) as i16;
        // defense = (short) (defense + (level / 3));
        let def_level = java_div(hero.level as i32, 3).expect("level / 3");
        hero.defense = (hero.defense as i32).wrapping_add(def_level) as i16;
        // if (hp > maxHp) hp = maxHp; if (mp > maxMp) mp = maxMp;
        if hero.hp > hero.max_hp {
            hero.hp = hero.max_hp;
        }
        if hero.mp > hero.max_mp {
            hero.mp = hero.max_mp;
        }
    }
    // GameLoop.gameScreen.markRedraw();
    crate::game_screen::mark_redraw(g);
}

/// `public final void init()` (`ao.a:()V`, overriding `Battler.init`) — (re)sets the
/// hero's FSM + combo/regen state. The overlay lists (`floaters`/`statuses`) are
/// non-null `Vec`s here, so `super.init`'s null-guards are vacuous; the
/// `activeGuardian != null → dismiss()` branch is skipped (guardian DEFERRED, null).
pub fn init(g: &mut Game, id: EntityId) {
    let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
    // super.init(): state = 1; facing = 2; moveDir = 2; animFrame = -1;
    hero.battler.state = 1;
    hero.battler.facing = 2;
    hero.battler.move_dir = 2;
    hero.battler.anim_frame = -1;
    // this.comboSteps = new byte[5]; this.comboIndex = -1;
    hero.combo_steps = vec![0i8; 5];
    hero.combo_index = -1;
    // this.hpRegenTimer = 67 + level < 100 ? (byte)(67 + level) : (byte) 100;
    let sum = 67i32.wrapping_add(hero.level as i32);
    hero.hp_regen_timer = if sum < 100 { sum as i8 } else { 100 };
    // this.mpRegenTimer = 21;
    hero.mp_regen_timer = 21;
    // this.lungeSteps = 0;
    hero.lunge_steps = 0;
    // this.invincible = false;
    hero.invincible = false;
    // if (activeGuardian != null) activeGuardian.dismiss();  — DEFERRED (null).
    // this.triggerChecked = false;
    hero.trigger_checked = false;
}

/// `public final void resetCombo()` (`ao`) — clears the queued attack steps.
pub fn reset_combo(g: &mut Game, id: EntityId) {
    let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
    // this.comboIndex = -1;
    hero.combo_index = -1;
    // for (int i = 0; i < comboSteps.length; i++) comboSteps[i] = 0;
    for i in 0..hero.combo_steps.len() {
        hero.combo_steps[i] = 0;
    }
}

/// `public final void update()` (`ao.d:()V`, overriding `Battler.update`) — the
/// hero's per-tick FSM: animation counter, HP/MP regen, status ticks, the
/// state-machine switch, the sub-tile step ([`crate::battler::move_`]) and the
/// per-step trigger check.
///
/// **This slice ports the MOVEMENT path** (state 1 idle / state 2 stepping). The
/// following branches reach as-yet-unported classes and are DEFERRED, each clearly
/// marked; none execute on the player-movement path:
/// - **Regen heals** (`addHp`/`addMp`): the timer *decrements* are ported (observable
///   every world frame), but the fire-body reaches `GameScreen.markHpDirty` (HUD
///   dirty flags owned by the render lane) and is deferred; the timers reset
///   faithfully so play stays bounded.
/// - **Status loop** (`statuses` / `StatusIcon` / poison `Floater`): `statuses` is
///   always empty in this slice (the `.evt` status machinery is unported), so the
///   loop never iterates — the whole block is deferred.
/// - **Attack/death states** (case 0/3/6: `advanceCombo`/`onDeath`): combat/death
///   FSM, deferred (the hero is state 1/2 on the walk path).
/// - **Blocked-turn sidestep** (inside `tryStepForward()`): reads `map.collisionGrid`
///   / `enemyAhead` / `setTarget`; unreachable because [`crate::battler::try_step_forward`]
///   is stubbed to never block (collision `.evt` deferred).
/// - **Tile/facing triggers** (`EventScript.checkTileTrigger`/`checkFacingTrigger`):
///   the `.evt` trigger tables are unported → no trigger fires (`triggered == false`),
///   so the `triggerChecked` latch is reproduced but the guarded `setState(1)` never runs.
pub fn update(g: &mut Game, id: EntityId) {
    {
        let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // this.animFrame = (byte) (this.animFrame + 1);
        h.battler.anim_frame = (h.battler.anim_frame as i32).wrapping_add(1) as i8;
        // if (this.comboLockout > 0) this.comboLockout = (byte) (this.comboLockout - 1);
        if h.combo_lockout > 0 {
            h.combo_lockout = (h.combo_lockout as i32).wrapping_sub(1) as i8;
        }
    }
    // if (GameState.screen == 2) { HP/MP regen }
    if g.game_state.screen == 2 {
        let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
        let state = h.battler.state;
        // if (state == 1) hpRegenTimer -= 2; else if (state == 2) hpRegenTimer -= 1;
        if state == 1 {
            h.hp_regen_timer = (h.hp_regen_timer as i32).wrapping_sub(2) as i8;
        } else if state == 2 {
            h.hp_regen_timer = (h.hp_regen_timer as i32).wrapping_sub(1) as i8;
        }
        // if (hpRegenTimer <= 0) { addHp(...); hpRegenTimer = 67+level < 100 ? (byte)(67+level) : 100; }
        if h.hp_regen_timer <= 0 {
            // DEFERRED: addHp((vitality + vitalityBonus) * (regenBoost ? 2 : 1)) —
            //   reaches GameScreen.markHpDirty (render-lane HUD flags). Timer reset kept.
            let sum = 67i32.wrapping_add(h.level as i32);
            h.hp_regen_timer = if sum < 100 { sum as i8 } else { 100 };
        }
        // if (state == 1) mpRegenTimer -= 3; else if (state == 2) mpRegenTimer -= 1;
        if state == 1 {
            h.mp_regen_timer = (h.mp_regen_timer as i32).wrapping_sub(3) as i8;
        } else if state == 2 {
            h.mp_regen_timer = (h.mp_regen_timer as i32).wrapping_sub(1) as i8;
        }
        // if (mpRegenTimer <= 0) { addMp(...); mpRegenTimer = 21; }
        if h.mp_regen_timer <= 0 {
            // DEFERRED: addMp((spirit + spiritBonus) * (regenBoost ? 2 : 1)). Reset kept.
            h.mp_regen_timer = 21;
        }
    }
    // if (state != 6 && state != 5) { for each StatusIcon: tick/poison/expire }
    //   — DEFERRED: `statuses` is always empty in this slice (the .evt status /
    //   StatusIcon machinery is unported), so the loop body never iterates.

    // switch (this.state)
    let state = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .battler
        .state;
    match state {
        // case 0: return;
        0 => return,
        // case 1 (idle):
        1 => {
            let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
            // this.queuedTurn = (byte) 0;
            h.queued_turn = 0;
            // if (this.animFrame == 1) this.animFrame = (byte) 0;
            if h.battler.anim_frame == 1 {
                h.battler.anim_frame = 0;
            }
        }
        // case 2 (stepping):
        2 => {
            let queued_turn = g.entity_arena[id].as_hero().expect("Hero node").queued_turn;
            let off_grid_x = g.entity_arena[id].off_grid_x;
            let off_grid_y = g.entity_arena[id].off_grid_y;
            // if (queuedTurn != 0 && !offGridX && !offGridY) { setFacing(queuedTurn); queuedTurn = 0; }
            //   (queuedTurn is 0 on this slice's walk path — the queued-turn producers are
            //   the DEFERRED blocked-turn / combat logic; reproduced structurally.)
            if queued_turn != 0 && !off_grid_x && !off_grid_y {
                let b = &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler;
                crate::battler::set_facing(b, queued_turn);
                g.entity_arena[id]
                    .as_hero_mut()
                    .expect("Hero node")
                    .queued_turn = 0;
            }
            // if (this.animFrame == 4) this.animFrame = (byte) 0;
            let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
            if h.battler.anim_frame == 4 {
                h.battler.anim_frame = 0;
            }
        }
        // case 3 (attacking): comboIndex init + advanceCombo() — DEFERRED (attack combo).
        // case 6 (dead): deathTimer / onDeath() — DEFERRED (death FSM).
        _ => {}
    }
    // byte stateBefore = this.state;  GameMap map = GameState.map;
    let state_before = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .battler
        .state;
    // if ((state == 2 || state == 4) && tryStepForward()) { blocked-turn sidestep }
    if (state_before == 2 || state_before == 4) && crate::battler::try_step_forward(g, id) {
        // DEFERRED: reads map.collisionGrid / enemyAhead / setTarget — collision (.evt)
        //   deferred; tryStepForward is stubbed to never block, so this is unreachable.
    }
    // if (state == 2 || state == 4) { super.move(8); this.triggerChecked = false; }
    let state = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .battler
        .state;
    if state == 2 || state == 4 {
        crate::battler::move_(g, id, 8);
        g.entity_arena[id]
            .as_hero_mut()
            .expect("Hero node")
            .trigger_checked = false;
    }
    // if (GameState.screen != 4) { trigger checks }
    if g.game_state.screen != 4 {
        // boolean triggered = false;
        // if (state != 3 && !triggerChecked) { triggered = checkTileTrigger(this); triggerChecked = true; }
        //   — DEFERRED: EventScript (.evt tile triggers) unported → triggered stays false;
        //   the triggerChecked latch is reproduced.
        {
            let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
            if h.battler.state != 3 && !h.trigger_checked {
                h.trigger_checked = true;
            }
        }
        // if (stateBefore == 2 && state == 1 && !triggered) triggered = checkFacingTrigger();
        //   — DEFERRED (false). if (triggered) { setState(1); queuedTurn = 0; animFrame = 0; }
        //   — `triggered` is always false here, so the guarded reset never runs.
    }
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ao.a:(Graphics;II)V`) — draws the layered character sprite. DEFERRED to the
/// render lane.
pub fn paint(_g: &mut Game, _id: EntityId, _origin_x: i32, _origin_y: i32) {
    unimplemented!("DEFERRED: Hero.paint — not ported in this slice")
}
