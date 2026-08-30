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
//! **This slice is the FIELD LAYER + the constructor only.** [`HeroData`] +
//! [`new_hero`] (`new Hero(0,0,8,8,classId)` field-init) land here; the combat FSM
//! ([`update`]), rendering ([`paint`]), class setup ([`init_class`]) and stat
//! recomputation ([`recompute_stats`]) — which reach `Guardian`/`Item.create`/
//! `GameLoop.gameScreen`/the map — are DEFERRED to later lanes.
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
use j2me_jvm::{java_ldiv, Clock, VirtualClock};

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

// --- DEFERRED: class setup, stat math, and the per-tick / render bodies --------

/// `public final void initClass(byte classId)` (`ao.a:(B)V`) — populates the
/// starting equipment/stats/guardians for the chosen class. Reaches `Item.create`,
/// `Guardian`, and `recomputeStats`. DEFERRED.
pub fn init_class(_g: &mut Game, _id: EntityId, _class_id: i8) {
    unimplemented!("DEFERRED: Hero.initClass — not ported in this slice")
}

/// `public final void recomputeStats()` (`ao.a:()V`) — recomputes attack/defense/
/// maxHp/maxMp/expToNext from stats + equipment (and `GameLoop.gameScreen`). DEFERRED.
pub fn recompute_stats(_g: &mut Game, _id: EntityId) {
    unimplemented!("DEFERRED: Hero.recomputeStats — not ported in this slice")
}

/// `public final void update()` (`ao.d:()V`) — the hero's per-tick combat/movement
/// FSM. DEFERRED.
pub fn update(_g: &mut Game, _id: EntityId) {
    unimplemented!("DEFERRED: Hero.update — not ported in this slice")
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ao.a:(Graphics;II)V`) — draws the layered character sprite. DEFERRED to the
/// render lane.
pub fn paint(_g: &mut Game, _id: EntityId, _origin_x: i32, _origin_y: i32) {
    unimplemented!("DEFERRED: Hero.paint — not ported in this slice")
}
