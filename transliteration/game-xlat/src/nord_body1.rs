//! Transliterated from `java/src/main/java/defpackage/NordBody1.java`
//! (original `ar.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! First-phase solo form of the **Nord** encounter (`NordBody1 extends Boss`, enemy-data
//! record 35), spawned by `GameMap.spawnNordBoss(true)`. It is a rooted caster: it never
//! walks ([`try_step_forward`] is forced `false`), only turning to face the hero and
//! lobbing a three-way projectile volley. Defeating it does not end the fight — [`die`]
//! immediately spawns the three-part phase-2 Nord (`spawnNordBoss(false)`).
//!
//! ## Entity model — [`crate::boss::BossSubclass::NordBody1`]
//!
//! A `NordBody1` is a [`crate::entity::EntityData::Boss`] node tagged
//! [`crate::boss::BossSubclass::NordBody1`] — a unit variant: it adds no instance fields.
//! It overrides `tryStepForward`/`tryAttack`/`resolveAttack`/`die`/`onDeath`; the
//! [`crate::boss`] dispatchers route each override to the functions here, and the
//! inherited `updateAi`/`chase`/`animate`/`stepDeathAnim`/`paint` are the `Boss`/`Enemy`
//! bodies reached through those dispatchers.
//!
//! ### Re-hosted `update` (the inherited `Boss.update`)
//!
//! `NordBody1` does NOT override `update()` (there is no `ar.d` shape) — it inherits
//! `Boss.update` (`av.d`). But `Boss.update`'s inherited-`Enemy.animate` self-call
//! resolves the virtual `resolveAttack` on the RUNTIME type, and its inherited-final
//! `Battler.stepIfMoving` self-call resolves the virtual `tryStepForward` — both of which
//! `NordBody1` overrides. In the flattened dispatch model those self-calls must resolve on
//! the `NordBody1` tag, and `battler.rs`/`enemy.rs` are read-only in this lane, so their
//! hard-wired `battler::try_step_forward` / `enemy::animate` self-calls cannot be reused.
//! [`update`] therefore RE-HOSTS the inherited `Boss.update` for a `NordBody1` receiver: a
//! verbatim copy of [`crate::boss::update_base`] whose `stepIfMoving` ([`step_if_moving`])
//! routes `tryStepForward` to [`try_step_forward`] here and whose `animate` routes through
//! the [`crate::boss::animate`] dispatcher (which lands `resolveAttack` on this module).
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **`GameMap.spawnNordBoss`.** [`die`] spawns the phase-2 encounter
//!   (`GameState.map.spawnNordBoss(false)`); `spawnNordBoss` is unported (`game_map.rs` is
//!   read-only in this lane), so that call is DEFERRED. The base `super.die()` despawn
//!   ([`crate::boss::die_base`]) IS run.
//! - **`AssetCache.attackEffectScripts` bank.** [`resolve_attack`]'s three-way volley reads
//!   this DEFERRED-loaded bank (a null element NPEs, faithfully; loaded before the boss
//!   acts in real play), mirroring [`crate::enemy`]/[`crate::rocky_boss`].
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ar.<init>:(BBBB)V => []`,
//! `ar.a:()Z (tryStepForward) => []`, `ar.i:()V (tryAttack) => []`,
//! `ar.j:()V (resolveAttack) => ["isub","i2b","iadd","i2b","iadd","i2b"]`,
//! `ar.l:()V (die) => []`, `ar.m:()V (onDeath) => []`.

use crate::battler;
use crate::boss::{self, BossSubclass};
use crate::enemy;
use crate::entity::EntityId;
use crate::game::Game;
use crate::game_map;
use crate::projectile;

/// `(byte[]) AssetCache.attackEffectScripts[statRow]` — the DEFERRED-loaded projectile
/// sprite bank (a null element NPEs, faithfully; loaded before the boss acts in real play).
/// Read once per spawned `Projectile`, mirroring [`crate::enemy`]/[`crate::rocky_boss`].
fn attack_script(g: &Game, stat_row: i8) -> Vec<i8> {
    g.asset_cache
        .attack_effect_scripts
        .as_ref()
        .expect("AssetCache.attackEffectScripts null in NordBody1.resolveAttack")[stat_row as usize]
        .clone()
        .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)")
}

/// `public NordBody1(byte tileX, byte tileY, byte kind, byte statRow)`
/// (`ar.<init>:(BBBB)V => []`). `super(tileX, tileY, kind, statRow, (byte) 1)` is
/// [`boss::new_boss`] (layer 1); the node is then tagged [`BossSubclass::NordBody1`] (a
/// unit variant — no extra instance fields).
pub fn new(g: &mut Game, tile_x: i8, tile_y: i8, kind: i8, stat_row: i8) -> EntityId {
    // super(tileX, tileY, kind, statRow, (byte) 1);
    let id = boss::new_boss(g, tile_x, tile_y, kind, stat_row, 1);
    // (no instance fields — NordBody1 just re-tags the Boss node.)
    g.entity_arena[id]
        .as_boss_mut()
        .expect("NordBody1 node is a Boss")
        .subclass = BossSubclass::NordBody1;
    id
}

/// Re-hosted inherited `Boss.update` (`av.d:()V => ["iadd","i2b"]`) for a `NordBody1`
/// receiver — see the module header. Advance `animFrame`, cache the hero tile-offset, then
/// the inherited `updateAi()` / `stepIfMoving()` / `animate()`, with the two virtual
/// self-calls (`tryStepForward`, `resolveAttack`) resolved on the `NordBody1` tag.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("NordBody1 enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in NordBody1.update");
    // this.heroDistX = tileDistX(hero); this.heroDistY = tileDistY(hero);
    let hero_dist_x = enemy::tile_dist_x(g, id, hero);
    let hero_dist_y = enemy::tile_dist_y(g, id, hero);
    {
        let b = g.entity_arena[id].as_boss_mut().expect("NordBody1");
        b.hero_dist_x = hero_dist_x;
        b.hero_dist_y = hero_dist_y;
    }
    // updateAi();   (inherited Enemy.updateAi re-host — NordBody1 does not override it)
    boss::update_ai(g, id);
    // stepIfMoving();   (inherited final Battler.stepIfMoving — tryStepForward → NordBody1)
    step_if_moving(g, id);
    // animate();   (inherited Enemy.animate re-host — dispatches NordBody1.resolveAttack)
    boss::animate(g, id);
}

/// The INHERITED `Battler.stepIfMoving()` (`o.e:()V`) re-hosted for a `NordBody1`
/// receiver — a verbatim copy of [`crate::battler::step_if_moving`] with its
/// `tryStepForward()` self-call routed to [`try_step_forward`] here (so the override
/// resolves). `battler.rs` is read-only in this lane, so its hard-wired
/// `battler::try_step_forward` self-call cannot be reused for a subclass that overrides it.
fn step_if_moving(g: &mut Game, id: EntityId) {
    // if (this.state == 2 || this.state == 4) tryStepForward();
    let state = g.entity_arena[id]
        .as_battler()
        .expect("NordBody1 battler")
        .state;
    if state == 2 || state == 4 {
        // tryStepForward();   — VIRTUAL: NordBody1.tryStepForward (always false).
        try_step_forward(g, id);
    }
    // if (this.state == 2 || this.state == 4) move(8);
    let state = g.entity_arena[id]
        .as_battler()
        .expect("NordBody1 battler")
        .state;
    if state == 2 || state == 4 {
        // move(8);   (inherited Battler.move)
        battler::move_(g, id, 8);
    }
}

/// `public final boolean tryStepForward()` (`ar.a:()Z => []`, overriding
/// `Battler.tryStepForward`) — rooted in place: this phase-1 form never advances a step, so
/// it never halts on a blocked tile (it never consults `GameMap.canStep`).
pub fn try_step_forward(g: &mut Game, id: EntityId) -> bool {
    // return false;
    let _ = (g, id);
    false
}

/// `public final void tryAttack()` (`ar.i:()V => []`, overriding `Enemy.tryAttack`) —
/// begin the volley when the hero is adjacent; otherwise face toward the hero's column and
/// enter the turning step, keyed off the cached `heroDistX`.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in NordBody1.tryAttack");
    let (hurt_cooldown, attack_cooldown) = {
        let e = g.entity_arena[id].as_enemy().expect("NordBody1 enemy");
        (e.hurt_cooldown, e.attack_cooldown)
    };
    let hero_dist_x = g.entity_arena[id].as_boss().expect("NordBody1").hero_dist_x;
    // if (this.hurtCooldown == 0 && ((Boss) this).heroDistX <= 1) { setFacing(2); beginAttack(); return; }
    if hurt_cooldown == 0 && hero_dist_x as i32 <= 1 {
        // setFacing((byte) 2);
        {
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("NordBody1 battler");
            battler::set_facing(b, 2);
        }
        // beginAttack();
        enemy::begin_attack(g, id);
        return;
    }
    // if (this.attackCooldown == 0) {
    if attack_cooldown == 0 {
        let (hero_tile_x, self_tile_x) = (
            g.entity_arena[hero].tile_x as i32,
            g.entity_arena[id].tile_x as i32,
        );
        if hero_dist_x as i32 <= 1 {
            // setState((byte) 1);   — Enemy.setState (no animFrame reset).
            g.entity_arena[id]
                .as_enemy_mut()
                .expect("NordBody1 enemy")
                .battler
                .state = 1;
            // setFacing((byte) 2);
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("NordBody1 battler");
            battler::set_facing(b, 2);
        } else if hero_tile_x > self_tile_x {
            // setState((byte) 2); setFacing((byte) 4);
            g.entity_arena[id]
                .as_enemy_mut()
                .expect("NordBody1 enemy")
                .battler
                .state = 2;
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("NordBody1 battler");
            battler::set_facing(b, 4);
        } else if hero_tile_x < self_tile_x {
            // setState((byte) 2); setFacing((byte) 3);
            g.entity_arena[id]
                .as_enemy_mut()
                .expect("NordBody1 enemy")
                .battler
                .state = 2;
            let b = g.entity_arena[id]
                .as_battler_mut()
                .expect("NordBody1 battler");
            battler::set_facing(b, 3);
        }
    }
}

/// `public final void resolveAttack()` (`ar.j:()V => ["isub","i2b","iadd","i2b","iadd",
/// "i2b"]`, overriding `Enemy.resolveAttack`) — on hit frame 2, lob a three-way projectile
/// volley (left of, right of, and one row below the caster).
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // if (this.animFrame == 2) {
    let anim_frame = g.entity_arena[id]
        .as_enemy()
        .expect("NordBody1 enemy")
        .battler
        .anim_frame;
    if anim_frame == 2 {
        let (tile_x, tile_y, stat_row, facing) = {
            let n = &g.entity_arena[id];
            let e = n.as_enemy().expect("NordBody1 enemy");
            (
                n.tile_x as i32,
                n.tile_y as i32,
                e.stat_row,
                e.battler.facing,
            )
        };
        // new Projectile((byte)(tileX - 1), tileY, script, this, facing, (byte) 13, (byte) 2)
        let script = attack_script(g, stat_row);
        let ptx = tile_x.wrapping_sub(1) as i8;
        let p = projectile::new_projectile_enemy(
            &mut g.entity_arena,
            ptx,
            tile_y as i8,
            script,
            id,
            facing,
            13,
            2,
        );
        game_map::add_entity(g, p);
        // new Projectile((byte)(tileX + 1), tileY, script, this, facing, (byte) 13, (byte) 2)
        let script = attack_script(g, stat_row);
        let ptx = tile_x.wrapping_add(1) as i8;
        let p = projectile::new_projectile_enemy(
            &mut g.entity_arena,
            ptx,
            tile_y as i8,
            script,
            id,
            facing,
            13,
            2,
        );
        game_map::add_entity(g, p);
        // new Projectile(tileX, (byte)(tileY + 1), script, this, facing, (byte) 13, (byte) 2)
        let script = attack_script(g, stat_row);
        let pty = tile_y.wrapping_add(1) as i8;
        let p = projectile::new_projectile_enemy(
            &mut g.entity_arena,
            tile_x as i8,
            pty,
            script,
            id,
            facing,
            13,
            2,
        );
        game_map::add_entity(g, p);
    }
}

/// `public final void die()` (`ar.l:()V => []`, overriding `Boss.die`) — the base despawn
/// plus the immediate spawn of the phase-2 three-part Nord.
pub fn die(g: &mut Game, id: EntityId) {
    // super.die();   (Boss.die → removeEntity)
    boss::die_base(g, id);
    // GameState.map.spawnNordBoss(false);
    //   DEFERRED: GameMap.spawnNordBoss (the phase-2 spawn path is unported; game_map.rs
    //   is read-only in this lane). In real play this immediately spawns NordBody2 +
    //   NordTentacle + NordHealer.
}

/// `public final void onDeath()` (`ar.m:()V => []`, overriding the abstract `Boss.onDeath`)
/// — no death-animation delay: the corpse is reaped immediately.
pub fn on_death(g: &mut Game, id: EntityId) {
    // this.deathTimer = (byte) 0;
    g.entity_arena[id]
        .as_enemy_mut()
        .expect("NordBody1 enemy")
        .death_timer = 0;
}
