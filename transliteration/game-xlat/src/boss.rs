//! Transliterated from `java/src/main/java/defpackage/Boss.java`
//! (original `av.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Abstract base for the game's multi-tile bosses (`Boss extends Enemy`). It refines
//! [`crate::enemy`]: the AI tick caches the hero offset each frame
//! ([`BossData::hero_dist_x`]/[`BossData::hero_dist_y`]); drawing uses the wider
//! 16-cell boss sprite bank (`AssetCache.bossFrames`); and the damage-intake paths
//! ([`take_guardian_hit`]/[`take_hero_hit`]) use a boss-tuned defense/dodge formula
//! and invoke the abstract [`on_death`] cleanup instead of dropping loot. Death
//! ([`die`]) simply unregisters the boss from the map.
//!
//! ## Entity model — [`crate::entity::EntityData::Boss`]
//!
//! Boss is modelled as its OWN [`crate::entity::EntityData`] variant / [`EntityKind`]
//! following the `Enemy`/`Npc` (and `Effect`→`Projectile`) precedent, **not** as a
//! flag inside [`crate::entity::EntityData::Enemy`]. It adds two fields
//! (`heroDistX`/`heroDistY`) Enemy lacks and overrides five virtual methods
//! (`update`/`paint`/`takeGuardianHit`/`takeHeroHit`/`die`) with genuinely different
//! behaviour, so a distinct kind is the faithful mirror of the single-inheritance
//! chain `Boss extends Enemy extends Battler extends Entity`. [`BossData`] embeds an
//! [`EnemyData`] as its "super"; a Java `((Enemy) this).*` / `(Enemy) this` upcast
//! becomes [`crate::entity::EntityNode::as_enemy`] (which answers for a `Boss` node
//! too, exactly as [`crate::entity::EntityNode::as_effect`] answers for a
//! `Projectile`), and the `Battler` upcast is [`crate::entity::EntityNode::as_battler`].
//! The concrete `heroDistX`/`heroDistY` access is [`crate::entity::EntityNode::as_boss`].
//!
//! Because a `Boss` is an `Enemy`, `GameMap.updateCombatants`'s `instanceof Enemy`
//! arm and `GameMap.drawEntities`'s virtual `paint` dispatch match a `Boss`; the
//! wiring in [`crate::game_map`] routes a `Boss` node to [`update`]/[`paint`] here
//! (the virtual override) instead of the `Enemy` versions.
//!
//! ## Inherited methods + the virtual-`die()` re-host ([`update_ai`])
//!
//! `Boss.update` calls the INHERITED `updateAi()` / `stepIfMoving()` / `animate()`.
//! `stepIfMoving` ([`battler::step_if_moving`]) and `animate` ([`enemy::animate`]) are
//! reused directly — they reach no method `Boss` overrides. `updateAi`, however,
//! makes a virtual `die()` call, and `Boss` overrides `die()`. In the codebase's
//! flattened dispatch model the virtual target must resolve on the receiver's runtime
//! type; the effect/projectile lane does this with a match-on-kind dispatcher in the
//! *base* (`effect::on_frame`), but `enemy.rs` is read-only here and its `update_ai`
//! hard-calls `enemy::die`. So [`update_ai`] RE-HOSTS the inherited `Enemy.updateAi`
//! for a `Boss` receiver: every self-call `Boss` does not override routes to the
//! `enemy::` function (`chase`/`try_attack`/`enter_idle`, and `Enemy.setState`), and
//! the one it does — `die()` — routes to [`die`] here. This is the honest
//! transliteration of the inherited method under the flattened model.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **Concrete boss subclasses.** `Boss` is abstract; the concrete encounters
//!   (`RockyBoss`, `Geb{Core,Head,HandLeft,HandRight}`, `Nord{Body1,Body2,Healer,
//!   Tentacle}`) are separate later batches and are DEFERRED — no `EntityData` leaf
//!   for them exists yet. [`on_death`] is `Boss`'s abstract `onDeath()`: a virtual
//!   call to the concrete subclass's despawn/cleanup, DEFERRED here.
//! - **Spawn call site (EventScript / the map `.evt` parse).** A `Boss` is
//!   instantiated by the DEFERRED cutscene/`.evt` machinery (`EventScript`); the
//!   spawn call site is DEFERRED. [`new_boss`] (the constructor) is ported and driven
//!   directly by the oracle.
//! - **`take_hero_hit` body.** Like [`enemy::take_hero_hit`], the pipeline past the
//!   death guard needs the unported `Weapon` (`hero.getEquip(0)`), `Guardian`
//!   (`hero.getActiveGuardian().element()`), `Hero.addMp`/`addHp` and
//!   `GameLoop.gameScreen.setTarget`; its callers (Hero's attack FSM, `Projectile`'s
//!   hero-owned block) are themselves DEFERRED, so it is never invoked in this slice.
//!   The body is DEFERRED after the death guard, recorded verbatim in the doc comment.
//! - **Boss sprite bank.** `AssetCache.bossFrames` is DEFERRED-loaded (see
//!   [`crate::asset_cache`]): every element is null, so [`paint`]'s frame-group draw
//!   no-ops (and the null-fallback branch is exercised) rather than blitting.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `av.<init>:(BBBBB)V => ["ishl","i2s","ishl","i2s"]` (`tileX<<4`/`tileY<<4` +
//! `(short)` narrows), `av.d:()V (update) => ["iadd","i2b"]` (the `animFrame + 1`),
//! `av.a:(IB)V (takeGuardianHit) => ["imul","idiv","isub","i2s","i2s"]`,
//! `av.a:(IZBZBBLao;)V (takeHeroHit) => 16 int ops` (DEFERRED body; multiset recorded),
//! `av.a:(…Graphics;II)V (paint) => 29 int ops`, `av.l:()V (die) => []`.

use crate::battler::{self, BattlerData};
use crate::directions::ELEMENT_DAMAGE_MULTIPLIER;
use crate::enemy::{self, EnemyData};
use crate::entity::{self, EntityData, EntityId, EntityNode};
use crate::floater;
use crate::game::Game;
use crate::game_map;
use crate::game_screen;
use j2me_jvm::{ishl, java_div};

/// The `Boss` (`av`) instance data — the subclass fields beyond the embedded
/// [`EnemyData`] "super". Boxed in [`EntityData::Boss`].
#[derive(Debug)]
pub struct BossData {
    /// The `Enemy` "super" (`super((short)(tileX<<4), (short)(tileY<<4), halfWidth,
    /// halfHeight)` — `Enemy`'s only constructor, so `halfWidth`/`halfHeight` are
    /// passed as `Enemy`'s `kind`/`statRow`).
    pub enemy: EnemyData,
    /// `public byte heroDistX;` (`av.f`) — cached absolute tile-column distance to the
    /// hero (recomputed each tick in [`update`]).
    pub hero_dist_x: i8,
    /// `public byte heroDistY;` (`av.g`) — cached absolute tile-row distance to the
    /// hero (recomputed each tick in [`update`]).
    pub hero_dist_y: i8,
}

/// `public Boss(byte tileX, byte tileY, byte halfWidth, byte halfHeight, byte layer)`
/// (`av.<init>:(BBBBB)V => ["ishl","i2s","ishl","i2s"]`). Allocates the boss node in
/// the arena and returns its [`EntityId`].
///
/// `super((short)(tileX << 4), (short)(tileY << 4), halfWidth, halfHeight)` invokes
/// `Enemy`'s only constructor with `pixelX = tileX<<4`, `pixelY = tileY<<4`,
/// `kind = halfWidth`, `statRow = halfHeight`; that super-constructor is reproduced
/// inline (mirroring [`enemy::new_enemy`]) because it must build an
/// [`EntityData::Boss`] node, not an [`EntityData::Enemy`] one. After it, `Boss`
/// overrides the layer and re-registers occupancy.
pub fn new_boss(
    g: &mut Game,
    tile_x: i8,
    tile_y: i8,
    half_width: i8,
    half_height: i8,
    layer: i8,
) -> EntityId {
    // super((short) (tileX << 4), (short) (tileY << 4), halfWidth, halfHeight);
    //   → Enemy(pixelX, pixelY, kind, statRow)   (Enemy's only constructor)
    let pixel_x = ishl(tile_x as i32, 4) as i16;
    let pixel_y = ishl(tile_y as i32, 4) as i16;
    let kind = half_width; // passed as Enemy `kind`
    let stat_row = half_height; // passed as Enemy `statRow`

    // --- reproduced Enemy(pixelX, pixelY, kind, statRow) super-constructor ---
    // this.stats = EnemyType.types[statRow];  (cloned; see enemy_type module deviation.)
    let stats = g
        .enemy_type
        .types
        .as_ref()
        .expect("EnemyType.types null in new Boss")[stat_row as usize]
        .clone()
        .expect("EnemyType.types[statRow] null in new Boss");
    // ((Entity) this).layer = this.stats.size == 2 ? (byte) 2 : (byte) 1;  (Enemy ctor)
    let enemy_layer: i8 = if stats.size == 2 { 2 } else { 1 };
    let enemy = EnemyData {
        // super(...) → Battler.init: state 1, facing 2, moveDir 2, animFrame -1, knockbackTimer 0.
        battler: BattlerData::new(),
        // this.kind = kind; this.statRow = statRow;
        kind,
        stat_row,
        // homeTileX/Y assigned below (after init_base derives the tile).
        home_tile_x: 0,
        home_tile_y: 0,
        // this.hp = this.stats.maxHp;
        hp: stats.max_hp,
        // this.attackCharge = 0;  (ctor)
        attack_charge: 0,
        // this.attackCooldown = this.stats.attackDelay;
        attack_cooldown: stats.attack_delay,
        // this.hurtCooldown = this.stats.hurtDelay;
        hurt_cooldown: stats.hurt_delay,
        // this.deathTimer = 0; this.recoilTimer = 0; this.recoilDir = 0;  (ctor)
        death_timer: 0,
        recoil_timer: 0,
        recoil_dir: 0,
        // this.aggroed = false;
        aggroed: false,
        // if (this.stats.ambush) this.hidden = true;  (else JVM default false)
        hidden: stats.ambush,
        // this.statusFlags = new boolean[5];
        status_flags: [false; 5],
        // (target null after construction)
        target: None,
        // this.summonTimer = (byte) -10;
        summon_timer: -10,
        // this.onScreen = true;
        on_screen: true,
        // (moved last: every `stats.*` read above is a Copy of a primitive field.)
        stats,
    };
    // super(pixelX, pixelY, (byte) 8, (byte) 8);  (Battler → Entity base)
    let mut node = EntityNode {
        data: EntityData::Boss(Box::new(BossData {
            enemy,
            // heroDistX / heroDistY at their JVM byte default.
            hero_dist_x: 0,
            hero_dist_y: 0,
        })),
        layer: enemy_layer,
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 8);
    // this.homeTileX = ((Entity) this).tileX; this.homeTileY = ((Entity) this).tileY;
    let (tx, ty) = (node.tile_x, node.tile_y);
    if let EntityData::Boss(b) = &mut node.data {
        b.enemy.home_tile_x = tx as i32;
        b.enemy.home_tile_y = ty as i32;
    }
    let id = g.entity_arena.alloc(node);
    // setPixelPos(pixelX, pixelY);   (Enemy override → occupancy register)
    enemy::set_pixel_pos(g, id, pixel_x, pixel_y);
    // --- end reproduced super-constructor ---

    // ((Entity) this).layer = layer;
    g.entity_arena[id].layer = layer;
    // setOccupancy();
    battler::set_occupancy(g, id);
    id
}

/// `public void update()` (`av.d:()V => ["iadd","i2b"]`, overriding `Enemy.update`) —
/// the boss's per-tick AI: advance `animFrame`, cache the hero tile-offset, then the
/// INHERITED `updateAi()` / `stepIfMoving()` / `animate()`. Unlike `Enemy.update` it
/// does NOT tick statuses, throttle off-screen, or decay the recoil timer.
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Boss enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in Boss.update");
    // this.heroDistX = tileDistX(hero); this.heroDistY = tileDistY(hero);
    let hero_dist_x = enemy::tile_dist_x(g, id, hero);
    let hero_dist_y = enemy::tile_dist_y(g, id, hero);
    {
        let b = g.entity_arena[id].as_boss_mut().expect("Boss");
        b.hero_dist_x = hero_dist_x;
        b.hero_dist_y = hero_dist_y;
    }
    // updateAi();   (inherited Enemy.updateAi, re-hosted with virtual die() → Boss.die)
    update_ai(g, id);
    // stepIfMoving();   (inherited final Battler.stepIfMoving)
    battler::step_if_moving(g, id);
    // animate();   (inherited Enemy.animate — reaches no method Boss overrides)
    enemy::animate(g, id);
}

/// The INHERITED `Enemy.updateAi()` (`al.n:()V`) re-hosted for a `Boss` receiver: the
/// AI state machine dispatching idle/chase/attack/knockback/death. Structurally a
/// verbatim copy of [`enemy::update_ai`], with each virtual self-call resolved on the
/// `Boss` runtime type — the ones `Boss` does not override route to the `enemy::`
/// functions, and the one it does, `die()`, routes to [`die`] here (see the module
/// header). `enemy.rs` is read-only in this lane, so the alternative — making
/// `enemy::die` a match-on-kind dispatcher — is not available.
pub fn update_ai(g: &mut Game, id: EntityId) {
    // boolean aligned = (offGridX || offGridY) ? false : true;
    let aligned = {
        let n = &g.entity_arena[id];
        !(n.off_grid_x || n.off_grid_y)
    };
    // if (this.state == 5) { if (deathTimer >= 1) { deathTimer--; return; } die(); }
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("Boss enemy")
        .battler
        .state;
    if state == 5 {
        let death_timer = g.entity_arena[id]
            .as_enemy()
            .expect("Boss enemy")
            .death_timer;
        if death_timer >= 1 {
            let e = g.entity_arena[id].as_enemy_mut().expect("Boss enemy");
            e.death_timer = (e.death_timer as i32).wrapping_sub(1) as i8;
            return;
        }
        // die();   — VIRTUAL: resolves to Boss.die (unregister), NOT Enemy.die (loot/exp).
        die(g, id);
    }
    // if (statusFlags[0] || statusFlags[2]) { enterIdle(false); return; }
    let (sf0, sf2) = {
        let e = g.entity_arena[id].as_enemy().expect("Boss enemy");
        (e.status_flags[0], e.status_flags[2])
    };
    if sf0 || sf2 {
        // enterIdle(false);   (inherited Enemy.enterIdle — not overridden by Boss)
        enemy::enter_idle(g, id, false);
        return;
    }
    // switch (this.state)  (re-read: die() above may have set state to 6)
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("Boss enemy")
        .battler
        .state;
    match state {
        // case 1: tryAttack();   (inherited Enemy.tryAttack)
        1 => enemy::try_attack(g, id),
        // case 2: if (aligned) chase();   (inherited Enemy.chase)
        2 => {
            if aligned {
                enemy::chase(g, id);
            }
        }
        // case 3: if (animFrame >= stats.castFrames) { enterIdle(false); tryAttack(); }
        3 => {
            let (anim_frame, cast_frames) = {
                let e = g.entity_arena[id].as_enemy().expect("Boss enemy");
                (e.battler.anim_frame, e.stats.cast_frames)
            };
            if anim_frame as i32 >= cast_frames as i32 {
                enemy::enter_idle(g, id, false);
                enemy::try_attack(g, id);
            }
        }
        // case 4: if (knockbackTimer < 1) setState(1); knockbackTimer--;
        4 => {
            let kt = g.entity_arena[id]
                .as_enemy()
                .expect("Boss enemy")
                .battler
                .knockback_timer;
            if kt < 1 {
                // setState((byte) 1);   — Enemy.setState (Boss does not override it): set
                //   the FSM state WITHOUT resetting animFrame.
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("Boss enemy")
                    .battler
                    .state = 1;
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("Boss enemy");
            e.battler.knockback_timer = (e.battler.knockback_timer as i32).wrapping_sub(1) as i8;
        }
        _ => {}
    }
}

/// `public abstract void onDeath()` (`av.m:()V`) — per-boss despawn/cleanup, run when
/// the boss dies.
///
/// **DEFERRED.** Abstract in `Boss`; a virtual call resolving to the concrete boss
/// subclass (`RockyBoss`/`Geb*`/`Nord*`), all of which are separate later batches and
/// unported. No concrete `Boss` leaf exists, so this is a DEFERRED no-op placeholder.
pub fn on_death(g: &mut Game, id: EntityId) {
    // (abstract — the concrete subclass's despawn/cleanup.)
    // DEFERRED: RockyBoss/Geb*/Nord* onDeath (concrete boss subclasses unported).
    let _ = (g, id);
}

/// `public void paint(Graphics graphics, int originX, int originY)`
/// (`av.a:(…Graphics;II)V`) — overrides `Enemy.paint`: the wider 16-cell boss sprite,
/// then the status icons + floaters. Unlike `Enemy.paint` there is NO off-screen cull
/// and NO ground shadow / layer-2 arc.
///
/// The boss sprite comes from `AssetCache.bossFrames[frameGroup]` (DEFERRED-loaded →
/// null element → the null-fallback branch fires and the frame-group draw no-ops).
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // int screenX = originX + pixelX + halfW + ((layer - 1) * 8);
    // int screenY = originY + pixelY + halfH;
    let (pixel_x, pixel_y, half_w, half_h, layer) = {
        let n = &g.entity_arena[id];
        (
            n.pixel_x as i32,
            n.pixel_y as i32,
            n.half_w as i32,
            n.half_h as i32,
            n.layer as i32,
        )
    };
    let screen_x = origin_x
        .wrapping_add(pixel_x)
        .wrapping_add(half_w)
        .wrapping_add(layer.wrapping_sub(1).wrapping_mul(8));
    let screen_y = origin_y.wrapping_add(pixel_y).wrapping_add(half_h);
    // Snapshot the frame-selection fields.
    let (state, move_dir, anim_frame, stat_row, size) = {
        let e = g.entity_arena[id].as_enemy().expect("Boss enemy");
        (
            e.battler.state,
            e.battler.move_dir,
            e.battler.anim_frame,
            e.stat_row,
            e.stats.size,
        )
    };
    // switch (state) { case 2: (statRow*16)+4+(moveDir-1); case 3: +12; case 5: +8; case 4/default: +0; }
    let mut frame_group = match state {
        2 => (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(4)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
        3 => (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(12)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
        5 => (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(8)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
        _ => (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(0)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
    };
    // if (AssetCache.bossFrames[frameGroup] == null) frameGroup = (statRow*16)+0+(moveDir-1);
    let boss_frames = g
        .asset_cache
        .boss_frames
        .as_ref()
        .expect("AssetCache.bossFrames null in Boss.paint");
    if boss_frames[frame_group as usize].is_none() {
        frame_group = (stat_row as i32)
            .wrapping_mul(16)
            .wrapping_add(0)
            .wrapping_add((move_dir as i32).wrapping_sub(1));
    }
    // (byte[]) AssetCache.bossFrames[frameGroup]  — DEFERRED-loaded → null element → no-op draw.
    let script = g.asset_cache.boss_frames.as_ref().unwrap()[frame_group as usize].clone();
    let width = g.game_screen.width;
    let world_height = g.game_screen.world_height;
    let Game {
        screen,
        asset_cache,
        entity_arena,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    // (re-establish GameMap.paint's persistent world clip on this fresh Graphics.)
    graphics.set_clip(0, 0, width, world_height);
    // GameScreen.drawFrameGroup(graphics, bossFrames[frameGroup], animFrame, screenX, screenY);
    game_screen::draw_frame_group(
        &mut graphics,
        asset_cache,
        script.as_deref(),
        anim_frame,
        screen_x,
        screen_y,
    );
    // drawStatusIcons(graphics, screenX, screenY - (stats.size * 3));
    {
        let e = entity_arena[id].as_enemy().expect("Boss enemy");
        battler::draw_status_icons(
            &e.battler,
            &mut graphics,
            screen_x,
            screen_y.wrapping_sub((size as i32).wrapping_mul(3)),
        );
    }
    // drawFloaters(graphics, screenX, screenY);
    {
        let e = entity_arena[id].as_enemy_mut().expect("Boss enemy");
        battler::draw_floaters(&mut e.battler, &mut graphics, screen_x, screen_y);
    }
}

/// `public final void takeGuardianHit(int rawDamage, byte guardianElement)`
/// (`av.a:(IB)V => ["imul","idiv","isub","i2s","i2s"]`) — the boss's intake of a
/// guardian's area attack. Boss-tuned: unlike `Enemy.takeGuardianHit` it does NOT
/// clear stun / set aggroed / hidden / recoil, and on a lethal hit it calls the
/// abstract [`on_death`] instead of arming a death timer.
pub fn take_guardian_hit(g: &mut Game, id: EntityId, raw_damage: i32, guardian_element: i8) {
    // if (this.state == 6 || this.state == 5) return;
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("Boss enemy")
        .battler
        .state;
    if state == 6 || state == 5 {
        return;
    }
    // if (rawDamage < 0) rawDamage = 0;
    let mut raw_damage = raw_damage;
    if raw_damage < 0 {
        raw_damage = 0;
    }
    // int finalDamage = (rawDamage * Directions.elementDamageMultiplier[guardianElement][stats.element]) / 10;
    let element = g.entity_arena[id]
        .as_enemy()
        .expect("Boss enemy")
        .stats
        .element;
    let mult = ELEMENT_DAMAGE_MULTIPLIER[guardian_element as usize][element as usize] as i32;
    let final_damage =
        java_div(raw_damage.wrapping_mul(mult), 10).expect("guardian rawDamage * mult / 10");
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Boss enemy");
        // ((Enemy) this).hp = (short) (((Enemy) this).hp - finalDamage);
        e.hp = (e.hp as i32).wrapping_sub(final_damage) as i16;
        // floaters.addElement(new Floater((byte) 7, (short) 4, (short) finalDamage));
        battler::add_floater(&mut e.battler, floater::new(7, 4, final_damage as i16));
        // floaters.addElement(new Floater((byte) 1));
        battler::add_floater(&mut e.battler, floater::new_default(1));
    }
    // if (((Enemy) this).hp <= 0) { setState((byte) 5); animFrame = 0; onDeath(); }
    let hp = g.entity_arena[id].as_enemy().expect("Boss enemy").hp;
    if hp <= 0 {
        {
            let e = g.entity_arena[id].as_enemy_mut().expect("Boss enemy");
            // setState((byte) 5);   — Enemy.setState (no animFrame reset); then animFrame = 0.
            e.battler.state = 5;
            e.battler.anim_frame = 0;
        }
        // onDeath();
        on_death(g, id);
    }
}

/// `public final void takeHeroHit(int rawDamage, boolean knockback, byte attackerDir,
/// boolean crit, byte hitFloaterKind, byte procKind, Hero hero)`
/// (`av.a:(IZBZBBLao;)V`) — the boss's full hero-attack resolution.
///
/// **DEFERRED body.** Like [`enemy::take_hero_hit`], past the death guard the pipeline
/// needs the unported `Weapon` (`hero.getEquip(0)`), `Guardian`
/// (`hero.getActiveGuardian().element()`), `Hero.addMp`/`addHp` and
/// `GameLoop.gameScreen.setTarget`. Its callers (Hero's attack FSM and `Projectile`'s
/// hero-owned hit block) are themselves DEFERRED, so it is never invoked in this slice.
/// The boss-tuned body (a simpler pipeline than `Enemy`'s: no armor halving, no
/// defense-break, no summon-ward, no stun/aggro; the dodge roll has NO 50-clamp and is
/// `× 2`; a lethal hit calls `onDeath()`) is recorded for when those land:
///
/// ```text
///   GameLoop.gameScreen.setTarget((Enemy) this, false);           // DEFERRED (GameScreen.setTarget)
///   Weapon weapon = (Weapon) hero.getEquip(0);                    // DEFERRED (Hero.getEquip → Weapon)
///   byte guardianElement = hero.getActiveGuardian().element();    // DEFERRED (Guardian)
///   int afterDefense = rawDamage - stats.defense;
///   int afterDefenseClamped = afterDefense; if (afterDefense < 0) afterDefenseClamped = 0;
///   int finalDamage = (afterDefenseClamped * Directions.elementDamageMultiplier[guardianElement][stats.element]) / 10;
///   if (crit) finalDamage += (finalDamage * weapon.critBonus) / 10;
///   boolean dodged = ByteUtil.randRange(0, 99) < (((((stats.evasion - (hero.agility + hero.agilityBonus)) - (weapon.refineLevel / 5)) + 10) * 2));
///   if (dodged) floaters.add(new Floater((byte) 2));
///   else { switch (procKind) { case 3: hero.addMp((finalDamage * 30) / 100); case 4: hero.addHp(finalDamage / 2); case 8: finalDamage *= 2; }
///          floaters.add(new Floater(hitFloaterKind)); damage(finalDamage);
///          if (hp <= 0) onDeath(); }
///   if (dodged) AudioManager.playSfx((byte) 14, false); else if (crit) playSfx((byte) 15, false); else playSfx((byte) 13, false);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn take_hero_hit(
    g: &mut Game,
    id: EntityId,
    raw_damage: i32,
    knockback: bool,
    attacker_dir: i8,
    crit: bool,
    hit_floater_kind: i8,
    proc_kind: i8,
    hero: EntityId,
) {
    // if (this.state == 6 || this.state == 5) return;
    let state = g.entity_arena[id]
        .as_enemy()
        .expect("Boss enemy")
        .battler
        .state;
    if state == 6 || state == 5 {
        return;
    }
    // (DEFERRED body — see the doc comment: Weapon/Guardian/addMp/addHp/setTarget unported.)
    let _ = (
        raw_damage,
        knockback,
        attacker_dir,
        crit,
        hit_floater_kind,
        proc_kind,
        hero,
    );
}

/// `public void die()` (`av.l:()V => []`, overriding `Enemy.die`) — death simply
/// unregisters the boss from the map (no loot / money / experience, unlike
/// `Enemy.die`).
pub fn die(g: &mut Game, id: EntityId) {
    // GameState.map.removeEntity(this);
    game_map::remove_entity(g, id);
}
