//! Transliterated from `java/src/main/java/defpackage/RockyBoss.java`
//! (original `cc.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The solo **Rocky Firebird** boss (`RockyBoss extends Boss`, enemy-data record 32,
//! fire element). It cycles a fixed attack schedule ([`PATTERN_SEQUENCE`] `{1,1,2,3,2,3}`):
//! pattern 1 is a melee lunge (teleport next to the hero), pattern 2 a four-way
//! projectile volley, pattern 3 a short double-damage slam. Between patterns it hops to a
//! random walkable tile near the hero, and defeating it fires story trigger 1.
//!
//! ## Entity model — [`crate::boss::BossSubclass::RockyBoss`]
//!
//! A `RockyBoss` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::RockyBoss`], which carries this class's three extra
//! instance fields ([`RockyBossData`]). It overrides `update`/`updateAi`/`tryAttack`/
//! `animate`/`resolveAttack`/`stepDeathAnim`/`die`/`onDeath`; the [`crate::boss`]
//! dispatchers route each of those to the functions here (see that module header). Its
//! `updateAi` (`cc.n`) is a *distinct* pattern-schedule machine — NOT the inherited
//! `Enemy.updateAi` — so it is re-hosted in full here, and `update` (`cc.d`) omits the
//! `stepIfMoving()` call `Boss.update` makes.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`AssetCache.bossExtraFrames`.** [`select_attack_pattern`] mirrors boss-type-1
//!   extra attack frame scripts from `bossExtraFrames` into `bossFrames` and reads the
//!   selected animation's frame count into `attackFrameCount`. `bossExtraFrames` is a
//!   DEFERRED-loaded sprite bank not present in [`crate::asset_cache`]
//!   (`asset_cache.rs` is read-only here); the frame-copy and `attackFrameCount`
//!   derivation are DEFERRED, so `attackFrameCount` stays at its JVM default `0`. Only
//!   the `attackPattern` field write is applied.
//! - **`GameMap.isWalkable` / `GameMap.canOccupy`.** [`animate`]'s hop (state 2, frame 5)
//!   and [`resolve_attack`]'s pattern-1 reposition (frame 7) gate `setPixelPos` on these
//!   GameMap predicates. Both are unported (`game_map.rs` is read-only here); those
//!   branches (and their `ByteUtil.randRange` rolls) are DEFERRED with named comments.
//! - **`Hero.takeHit`.** The landed melee/slam strikes are DEFERRED exactly as in
//!   [`crate::enemy::resolve_attack`] (the hero combat FSM is partial).
//! - **`EventScript.fire`.** [`die`] fires story trigger 1; `EventScript` is unported.
//! - **`AssetCache.attackEffectScripts` bank.** The pattern-2 projectile volley and the
//!   death-effect spawn read this DEFERRED-loaded bank (a null element NPEs, faithfully;
//!   in real play the bank is loaded before the boss acts), mirroring [`crate::enemy`].
//!
//! Opcode shapes (R8): `cc.<init>:(BBBB)V => []`, `cc.d:()V (update) => ["iadd","i2b"]`,
//! `cc.n:()V (updateAi) => ["iadd","i2b","isub","i2b","isub","i2b"]`,
//! `cc.i:()V (tryAttack) => ["imul","imul"]`,
//! `cc.o:()V (animate) => [hop/pattern arithmetic, DEFERRED hop drops the ishl/randRange
//! ops]`, `cc.j:()V (resolveAttack) => [pattern geometry]`,
//! `cc.k:()V (stepDeathAnim) => ["iadd","i2b","iadd","i2b"]`,
//! `cc.d:(B)V (selectAttackPattern) => ["isub","imul","imul","iadd","iadd","iadd","iinc",
//! "imul","iadd"] (DEFERRED frame-bank body; only `attackPattern =` applied)`,
//! `cc.l:()V (die) => []`, `cc.m:()V (onDeath) => []`.

use crate::boss::{self, BossSubclass};
use crate::byte_util;
use crate::directions::{DIR_DX, DIR_DY};
use crate::effect;
use crate::enemy;
use crate::entity::{EntityId, EntityNode};
use crate::game::Game;
use crate::game_map;
use crate::projectile;

/// `private static final byte[] patternSequence = {1,1,2,3,2,3};` (`cc.h`) — the ordered
/// schedule of attack-pattern ids the boss rotates through. A `static final` constant
/// reproduced as a `const`.
pub const PATTERN_SEQUENCE: [i8; 6] = [1, 1, 2, 3, 2, 3];

/// `private static final byte[] patternCooldowns = {2,2,12,24,12,24};` (`cc.i`) — the
/// cooldown (ticks) applied on entering each corresponding schedule step. A
/// `static final` constant reproduced as a `const`.
pub const PATTERN_COOLDOWNS: [i8; 6] = [2, 2, 12, 24, 12, 24];

/// The `RockyBoss` (`cc`) instance fields beyond the `Boss`/`Enemy` base — carried in
/// [`crate::boss::BossSubclass::RockyBoss`].
#[derive(Debug)]
pub struct RockyBossData {
    /// `private byte patternIndex;` (`cc.v`) — cursor into [`PATTERN_SEQUENCE`].
    pub pattern_index: i8,
    /// `private byte attackPattern;` (`cc.w`) — the currently selected pattern
    /// (1 melee, 2 volley, 3 slam).
    pub attack_pattern: i8,
    /// `private byte attackFrameCount;` (`cc.x`) — frame count of the selected attack
    /// animation. DEFERRED-derived from the boss frame bank (see the module header);
    /// stays `0` in this slice.
    pub attack_frame_count: i8,
}

/// The [`RockyBossData`] behind a `RockyBoss` node.
fn data(node: &EntityNode) -> &RockyBossData {
    match &node.as_boss().expect("RockyBoss node is a Boss").subclass {
        BossSubclass::RockyBoss(d) => d,
        _ => unreachable!("rocky_boss dispatched on a non-RockyBoss node"),
    }
}

/// Mutable [`data`].
fn data_mut(node: &mut EntityNode) -> &mut RockyBossData {
    match &mut node
        .as_boss_mut()
        .expect("RockyBoss node is a Boss")
        .subclass
    {
        BossSubclass::RockyBoss(d) => d,
        _ => unreachable!("rocky_boss dispatched on a non-RockyBoss node"),
    }
}

/// `public RockyBoss(byte tileX, byte tileY, byte kind, byte statRow)`
/// (`cc.<init>:(BBBB)V => []`). `super(tileX, tileY, kind, statRow, (byte) 1)` is
/// [`boss::new_boss`] (layer 1); the node is tagged [`BossSubclass::RockyBoss`], then the
/// first pattern is selected.
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 1);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 1);
    // this.patternIndex = (byte) 0;  (attackPattern/attackFrameCount at JVM byte default 0)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("RockyBoss node is a Boss")
        .subclass = BossSubclass::RockyBoss(RockyBossData {
        pattern_index: 0,
        attack_pattern: 0,
        attack_frame_count: 0,
    });
    // selectAttackPattern(patternSequence[this.patternIndex]);
    select_attack_pattern(g, id, PATTERN_SEQUENCE[0]);
    id
}

/// `public final void update()` (`cc.d:()V => ["iadd","i2b"]`, overriding `Boss.update`)
/// — advance `animFrame`, cache the hero tile-offset, then `updateAi()`/`animate()`.
/// Unlike `Boss.update` there is NO `stepIfMoving()`.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in RockyBoss.update");
    // ((Boss) this).heroDistX = tileDistX(hero); this.heroDistY = tileDistY(hero);
    let hero_dist_x = enemy::tile_dist_x(g, id, hero);
    let hero_dist_y = enemy::tile_dist_y(g, id, hero);
    {
        let b = g.entity_arena[id].as_boss_mut().expect("RockyBoss");
        b.hero_dist_x = hero_dist_x;
        b.hero_dist_y = hero_dist_y;
    }
    // updateAi();
    update_ai(g, id);
    // animate();
    animate(g, id);
}

/// `public final void updateAi()` (`cc.n:()V => ["iadd","i2b","isub","i2b","isub","i2b"]`,
/// overriding `Enemy.updateAi`) — the RockyBoss pattern-schedule state machine. Distinct
/// from the inherited `Enemy.updateAi` (re-hosted in [`crate::boss::update_ai_base`]).
pub fn update_ai(g: &mut Game, id: EntityId) {
    // switch (((Battler) this).state)
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("RockyBoss enemy")
        .battler
        .state;
    match state {
        // case 1: tryAttack();
        1 => try_attack(g, id),
        // case 2: if (animFrame >= stats.attackFrames) { enterIdle(false); tryAttack(); }
        2 => {
            let (anim_frame, attack_frames) = {
                let e = g.entity_arena[id].as_enemy().expect("RockyBoss enemy");
                (e.battler.anim_frame, e.stats.attack_frames)
            };
            if anim_frame as i32 >= attack_frames as i32 {
                enemy::enter_idle(g, id, false);
                try_attack(g, id);
            }
        }
        // case 3: if (animFrame >= attackFrameCount) { enterIdle(false); patternIndex++;
        //   wrap; selectAttackPattern(...); hurtCooldown = attackCooldown = patternCooldowns[patternIndex]; tryAttack(); }
        3 => {
            let anim_frame = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .anim_frame;
            let attack_frame_count = data(&g.entity_arena[id]).attack_frame_count;
            if anim_frame as i32 >= attack_frame_count as i32 {
                enemy::enter_idle(g, id, false);
                // this.patternIndex = (byte) (this.patternIndex + 1);
                // if (this.patternIndex >= patternSequence.length) this.patternIndex = (byte) 0;
                {
                    let d = data_mut(&mut g.entity_arena[id]);
                    d.pattern_index = (d.pattern_index as i32).wrapping_add(1) as i8;
                    if d.pattern_index as i32 >= PATTERN_SEQUENCE.len() as i32 {
                        d.pattern_index = 0;
                    }
                }
                let pattern_index = data(&g.entity_arena[id]).pattern_index;
                // selectAttackPattern(patternSequence[this.patternIndex]);
                select_attack_pattern(g, id, PATTERN_SEQUENCE[pattern_index as usize]);
                // this.hurtCooldown = patternCooldowns[patternIndex]; this.attackCooldown = patternCooldowns[patternIndex];
                let cd = PATTERN_COOLDOWNS[pattern_index as usize];
                {
                    let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
                    e.hurt_cooldown = cd;
                    e.attack_cooldown = cd;
                }
                // tryAttack();
                try_attack(g, id);
            }
        }
        // case 4: if (knockbackTimer < 1) setState(1); knockbackTimer--;
        4 => {
            let kt = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .knockback_timer;
            if kt < 1 {
                // setState((byte) 1);   — Enemy.setState (no animFrame reset).
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("RockyBoss enemy")
                    .battler
                    .state = 1;
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
            e.battler.knockback_timer = (e.battler.knockback_timer as i32).wrapping_sub(1) as i8;
        }
        // case 5: if (deathTimer < 1) die(); deathTimer--;
        5 => {
            let dt = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .death_timer;
            if dt < 1 {
                // die();   — RockyBoss.die (super.die + EventScript.fire).
                die(g, id);
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
            e.death_timer = (e.death_timer as i32).wrapping_sub(1) as i8;
        }
        _ => {}
    }
}

/// `public final void tryAttack()` (`cc.i:()V => ["imul","imul"]`, overriding
/// `Enemy.tryAttack`) — pattern-gated: begin the attack when the hero is in range, else
/// step toward it (`setState(2)`), keyed off the cached `heroDistX`/`heroDistY`.
pub fn try_attack(g: &mut Game, id: EntityId) {
    let hurt_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("RockyBoss enemy")
        .hurt_cooldown;
    let (hero_dist_x, hero_dist_y) = {
        let b = g.entity_arena[id].as_boss().expect("RockyBoss");
        (b.hero_dist_x as i32, b.hero_dist_y as i32)
    };
    let attack_pattern = data(&g.entity_arena[id]).attack_pattern;
    // if (this.hurtCooldown == 0) switch (this.attackPattern) { ... }
    if hurt_cooldown == 0 {
        match attack_pattern {
            // case 1: if (heroDistX < 4 && heroDistY < 4) beginAttack();
            1 => {
                if hero_dist_x < 4 && hero_dist_y < 4 {
                    enemy::begin_attack(g, id);
                }
            }
            // case 2: if (heroDistX * heroDistY == 0 && heroDistX < 4 && heroDistY < 4) { beginAttack(); return; }
            2 => {
                if hero_dist_x.wrapping_mul(hero_dist_y) == 0 && hero_dist_x < 4 && hero_dist_y < 4
                {
                    enemy::begin_attack(g, id);
                    return;
                }
            }
            // case 3: beginAttack(); return;
            3 => {
                enemy::begin_attack(g, id);
                return;
            }
            _ => {}
        }
    }
    // if (this.attackCooldown == 0) switch (this.attackPattern) { ... setState(2); animFrame = 0; }
    let attack_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("RockyBoss enemy")
        .attack_cooldown;
    if attack_cooldown == 0 {
        let step = match attack_pattern {
            // case 1: heroDistX >= 4 || heroDistY >= 4
            1 => hero_dist_x >= 4 || hero_dist_y >= 4,
            // case 2: heroDistX * heroDistY != 0 || heroDistX >= 4 || heroDistY >= 4
            2 => hero_dist_x.wrapping_mul(hero_dist_y) != 0 || hero_dist_x >= 4 || hero_dist_y >= 4,
            // case 3: heroDistX >= 3 || heroDistY >= 3
            3 => hero_dist_x >= 3 || hero_dist_y >= 3,
            _ => false,
        };
        if step {
            let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
            // setState((byte) 2);   — Enemy.setState (no animFrame reset); then animFrame = 0.
            e.battler.state = 2;
            e.battler.anim_frame = 0;
        }
    }
}

/// `public final void animate()` (`cc.o:()V`, overriding `Enemy.animate`) — the hop /
/// resolve / walk-cooldown / death animation switch. Unlike `Enemy.animate` there is NO
/// summon-timer block, and state 2 performs the near-hero hop (DEFERRED — see below).
pub fn animate(g: &mut Game, id: EntityId) {
    // switch (((Battler) this).state)
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("RockyBoss enemy")
        .battler
        .state;
    match state {
        // case 2: if (animFrame == 5) { hop onto a random walkable adjacent tile }
        2 => {
            let anim_frame = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .anim_frame;
            if anim_frame == 5 {
                // DEFERRED: the random adjacent-tile hop —
                //   while (triesLeft > 0 && !GameState.map.isWalkable(hopTileX, hopTileY)) {
                //     hopTileX/hopTileY = ByteUtil.randRange(...); triesLeft--; }
                //   if (triesLeft > 0) setPixelPos((short)(hopTileX << 4), (short)(hopTileY << 4));
                //   `GameMap.isWalkable` is unported (game_map.rs read-only in this lane); the
                //   loop condition cannot be evaluated, so the hop (and its ByteUtil.randRange
                //   rolls) is DEFERRED. Reached only mid-encounter in the DEFERRED boss spawn.
            }
        }
        // case 3: resolveAttack();
        3 => resolve_attack(g, id),
        // case 5: stepDeathAnim();
        5 => step_death_anim(g, id),
        // case 4: default: walk-anim wrap + cooldown decrements.
        _ => {
            // if (animFrame >= stats.walkFrames) animFrame = 0;
            let (anim_frame, walk_frames) = {
                let e = g.entity_arena[id].as_enemy().expect("RockyBoss enemy");
                (e.battler.anim_frame, e.stats.walk_frames)
            };
            if anim_frame as i32 >= walk_frames as i32 {
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("RockyBoss enemy")
                    .battler
                    .anim_frame = 0;
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("RockyBoss enemy");
            // if (hurtCooldown > 0) hurtCooldown--;
            if e.hurt_cooldown > 0 {
                e.hurt_cooldown = (e.hurt_cooldown as i32).wrapping_sub(1) as i8;
            }
            // if (attackCooldown > 0) attackCooldown--;
            if e.attack_cooldown > 0 {
                e.attack_cooldown = (e.attack_cooldown as i32).wrapping_sub(1) as i8;
            }
        }
    }
}

/// `public final void resolveAttack()` (`cc.j:()V`, overriding `Enemy.resolveAttack`) —
/// the per-pattern landed strike: pattern-1 melee reposition + hit, pattern-2 projectile
/// volley, pattern-3 slam.
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let _hero = g
        .game_state
        .hero
        .expect("GameState.hero null in RockyBoss.resolveAttack");
    let attack_pattern = data(&g.entity_arena[id]).attack_pattern;
    match attack_pattern {
        // case 1: frame-7 reposition (canOccupy) + frame-11 melee.
        1 => {
            let anim_frame = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .anim_frame;
            if anim_frame == 7 {
                // DEFERRED: reposition onto the first occupiable tile next to the hero —
                //   for (byte dir = 1; dir <= 4; dir++)
                //     if (map.canOccupy(this, hero.tileX+dirDx[dir], hero.tileY+dirDy[dir])) {
                //       setPixelPos((short)((hero.tileX+dirDx[dir]) << 4), (short)((hero.tileY+dirDy[dir]) << 4)); break; }
                //   `GameMap.canOccupy` is unported (game_map.rs read-only); DEFERRED.
            }
            // if (animFrame == 11 && heroDistX + heroDistY <= 1) hero.takeHit((Enemy) this, facing);
            let (hero_dist_x, hero_dist_y) = {
                let b = g.entity_arena[id].as_boss().expect("RockyBoss");
                (b.hero_dist_x as i32, b.hero_dist_y as i32)
            };
            if anim_frame == 11 && hero_dist_x.wrapping_add(hero_dist_y) <= 1 {
                // hero.takeHit((Enemy) this, ((Battler) this).facing);
                //   DEFERRED: Hero.takeHit (hero combat FSM partial), as in enemy.rs.
            }
        }
        // case 2: frame-7 four-way projectile volley (PORTED).
        2 => {
            let anim_frame = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .anim_frame;
            if anim_frame == 7 {
                // for (byte dir = 1; dir <= 4; dir++) map.addEntity(new Projectile(...));
                let mut dir: i8 = 1;
                while dir <= 4 {
                    let (tile_x, tile_y, stat_row) = {
                        let n = &g.entity_arena[id];
                        let e = n.as_enemy().expect("RockyBoss enemy");
                        (n.tile_x as i32, n.tile_y as i32, e.stat_row)
                    };
                    // (byte[]) AssetCache.attackEffectScripts[statRow]  — DEFERRED-loaded bank.
                    let script =
                        g.asset_cache.attack_effect_scripts.as_ref().expect(
                            "AssetCache.attackEffectScripts null in RockyBoss.resolveAttack",
                        )[stat_row as usize]
                            .clone()
                            .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)");
                    // new Projectile((byte)(tileX + dirDx[dir]), (byte)(tileY + dirDy[dir]), script, this, dir, (byte) 3, (byte) 2)
                    let ptx = tile_x.wrapping_add(DIR_DX[dir as usize] as i32) as i8;
                    let pty = tile_y.wrapping_add(DIR_DY[dir as usize] as i32) as i8;
                    let new_id = projectile::new_projectile_enemy(
                        &mut g.entity_arena,
                        ptx,
                        pty,
                        script,
                        id,
                        dir,
                        3,
                        2,
                    );
                    game_map::add_entity(g, new_id);
                    dir = (dir as i32).wrapping_add(1) as i8;
                }
            }
        }
        // case 3: frame-4 double-damage slam.
        3 => {
            let anim_frame = g.entity_arena[id]
                .as_enemy()
                .expect("RockyBoss enemy")
                .battler
                .anim_frame;
            let (hero_dist_x, hero_dist_y) = {
                let b = g.entity_arena[id].as_boss().expect("RockyBoss");
                (b.hero_dist_x as i32, b.hero_dist_y as i32)
            };
            if anim_frame == 4 && hero_dist_x <= 2 && hero_dist_y <= 2 {
                // hero.takeHit((Enemy) this, (short)(((Enemy) this).stats.attack * 2), facing);
                //   DEFERRED: Hero.takeHit (the `stats.attack * 2` payload is applied there).
            }
        }
        _ => {}
    }
}

/// `public final void stepDeathAnim()` (`cc.k:()V => ["iadd","i2b","iadd","i2b"]`,
/// overriding `Enemy.stepDeathAnim`) — while the death timer is high, spatter fire
/// effects around the corpse.
pub fn step_death_anim(g: &mut Game, id: EntityId) {
    // if (this.deathTimer > 8) { GameState.map.addEntity(new Effect(...)); }
    let death_timer = g.entity_arena[id]
        .as_enemy()
        .expect("RockyBoss enemy")
        .death_timer;
    if death_timer > 8 {
        let stat_row = g.entity_arena[id]
            .as_enemy()
            .expect("RockyBoss enemy")
            .stat_row;
        let (tile_x, tile_y) = {
            let n = &g.entity_arena[id];
            (n.tile_x as i32, n.tile_y as i32)
        };
        // new Effect((byte)(tileX + randRange(-1,1)), (byte)(tileY + randRange(0,3)), script)
        let r1 = byte_util::rand_range(&mut g.byte_util, -1, 1);
        let r2 = byte_util::rand_range(&mut g.byte_util, 0, 3);
        let etx = tile_x.wrapping_add(r1) as i8;
        let ety = tile_y.wrapping_add(r2) as i8;
        // (byte[]) AssetCache.attackEffectScripts[statRow]  — DEFERRED-loaded bank.
        let script = g
            .asset_cache
            .attack_effect_scripts
            .as_ref()
            .expect("AssetCache.attackEffectScripts null in RockyBoss.stepDeathAnim")
            [stat_row as usize]
            .clone()
            .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)");
        let eff = effect::new_effect_from_script(&mut g.entity_arena, etx, ety, script);
        game_map::add_entity(g, eff);
    }
}

/// `private final void selectAttackPattern(byte pattern)` (`cc.d:(B)V`) — switches the
/// active attack pattern. The `attackPattern` field write is applied; the frame-bank copy
/// and `attackFrameCount` derivation are DEFERRED (`AssetCache.bossExtraFrames` unported;
/// see the module header).
fn select_attack_pattern(g: &mut Game, id: EntityId, pattern: i8) {
    // this.attackPattern = pattern;
    data_mut(&mut g.entity_arena[id]).attack_pattern = pattern;
    // DEFERRED (AssetCache.bossExtraFrames unported):
    //   int sourceOffset = (pattern - 1) * 4;
    //   for (int i = 0; i < 4; i++) AssetCache.bossFrames[(statRow*16)+12+i] = AssetCache.bossExtraFrames[sourceOffset+i];
    //   this.attackFrameCount = ((byte[]) AssetCache.bossFrames[(statRow*16)+12])[0];
}

/// `public final void die()` (`cc.l:()V => []`, overriding `Boss.die`) — the base despawn
/// plus the encounter's story trigger.
pub fn die(g: &mut Game, id: EntityId) {
    // super.die();   (Boss.die → removeEntity)
    boss::die_base(g, id);
    // EventScript.fire((byte) 1);
    //   DEFERRED: EventScript.fire (story-trigger machinery unported).
}

/// `public final void onDeath()` (`cc.m:()V => []`, overriding the abstract
/// `Boss.onDeath`) — arms the (long) death-animation countdown.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 24;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("RockyBoss enemy")
        .death_timer = 24;
}
