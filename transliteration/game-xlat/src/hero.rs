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

use crate::asset_cache::AssetCacheState;
use crate::battler::BattlerData;
use crate::directions::{DIR_DX, DIR_DY, REVERSE};
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game::Game;
use crate::game_screen;
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

/// `public final void update()` (`ao.d:()V`) — the hero's per-tick combat/movement
/// FSM. DEFERRED.
pub fn update(_g: &mut Game, _id: EntityId) {
    unimplemented!("DEFERRED: Hero.update — not ported in this slice")
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
