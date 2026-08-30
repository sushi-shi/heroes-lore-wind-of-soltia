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
//! **This slice ports the FIELD LAYER + the constructor + New Game class setup +
//! the RMS save format.** [`HeroData`] + [`new_hero`] (`new Hero(0,0,8,8,classId)`),
//! [`init_class`] (class base stats/level/gold **and the five starting `Item.create`
//! equipment slots**), [`recompute_stats`] (derived attack/defense/maxHp/maxMp/
//! expToNext **including the equipment gear bonuses**), [`init`], [`reset_combo`],
//! and [`save`]/[`load`] (serialize/deserialize the hero core stats + equipment to/
//! from bytes). The **guardian** companion setup inside `init_class`/`save`/`load` is
//! DEFERRED (`// DEFERRED: Guardian` — a separate later batch; the `guardians`/
//! `activeGuardian` slots are all-null in this slice).
//!
//! **The COMBAT FSM also lands here.** [`update`]'s attack/hurt/death states (case 3
//! `advanceCombo`, case 6 death → [`on_death`]) and the [`paint`] attack/death poses,
//! plus the incoming-damage resolver [`take_hit`]/[`take_hit_raw`], the combo builders
//! ([`queue_combo_step`]/[`advance_combo`]/[`perform_attack`]/[`end_combo`]), the three
//! class hit routines ([`attack_class6`]/[`attack_class7`]/[`attack_class8`]), the
//! damage/crit/proc rolls ([`roll_damage`]/[`roll_crit`]/[`roll_proc`]) and
//! [`add_hp`]/[`add_mp`] are ported. The hero's own melee/ranged attack works against a
//! ported [`crate::enemy`]; the guardian companion paths (reflect-damage strike-back in
//! [`take_hit`], `activeGuardian` reads) stay `// DEFERRED: Guardian`, the game-over
//! `EventScript` hook / render-lane HUD dirty flags (`markHpDirty`/`setTarget`) stay
//! DEFERRED, and the warrior lunge sub-branch (`.evt` `collisionGrid`/`isWalkable`)
//! stays DEFERRED. Enemy damage-application through `Enemy.takeHeroHit` is the enemy
//! lane's own DEFERRED body — [`update`]'s attack faithfully *calls* it.
//!
//! `Hero` has no mutable `static` fields; `COMBO_FRAMES_CLASS6/7/8` are
//! `static final` constant tables, reproduced as `const` (crcTable/QUICK_TYPES
//! precedent) — no `ownership.tsv` rows.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`):
//! `ao.<init>:(SSBBB)V => [ldiv,l2i]` — the whole ctor's only arithmetic is
//! `sessionStartSec = (int) (System.currentTimeMillis() / 1000)`.

use crate::asset_cache::AssetCacheState;
use crate::battler::BattlerData;
use crate::directions::{DIAG_CCW, DIAG_CW, DIR_DX, DIR_DY, REVERSE};
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game::Game;
use crate::game_screen;
use crate::item::{self, Item};
use crate::item_bag::{self, ItemBag, ItemRef};
use j2me_jvm::{ishr, java_div, java_ldiv, java_rem, Clock, VirtualClock};
use std::cell::RefCell;
use std::rc::Rc;

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
/// stats/level/gold for the chosen class, creates the five starting equipment items,
/// and recomputes the derived stats.
///
/// **Guardian DEFERRED; equipment PORTED.** The leading
/// `Debug.assertTrue(guardians[0/1] != null)` + `setActiveGuardian(guardians[0])` are
/// the guardian setup (`guardians` all-null in this slice — the guardian-summon path
/// that fills them is a later batch), DEFERRED with `// DEFERRED: Guardian`. The five
/// `equipment[i] = (Equipment) Item.create(...)` slots ARE created here (via the
/// ported [`item::create`], driving `AssetCache.loadItemRecord`), so
/// [`recompute_stats`] now folds their gear bonuses. The class base stats, `level`,
/// `maxCombo`, `gold`, `statPoints`, and `hp`/`mp`/`exp` are set faithfully.
pub fn init_class(g: &mut Game, id: EntityId, class_id: i8) {
    // Debug.assertTrue(guardians[0] != null); Debug.assertTrue(guardians[1] != null);
    // setActiveGuardian(guardians[0]);   — DEFERRED: Guardian (guardian setup; the
    //   `guardians` array is all-null in this slice, so these three lines are skipped).
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
    }
    // switch (classId): the class weapon (slot 0), plus the mage's shield (slot 1).
    match class_id {
        // case 6: equipment[0] = (Equipment) Item.create((byte) 0, (byte) 0, true, false);
        6 => create_starting_equipment(g, id, 0, 0, 0),
        // case 7: equipment[0] = (Equipment) Item.create((byte) 2, (byte) 0, true, false);
        7 => create_starting_equipment(g, id, 0, 2, 0),
        // case 8: equipment[0] = Item.create((byte) 1, ...); equipment[1] = Item.create((byte) 3, ...);
        8 => {
            create_starting_equipment(g, id, 0, 1, 0);
            create_starting_equipment(g, id, 1, 3, 0);
        }
        _ => {}
    }
    // equipment[2] = (Equipment) Item.create((byte) 5, (byte) 0, true, false);
    create_starting_equipment(g, id, 2, 5, 0);
    // equipment[3] = (Equipment) Item.create((byte) 6, (byte) 0, true, false);
    create_starting_equipment(g, id, 3, 6, 0);
    // equipment[4] = (Equipment) Item.create((byte) 4, (byte) 0, true, false);
    create_starting_equipment(g, id, 4, 4, 0);
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
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

/// The three-line `equipment[slot] = (Equipment) Item.create(type, subId, true, false);
/// equipment[slot].identified = true; equipment[slot].quantity = (byte) 1;` idiom
/// `initClass` repeats for each starting slot. The freshly-created item is not aliased
/// anywhere until it is stored, so setting the two fields on it before wrapping it into
/// the shared [`ItemRef`] slot is identical to the Java's store-then-mutate order.
fn create_starting_equipment(g: &mut Game, id: EntityId, slot: usize, r#type: i8, sub_id: i8) {
    // this.equipment[slot] = (Equipment) Item.create(type, subId, true, false);
    let mut created: Item = item::create(g, r#type, sub_id, true, false);
    // this.equipment[slot].identified = true;
    created.identified = true;
    // this.equipment[slot].quantity = (byte) 1;
    created.quantity = 1;
    let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
    hero.equipment[slot] = Some(Rc::new(RefCell::new(created)));
}

/// `public final void recomputeStats()` (`ao.n:()V`) — recomputes attack/defense/
/// maxHp/maxMp/expToNext from the base stats + the five equipment slots' enchant
/// bonuses, per-slot `value`/`refineLevel`. Now that [`init_class`] creates the
/// starting gear (item creation ported), the `equip[i] != null` branches fold in the
/// real bonuses. `GameLoop.gameScreen.markRedraw()`.
///
/// The four divisions (`(refineLevel*5)/2`, `((strength+strengthBonus)*4)/5`,
/// `(strength+strengthBonus)/5`, `level/3`) are the method's four `idiv`s in the R8
/// shape; each divisor is a nonzero constant, so `java_div(...).expect(...)` is the
/// faithful `ArithmeticException` site (never taken).
pub fn recompute_stats(g: &mut Game, id: EntityId) {
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // Equipment[] equip = this.equipment;  — snapshot the four gear fields the
        // arithmetic below reads (value/refineLevel/enchant[0..4]), so the shared
        // `RefCell` borrows do not overlap the `hero` mutations.
        let mut eq_present = [false; 5];
        let mut eq_value = [0i16; 5];
        let mut eq_refine = [0i8; 5];
        let mut eq_enchant = [[0i8; 4]; 5];
        for i in 0..5 {
            if let Some(eq) = hero.equipment[i].as_ref() {
                let b = eq.borrow();
                eq_present[i] = true;
                eq_value[i] = b.value;
                eq_refine[i] = b.refine_level;
                eq_enchant[i] = [b.enchant[0], b.enchant[1], b.enchant[2], b.enchant[3]];
            }
        }
        // strengthBonus = vitalityBonus = agilityBonus = spiritBonus = 0;
        hero.strength_bonus = 0;
        hero.vitality_bonus = 0;
        hero.agility_bonus = 0;
        hero.spirit_bonus = 0;
        // for (i=0;i<5;i++) if (equip[i]!=null) { strengthBonus = (byte)(strengthBonus +
        //   equip[i].enchant[0]); vitalityBonus += enchant[1]; agilityBonus += enchant[2];
        //   spiritBonus += enchant[3]; }
        for i in 0..5 {
            if eq_present[i] {
                hero.strength_bonus =
                    (hero.strength_bonus as i32).wrapping_add(eq_enchant[i][0] as i32) as i8;
                hero.vitality_bonus =
                    (hero.vitality_bonus as i32).wrapping_add(eq_enchant[i][1] as i32) as i8;
                hero.agility_bonus =
                    (hero.agility_bonus as i32).wrapping_add(eq_enchant[i][2] as i32) as i8;
                hero.spirit_bonus =
                    (hero.spirit_bonus as i32).wrapping_add(eq_enchant[i][3] as i32) as i8;
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
        // attack = (short) (attack + (equip[0] != null ? equip[0].value +
        //   ((equip[0].refineLevel * 5) / 2) : 0));
        let attack_equip0: i32 = if eq_present[0] {
            (eq_value[0] as i32).wrapping_add(
                java_div((eq_refine[0] as i32).wrapping_mul(5), 2)
                    .expect("(equip[0].refineLevel * 5) / 2"),
            )
        } else {
            0
        };
        hero.attack = (hero.attack as i32).wrapping_add(attack_equip0) as i16;
        // attack = (short) (attack + ((strength + strengthBonus) * 4) / 5);
        let str_term = java_div(
            (hero.strength as i32)
                .wrapping_add(hero.strength_bonus as i32)
                .wrapping_mul(4),
            5,
        )
        .expect("((strength + strengthBonus) * 4) / 5");
        hero.attack = (hero.attack as i32).wrapping_add(str_term) as i16;
        // defense = (short) (defense + (equip[1] != null ? equip[1].value + equip[1].refineLevel : 0));
        let def_equip1: i32 = if eq_present[1] {
            (eq_value[1] as i32).wrapping_add(eq_refine[1] as i32)
        } else {
            0
        };
        hero.defense = (hero.defense as i32).wrapping_add(def_equip1) as i16;
        // defense = (short) (defense + (equip[2] != null ? equip[2].value + (equip[2].refineLevel * 2) : 0));
        let def_equip2: i32 = if eq_present[2] {
            (eq_value[2] as i32).wrapping_add((eq_refine[2] as i32).wrapping_mul(2))
        } else {
            0
        };
        hero.defense = (hero.defense as i32).wrapping_add(def_equip2) as i16;
        // defense = (short) (defense + (equip[3] != null ? equip[3].value : (short) 0));
        let def_equip3: i32 = if eq_present[3] { eq_value[3] as i32 } else { 0 };
        hero.defense = (hero.defense as i32).wrapping_add(def_equip3) as i16;
        // defense = (short) (defense + (equip[4] != null ? equip[4].value : (short) 0));
        let def_equip4: i32 = if eq_present[4] { eq_value[4] as i32 } else { 0 };
        hero.defense = (hero.defense as i32).wrapping_add(def_equip4) as i16;
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
/// **This slice ports the MOVEMENT path (state 1 idle / state 2 stepping) AND the
/// COMBAT/DEATH states (case 3 → [`advance_combo`], case 6 → [`on_death`]).** The
/// following branches reach as-yet-unported classes and are DEFERRED, each clearly
/// marked:
/// - **Regen heals** (`addHp`/`addMp`): the timer *decrements* are ported (observable
///   every world frame), but the fire-body reaches `GameScreen.markHpDirty` (HUD
///   dirty flags owned by the render lane) and is deferred; the timers reset
///   faithfully so play stays bounded.
/// - **Status loop** (`statuses` / `StatusIcon` / poison `Floater`): `statuses` is
///   always empty in this slice (the `.evt` status machinery is unported), so the
///   loop never iterates — the whole block is deferred.
/// - **Death game-over hop** (case 6 → [`on_death`] → `GameState.requestState(16)` +
///   `AudioManager.stopBgm`): ported (`requestState` queues state 16; `stopBgm`).
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
        // case 3 (attacking):
        3 => {
            // if (this.comboIndex == -1) this.comboIndex = (byte) 0;
            {
                let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
                if h.combo_index == -1 {
                    h.combo_index = 0;
                }
            }
            // advanceCombo();
            advance_combo(g, id);
        }
        // case 6 (dead):
        6 => {
            // this.animFrame = (byte) 0;
            {
                let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
                h.battler.anim_frame = 0;
            }
            // if (this.deathTimer <= 0) { onDeath(); return; }
            let death_timer = g.entity_arena[id].as_hero().expect("Hero node").death_timer;
            if death_timer <= 0 {
                on_death(g);
                return;
            }
            // this.deathTimer = (byte) (this.deathTimer - 1);
            let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
            h.death_timer = (death_timer as i32).wrapping_sub(1) as i8;
        }
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

/// `private final void advanceCombo()` (`ao.o:()V`) — advances the attack combo: steps
/// to the next chain link when the current step's frame count is reached, spends MP on
/// entering a step, and fires the class-specific hit(s) on the strike frame.
///
/// After `endCombo` resets the chain (`comboIndex = -1`), the Java falls through to the
/// `if (animFrame == 0)` MP block, which reads `comboSteps[comboIndex]` at `-1` — an
/// `ArrayIndexOutOfBoundsException` in the original. That path is transliterated
/// verbatim (the negative index panics exactly as the uncaught throw terminated); it is
/// only reached on the frame a combo *ends*, so the per-strike drive never enters it.
fn advance_combo(g: &mut Game, id: EntityId) {
    // if (animFrame == comboFrames[comboSteps[comboIndex] - 1][comboIndex]) { ... }
    let (anim_frame, combo_index, max_combo) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (h.battler.anim_frame, h.combo_index, h.max_combo)
    };
    let frame_target = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        let step_kind = h.combo_steps[combo_index as usize];
        h.combo_frames[(step_kind as i32).wrapping_sub(1) as usize][combo_index as usize]
    };
    if anim_frame == frame_target {
        // if (comboIndex + 1 == maxCombo || comboSteps[comboIndex + 1] == 0) endCombo(comboIndex);
        let next_step_zero = {
            let h = g.entity_arena[id].as_hero().expect("Hero node");
            h.combo_steps[(combo_index as i32).wrapping_add(1) as usize] == 0
        };
        if (combo_index as i32).wrapping_add(1) == max_combo as i32 || next_step_zero {
            end_combo(g, id, combo_index as i32);
        } else {
            // else { comboIndex = (byte)(comboIndex + 1); animFrame = (byte) 0; }
            let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
            h.combo_index = (combo_index as i32).wrapping_add(1) as i8;
            h.battler.anim_frame = 0;
        }
    }
    // this.lungeSteps = (byte) 0;
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .lunge_steps = 0;
    // if (this.animFrame == 0) { MP cost / gate }
    let anim_frame = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .battler
        .anim_frame;
    if anim_frame == 0 {
        let combo_index = g.entity_arena[id].as_hero().expect("Hero node").combo_index;
        // int mpCost = (((Equipment) ((Weapon) getEquip(0))).value / 4) + 4;
        let weapon_value = {
            let h = g.entity_arena[id].as_hero().expect("Hero node");
            h.equipment[0]
                .as_ref()
                .expect("NullPointerException: getEquip(0) null")
                .borrow()
                .value
        };
        let mut mp_cost = java_div(weapon_value as i32, 4)
            .expect("weapon.value / 4")
            .wrapping_add(4);
        // if (comboSteps[comboIndex] == 2) mpCost = (mpCost * 7) / 5;
        let step_special =
            g.entity_arena[id].as_hero().expect("Hero node").combo_steps[combo_index as usize] == 2;
        if step_special {
            mp_cost = java_div(mp_cost.wrapping_mul(7), 5).expect("(mpCost * 7) / 5");
        }
        // if (mp < mpCost && (comboIndex != 0 || comboSteps[comboIndex] != 1)) { endCombo(comboIndex - 1); return; }
        let (mp, step_kind) = {
            let h = g.entity_arena[id].as_hero().expect("Hero node");
            (h.mp, h.combo_steps[combo_index as usize])
        };
        if mp < mp_cost && (combo_index != 0 || step_kind != 1) {
            end_combo(g, id, (combo_index as i32).wrapping_sub(1));
            return;
        }
        // addMp(-mpCost);
        add_mp(g, id, mp_cost.wrapping_neg());
    }
    // byte attackType = this.comboSteps[this.comboIndex];
    let (attack_type, combo_index, anim_frame) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (
            h.combo_steps[h.combo_index as usize],
            h.combo_index,
            h.battler.anim_frame,
        )
    };
    // switch (GameState.classId) { case 6/7/8: <hit-frame> performAttack(); }
    match g.game_state.class_id {
        6 => {
            // if (attackType == 2 && comboIndex == 4) { if (animFrame == 1 || animFrame == 6) performAttack(); }
            // else if (animFrame == 2) performAttack();
            if attack_type == 2 && combo_index == 4 {
                if anim_frame == 1 || anim_frame == 6 {
                    perform_attack(g, id);
                }
            } else if anim_frame == 2 {
                perform_attack(g, id);
            }
        }
        7 => {
            // if (attackType==2 && comboIndex==4) { if (animFrame in {0,2,4,6,8,10}) performAttack(); }
            // else if (attackType==2 && comboIndex==3) { if (animFrame==4) performAttack(); }
            // else if (animFrame==1) performAttack();
            if attack_type == 2 && combo_index == 4 {
                if anim_frame == 0
                    || anim_frame == 2
                    || anim_frame == 4
                    || anim_frame == 6
                    || anim_frame == 8
                    || anim_frame == 10
                {
                    perform_attack(g, id);
                }
            } else if attack_type == 2 && combo_index == 3 {
                if anim_frame == 4 {
                    perform_attack(g, id);
                }
            } else if anim_frame == 1 {
                perform_attack(g, id);
            }
        }
        8 => {
            // if (attackType==2 && comboIndex==4) { if (animFrame==4) performAttack(); }
            // else if (animFrame==1) performAttack();
            if attack_type == 2 && combo_index == 4 {
                if anim_frame == 4 {
                    perform_attack(g, id);
                }
            } else if anim_frame == 1 {
                perform_attack(g, id);
            }
        }
        _ => {}
    }
}

/// `private final void performAttack()` (`ao.p:()V`) — rolls damage/crit/proc for this
/// strike, shows the proc floaters (only when an enemy is directly ahead), applies the
/// class-specific hit, and plays the miss sfx when nothing was struck.
fn perform_attack(g: &mut Game, id: EntityId) {
    // this.rolledDamage = rollDamage();
    let rolled_damage = roll_damage(g, id);
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .rolled_damage = rolled_damage;
    // this.rolledProc = rollProc();
    let rolled_proc = roll_proc(g, id);
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .rolled_proc = rolled_proc;
    // this.rolledCrit = rollCrit();
    let rolled_crit = roll_crit(g, id);
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .rolled_crit = rolled_crit;
    // if (enemyAhead() != null) { switch (rolledProc) { ... } }
    if enemy_ahead(g, id).is_some() {
        match rolled_proc {
            // case 2: addFloater(new Floater((byte)10, (short)8, (short)8));
            2 => {
                let f = crate::floater::new(10, 8, 8);
                crate::battler::add_floater(
                    &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
                    f,
                );
            }
            // case 3: addFloater(new Floater((byte)10, (short)8, (short)10));
            3 => {
                let f = crate::floater::new(10, 8, 10);
                crate::battler::add_floater(
                    &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
                    f,
                );
            }
            // case 4: addFloater(new Floater((byte)10, (short)8, (short)11));
            4 => {
                let f = crate::floater::new(10, 8, 11);
                crate::battler::add_floater(
                    &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
                    f,
                );
            }
            // case 8: { int selfDamage = maxHp / 25; addHp(-selfDamage);
            //   floaters.addElement(new Floater((byte)7,(short)4,(short)(-selfDamage)));
            //   addFloater(new Floater((byte)10,(short)8,(short)0)); }
            8 => {
                let self_damage = {
                    let h = g.entity_arena[id].as_hero().expect("Hero node");
                    java_div(h.max_hp, 25).expect("maxHp / 25")
                };
                add_hp(g, id, self_damage.wrapping_neg());
                let f7 = crate::floater::new(7, 4, self_damage.wrapping_neg() as i16);
                crate::battler::add_floater(
                    &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
                    f7,
                );
                let f10 = crate::floater::new(10, 8, 0);
                crate::battler::add_floater(
                    &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
                    f10,
                );
            }
            _ => {}
        }
    }
    // boolean anyHit = false; switch (GameState.classId) { case 6/7/8: anyHit = attackClassN(); }
    let any_hit = match g.game_state.class_id {
        6 => attack_class6(g, id),
        7 => attack_class7(g, id),
        8 => attack_class8(g, id),
        _ => false,
    };
    // if (anyHit) return;
    if any_hit {
        return;
    }
    // AudioManager.playSfx((byte) 14, false);
    crate::audio_manager::play_sfx(g, 14, false);
}

/// The `target.takeHeroHit(rolledDamage, knockback, dir, rolledCrit, hitFloaterKind,
/// rolledProc, this)` idiom the three class-attack routines repeat. Reads the strike's
/// rolled fields fresh each call (as the Java re-reads `this.rolledDamage` etc.) and
/// routes into the enemy lane's [`crate::enemy::take_hero_hit`] (whose body is the
/// enemy lane's own DEFERRED slice — the call is faithful).
fn deal_hero_hit(
    g: &mut Game,
    hero: EntityId,
    target: EntityId,
    knockback: bool,
    dir: i8,
    hit_floater_kind: i8,
) {
    let (rolled_damage, rolled_crit, rolled_proc) = {
        let h = g.entity_arena[hero].as_hero().expect("Hero node");
        (h.rolled_damage, h.rolled_crit, h.rolled_proc)
    };
    crate::enemy::take_hero_hit(
        g,
        target,
        rolled_damage,
        knockback,
        dir,
        rolled_crit,
        hit_floater_kind,
        rolled_proc,
        hero,
    );
}

/// `private final boolean attackClass6()` (`ao.b:()Z`) — warrior (class 6) hit
/// resolution: front-arc, multi-target and lunge patterns.
///
/// **Lunge sub-branch DEFERRED.** The special step-2/3 lunge reads `map.collisionGrid`
/// and `map.isWalkable` (the `.evt` collision, unported — see [`crate::game_map`]) to
/// move and set `lungeSteps`, then re-strikes `enemyAhead()`; that movement + follow-up
/// hit are `// DEFERRED` (collision unported). The leading `enemyAhead()` strike of the
/// step-2/3 branch is ported.
fn attack_class6(g: &mut Game, id: EntityId) -> bool {
    // byte attackType = comboSteps[comboIndex]; byte hitFloaterKind = 1;
    let (attack_type, combo_index, anim_frame, facing) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (
            h.combo_steps[h.combo_index as usize],
            h.combo_index,
            h.battler.anim_frame,
            h.battler.facing,
        )
    };
    // if ((attackType==1 && comboIndex==3) || (attackType==2 && comboIndex==4)) hitFloaterKind = 5;
    let hit_floater_kind: i8 =
        if (attack_type == 1 && combo_index == 3) || (attack_type == 2 && combo_index == 4) {
            5
        } else {
            1
        };
    // boolean anyHit = false;
    let mut any_hit = false;
    if (attack_type == 1 && (combo_index == 0 || combo_index == 3))
        || (attack_type == 2 && combo_index == 4 && anim_frame == 6)
    {
        // Enemy target = enemyAhead(); if (target != null) { target.takeHeroHit(..., false, facing, ...); anyHit = true; }
        if let Some(target) = enemy_ahead(g, id) {
            deal_hero_hit(g, id, target, false, facing, hit_floater_kind);
            any_hit = true;
        }
    } else if (attack_type != 1 || (combo_index != 1 && combo_index != 2))
        && (attack_type != 2 || combo_index != 0)
    {
        if attack_type == 2 && combo_index == 4 {
            // for (byte d = 1; d <= 8; d++) { Enemy t = enemyInDir(d); if (t != null) { t.takeHeroHit(..., true, d, ...); anyHit = true; } }
            let mut dir: i8 = 1;
            loop {
                let d = dir;
                if d > 8 {
                    break;
                }
                if let Some(target) = enemy_in_dir(g, id, d) {
                    deal_hero_hit(g, id, target, true, d, hit_floater_kind);
                    any_hit = true;
                }
                dir = (d as i32).wrapping_add(1) as i8;
            }
        } else if attack_type == 2 && (combo_index == 2 || combo_index == 3) {
            // Enemy target = enemyAhead(); if (target != null) { target.takeHeroHit(..., false, facing, ...); anyHit = true; }
            if let Some(target) = enemy_ahead(g, id) {
                deal_hero_hit(g, id, target, false, facing, hit_floater_kind);
                any_hit = true;
            }
            // byte dx = dirDx[facing]; byte dy = dirDy[facing];
            // if (map.collisionGrid[tileY+dy][tileX+dx] == 0 && map.isWalkable(tileX+dx*2, tileY+dy*2)) { super.move(32); triggerChecked=false; lungeSteps=2; }
            // else if (map.isWalkable(tileX+dx, tileY+dy)) { super.move(16); triggerChecked=false; lungeSteps=1; }
            // if (comboIndex==3 && lungeSteps!=0 && (lungeTarget=enemyAhead())!=null) { lungeTarget.takeHeroHit(...); anyHit=true; }
            //   DEFERRED: GameMap.collisionGrid / GameMap.isWalkable (.evt collision unported).
        }
    } else {
        // Enemy target = enemyInDir(facing); if (target != null) { target.takeHeroHit(...); anyHit = true; }
        let target = enemy_in_dir(g, id, facing);
        if let Some(t) = target {
            deal_hero_hit(g, id, t, false, facing, hit_floater_kind);
            any_hit = true;
        }
        // Enemy sideTarget = enemyInDir(diagCCW[facing]); if (sideTarget != null && sideTarget != target) { ...; anyHit = true; }
        let side_target = enemy_in_dir(g, id, DIAG_CCW[facing as usize]);
        if let Some(st) = side_target {
            if Some(st) != target {
                deal_hero_hit(g, id, st, false, facing, hit_floater_kind);
                any_hit = true;
            }
        }
        // Enemy sideTarget2 = enemyInDir(diagCW[facing]); if (sideTarget2 != null && sideTarget2 != sideTarget) { ...; anyHit = true; }
        let side_target2 = enemy_in_dir(g, id, DIAG_CW[facing as usize]);
        if let Some(st2) = side_target2 {
            if Some(st2) != side_target {
                deal_hero_hit(g, id, st2, false, facing, hit_floater_kind);
                any_hit = true;
            }
        }
    }
    // return anyHit;
    any_hit
}

/// `private final boolean attackClass7()` (`ao.c:()Z`) — rogue (class 7) hit
/// resolution: a spread of enemies ahead (step-2/3), else the two tiles in a line.
fn attack_class7(g: &mut Game, id: EntityId) -> bool {
    let (step_kind, combo_index, facing) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (
            h.combo_steps[h.combo_index as usize],
            h.combo_index,
            h.battler.facing,
        )
    };
    // boolean anyHit = false;
    let mut any_hit = false;
    // if (comboSteps[comboIndex] == 1 && comboIndex == 3) { spread }
    if step_kind == 1 && combo_index == 3 {
        // Enemy target = enemyInDir(facing); if (target != null) { target.takeHeroHit(..., (byte)1, ...); anyHit = true; }
        let target = enemy_in_dir(g, id, facing);
        if let Some(t) = target {
            deal_hero_hit(g, id, t, false, facing, 1);
            any_hit = true;
        }
        // Enemy sideTarget = enemyInDir(diagCCW[facing]); if (sideTarget != null && sideTarget != target) { ...; anyHit = true; }
        let side_target = enemy_in_dir(g, id, DIAG_CCW[facing as usize]);
        if let Some(st) = side_target {
            if Some(st) != target {
                deal_hero_hit(g, id, st, false, facing, 1);
                any_hit = true;
            }
        }
        // Enemy sideTarget2 = enemyInDir(diagCW[facing]); if (sideTarget2 != null && sideTarget2 != sideTarget) { ...; anyHit = true; }
        let side_target2 = enemy_in_dir(g, id, DIAG_CW[facing as usize]);
        if let Some(st2) = side_target2 {
            if Some(st2) != side_target {
                deal_hero_hit(g, id, st2, false, facing, 1);
                any_hit = true;
            }
        }
    } else {
        // Entity target = neighbor(facing, (byte)1); if (target instanceof Enemy) { ((Enemy)target).takeHeroHit(...); anyHit = true; }
        if let Some(t) = neighbor(g, id, facing, 1) {
            if g.entity_arena[t].as_enemy().is_some() {
                deal_hero_hit(g, id, t, false, facing, 1);
                any_hit = true;
            }
        }
        // Entity target2 = neighbor(facing, (byte)2); if (target2 instanceof Enemy) { ((Enemy)target2).takeHeroHit(...); anyHit = true; }
        if let Some(t2) = neighbor(g, id, facing, 2) {
            if g.entity_arena[t2].as_enemy().is_some() {
                deal_hero_hit(g, id, t2, false, facing, 1);
                any_hit = true;
            }
        }
    }
    // return anyHit;
    any_hit
}

/// `private final boolean attackClass8()` (`ao.d:()Z`) — mage (class 8) hit resolution:
/// fires an aura projectile for special steps 2/3/4, else a melee strike ahead.
///
/// The projectile spawn reads the DEFERRED-loaded `AssetCache.mageAuraScripts` bank (a
/// null element NPEs, faithfully — mirroring the enemy lane's `attackEffectScripts`);
/// the `Projectile` class itself is ported (see [`crate::projectile::new_projectile_hero`]).
fn attack_class8(g: &mut Game, id: EntityId) -> bool {
    let (attack_type, combo_index, facing, tile_x, tile_y) = {
        let n = &g.entity_arena[id];
        let h = n.as_hero().expect("Hero node");
        (
            h.combo_steps[h.combo_index as usize],
            h.combo_index,
            h.battler.facing,
            n.tile_x as i32,
            n.tile_y as i32,
        )
    };
    // boolean anyHit = false;
    let mut any_hit = false;
    if attack_type == 2 && (combo_index == 2 || combo_index == 3 || combo_index == 4) {
        // GameState.map.addEntity(new Projectile((byte)(tileX+dirDx[facing]), (byte)(tileY+dirDy[facing]),
        //   (byte[]) AssetCache.mageAuraScripts[<0|1|2>], this, true, facing, (byte)3, (byte)2,
        //   rolledDamage, rolledProc, rolledCrit));
        let script_index: usize = match combo_index {
            2 => 0,
            3 => 1,
            _ => 2, // comboIndex == 4
        };
        let script = g
            .asset_cache
            .mage_aura_scripts
            .as_ref()
            .expect("AssetCache.mageAuraScripts null in Hero.attackClass8")[script_index]
            .clone()
            .expect("mageAuraScripts[k] null (DEFERRED-loaded bank)");
        let (rolled_damage, rolled_proc, rolled_crit) = {
            let h = g.entity_arena[id].as_hero().expect("Hero node");
            (h.rolled_damage, h.rolled_proc, h.rolled_crit)
        };
        let ptx = tile_x.wrapping_add(DIR_DX[facing as usize] as i32) as i8;
        let pty = tile_y.wrapping_add(DIR_DY[facing as usize] as i32) as i8;
        let new_id = crate::projectile::new_projectile_hero(
            &mut g.entity_arena,
            ptx,
            pty,
            script,
            id,
            true,
            facing,
            3,
            2,
            rolled_damage,
            rolled_proc,
            rolled_crit,
        );
        crate::game_map::add_entity(g, new_id);
    } else {
        // Enemy target = enemyAhead(); if (target != null) { target.takeHeroHit(..., (byte)1, ...); anyHit = true; }
        if let Some(target) = enemy_ahead(g, id) {
            deal_hero_hit(g, id, target, false, facing, 1);
            any_hit = true;
        }
    }
    // return anyHit;
    any_hit
}

/// `private final int rollDamage()` (`ao.a:()I`) — rolls attack damage: base
/// [`attack`](HeroData::attack) (×3/2 while `attackUp`), scaled by the per-combo-step
/// multiplier (100/120/130/140/170%, or 170% for a special), plus up to 10% RNG. The
/// four `idiv`s and the `%` route through [`java_div`]/[`java_rem`]; the RNG term reads
/// [`crate::entity::EntityState::rng`].
fn roll_damage(g: &mut Game, id: EntityId) -> i32 {
    let (attack, attack_up, combo_index, step_special) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (
            h.attack,
            h.attack_up,
            h.combo_index,
            h.combo_steps[h.combo_index as usize] == 2,
        )
    };
    // int dmg = this.attack;
    let mut dmg: i32 = attack as i32;
    // if (this.attackUp) dmg = (this.attack * 3) / 2;
    if attack_up {
        dmg = java_div((attack as i32).wrapping_mul(3), 2).expect("(attack * 3) / 2");
    }
    // if (comboSteps[comboIndex] != 2) switch (comboIndex) { ... } else dmg = (dmg * 17) / 10;
    if !step_special {
        match combo_index {
            0 => dmg = java_div(dmg.wrapping_mul(10), 10).expect("(dmg * 10) / 10"),
            1 => dmg = java_div(dmg.wrapping_mul(12), 10).expect("(dmg * 12) / 10"),
            2 => dmg = java_div(dmg.wrapping_mul(13), 10).expect("(dmg * 13) / 10"),
            3 => dmg = java_div(dmg.wrapping_mul(14), 10).expect("(dmg * 14) / 10"),
            4 => dmg = java_div(dmg.wrapping_mul(17), 10).expect("(dmg * 17) / 10"),
            _ => {}
        }
    } else {
        dmg = java_div(dmg.wrapping_mul(17), 10).expect("(dmg * 17) / 10");
    }
    // return dmg + (dmg >= 10 ? Entity.rng.nextInt() % (dmg / 10) : 0);
    let extra = if dmg >= 10 {
        java_rem(
            g.entity.rng.next_int(),
            java_div(dmg, 10).expect("dmg / 10"),
        )
        .expect("nextInt() % (dmg / 10)")
    } else {
        0
    };
    dmg.wrapping_add(extra)
}

/// `private final boolean rollCrit()` (`ao.e:()Z`) — rolls a critical: chance =
/// `agility/3 + spirit/10 + weapon.accuracy`, out of 100. `Math.abs` is inlined
/// (`Math.abs(i32::MIN)` overflows, unlike `i32::abs`).
fn roll_crit(g: &mut Game, id: EntityId) -> bool {
    let (agility, agility_bonus, spirit, spirit_bonus, accuracy) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        let accuracy = h.equipment[0]
            .as_ref()
            .expect("NullPointerException: equipment[0] null")
            .borrow()
            .accuracy;
        (
            h.agility,
            h.agility_bonus,
            h.spirit,
            h.spirit_bonus,
            accuracy,
        )
    };
    // Math.abs(Entity.rng.nextInt() % 100) < (((agility+agilityBonus)/3) + ((spirit+spiritBonus)/10)) + weapon.accuracy
    let modded = java_rem(g.entity.rng.next_int(), 100).expect("nextInt() % 100");
    let lhs = if modded < 0 {
        modded.wrapping_neg()
    } else {
        modded
    };
    let a_term = java_div((agility as i32).wrapping_add(agility_bonus as i32), 3)
        .expect("(agility + agilityBonus) / 3");
    let s_term = java_div((spirit as i32).wrapping_add(spirit_bonus as i32), 10)
        .expect("(spirit + spiritBonus) / 10");
    let rhs = a_term.wrapping_add(s_term).wrapping_add(accuracy as i32);
    lhs < rhs
}

/// `private final byte rollProc()` (`ao.a:()B`) — rolls a weapon-then-armour status
/// proc (`Armor.PROC_CHANCE[attribute]` out of 100), or `-1`. `equipment[1]` (armour)
/// may be `null` (the warrior/rogue have no shield), guarded here.
fn roll_proc(g: &mut Game, id: EntityId) -> i8 {
    // Weapon weapon = (Weapon) equipment[0]; Armor armor = (Armor) equipment[1];
    let weapon_attribute = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        h.equipment[0]
            .as_ref()
            .expect("NullPointerException: equipment[0] null")
            .borrow()
            .attribute
    };
    let armor_attribute: Option<i8> = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        h.equipment[1].as_ref().map(|a| a.borrow().attribute)
    };
    // byte proc = -1;
    let mut proc: i8 = -1;
    // if (weapon.attribute != -1 && ByteUtil.randRange(0,99) < Armor.PROC_CHANCE[weapon.attribute]) proc = weapon.attribute;
    if weapon_attribute != -1
        && crate::byte_util::rand_range(&mut g.byte_util, 0, 99)
            < crate::armor::PROC_CHANCE[weapon_attribute as usize] as i32
    {
        proc = weapon_attribute;
    }
    // if (proc == -1 && armor != null && armor.attribute != -1 && ByteUtil.randRange(0,99) < Armor.PROC_CHANCE[armor.attribute]) proc = armor.attribute;
    if proc == -1 {
        if let Some(attr) = armor_attribute {
            if attr != -1
                && crate::byte_util::rand_range(&mut g.byte_util, 0, 99)
                    < crate::armor::PROC_CHANCE[attr as usize] as i32
            {
                proc = attr;
            }
        }
    }
    // return proc;
    proc
}

/// `private final void endCombo(int step)` (`ao.h:(I)V`) — ends the current combo at
/// step `step`, setting the recovery lockout (1 for a light finish, 3 otherwise).
fn end_combo(g: &mut Game, id: EntityId, step: i32) {
    {
        let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // if (step == -1 || (step == 0 && comboSteps[step] == 1)) comboLockout = 1;
        if step == -1 || (step == 0 && h.combo_steps[step as usize] == 1) {
            h.combo_lockout = 1;
        // else if (!(step == 0 && comboSteps[step] == 2) && comboSteps[step] == 1) comboLockout = 1;
        } else if !(step == 0 && h.combo_steps[step as usize] == 2)
            && h.combo_steps[step as usize] == 1
        {
            h.combo_lockout = 1;
        // else comboLockout = 3;
        } else {
            h.combo_lockout = 3;
        }
    }
    // resetCombo();
    reset_combo(g, id);
    // setState((byte) 1); this.animFrame = (byte) 0;
    let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
    crate::battler::set_state(&mut h.battler, 1);
    h.battler.anim_frame = 0;
}

/// `private final void onDeath()` (`ao.q:()V`) — player death: request the game-over
/// screen (`requestState(16)`) and stop the music.
fn on_death(g: &mut Game) {
    // GameState.requestState((byte) 16);
    crate::game_state::request_state(g, 16);
    // AudioManager.stopBgm();
    crate::audio_manager::stop_bgm(g);
}

/// `public final boolean queueComboStep(boolean special)` (`ao.a:(Z)Z`) — queues the
/// next combo step (normal or special), if combat is enabled, the combo has room, and it
/// is not locked out. Returns whether a step was queued (or is already pending).
pub fn queue_combo_step(g: &mut Game, id: EntityId, special: bool) -> bool {
    // if (!GameState.map.combatEnabled || comboIndex + 1 >= maxCombo || comboLockout > 0) return false;
    let combat_enabled = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map null in Hero.queueComboStep")
        .combat_enabled;
    let (combo_index, max_combo, combo_lockout) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (h.combo_index, h.max_combo, h.combo_lockout)
    };
    if !combat_enabled
        || (combo_index as i32).wrapping_add(1) >= max_combo as i32
        || combo_lockout > 0
    {
        return false;
    }
    let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
    // if (comboSteps[comboIndex + 1] != 0) return true;
    if h.combo_steps[(combo_index as i32).wrapping_add(1) as usize] != 0 {
        return true;
    }
    // if (comboIndex >= 0 && comboSteps[comboIndex] == 2) return false;
    if combo_index >= 0 && h.combo_steps[combo_index as usize] == 2 {
        return false;
    }
    // if (comboIndex == 0 && special) return false;
    if combo_index == 0 && special {
        return false;
    }
    // if (comboIndex == 3 && !special) return false;
    if combo_index == 3 && !special {
        return false;
    }
    // comboSteps[comboIndex + 1] = special ? (byte) 2 : (byte) 1; return true;
    h.combo_steps[(combo_index as i32).wrapping_add(1) as usize] = if special { 2 } else { 1 };
    true
}

/// `public final void addHp(int amount)` (`ao.a:(I)V`) — adds `amount` HP (clamped to
/// `[0, maxHp]`), then on reaching 0 enters death (state 6) with a 24-frame death timer.
/// `GameLoop.gameScreen.markHpDirty()` is the render-lane HUD flag — DEFERRED.
pub fn add_hp(g: &mut Game, id: EntityId, amount: i32) {
    let hp_zero = {
        let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
        // this.hp += amount;
        h.hp = h.hp.wrapping_add(amount);
        // if (hp > maxHp) hp = maxHp; if (hp < 0) hp = 0;
        if h.hp > h.max_hp {
            h.hp = h.max_hp;
        }
        if h.hp < 0 {
            h.hp = 0;
        }
        h.hp == 0
    };
    // GameLoop.gameScreen.markHpDirty();  — DEFERRED (render-lane HUD dirty flag).
    // if (this.hp == 0) { setState((byte) 6); this.animFrame = (byte) 0; this.deathTimer = (byte) 24; }
    if hp_zero {
        let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
        crate::battler::set_state(&mut h.battler, 6);
        h.battler.anim_frame = 0;
        h.death_timer = 24;
    }
}

/// `public final void addMp(int amount)` (`ao.a:(I)V`) — adds `amount` MP (clamped to
/// `[0, maxMp]`). `GameLoop.gameScreen.markMpDirty()` is the render-lane HUD flag —
/// DEFERRED.
pub fn add_mp(g: &mut Game, id: EntityId, amount: i32) {
    let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
    // this.mp += amount;
    h.mp = h.mp.wrapping_add(amount);
    // if (mp > maxMp) mp = maxMp; if (mp < 0) mp = 0;
    if h.mp > h.max_mp {
        h.mp = h.max_mp;
    }
    if h.mp < 0 {
        h.mp = 0;
    }
    // GameLoop.gameScreen.markMpDirty();  — DEFERRED (render-lane HUD dirty flag).
}

/// `public final void takeHit(Enemy attacker, byte dir)` (`ao.a:(Lal;B)V`) — the
/// template-attack overload: `takeHit(attacker, attacker.stats.attack, dir)`.
pub fn take_hit(g: &mut Game, id: EntityId, attacker: EntityId, dir: i8) {
    // takeHit(attacker, attacker.stats.attack, dir);
    let raw_damage = g.entity_arena[attacker]
        .as_enemy()
        .expect("takeHit attacker is not an Enemy")
        .stats
        .attack;
    take_hit_raw(g, id, attacker, raw_damage, dir);
}

/// `public final void takeHit(Enemy attacker, short rawDamage, byte dir)`
/// (`ao.a:(Lal;SB)V`) — resolves incoming damage of magnitude `rawDamage` from
/// `attacker`. Dodge chance = `clamp((agility+bonus) - evasion + 10 + accessory, 8,
/// 60)%`; on a hit, damage = `rawDamage ±10% - defense` (doubled while `defenseUp`).
/// Ignored entirely while `invincible`; a melee-type-1 (`aiType == 1`) attacker has a
/// 15% chance to inflict poison (status 7). On a lethal hit [`add_hp`] enters death
/// (state 6).
///
/// **DEFERRED cross-class hops.** The `reflectDamage` strike-back
/// (`attacker.damage((activeGuardian.level * 2) + 40 + spirit)`) reads the unported
/// `Guardian` — `// DEFERRED: Guardian`; `reflectDamage` is always false in this slice,
/// so the branch is unreachable. `GameLoop.gameScreen.setTarget(attacker, true)` is the
/// render-lane HUD target flag — DEFERRED.
// The dodge cap is the Java's two sequential `if`s (`> 60 → 60`, then `< 8 → 8`), not a
// single `clamp` — the source structure/order is preserved verbatim.
#[allow(clippy::manual_clamp)]
pub fn take_hit_raw(g: &mut Game, id: EntityId, attacker: EntityId, raw_damage: i16, dir: i8) {
    // if (state == 6 || state == 5 || invincible) return;
    let (state, invincible, reflect_damage) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (h.battler.state, h.invincible, h.reflect_damage)
    };
    if state == 6 || state == 5 || invincible {
        return;
    }
    // if (this.reflectDamage) attacker.damage((activeGuardian.level * 2) + 40 + spirit);
    if reflect_damage {
        // DEFERRED: Guardian — the reflect strike-back reads activeGuardian.level (the
        //   unported Guardian). reflectDamage is always false in this slice (its only
        //   producer is the DEFERRED guardian buff), so this branch is unreachable.
        unreachable!("DEFERRED: Guardian — reflectDamage requires activeGuardian");
    }
    // GameLoop.gameScreen.setTarget(attacker, true);  — DEFERRED (render-lane HUD target).
    // int dodgeChance = ((agility + agilityBonus) - attacker.stats.evasion) + 10;
    let (agility, agility_bonus, defense, defense_up, acc_refine) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        let acc_refine = h.equipment[2].as_ref().map(|e| e.borrow().refine_level);
        (
            h.agility,
            h.agility_bonus,
            h.defense,
            h.defense_up,
            acc_refine,
        )
    };
    let (evasion, ai_type) = {
        let e = g.entity_arena[attacker]
            .as_enemy()
            .expect("takeHit attacker is not an Enemy");
        (e.stats.evasion, e.stats.ai_type)
    };
    let mut dodge_chance = (agility as i32)
        .wrapping_add(agility_bonus as i32)
        .wrapping_sub(evasion as i32)
        .wrapping_add(10);
    // if (this.equipment[2] != null) dodgeChance += this.equipment[2].refineLevel;
    if let Some(rl) = acc_refine {
        dodge_chance = dodge_chance.wrapping_add(rl as i32);
    }
    // if (dodgeChance > 60) dodgeChance = 60; if (dodgeChance < 8) dodgeChance = 8;
    if dodge_chance > 60 {
        dodge_chance = 60;
    }
    if dodge_chance < 8 {
        dodge_chance = 8;
    }
    // if (ByteUtil.randRange(0, 99) < dodgeChance) { floaters.addElement(new Floater((byte) 2)); return; }
    if crate::byte_util::rand_range(&mut g.byte_util, 0, 99) < dodge_chance {
        let f = crate::floater::new_default(2);
        crate::battler::add_floater(
            &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
            f,
        );
        return;
    }
    // int finalDamage = (rawDamage + ByteUtil.randRange(-(rawDamage/10), rawDamage/10)) - (defenseUp ? defense*2 : defense);
    let variance_bound = java_div(raw_damage as i32, 10).expect("rawDamage / 10");
    let variance = crate::byte_util::rand_range(
        &mut g.byte_util,
        variance_bound.wrapping_neg(),
        variance_bound,
    );
    let def_term = if defense_up {
        (defense as i32).wrapping_mul(2)
    } else {
        defense as i32
    };
    let final_damage = (raw_damage as i32)
        .wrapping_add(variance)
        .wrapping_sub(def_term);
    // int appliedDamage = finalDamage;
    let mut applied_damage = final_damage;
    // if (finalDamage > 0) { addHp(-appliedDamage); addFloater(new Floater((byte) 6)); }
    if final_damage > 0 {
        add_hp(g, id, applied_damage.wrapping_neg());
        let f6 = crate::floater::new_default(6);
        crate::battler::add_floater(
            &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
            f6,
        );
    }
    // if (appliedDamage < 0) appliedDamage = 0;
    if applied_damage < 0 {
        applied_damage = 0;
    }
    // floaters.addElement(new Floater((byte) 7, (short) 4, (short) (-appliedDamage)));
    {
        let f7 = crate::floater::new(7, 4, applied_damage.wrapping_neg() as i16);
        crate::battler::add_floater(
            &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
            f7,
        );
    }
    // if (attacker.stats.aiType == 1 && ByteUtil.randRange(0, 99) < 15) applyStatus((byte) 7);
    if ai_type == 1 && crate::byte_util::rand_range(&mut g.byte_util, 0, 99) < 15 {
        crate::battler::apply_status(
            &mut g.entity_arena[id].as_hero_mut().expect("Hero node").battler,
            7,
        );
    }
    // this.recoilTimer = (byte) 1; this.recoilDir = dir;
    let h = g.entity_arena[id].as_hero_mut().expect("Hero node");
    h.recoil_timer = 1;
    h.recoil_dir = dir;
}

// --- Entity/Battler occupancy scans, inlined in the Hero combat lane. `Battler`
//     (`o`) and `Entity` (`ck`) are read-only in this lane, and the Hero's combat FSM
//     is the sole caller in this batch — reproduced here as `enemy.rs` reproduces the
//     same helpers (`enemy::neighbor`/`entity_in_dir` are private to that module). ----

/// `public final Enemy enemyAhead()` (`o.a:()Lal;`) — the [`crate::enemy`] on the tile
/// straight ahead (facing), or `None` (a non-Enemy occupant answers `None`).
fn enemy_ahead(g: &Game, id: EntityId) -> Option<EntityId> {
    // Entity ahead = entityInDir(this.facing, null); return (ahead instanceof Enemy) ? (Enemy) ahead : null;
    let facing = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .battler
        .facing;
    let ahead = entity_in_dir(g, id, facing, None)?;
    if g.entity_arena[ahead].as_enemy().is_some() {
        Some(ahead)
    } else {
        None
    }
}

/// `public final Enemy enemyInDir(byte dir)` (`o.a:(B)Lal;`) — the [`crate::enemy`] on
/// the adjacent tile in `dir`, or `None` (a non-Enemy occupant answers `None`).
fn enemy_in_dir(g: &Game, id: EntityId, dir: i8) -> Option<EntityId> {
    // Entity found = entityInDir(dir, null); return (found instanceof Enemy) ? (Enemy) found : null;
    let found = entity_in_dir(g, id, dir, None)?;
    if g.entity_arena[found].as_enemy().is_some() {
        Some(found)
    } else {
        None
    }
}

/// `public final Entity entityInDir(byte dir, Entity wanted)` (`o.a:(BLck;)Lck;`) —
/// scans the `layer` tiles adjacent in `dir`: with `wanted == None` returns the first
/// occupant found, else returns `wanted` iff it occupies one of those tiles.
fn entity_in_dir(g: &Game, id: EntityId, dir: i8, wanted: Option<EntityId>) -> Option<EntityId> {
    let (tile_x, tile_y, layer) = {
        let n = &g.entity_arena[id];
        (n.tile_x as i32, n.tile_y as i32, n.layer as i32)
    };
    let map = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map null in Battler.entityInDir");
    let (width_tiles, height_tiles) = (map.width_tiles, map.height_tiles);
    let occ = map
        .occupancy
        .as_ref()
        .expect("occupancy null in Battler.entityInDir");
    // for (int col = 0; col < this.layer; col++) {
    let mut col: i32 = 0;
    while col < layer {
        // int scanX = tileX + Directions.dirDx[dir] + col; int scanY = tileY + Directions.dirDy[dir];
        let scan_x = tile_x
            .wrapping_add(DIR_DX[dir as usize] as i32)
            .wrapping_add(col);
        let scan_y = tile_y.wrapping_add(DIR_DY[dir as usize] as i32);
        // Debug.assertTrue(scanX >= 0 && scanX < widthTiles && scanY >= 0 && scanY < heightTiles);
        crate::debug::assert_true(scan_x >= 0);
        crate::debug::assert_true(scan_x < width_tiles);
        crate::debug::assert_true(scan_y >= 0);
        crate::debug::assert_true(scan_y < height_tiles);
        // Entity occupant = occupancy[scanY][scanX];
        let occupant = occ[scan_y as usize][scan_x as usize];
        // if (occupant != this) {
        if occupant != Some(id) {
            // if (wanted == null && occupant != null) return occupant;
            if wanted.is_none() && occupant.is_some() {
                return occupant;
            }
            // if (wanted != null && occupant == wanted) return occupant;
            if wanted.is_some() && occupant == wanted {
                return occupant;
            }
        }
        col = col.wrapping_add(1);
    }
    // return null;
    None
}

/// `public final Entity neighbor(byte direction, byte distance)` (`ck.a:(BB)Lck;`) —
/// the entity `distance` tiles away in `direction` (1 up, 2 down, 3 left, 4 right), or
/// `None` when off-map / empty.
fn neighbor(g: &Game, id: EntityId, direction: i8, distance: i8) -> Option<EntityId> {
    let (tile_x, tile_y) = {
        let n = &g.entity_arena[id];
        (n.tile_x as i32, n.tile_y as i32)
    };
    let map = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map null in Entity.neighbor");
    let occ = map
        .occupancy
        .as_ref()
        .expect("occupancy null in Entity.neighbor");
    let dist = distance as i32;
    match direction {
        // case 1: if (tileY - distance < 0) return null; return occupancy[tileY - distance][tileX];
        1 => {
            if tile_y.wrapping_sub(dist) < 0 {
                None
            } else {
                occ[tile_y.wrapping_sub(dist) as usize][tile_x as usize]
            }
        }
        // case 2: if (tileY + distance >= heightTiles) return null; return occupancy[tileY + distance][tileX];
        2 => {
            if tile_y.wrapping_add(dist) >= map.height_tiles {
                None
            } else {
                occ[tile_y.wrapping_add(dist) as usize][tile_x as usize]
            }
        }
        // case 3: if (tileX - distance < 0) return null; return occupancy[tileY][tileX - distance];
        3 => {
            if tile_x.wrapping_sub(dist) < 0 {
                None
            } else {
                occ[tile_y as usize][tile_x.wrapping_sub(dist) as usize]
            }
        }
        // case 4: if (tileX + distance >= widthTiles) return null; return occupancy[tileY][tileX + distance];
        4 => {
            if tile_x.wrapping_add(dist) >= map.width_tiles {
                None
            } else {
                occ[tile_y as usize][tile_x.wrapping_add(dist) as usize]
            }
        }
        // default: return null;
        _ => None,
    }
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ao.a:(…Graphics;II)V => [iadd,iadd,iadd,iadd,imul,iadd,imul,iadd,isub,i2b,isub,
/// imul,imul,iadd,imul,imul,iadd,isub]`) — draws the hero: the ground shadow, then
/// the layered character/attack sprite selected by [`BattlerData::state`], then the
/// (empty in this slice) status/floater overlays.
///
/// **MILESTONE bridge (the DEFERRED pre-paint update).** In the original,
/// `GameScreen.paint` case 2 runs `GameState.update()` — which ticks `hero.update()`,
/// advancing `animFrame` from its post-`init` sentinel `-1` to `0` — *before*
/// `map.paint` reaches here. That world-sim update is owned by the parallel movement
/// lane and is DEFERRED, so on this slice's first world frame `animFrame` is still
/// `-1`. [`draw_frame`](crate::game_screen::draw_frame) is byte-exact and would index
/// the draw script negatively on `-1`; to render the idle rest pose (exactly the
/// frame the first real `update()` produces) without that fault, the paint entry
/// normalizes the `-1` sentinel up to `0` here. Once the FSM lane lands (advancing
/// `animFrame` in `update`), `animFrame >= 0` and this normalization is identity.
///
/// `drawStatusIcons`/`drawFloaters` iterate the hero's `statuses`/`floaters` overlay
/// lists, which are empty in this slice (the overlay classes are DEFERRED — see
/// [`crate::battler`]), so they are no-ops and their bodies stay DEFERRED.
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // int screenX = originX + pixelX + halfW; int screenY = originY + pixelY + halfH;
    let (pixel_x, pixel_y, half_w, half_h) = {
        let n = &g.entity_arena[id];
        (
            n.pixel_x as i32,
            n.pixel_y as i32,
            n.half_w as i32,
            n.half_h as i32,
        )
    };
    let mut screen_x = origin_x.wrapping_add(pixel_x).wrapping_add(half_w);
    let mut screen_y = origin_y.wrapping_add(pixel_y).wrapping_add(half_h);
    // if (recoilTimer == 1) { screenX += dirDx[recoilDir]*2; screenY += dirDy[recoilDir]*2;
    //   recoilTimer = (byte)(recoilTimer - 1); }
    let recoil_timer = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .recoil_timer;
    if recoil_timer == 1 {
        let recoil_dir = g.entity_arena[id].as_hero().expect("Hero node").recoil_dir;
        screen_x = screen_x.wrapping_add((DIR_DX[recoil_dir as usize] as i32).wrapping_mul(2));
        screen_y = screen_y.wrapping_add((DIR_DY[recoil_dir as usize] as i32).wrapping_mul(2));
        g.entity_arena[id]
            .as_hero_mut()
            .expect("Hero node")
            .recoil_timer = (recoil_timer as i32).wrapping_sub(1) as i8;
    }
    // Snapshot the FSM fields the switch reads (state / facing / animFrame / comboIndex /
    // comboSteps / lungeSteps).
    let (state, facing, anim_frame_raw, combo_index, lunge_steps) = {
        let h = g.entity_arena[id].as_hero().expect("Hero node");
        (
            h.battler.state,
            h.battler.facing,
            h.battler.anim_frame,
            h.combo_index,
            h.lunge_steps,
        )
    };
    let combo_steps = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .combo_steps
        .clone();
    // MILESTONE bridge: normalize the -1 sentinel (see the doc note above).
    let anim_frame = if anim_frame_raw < 0 {
        0
    } else {
        anim_frame_raw
    };
    // The weapon/aura layers are gated on GameState.map.combatEnabled.
    let combat_enabled = g
        .game_state
        .map
        .as_ref()
        .expect("GameState.map null in Hero.paint")
        .combat_enabled;
    // The world clip GameMap.paint left active (0, 0, width, worldHeight).
    let width = g.game_screen.width;
    let world_height = g.game_screen.world_height;

    let Game {
        screen,
        asset_cache,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    // (re-establish GameMap.paint's persistent world clip on this fresh Graphics.)
    graphics.set_clip(0, 0, width, world_height);
    // graphics.drawImage(AssetCache.entityShadow, screenX, screenY - 3, 17);
    let shadow = asset_cache
        .entity_shadow
        .as_ref()
        .expect("NullPointerException: entityShadow null");
    graphics
        .draw_image(shadow, screen_x, screen_y.wrapping_sub(3), 17)
        .expect("drawImage(entityShadow)");
    // switch (this.state)  (1/4 idle → pose 0, 2 stepping → pose 1, 3 attack, 6 dead → pose 2)
    match state {
        1 | 4 => draw_character_sprite(
            &mut graphics,
            asset_cache,
            combat_enabled,
            0,
            facing,
            anim_frame,
            screen_x,
            screen_y,
        ),
        2 => draw_character_sprite(
            &mut graphics,
            asset_cache,
            combat_enabled,
            1,
            facing,
            anim_frame,
            screen_x,
            screen_y,
        ),
        3 => {
            draw_attack_sprite(
                &mut graphics,
                asset_cache,
                combat_enabled,
                combo_index,
                &combo_steps,
                facing,
                anim_frame,
                screen_x,
                screen_y,
            );
            // if (lungeSteps != 0) drawAttackSprite at the afterimage offset.
            if lunge_steps != 0 {
                let rev = REVERSE[facing as usize];
                let lunge_x = screen_x.wrapping_add(
                    (DIR_DX[rev as usize] as i32)
                        .wrapping_mul(16)
                        .wrapping_mul(lunge_steps as i32),
                );
                let lunge_y = screen_y.wrapping_add(
                    (DIR_DY[rev as usize] as i32)
                        .wrapping_mul(16)
                        .wrapping_mul(lunge_steps as i32),
                );
                draw_attack_sprite(
                    &mut graphics,
                    asset_cache,
                    combat_enabled,
                    combo_index,
                    &combo_steps,
                    facing,
                    anim_frame,
                    lunge_x,
                    lunge_y,
                );
            }
        }
        6 => draw_character_sprite(
            &mut graphics,
            asset_cache,
            combat_enabled,
            2,
            1,
            anim_frame,
            screen_x,
            screen_y,
        ),
        _ => {}
    }
    // drawStatusIcons(graphics, screenX, screenY - 8); drawFloaters(graphics, screenX, screenY);
    //   — statuses/floaters empty in this slice → no-op (overlay render DEFERRED).
}

/// `private void drawCharacterSprite(byte pose, byte dir, Graphics graphics, int x,
/// int y)` (`ao.a:(BB…Graphics;II)V => [imul, isub, imul, iadd, iadd, iadd, iinc]`)
/// — draws the 9-layer character sprite for `pose` facing `dir`. Layer 7 (aura) is a
/// frame *group*; every other layer is a single frame. Layers 6/7 (weapon/aura) draw
/// only while `combatEnabled`.
#[allow(clippy::too_many_arguments)]
fn draw_character_sprite(
    graphics: &mut j2me_me::Graphics,
    asset_cache: &AssetCacheState,
    combat_enabled: bool,
    pose: i8,
    dir: i8,
    anim_frame: i8,
    x: i32,
    y: i32,
) {
    // int baseIndex = (pose * 36) + ((dir - 1) * 9);
    let base_index = (pose as i32)
        .wrapping_mul(36)
        .wrapping_add((dir as i32).wrapping_sub(1).wrapping_mul(9));
    // for (int layer = 0; layer < 9; layer++)
    let mut layer: i32 = 0;
    while layer < 9 {
        // if ((layer != 6 && layer != 7) || GameState.map.combatEnabled)
        if (layer != 6 && layer != 7) || combat_enabled {
            // heroFrames[baseIndex + layer]  (a byte[] draw script, or Java null).
            let idx = base_index.wrapping_add(layer);
            let frames = asset_cache
                .hero_frames
                .as_ref()
                .expect("NullPointerException: heroFrames null")[idx as usize]
                .as_deref();
            if layer == 7 {
                // GameScreen.drawFrameGroup(graphics, heroFrames[baseIndex+7], animFrame, x, y);
                game_screen::draw_frame_group(graphics, asset_cache, frames, anim_frame, x, y);
            } else {
                // GameScreen.drawFrame(graphics, heroFrames[baseIndex+layer], animFrame, x, y);
                game_screen::draw_frame(graphics, asset_cache, frames, anim_frame, x, y);
            }
        }
        layer = layer.wrapping_add(1);
    }
}

/// `private void drawAttackSprite(Graphics graphics, int x, int y)`
/// (`ao.e:(…Graphics;II)V => []`) — selects the attack pose for the current combo
/// step (`comboIndex`, and whether it is a normal `1` or special step) and draws it
/// via [`draw_character_sprite`]. Only reached in `state == 3`, where `comboIndex`
/// is always `0..=4`.
#[allow(clippy::too_many_arguments)]
fn draw_attack_sprite(
    graphics: &mut j2me_me::Graphics,
    asset_cache: &AssetCacheState,
    combat_enabled: bool,
    combo_index: i8,
    combo_steps: &[i8],
    facing: i8,
    anim_frame: i8,
    x: i32,
    y: i32,
) {
    // byte pose = -1; switch (comboIndex) { ... }
    let mut pose: i8 = -1;
    match combo_index {
        0 => {
            pose = if combo_steps[combo_index as usize] != 1 {
                7
            } else {
                3
            }
        }
        1 => pose = 4,
        2 => {
            pose = if combo_steps[combo_index as usize] != 1 {
                8
            } else {
                5
            }
        }
        3 => {
            pose = if combo_steps[combo_index as usize] != 1 {
                9
            } else {
                6
            }
        }
        4 => pose = 10,
        _ => {}
    }
    // drawCharacterSprite(pose, facing, graphics, x, y);
    draw_character_sprite(
        graphics,
        asset_cache,
        combat_enabled,
        pose,
        facing,
        anim_frame,
        x,
        y,
    );
}

/// `public final byte[] save()` (`ao.a:()[B => [iinc, iinc, iadd, i2b, ldiv, i2l,
/// lsub, l2i, iadd]`) — serializes the hero to a `byte[]` through a
/// `DataOutputStream` over a `ByteArrayOutputStream`: `classId`/`level`, the four
/// `int`s `hp`/`mp`/`exp` + the recomputable `maxHp`/`maxMp`/`expToNext`,
/// `maxCombo`/`statPoints`, the four base stats, then five equipment slots
/// (`[flag][10-byte Item.serialize]`) and five guardian slots (`[flag]…`), finally the
/// accumulated `playSeconds`.
///
/// The `try { … } catch (IOException)` around the in-memory streams never throws (cf.
/// [`crate::item_bag::serialize`]), so a `byte[]` is always produced (never null);
/// modelled as an infallible `Vec<i8>` return. The `DataOutputStream` big-endian byte
/// work is reproduced inline (as `item_bag` reproduces `writeInt`).
///
/// **Guardian DEFERRED.** The `guardians` slots are all-null in this slice, so the
/// guardian presence loop writes a single `0` byte per slot (faithful for null
/// guardians); the non-null else-branch (writing `type`/`level`/`exp`/`skillSlotA`/
/// `skillSlotB` — reaching the unported `Guardian`) is unreachable and marked
/// `// DEFERRED: Guardian`. The active-guardian block —
/// `Debug.assertTrue(activeGuardian != null)`, the `activeIndex` search (whose
/// `b3 = (byte)(b3 + 1)` is the shape's `iadd,i2b`), and `writeByte(activeIndex)` — is
/// `// DEFERRED: Guardian`: `activeGuardian` is null here (its assignment is the
/// deferred guardian-summon setup), so the assert would fire and no index is
/// meaningful. That single `activeIndex` byte (and its `iadd,i2b`) is therefore
/// omitted from the blob; [`load`] omits the matching read, so the pair round-trips.
/// The trailing `playSeconds` `int` IS written faithfully (`ldiv/i2l/lsub/l2i/iadd`).
pub fn save(g: &mut Game, id: EntityId) -> Vec<i8> {
    // ByteArrayOutputStream + DataOutputStream — an in-memory byte sink.
    let mut out: Vec<i8> = Vec::new();
    {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        // writeByte(this.classId); writeByte(this.level);
        write_byte(&mut out, hero.class_id);
        write_byte(&mut out, hero.level);
        // writeInt(hp); writeInt(mp); writeInt(exp); writeInt(maxHp); writeInt(maxMp); writeInt(expToNext);
        write_int(&mut out, hero.hp);
        write_int(&mut out, hero.mp);
        write_int(&mut out, hero.exp);
        write_int(&mut out, hero.max_hp);
        write_int(&mut out, hero.max_mp);
        write_int(&mut out, hero.exp_to_next);
        // writeByte(maxCombo);
        write_byte(&mut out, hero.max_combo);
        // writeShort(statPoints); writeShort(strength); writeShort(vitality); writeShort(agility); writeShort(spirit);
        write_short(&mut out, hero.stat_points);
        write_short(&mut out, hero.strength);
        write_short(&mut out, hero.vitality);
        write_short(&mut out, hero.agility);
        write_short(&mut out, hero.spirit);
        // for (int i = 0; i < 5; i++) { if (equipment[i] == null) writeByte(0);
        //   else { writeByte(1); write(equipment[i].serialize()); } }
        for i in 0..5 {
            match hero.equipment[i].as_ref() {
                None => write_byte(&mut out, 0),
                Some(eq) => {
                    write_byte(&mut out, 1);
                    out.extend_from_slice(&item::serialize(&eq.borrow()));
                }
            }
        }
        // for (int i2 = 0; i2 < guardians.length; i2++) { if (guardians[i2] == null) writeByte(0);
        //   else { writeByte(1); writeByte(type); writeShort(level); writeInt(1); writeInt(1);
        //          writeInt(exp); writeByte(skillSlotA); writeByte(skillSlotB); } }
        for i2 in 0..hero.guardians.len() {
            if hero.guardians[i2].is_none() {
                write_byte(&mut out, 0);
            } else {
                // DEFERRED: Guardian — the guardian-object fields reach the unported
                //   `Guardian`; unreachable (guardians are all-null in this slice).
                unreachable!("DEFERRED: Guardian — guardians[] are all null in this slice");
            }
        }
        // Debug.assertTrue(activeGuardian != null); byte activeIndex = -1;
        //   for (byte b3 = 0; b3 < guardians.length; b3 = (byte)(b3+1)) if (activeGuardian == guardians[b3]) { activeIndex = b3; break; }
        //   Debug.assertTrue(activeIndex != -1); writeByte(activeIndex);
        //   — DEFERRED: Guardian (activeGuardian null here; the index byte + its iadd,i2b are
        //     omitted, and load omits the matching read).
    }
    // writeInt(this.playSeconds + ((int) ((System.currentTimeMillis() / 1000) - ((long) this.sessionStartSec))));
    let (play_seconds, session_start_sec) = {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        (hero.play_seconds, hero.session_start_sec)
    };
    let now_secs =
        java_ldiv(g.clock.current_time_millis(), 1000).expect("currentTimeMillis / 1000");
    let elapsed = now_secs.wrapping_sub(session_start_sec as i64) as i32;
    write_int(&mut out, play_seconds.wrapping_add(elapsed));
    // return byteArrayOutputStream.toByteArray();
    out
}

/// `public final void load(byte[] bArr)` (`ao.a:([B)V => [iinc, iinc]`) — restores the
/// hero from its [`save`] form through a `DataInputStream` over a
/// `ByteArrayInputStream`, then `recomputeStats()`. The three `readInt()`s for
/// `maxHp`/`maxMp`/`expToNext` are read and **discarded** (the values are recomputed);
/// reproduced as read-and-drop.
///
/// The `try { … } catch (IOException)` never fires for the in-memory stream. The two
/// `Debug.assertTrue(guardians[0] == null)` / `assertTrue(activeGuardian == null)`
/// preconditions hold on a fresh hero (both null in this slice) and are reproduced.
///
/// **Guardian DEFERRED.** The guardian presence loop reads five `0` flags ([`save`]
/// wrote `0` for every null guardian), so the `!= 0` body — `findOrCreateGuardian` +
/// `Guardian` field restore — is unreachable and marked `// DEFERRED: Guardian`.
/// `setActiveGuardian(guardians[readByte()])` — which would consume the `activeIndex`
/// byte [`save`] omitted — is `// DEFERRED: Guardian` and reads no byte, keeping the
/// cursor aligned with the writer.
pub fn load(g: &mut Game, id: EntityId, data: &[i8]) {
    // ByteArrayInputStream + DataInputStream — a cursor over `data`.
    let mut pos: usize = 0;
    // classId = readByte(); level = readByte();
    let class_id = read_byte(data, &mut pos);
    let level = read_byte(data, &mut pos);
    // hp = readInt(); mp = readInt(); exp = readInt();
    let hp = read_int(data, &mut pos);
    let mp = read_int(data, &mut pos);
    let exp = read_int(data, &mut pos);
    // readInt(); readInt(); readInt();  — maxHp/maxMp/expToNext discarded (recomputed below).
    let _ = read_int(data, &mut pos);
    let _ = read_int(data, &mut pos);
    let _ = read_int(data, &mut pos);
    // maxCombo = readByte();
    let max_combo = read_byte(data, &mut pos);
    // statPoints = readShort(); strength = readShort(); vitality = readShort(); agility = readShort(); spirit = readShort();
    let stat_points = read_short(data, &mut pos);
    let strength = read_short(data, &mut pos);
    let vitality = read_short(data, &mut pos);
    let agility = read_short(data, &mut pos);
    let spirit = read_short(data, &mut pos);
    {
        let hero = g.entity_arena[id].as_hero_mut().expect("Hero node");
        hero.class_id = class_id;
        hero.level = level;
        hero.hp = hp;
        hero.mp = mp;
        hero.exp = exp;
        hero.max_combo = max_combo;
        hero.stat_points = stat_points;
        hero.strength = strength;
        hero.vitality = vitality;
        hero.agility = agility;
        hero.spirit = spirit;
    }
    // for (int i = 0; i < 5; i++) { if (readByte() != 0) {
    //   byte[] itemBytes = new byte[10]; read(itemBytes); equipment[i] = (Equipment) Item.deserialize(itemBytes); } }
    for i in 0..5 {
        if read_byte(data, &mut pos) != 0 {
            // byte[] bArr2 = new byte[10]; dataInputStream.read(bArr2);
            let mut item_bytes = vec![0i8; 10];
            item_bytes.copy_from_slice(&data[pos..pos + 10]);
            pos += 10;
            // this.equipment[i] = (Equipment) Item.deserialize(bArr2);
            let it = item::deserialize(g, &item_bytes);
            g.entity_arena[id]
                .as_hero_mut()
                .expect("Hero node")
                .equipment[i] = Some(Rc::new(RefCell::new(it)));
        }
    }
    // Debug.assertTrue(guardians[0] == null); Debug.assertTrue(activeGuardian == null);
    {
        let hero = g.entity_arena[id].as_hero().expect("Hero node");
        crate::debug::assert_true(hero.guardians[0].is_none());
        crate::debug::assert_true(hero.active_guardian.is_none());
    }
    // for (int i2 = 0; i2 < guardians.length; i2++) { if (readByte() != 0) { Guardian restore } }
    let guardian_len = g.entity_arena[id]
        .as_hero()
        .expect("Hero node")
        .guardians
        .len();
    for _i2 in 0..guardian_len {
        if read_byte(data, &mut pos) != 0 {
            // DEFERRED: Guardian — findOrCreateGuardian + level/exp/equipSkill restore reaches
            //   the unported `Guardian`; unreachable (save wrote 0 for every null guardian).
            unreachable!("DEFERRED: Guardian — save wrote 0 for every null guardian");
        }
    }
    // setActiveGuardian(this.guardians[dataInputStream.readByte()]);
    //   — DEFERRED: Guardian (the activeIndex byte was omitted by save; no read here).
    // this.playSeconds = readInt();
    let play_seconds = read_int(data, &mut pos);
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .play_seconds = play_seconds;
    // recomputeStats();
    recompute_stats(g, id);
}

/// `DataOutputStream.writeByte(int)` — the low 8 bits (JDK semantics). The port's
/// callers pass an `i8` (a `byte` field, or a small `0`/`1` literal), which is exactly
/// that low byte.
fn write_byte(out: &mut Vec<i8>, v: i8) {
    out.push(v);
}

/// `DataOutputStream.writeShort(int)` — big-endian 2-byte write (JDK semantics,
/// inlined). `v` is a `short` sign-extended to `int`; `& 255` selects each byte.
fn write_short(out: &mut Vec<i8>, v: i16) {
    out.push((ishr(v as i32, 8) & 255) as i8);
    out.push((v as i32 & 255) as i8);
}

/// `DataOutputStream.writeInt(int)` — big-endian 4-byte write (JDK semantics, inlined;
/// cf. [`crate::item_bag`]'s `writeInt`).
fn write_int(out: &mut Vec<i8>, v: i32) {
    out.push((ishr(v, 24) & 255) as i8);
    out.push((ishr(v, 16) & 255) as i8);
    out.push((ishr(v, 8) & 255) as i8);
    out.push((v & 255) as i8);
}

/// `DataInputStream.readByte()` — one signed byte (JDK semantics).
fn read_byte(data: &[i8], pos: &mut usize) -> i8 {
    let b = data[*pos];
    *pos += 1;
    b
}

/// `DataInputStream.readShort()` — big-endian signed 16-bit (JDK semantics, inlined).
fn read_short(data: &[i8], pos: &mut usize) -> i16 {
    let hi = (data[*pos] as i32) & 255;
    *pos += 1;
    let lo = (data[*pos] as i32) & 255;
    *pos += 1;
    ((hi << 8) | lo) as i16
}

/// `DataInputStream.readInt()` — big-endian signed 32-bit (JDK semantics, inlined; cf.
/// [`crate::item_bag`]'s `readInt`).
fn read_int(data: &[i8], pos: &mut usize) -> i32 {
    let b0 = (data[*pos] as i32) & 255;
    *pos += 1;
    let b1 = (data[*pos] as i32) & 255;
    *pos += 1;
    let b2 = (data[*pos] as i32) & 255;
    *pos += 1;
    let b3 = (data[*pos] as i32) & 255;
    *pos += 1;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}
