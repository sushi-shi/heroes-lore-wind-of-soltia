//! Transliterated from `java/src/main/java/defpackage/Enemy.java`
//! (original `al.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A hostile monster actor (the base class of every `Boss`). Its behaviour is driven
//! by a shared [`crate::enemy_type::EnemyTypeData`] stat template ([`EnemyData::stats`]):
//! the AI tick ([`update`]) runs a target-selection / chase / attack / animation
//! pipeline, and two damage-intake paths resolve incoming hits — a guardian's area
//! attack ([`take_guardian_hit`]) and the full hero-attack resolution
//! ([`take_hero_hit`]). On death ([`die`]) it rolls the loot table, drops money, and
//! awards experience. `Enemy extends Battler` (an [`EntityData::Enemy`]).
//!
//! ## Flattened subclass ([`EnemyData`])
//!
//! Modelled exactly as [`crate::hero`]/[`crate::npc`] flatten their leaves: an
//! [`EnemyData`] embeds a [`BattlerData`] as its "super" and is boxed in
//! [`EntityData::Enemy`]. `Battler` generic access is
//! [`crate::entity::EntityNode::as_battler`] (matching an `Enemy` node too); the
//! concrete access is [`crate::entity::EntityNode::as_enemy`]. The `Enemy.setState`
//! override (set state WITHOUT resetting `animFrame`, unlike `Battler.setState`) is the
//! private [`set_state`] helper, used by every internal `setState` call.
//!
//! ## DEFERRED cross-class boundaries
//!
//! - **Hero-damage resolution.** [`resolve_attack`]'s landed strike calls
//!   `hero.takeHit(this, facing)` — DEFERRED (`Hero`'s combat FSM is partial): no hero
//!   HP is applied. [`take_hero_hit`] (the enemy's intake of a hero strike) needs the
//!   unported `Weapon` (`hero.getEquip(0)`) and `Guardian` (`hero.getActiveGuardian`),
//!   both null in this slice, so its body is DEFERRED after the death guard.
//! - **Guardian companion.** [`try_attack`]/[`pick_target`] read
//!   `hero.getActiveGuardian()` (unported) — null in this slice; the guardian-target
//!   branches and `Battler.approach` (unimplemented in this lane) are DEFERRED, so the
//!   pursuit *step* is deferred while target acquisition (`target = picked`) is applied.
//! - **World hooks.** `GameMap.spawnEnemyAt`/`queueEnemySpawn`/`dropPickup` and
//!   `Hero.addExp` are unported → DEFERRED (the rng rolls that GATE them are preserved
//!   so the PRNG cadence stays faithful). The enemy sprite/effect script banks
//!   (`enemyFrames`/`attackEffectScripts`/`deathFxScripts`) are DEFERRED-loaded (see
//!   [`crate::asset_cache`]): a frame-group draw over a null bank no-ops.

use crate::battler::{self, BattlerData};
use crate::byte_util;
use crate::debug;
use crate::directions::{DIR_DX, DIR_DY, ELEMENT_DAMAGE_MULTIPLIER};
use crate::effect;
use crate::enemy_type::{self, EnemyTypeData};
use crate::entity::{self, EntityData, EntityId, EntityNode};
use crate::floater;
use crate::game::Game;
use crate::game_map;
use crate::game_screen;
use crate::projectile;
use crate::status_icon;
use j2me_jvm::{java_div, java_rem};

/// The `Enemy` (`al`) instance data — the subclass fields beyond the embedded
/// [`BattlerData`] "super". Boxed in [`EntityData::Enemy`].
#[derive(Debug)]
pub struct EnemyData {
    /// The `Battler` "super" (`super(pixelX, pixelY, (byte) 8, (byte) 8)`).
    pub battler: BattlerData,
    /// `public byte kind;` (`al.m`) — enemy kind (indexes `EnemyType.attackHitFrame`).
    pub kind: i8,
    /// `public byte statRow;` (`al.n`) — index into `EnemyType.types`.
    pub stat_row: i8,
    /// `private int homeTileX;` (`al.a`) — spawn tile X (for a queued respawn).
    pub home_tile_x: i32,
    /// `private int homeTileY;` (`al.b`) — spawn tile Y.
    pub home_tile_y: i32,
    /// `public EnemyType stats;` (`al.a`) — the shared stat template. Modelled as a
    /// per-instance **clone** (immutable after the load-time bind; see
    /// [`crate::enemy_type`] module header).
    pub stats: EnemyTypeData,
    /// `public short hp;` (`al.a`) — current hit points (max is `stats.maxHp`).
    pub hp: i16,
    /// `private byte attackCharge;` (`al.r`) — attack charge accumulated while chasing.
    pub attack_charge: i8,
    /// `public byte attackCooldown;` (`al.o`) — frames before the next attack.
    pub attack_cooldown: i8,
    /// `public byte hurtCooldown;` (`al.p`) — recovery frames after being hit.
    pub hurt_cooldown: i8,
    /// `public byte deathTimer;` (`al.q`) — death-animation countdown (state 5).
    pub death_timer: i8,
    /// `private byte recoilTimer;` (`al.s`) — hit-recoil display countdown.
    pub recoil_timer: i8,
    /// `private byte recoilDir;` (`al.t`) — direction of the recoil shake offset.
    pub recoil_dir: i8,
    /// `private boolean aggroed;` (`al.d`) — true once engaged (hit or aggroed).
    pub aggroed: bool,
    /// `private boolean hidden;` (`al.e`) — spawned hidden (ambush / summon phases).
    pub hidden: bool,
    /// `private boolean[] statusFlags;` (`al.b`) — active status effects (0..4).
    pub status_flags: [bool; 5],
    /// `private Entity target;` (`al.c`) — current AI target (hero/guardian) or null.
    pub target: Option<EntityId>,
    /// `private byte summonTimer;` (`al.u`) — summon cooldown (-10 idle, counts from 40).
    pub summon_timer: i8,
    /// `private boolean onScreen;` (`al.f`) — whether on screen last frame (throttles AI).
    pub on_screen: bool,
}

/// `Enemy.setState(byte newState)` (`al` override of `Battler.setState`) — sets the
/// FSM state **without** resetting `animFrame` (the base resets it to `-1`). Every
/// internal `setState(...)` in `Enemy` routes here.
fn set_state(e: &mut EnemyData, new_state: i8) {
    // this.state = newState;
    e.battler.state = new_state;
}

/// `public Enemy(short pixelX, short pixelY, byte kind, byte statRow)`
/// (`al.<init>:(SSBB)V`). Allocates the enemy node in the arena and returns its
/// [`EntityId`].
///
/// `super(pixelX, pixelY, (byte) 8, (byte) 8)` is modelled by [`entity::init_base`] +
/// [`BattlerData::new`] (the base Entity/Battler init, mirroring [`crate::npc::new_npc`]).
/// The trailing explicit `setPixelPos(pixelX, pixelY)` (the `Enemy` override) registers
/// occupancy — so `GameState.map` must exist when an enemy is constructed.
pub fn new_enemy(g: &mut Game, pixel_x: i16, pixel_y: i16, kind: i8, stat_row: i8) -> EntityId {
    // this.stats = EnemyType.types[statRow];  (cloned; see the module deviation.)
    let stats = g
        .enemy_type
        .types
        .as_ref()
        .expect("EnemyType.types null in new Enemy")[stat_row as usize]
        .clone()
        .expect("EnemyType.types[statRow] null in new Enemy");
    // ((Entity) this).layer = this.stats.size == 2 ? (byte) 2 : (byte) 1;
    let layer: i8 = if stats.size == 2 { 2 } else { 1 };
    let enemy = EnemyData {
        // super(...) → Battler.init: state 1, facing 2, moveDir 2, animFrame -1, knockbackTimer 0.
        battler: BattlerData::new(),
        // this.kind = kind; this.statRow = statRow;
        kind,
        stat_row,
        // homeTileX/Y are assigned below (after init_base derives the tile).
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
    // super(pixelX, pixelY, (byte) 8, (byte) 8);
    let mut node = EntityNode {
        data: EntityData::Enemy(Box::new(enemy)),
        layer,
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 8);
    // this.homeTileX = ((Entity) this).tileX; this.homeTileY = ((Entity) this).tileY;
    let (tx, ty) = (node.tile_x, node.tile_y);
    if let EntityData::Enemy(e) = &mut node.data {
        e.home_tile_x = tx as i32;
        e.home_tile_y = ty as i32;
    }
    let id = g.entity_arena.alloc(node);
    // setPixelPos(pixelX, pixelY);   (Enemy override → occupancy register)
    set_pixel_pos(g, id, pixel_x, pixel_y);
    id
}

/// `public final void setPixelPos(short pixelX, short pixelY)` (`al.a:(SS)V`) —
/// overrides `Entity.setPixelPos`: clear the footprint, move, re-derive the tile, and
/// re-register occupancy (unconditionally, unlike `Npc`).
pub fn set_pixel_pos(g: &mut Game, id: EntityId, pixel_x: i16, pixel_y: i16) {
    // clearOccupancy();
    battler::clear_occupancy(g, id);
    // super.setPixelPos(pixelX, pixelY);
    entity::set_pixel_pos(&mut g.entity_arena[id], pixel_x, pixel_y);
    // syncTile();
    entity::sync_tile(&mut g.entity_arena[id]);
    // setOccupancy();
    battler::set_occupancy(g, id);
}

/// `public void paint(Graphics graphics, int originX, int originY)`
/// (`al.a:(…Graphics;II)V`) — overrides `Entity.paint`: cull off-screen (still drawing
/// floaters), else blit the ground shadow (or the layer-2 boss arc), the state-selected
/// enemy sprite, then the status icons + floaters.
///
/// The enemy sprite comes from `AssetCache.enemyFrames[frameGroup]` (DEFERRED-loaded →
/// null element → the frame-group draw no-ops). The `instanceof Boss` / `instanceof
/// RockyBoss` shadow gate is constant here — no `Boss` node exists in this slice
/// (`Boss` is a later batch) — so it reduces to `kind != 22 && kind != 16`.
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
    let width = g.game_screen.width;
    let world_height = g.game_screen.world_height;
    // if (screenX + 16 < 0 || screenY < 0 || screenX - 16 > width || screenY > worldHeight + 32) {
    //   drawFloaters(graphics, screenX, screenY); this.onScreen = false; return; }
    if screen_x.wrapping_add(16) < 0
        || screen_y < 0
        || screen_x.wrapping_sub(16) > width
        || screen_y > world_height.wrapping_add(32)
    {
        {
            let Game {
                screen,
                entity_arena,
                ..
            } = &mut *g;
            let target = screen.as_mut().expect("framebuffer");
            let mut graphics = j2me_me::Graphics::new(target);
            // (re-establish GameMap.paint's persistent world clip on this fresh Graphics.)
            graphics.set_clip(0, 0, width, world_height);
            let e = entity_arena[id].as_enemy_mut().expect("Enemy");
            battler::draw_floaters(&mut e.battler, &mut graphics, screen_x, screen_y);
        }
        g.entity_arena[id].as_enemy_mut().expect("Enemy").on_screen = false;
        return;
    }
    // this.onScreen = true;
    g.entity_arena[id].as_enemy_mut().expect("Enemy").on_screen = true;
    // if (this.hidden) return;
    if g.entity_arena[id].as_enemy().expect("Enemy").hidden {
        return;
    }
    // int drawX = screenX; int drawY = screenY;
    let mut draw_x = screen_x;
    let mut draw_y = screen_y;
    // if (recoilTimer == 3 || recoilTimer == 1) { drawY += dirDy[recoilDir]*3; drawX += dirDx[recoilDir]*3; }
    let (recoil_timer, recoil_dir) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.recoil_timer, e.recoil_dir)
    };
    if recoil_timer == 3 || recoil_timer == 1 {
        draw_y = draw_y.wrapping_add((DIR_DY[recoil_dir as usize] as i32).wrapping_mul(3));
        draw_x = draw_x.wrapping_add((DIR_DX[recoil_dir as usize] as i32).wrapping_mul(3));
    }
    // Snapshot the frame-selection + shadow-gate fields.
    let (state, move_dir, anim_frame, stat_row, kind, size) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (
            e.battler.state,
            e.battler.move_dir,
            e.battler.anim_frame,
            e.stat_row,
            e.kind,
            e.stats.size,
        )
    };
    // switch (state) { case 2: (statRow*12)+4+(moveDir-1); case 3: +8+; default: +0+; }
    let frame_group = match state {
        2 => (stat_row as i32)
            .wrapping_mul(12)
            .wrapping_add(4)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
        3 => (stat_row as i32)
            .wrapping_mul(12)
            .wrapping_add(8)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
        _ => (stat_row as i32)
            .wrapping_mul(12)
            .wrapping_add(0)
            .wrapping_add((move_dir as i32).wrapping_sub(1)),
    };
    // (byte[]) AssetCache.enemyFrames[frameGroup]  — DEFERRED-loaded → null element → no-op draw.
    let script = g
        .asset_cache
        .enemy_frames
        .as_ref()
        .and_then(|banks| banks[frame_group as usize].clone());
    // if ((kind != 22 && kind != 16 && !(this instanceof Boss)) || (this instanceof RockyBoss))
    //   — no Boss node exists here, so `instanceof Boss` = false, `instanceof RockyBoss` = false.
    let draw_shadow = kind != 22 && kind != 16;
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
    if draw_shadow {
        // if (layer == 1) graphics.drawImage(entityShadow, drawX, drawY - 3, 17);
        if layer == 1 {
            let shadow = asset_cache
                .entity_shadow
                .as_ref()
                .expect("NullPointerException: entityShadow null");
            graphics
                .draw_image(shadow, draw_x, draw_y.wrapping_sub(3), 17)
                .expect("drawImage(entityShadow)");
        } else {
            // else { graphics.setColor(2047807); graphics.fillArc(drawX-11, drawY-6, 22, 9, 0, 360); }
            graphics.set_color(2047807);
            graphics.fill_arc(
                draw_x.wrapping_sub(11),
                draw_y.wrapping_sub(6),
                22,
                9,
                0,
                360,
            );
        }
    }
    // GameScreen.drawFrameGroup(graphics, enemyFrames[frameGroup], animFrame, drawX, drawY);
    game_screen::draw_frame_group(
        &mut graphics,
        asset_cache,
        script.as_deref(),
        anim_frame,
        draw_x,
        draw_y,
    );
    // drawStatusIcons(graphics, screenX, screenY - (stats.size * 3));
    {
        let e = entity_arena[id].as_enemy().expect("Enemy");
        battler::draw_status_icons(
            &e.battler,
            &mut graphics,
            screen_x,
            screen_y.wrapping_sub((size as i32).wrapping_mul(3)),
        );
    }
    // drawFloaters(graphics, screenX, screenY);
    {
        let e = entity_arena[id].as_enemy_mut().expect("Enemy");
        battler::draw_floaters(&mut e.battler, &mut graphics, screen_x, screen_y);
    }
}

/// `public void update()` (`al.d:()V`, overriding `Battler.update`) — the enemy's
/// per-tick AI: advance `animFrame`, tick statuses, off-screen throttle, recoil decay,
/// then [`update_ai`] / `stepIfMoving` / [`animate`].
pub fn update(g: &mut Game, id: EntityId) {
    // this.animFrame = (byte) (this.animFrame + 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        e.battler.anim_frame = (e.battler.anim_frame as i32).wrapping_add(1) as i8;
    }
    // tickStatuses();
    tick_statuses(g, id);
    // if (!this.onScreen) { Hero hero = GameState.hero(); ... off-screen AI throttle }
    let on_screen = g.entity_arena[id].as_enemy().expect("Enemy").on_screen;
    if !on_screen {
        // Hero hero = GameState.hero();
        let hero = g
            .game_state
            .hero
            .expect("GameState.hero null in Enemy.update");
        // byte distX = tileDistX(hero); byte distY = tileDistY(hero);
        let dist_x = tile_dist_x(g, id, hero);
        let dist_y = tile_dist_y(g, id, hero);
        let (sight_range, has_target) = {
            let e = g.entity_arena[id].as_enemy().expect("Enemy");
            (e.stats.sight_range, e.target.is_some())
        };
        // if ((distX > sightRange || distY > sightRange) && target == null) return;
        if (dist_x as i32 > sight_range as i32 || dist_y as i32 > sight_range as i32) && !has_target
        {
            return;
        }
    }
    // if (this.recoilTimer > 0) this.recoilTimer = (byte) (this.recoilTimer - 1);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        if e.recoil_timer > 0 {
            e.recoil_timer = (e.recoil_timer as i32).wrapping_sub(1) as i8;
        }
    }
    // updateAi();
    update_ai(g, id);
    // stepIfMoving();   (inherited final Battler.stepIfMoving — Enemy uses base move/tryStepForward)
    battler::step_if_moving(g, id);
    // animate();
    animate(g, id);
}

/// `private final void tickStatuses()` (`al.s:()V`) — ticks each active status icon;
/// poison (kind 3) deals 15-25 damage every 8 frames; reaps finished icons.
fn tick_statuses(g: &mut Game, id: EntityId) {
    // for (int i = statuses.size() - 1; i >= 0; i--) {
    let mut i = (g.entity_arena[id]
        .as_enemy()
        .expect("Enemy")
        .battler
        .statuses
        .len() as i32)
        .wrapping_sub(1);
    while i >= 0 {
        // StatusIcon icon = (StatusIcon) statuses.elementAt(i); icon.tick();
        {
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            status_icon::tick(&mut e.battler.statuses[i as usize]);
        }
        let (kind, elapsed, finished) = {
            let e = g.entity_arena[id].as_enemy().expect("Enemy");
            let icon = &e.battler.statuses[i as usize];
            let kind = icon
                .as_status_icon()
                .expect("ClassCastException: statuses element is not a StatusIcon")
                .kind;
            (kind, status_icon::elapsed(icon), icon.finished)
        };
        // if (icon.kind == 3 && icon.elapsed() % 8 == 0) damage(ByteUtil.randRange(15, 25));
        if kind == 3 && java_rem(elapsed as i32, 8).expect("icon.elapsed() % 8") == 0 {
            let amount = byte_util::rand_range(&mut g.byte_util, 15, 25);
            damage(g, id, amount);
        }
        // if (icon.finished) { statuses.removeElementAt(i); statusFlags[icon.kind] = false; }
        if finished {
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            e.battler.statuses.remove(i as usize);
            e.status_flags[kind as usize] = false;
        }
        i = i.wrapping_sub(1);
    }
}

/// `public void updateAi()` (`al.n:()V`) — the AI state machine: idle/chase/attack/
/// knockback/death dispatch.
pub fn update_ai(g: &mut Game, id: EntityId) {
    // boolean aligned = (offGridX || offGridY) ? false : true;
    let aligned = {
        let n = &g.entity_arena[id];
        !(n.off_grid_x || n.off_grid_y)
    };
    // if (this.state == 5) { if (deathTimer >= 1) { deathTimer--; return; } die(); }
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
    if state == 5 {
        let death_timer = g.entity_arena[id].as_enemy().expect("Enemy").death_timer;
        if death_timer >= 1 {
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            e.death_timer = (e.death_timer as i32).wrapping_sub(1) as i8;
            return;
        }
        die(g, id);
    }
    // if (statusFlags[0] || statusFlags[2]) { enterIdle(false); return; }
    let (sf0, sf2) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.status_flags[0], e.status_flags[2])
    };
    if sf0 || sf2 {
        enter_idle(g, id, false);
        return;
    }
    // switch (this.state)  (re-read: die() above may have set state to 6)
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
    match state {
        // case 1: tryAttack();
        1 => try_attack(g, id),
        // case 2: if (aligned) chase();
        2 => {
            if aligned {
                chase(g, id);
            }
        }
        // case 3: if (animFrame >= stats.castFrames) { enterIdle(false); tryAttack(); }
        3 => {
            let (anim_frame, cast_frames) = {
                let e = g.entity_arena[id].as_enemy().expect("Enemy");
                (e.battler.anim_frame, e.stats.cast_frames)
            };
            if anim_frame as i32 >= cast_frames as i32 {
                enter_idle(g, id, false);
                try_attack(g, id);
            }
        }
        // case 4: if (knockbackTimer < 1) setState(1); knockbackTimer--;
        4 => {
            let kt = g.entity_arena[id]
                .as_enemy()
                .expect("Enemy")
                .battler
                .knockback_timer;
            if kt < 1 {
                let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
                set_state(e, 1);
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            e.battler.knockback_timer = (e.battler.knockback_timer as i32).wrapping_sub(1) as i8;
        }
        _ => {}
    }
}

/// `public void chase()` (`al.h:()V`) — either attacks now, or banks attack charge and
/// keeps pressing.
pub fn chase(g: &mut Game, id: EntityId) {
    let (attack_charge, attack_delay) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.attack_charge, e.stats.attack_delay)
    };
    // if (attackCharge >= stats.attackDelay * 2 || Entity.rng.nextInt() <= 0) {
    if attack_charge as i32 >= (attack_delay as i32).wrapping_mul(2) || g.entity.rng.next_int() <= 0
    {
        // enterIdle(false); tryAttack();
        enter_idle(g, id, false);
        try_attack(g, id);
    } else {
        {
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            // attackCharge = (byte) (attackCharge + stats.attackDelay);
            e.attack_charge = (e.attack_charge as i32).wrapping_add(attack_delay as i32) as i8;
            // attackCooldown = 0;
            e.attack_cooldown = 0;
        }
        // tryAttack();
        try_attack(g, id);
    }
}

/// `public void tryAttack()` (`al.i:()V`) — strike the hero if in range, else pick a
/// target and (DEFERRED) approach it.
///
/// The `attackCooldown == 0` block's pursuit (`Battler.approach`, unimplemented in this
/// lane) and its guardian-target early-returns (`Guardian`, unported) are DEFERRED; the
/// target *acquisition* (`this.target = picked`) and [`wander`] are applied.
pub fn try_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in Enemy.tryAttack");
    // Guardian guardian = hero.getActiveGuardian();
    //   DEFERRED: Hero.getActiveGuardian (activeGuardian null in this slice). Only the
    //   attackCooldown==0 block below reads it, and that block's uses are DEFERRED.
    let (hurt_cooldown, ai_type, facing) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.hurt_cooldown, e.stats.ai_type, e.battler.facing)
    };
    // if (this.hurtCooldown == 0) {
    if hurt_cooldown == 0 {
        // if ((aiType != 0 && aiType != 1) || entityInDir(facing, hero) != hero) {
        let in_dir_is_hero = entity_in_dir(g, id, facing, Some(hero)) == Some(hero);
        if (ai_type != 0 && ai_type != 1) || !in_dir_is_hero {
            // if (aiType == 2 || aiType == 3) { for d in 1..=3 if neighbor(facing,d)==hero { beginAttack(); return; } }
            if ai_type == 2 || ai_type == 3 {
                let mut dist: i8 = 1;
                loop {
                    let d = dist;
                    if d > 3 {
                        break;
                    }
                    if neighbor(g, id, facing, d) == Some(hero) {
                        begin_attack(g, id);
                        return;
                    }
                    dist = (d as i32).wrapping_add(1) as i8;
                }
            }
        } else {
            // beginAttack(); return;
            begin_attack(g, id);
            return;
        }
    }
    // if (this.attackCooldown == 0) {
    let attack_cooldown = g.entity_arena[id]
        .as_enemy()
        .expect("Enemy")
        .attack_cooldown;
    if attack_cooldown == 0 {
        // byte range = (aiType == 2 || aiType == 3) ? 3 : 1;
        let range: i8 = if ai_type == 2 || ai_type == 3 { 3 } else { 1 };
        // if (target == guardian && guardian.castState == 2) { approach(guardian, range); return; }
        // if (target == hero && !guardian.isBusy()) { approach(hero, range); return; }
        //   DEFERRED: both early-returns reach the unported Guardian (castState/isBusy) and
        //   Battler.approach (unimplemented in this lane). activeGuardian is null in this
        //   slice (so no enemy ever targets a guardian); re-picking via pickTarget below —
        //   which returns only hero/null here — is behaviourally identical.
        // Entity picked = pickTarget(hero, guardian);
        let picked = pick_target(g, id, hero);
        // if (picked == null) wander(); else { approach(picked, range); target = picked; }
        match picked {
            None => wander(g, id),
            Some(p) => {
                // approach(picked, range);
                //   DEFERRED: Battler.approach — the pursuit step (unimplemented in this
                //   lane). The target acquisition below is applied.
                let _ = range;
                g.entity_arena[id].as_enemy_mut().expect("Enemy").target = Some(p);
            }
        }
    }
}

/// `public void animate()` (`al.o:()V`) — advances animation frames, ticks cooldowns,
/// and runs the summon timer.
pub fn animate(g: &mut Game, id: EntityId) {
    // if (stats.summonsAllies && aggroed) { summon timer + spawnEnemyAt }
    let (summons, aggroed) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.stats.summons_allies, e.aggroed)
    };
    if summons && aggroed {
        // if (summonTimer > 0) summonTimer--;
        let summon_timer = g.entity_arena[id].as_enemy().expect("Enemy").summon_timer;
        if summon_timer > 0 {
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            e.summon_timer = (e.summon_timer as i32).wrapping_sub(1) as i8;
        }
        // if (summonTimer == 0) { GameState.map.spawnEnemyAt(...); summonTimer = -10; }
        let summon_timer = g.entity_arena[id].as_enemy().expect("Enemy").summon_timer;
        if summon_timer == 0 {
            // GameState.map.spawnEnemyAt(tileX, tileY, kind, statRow, true, (byte) 1, (byte) 5);
            //   DEFERRED: GameMap.spawnEnemyAt (the enemy-spawn path is unported). The timer
            //   reset is preserved so the summoner's cadence stays bounded.
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
            e.summon_timer = -10;
        }
    }
    // switch (this.state)  (case 4 and default share the walk/cooldown body)
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
    match state {
        // case 2: if (animFrame >= stats.attackFrames) animFrame = 0;
        2 => {
            let (anim_frame, attack_frames) = {
                let e = g.entity_arena[id].as_enemy().expect("Enemy");
                (e.battler.anim_frame, e.stats.attack_frames)
            };
            if anim_frame as i32 >= attack_frames as i32 {
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("Enemy")
                    .battler
                    .anim_frame = 0;
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
                let e = g.entity_arena[id].as_enemy().expect("Enemy");
                (e.battler.anim_frame, e.stats.walk_frames)
            };
            if anim_frame as i32 >= walk_frames as i32 {
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("Enemy")
                    .battler
                    .anim_frame = 0;
            }
            let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
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

/// `public void resolveAttack()` (`al.j:()V`) — on the attack's hit frame, applies the
/// strike to the hero (melee / ranged floater / projectile).
///
/// The landed hero-melee/ranged strikes call `hero.takeHit(this, facing)` — DEFERRED
/// (`Hero.takeHit`: the hero combat FSM is partial), so no hero HP is applied. The
/// aiType-3 projectile spawn IS ported (the `Projectile` class landed) and reads the
/// DEFERRED-loaded `AssetCache.attackEffectScripts` bank (loaded before enemies act in
/// real play; a null element would NPE, faithfully).
pub fn resolve_attack(g: &mut Game, id: EntityId) {
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("GameState.hero null in Enemy.resolveAttack");
    // this.hidden = false;
    g.entity_arena[id].as_enemy_mut().expect("Enemy").hidden = false;
    let (anim_frame, kind, ai_type, facing, move_dir, stat_row) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (
            e.battler.anim_frame,
            e.kind,
            e.stats.ai_type,
            e.battler.facing,
            e.battler.move_dir,
            e.stat_row,
        )
    };
    // if (animFrame != EnemyType.attackHitFrame[kind] - 1) return;
    if anim_frame as i32 != (enemy_type::ATTACK_HIT_FRAME[kind as usize] as i32).wrapping_sub(1) {
        return;
    }
    // if ((aiType == 0 || aiType == 1) && entityInDir(facing, hero) == hero) { hero.takeHit(this, facing); return; }
    if (ai_type == 0 || ai_type == 1) && entity_in_dir(g, id, facing, Some(hero)) == Some(hero) {
        // hero.takeHit(this, this.facing);
        //   DEFERRED: Hero.takeHit — the hero-damage resolution (Hero's combat FSM is partial).
        return;
    }
    if ai_type == 2 {
        // ranged melee: emote floater + hero.takeHit
        let mut dist: i8 = 1;
        loop {
            let d = dist;
            if d > 3 {
                return;
            }
            if neighbor(g, id, facing, d) == Some(hero) {
                // hero.addFloater(new Floater((byte) 9, (short) -1, this.statRow));
                let floater = floater::new(9, -1, stat_row as i16);
                let hb = g.entity_arena[hero].as_battler_mut().expect("Hero battler");
                battler::add_floater(hb, floater);
                // hero.takeHit(this, this.facing);
                //   DEFERRED: Hero.takeHit — the hero-damage resolution.
                return;
            }
            dist = (d as i32).wrapping_add(1) as i8;
        }
    } else {
        // if (aiType != 3) return;
        if ai_type != 3 {
            return;
        }
        let mut dist2: i8 = 1;
        loop {
            let d2 = dist2;
            if d2 > 3 {
                return;
            }
            if neighbor(g, id, facing, d2) == Some(hero) {
                // GameState.map.addEntity(new Projectile((byte)(tileX + dirDx[moveDir]),
                //   (byte)(tileY + dirDy[moveDir]), (byte[]) AssetCache.attackEffectScripts[statRow],
                //   this, moveDir, (byte) 3, (byte) 2));
                let (tile_x, tile_y) = {
                    let n = &g.entity_arena[id];
                    (n.tile_x as i32, n.tile_y as i32)
                };
                let script = g
                    .asset_cache
                    .attack_effect_scripts
                    .as_ref()
                    .expect("AssetCache.attackEffectScripts null in Enemy.resolveAttack")
                    [stat_row as usize]
                    .clone()
                    .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)");
                let ptx = tile_x.wrapping_add(DIR_DX[move_dir as usize] as i32) as i8;
                let pty = tile_y.wrapping_add(DIR_DY[move_dir as usize] as i32) as i8;
                let new_id = projectile::new_projectile_enemy(
                    &mut g.entity_arena,
                    ptx,
                    pty,
                    script,
                    id,
                    move_dir,
                    3,
                    2,
                );
                game_map::add_entity(g, new_id);
                return;
            }
            dist2 = (d2 as i32).wrapping_add(1) as i8;
        }
    }
}

/// `public void stepDeathAnim()` (`al.k:()V`) — wraps the death-animation frame counter.
pub fn step_death_anim(g: &mut Game, id: EntityId) {
    // if (animFrame >= stats.walkFrames) animFrame = 0;
    let (anim_frame, walk_frames) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.battler.anim_frame, e.stats.walk_frames)
    };
    if anim_frame as i32 >= walk_frames as i32 {
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("Enemy")
            .battler
            .anim_frame = 0;
    }
}

/// `public final void spawnEffectAt(byte tileX2, byte tileY2)` (`al.a:(BB)V`) — spawns
/// this enemy's effect sprite at (`tileX2`,`tileY2`). Reads the DEFERRED-loaded
/// `AssetCache.attackEffectScripts` bank (a null element NPEs, faithfully).
pub fn spawn_effect_at(g: &mut Game, id: EntityId, tile_x2: i8, tile_y2: i8) {
    let stat_row = g.entity_arena[id].as_enemy().expect("Enemy").stat_row;
    // GameState.map.addEntity(new Effect(tileX2, tileY2, (byte[]) AssetCache.attackEffectScripts[statRow]));
    let script = g
        .asset_cache
        .attack_effect_scripts
        .as_ref()
        .expect("AssetCache.attackEffectScripts null in Enemy.spawnEffectAt")[stat_row as usize]
        .clone()
        .expect("attackEffectScripts[statRow] null (DEFERRED-loaded bank)");
    let eff = effect::new_effect_from_script(&mut g.entity_arena, tile_x2, tile_y2, script);
    game_map::add_entity(g, eff);
}

/// `public final void deathEffect()` (`al.p:()V`) — spawns the death explosion effect
/// for this enemy's size. Reads the DEFERRED-loaded `AssetCache.deathFxScripts` bank
/// (a null element NPEs, faithfully; enemies only die after it is loaded in real play).
pub fn death_effect(g: &mut Game, id: EntityId) {
    let tile_x = g.entity_arena[id].tile_x;
    let tile_y = g.entity_arena[id].tile_y;
    let size = g.entity_arena[id].as_enemy().expect("Enemy").stats.size;
    // GameState.map.addEntity(new Effect(tileX, tileY, (byte[]) AssetCache.deathFxScripts[stats.size]));
    let script = g
        .asset_cache
        .death_fx_scripts
        .as_ref()
        .expect("AssetCache.deathFxScripts null in Enemy.deathEffect")[size as usize]
        .clone()
        .expect("deathFxScripts[size] null (DEFERRED-loaded bank)");
    let eff = effect::new_effect_from_script(&mut g.entity_arena, tile_x, tile_y, script);
    game_map::add_entity(g, eff);
}

/// `public final void enterIdle(boolean quickRecover)` (`al.a:(Z)V`) — returns to idle
/// (state 1), arming the attack cooldown (plus banked charge) and the hurt cooldown;
/// `quickRecover` randomly shortens the latter.
pub fn enter_idle(g: &mut Game, id: EntityId, quick_recover: bool) {
    let (attack_delay, hurt_delay, sf1) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.stats.attack_delay, e.stats.hurt_delay, e.status_flags[1])
    };
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        // this.attackCooldown = (byte) (stats.attackDelay + attackCharge);
        e.attack_cooldown = (attack_delay as i32).wrapping_add(e.attack_charge as i32) as i8;
        // this.attackCharge = 0;
        e.attack_charge = 0;
        // if (statusFlags[1]) hurtCooldown = (byte)((stats.hurtDelay*2)+1); else hurtCooldown = (byte)(stats.hurtDelay+1);
        if sf1 {
            e.hurt_cooldown = (hurt_delay as i32).wrapping_mul(2).wrapping_add(1) as i8;
        } else {
            e.hurt_cooldown = (hurt_delay as i32).wrapping_add(1) as i8;
        }
    }
    // if (quickRecover) hurtCooldown = (byte)((hurtCooldown * ByteUtil.randRange(1, 7)) / 10);
    if quick_recover {
        let hc = g.entity_arena[id].as_enemy().expect("Enemy").hurt_cooldown;
        let r = byte_util::rand_range(&mut g.byte_util, 1, 7);
        let v = java_div((hc as i32).wrapping_mul(r), 10)
            .expect("(hurtCooldown * randRange(1,7)) / 10");
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("Enemy")
            .hurt_cooldown = v as i8;
    }
    // setState((byte) 1); this.animFrame = 0;
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        set_state(e, 1);
        e.battler.anim_frame = 0;
    }
}

/// `public final void beginAttack()` (`al.q:()V`) — begins the attack animation
/// (state 3), arming the cooldowns.
pub fn begin_attack(g: &mut Game, id: EntityId) {
    let (attack_delay, hurt_delay, sf1) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.stats.attack_delay, e.stats.hurt_delay, e.status_flags[1])
    };
    let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
    // this.hidden = false;
    e.hidden = false;
    // this.attackCooldown = (byte) (stats.attackDelay + attackCharge);
    e.attack_cooldown = (attack_delay as i32).wrapping_add(e.attack_charge as i32) as i8;
    // this.attackCharge = 0;
    e.attack_charge = 0;
    // if (statusFlags[1]) hurtCooldown = (byte)((stats.hurtDelay*2)+1); else hurtCooldown = (byte)(stats.hurtDelay+1);
    if sf1 {
        e.hurt_cooldown = (hurt_delay as i32).wrapping_mul(2).wrapping_add(1) as i8;
    } else {
        e.hurt_cooldown = (hurt_delay as i32).wrapping_add(1) as i8;
    }
    // setState((byte) 3); this.animFrame = 0;
    set_state(e, 3);
    e.battler.anim_frame = 0;
}

/// `public final void wander()` (`al.r:()V`) — random idle wander: step or stand, then
/// face a random direction.
pub fn wander(g: &mut Game, id: EntityId) {
    // if (Entity.rng.nextInt() > 0) setState((byte) 2); else enterIdle(true);
    if g.entity.rng.next_int() > 0 {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        set_state(e, 2);
    } else {
        enter_idle(g, id, true);
    }
    // setFacing((byte) (((Entity.rng.nextInt() & 255) % 4) + 1));
    let r = g.entity.rng.next_int();
    let facing = java_rem(r & 255, 4)
        .expect("(rng.nextInt() & 255) % 4")
        .wrapping_add(1) as i8;
    let b = g.entity_arena[id].as_battler_mut().expect("Enemy battler");
    battler::set_facing(b, facing);
}

/// `public void die()` (`al.l:()V`) — enters the corpse state, queues a home respawn
/// (unless unique), rolls the weighted loot table, drops money and a rare item, and
/// awards the hero experience scaled by the level gap.
///
/// The world hooks `GameMap.queueEnemySpawn` / `GameMap.dropPickup` and `Hero.addExp`
/// are unported → DEFERRED; every rng roll that GATES them is preserved (the PRNG
/// cadence stays faithful). `deathEffect()` IS ported (reads the DEFERRED-loaded
/// `deathFxScripts` bank; enemies only die after it is loaded in real play).
pub fn die(g: &mut Game, id: EntityId) {
    // setState((byte) 6);
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        set_state(e, 6);
    }
    // if (stats.elemColor != 2) map.queueEnemySpawn(kind, stats.expReward, homeTileX, homeTileY);
    //   DEFERRED: GameMap.queueEnemySpawn (the delayed enemy-respawn queue; no rng consumed).
    // int dropRoll = ByteUtil.randRange(1, 150);
    let mut drop_roll = byte_util::rand_range(&mut g.byte_util, 1, 150);
    // int dropCount = stats.dropTable.length / 3;
    let drop_count = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        java_div(e.stats.drop_table.len() as i32, 3).expect("dropTable.length / 3")
    };
    // byte dropKind = -1; byte dropParam = -1;
    let mut drop_kind: i8 = -1;
    let mut drop_param: i8 = -1;
    // for (int i = 0; i < dropCount; i++) {
    let mut i: i32 = 0;
    while i < drop_count {
        let (weight, k, p) = {
            let e = g.entity_arena[id].as_enemy().expect("Enemy");
            (
                e.stats.drop_table[i.wrapping_mul(3).wrapping_add(2) as usize],
                e.stats.drop_table[i.wrapping_mul(3) as usize],
                e.stats.drop_table[i.wrapping_mul(3).wrapping_add(1) as usize],
            )
        };
        // int remaining = dropRoll - dropTable[i*3+2]; dropRoll = remaining;
        let remaining = drop_roll.wrapping_sub(weight as i32);
        drop_roll = remaining;
        // if (remaining <= 0) {
        if remaining <= 0 {
            // if (dropTable[i*3+2] == 1) { if (randRange(1,100) <= 20) { dropKind=..; dropParam=..; break; } }
            if weight == 1 {
                if byte_util::rand_range(&mut g.byte_util, 1, 100) <= 20 {
                    drop_kind = k;
                    drop_param = p;
                    break;
                }
            } else {
                // else { dropKind = dropTable[i*3]; dropParam = dropTable[i*3+1]; break; }
                drop_kind = k;
                drop_param = p;
                break;
            }
        }
        i = i.wrapping_add(1);
    }
    // if (dropKind != -1) map.queueEnemySpawn(tileX, tileY, dropKind, dropParam);
    //   DEFERRED: GameMap.queueEnemySpawn (no rng consumed).
    let _ = (drop_kind, drop_param);
    // if (ByteUtil.randRange(1,100) <= 60) map.dropPickup(tileX, tileY, (short)(stats.level*3));
    if byte_util::rand_range(&mut g.byte_util, 1, 100) <= 60 {
        // map.dropPickup(...);  — DEFERRED: GameMap.dropPickup.
    }
    // Read the level gap for the next roll + the exp scale.
    let (level, hero_level) = {
        let hero = g.game_state.hero.expect("GameState.hero null in Enemy.die");
        let level = g.entity_arena[id].as_enemy().expect("Enemy").stats.level;
        let hero_level = g.entity_arena[hero].as_hero().expect("Hero node").level;
        (level, hero_level)
    };
    // if (randRange(1,100) <= 20 + (stats.level - GameState.hero().level)) map.queueEnemySpawn(tileX, tileY, 11, 0);
    if byte_util::rand_range(&mut g.byte_util, 1, 100)
        <= 20i32.wrapping_add((level as i32).wrapping_sub(hero_level as i32))
    {
        // map.queueEnemySpawn(tileX, tileY, (byte) 11, (byte) 0);  — DEFERRED.
    }
    // int expScale = 20 - (GameState.hero().level - stats.level);
    let mut exp_scale = 20i32.wrapping_sub((hero_level as i32).wrapping_sub(level as i32));
    // if (expScale > 26) expScale = 26;
    if exp_scale > 26 {
        exp_scale = 26;
    }
    // int expGain = (stats.level * expScale) / 2;
    let exp_gain =
        java_div((level as i32).wrapping_mul(exp_scale), 2).expect("(stats.level * expScale) / 2");
    // if (expGain > 0) GameState.hero().addExp(expGain);
    if exp_gain > 0 {
        // GameState.hero().addExp(expGain);  — DEFERRED: Hero.addExp (leveling; Hero FSM partial).
    }
    // deathEffect();
    death_effect(g, id);
}

/// `public final byte tileDistX(Entity other)` (`al.a:(Lck;)B`) — absolute tile-column
/// distance to `other`.
pub fn tile_dist_x(g: &Game, id: EntityId, other: EntityId) -> i8 {
    // int dx = other.tileX - this.tileX;
    let dx = (g.entity_arena[other].tile_x as i32).wrapping_sub(g.entity_arena[id].tile_x as i32);
    // return dx > 0 ? (byte) dx : (byte) (-dx);
    if dx > 0 {
        dx as i8
    } else {
        dx.wrapping_neg() as i8
    }
}

/// `public final byte tileDistY(Entity other)` (`al.b:(Lck;)B`) — absolute tile-row
/// distance to `other`.
pub fn tile_dist_y(g: &Game, id: EntityId, other: EntityId) -> i8 {
    // int dy = other.tileY - this.tileY;
    let dy = (g.entity_arena[other].tile_y as i32).wrapping_sub(g.entity_arena[id].tile_y as i32);
    // return dy > 0 ? (byte) dy : (byte) (-dy);
    if dy > 0 {
        dy as i8
    } else {
        dy.wrapping_neg() as i8
    }
}

/// `public void takeGuardianHit(int rawDamage, byte guardianElement)` (`al.a:(IB)V`) —
/// applies a guardian's area attack: scaled by the element table `/10`, floated, and —
/// if lethal — starts the death sequence.
pub fn take_guardian_hit(g: &mut Game, id: EntityId, raw_damage: i32, guardian_element: i8) {
    // if (state == 6 || state == 5) return;
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
    if state == 6 || state == 5 {
        return;
    }
    // clearStun();
    clear_stun(g, id);
    // this.aggroed = true; this.hidden = false;
    let element = {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        e.aggroed = true;
        e.hidden = false;
        e.stats.element
    };
    // if (rawDamage < 0) rawDamage = 0;
    let mut raw_damage = raw_damage;
    if raw_damage < 0 {
        raw_damage = 0;
    }
    // int finalDamage = (rawDamage * Directions.elementDamageMultiplier[guardianElement][stats.element]) / 10;
    let mult = ELEMENT_DAMAGE_MULTIPLIER[guardian_element as usize][element as usize] as i32;
    let final_damage =
        java_div(raw_damage.wrapping_mul(mult), 10).expect("guardian rawDamage * mult / 10");
    {
        let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
        // this.hp = (short) (this.hp - finalDamage);
        e.hp = (e.hp as i32).wrapping_sub(final_damage) as i16;
        // floaters.addElement(new Floater((byte) 7, (short) 4, (short) finalDamage));
        battler::add_floater(&mut e.battler, floater::new(7, 4, final_damage as i16));
        // floaters.addElement(new Floater((byte) 1));
        battler::add_floater(&mut e.battler, floater::new_default(1));
        // this.recoilTimer = (byte) 4; this.recoilDir = (byte) 4;
        e.recoil_timer = 4;
        e.recoil_dir = 4;
        // if (this.hp <= 0) { setState((byte) 5); this.animFrame = 0; this.deathTimer = 3; }
        if e.hp <= 0 {
            set_state(e, 5);
            e.battler.anim_frame = 0;
            e.death_timer = 3;
        }
    }
}

/// `public void takeHeroHit(int rawDamage, boolean knockback, byte attackerDir,
/// boolean crit, byte hitFloaterKind, byte procKind, Hero hero)` (`al.a:(IZBZBBLao;)V`)
/// — the full hero-attack resolution on this enemy.
///
/// **DEFERRED body.** Past the death guard, the pipeline needs the unported `Weapon`
/// (`hero.getEquip(0)`) and `Guardian` (`hero.getActiveGuardian().element()`) — both
/// null in this slice (equipment + guardian setup are DEFERRED in `Hero.initClass`) —
/// plus `Hero.addMp`/`addHp` and `GameLoop.gameScreen.setTarget`. Its callers (Hero's
/// attack FSM and `Projectile`'s hero-owned hit block) are themselves DEFERRED, so it is
/// never invoked in this slice. The full body is recorded for when those land:
///
/// ```text
///   GameLoop.gameScreen.setTarget(this, false);                   // DEFERRED (GameScreen.setTarget)
///   Weapon weapon = (Weapon) hero.getEquip(0);                    // DEFERRED (Hero.getEquip → Weapon)
///   byte guardianElement = hero.getActiveGuardian().element();    // DEFERRED (Guardian)
///   if (!aggroed && stats.summonsAllies && guardianElement != stats.summonWardElement) summonTimer = 40;
///   clearStun(); aggroed = true; hidden = false;
///   if (stats.armored) rawDamage /= 2;
///   int afterDefense = statusFlags[4] ? rawDamage - (stats.defense / 2) : rawDamage - stats.defense;
///   if (afterDefense < 0) afterDefense = 0;
///   int finalDamage = (afterDefense * Directions.elementDamageMultiplier[guardianElement][stats.element]) / 10;
///   if (crit) finalDamage += (finalDamage * weapon.critBonus) / 10;
///   int dodgeChance = ((stats.evasion - (hero.agility + hero.agilityBonus)) - (weapon.refineLevel / 5)) + 10;
///   boolean dodged = ByteUtil.randRange(0, 99) < (dodgeChance > 50 ? 50 : dodgeChance);
///   byte statusToInflict = procKind == -1 ? -1 : Armor.PROC_STATUS[procKind];
///   if (dodged) floaters.add(new Floater((byte) 2));
///   else { switch (procKind) { case 2: finalDamage = stats.maxHp; case 3: hero.addMp((finalDamage*80)/100);
///          case 4: hero.addHp(finalDamage/2); case 8: finalDamage *= 2; }
///          if (statusToInflict != -1) { applyStatus(statusToInflict); statusFlags[statusToInflict] = true; }
///          floaters.add(new Floater(hitFloaterKind));
///          if (knockback && hp > 0 && !offGridX && !offGridY) { setState((byte)4); knockbackTimer = 2; facing = attackerDir; }
///          recoilTimer = 4; recoilDir = attackerDir; damage(finalDamage); }
///   if (dodged) AudioManager.playSfx((byte)14, false); else if (crit) playSfx((byte)15, false); else playSfx((byte)13, false);
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
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
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

/// `private final void clearStun()` (`al.t:()V`) — clears the freeze/stun status
/// (index 0) and removes its icon.
fn clear_stun(g: &mut Game, id: EntityId) {
    // if (this.statusFlags[0]) {
    let sf0 = g.entity_arena[id].as_enemy().expect("Enemy").status_flags[0];
    if sf0 {
        // this.statusFlags[0] = false;
        g.entity_arena[id]
            .as_enemy_mut()
            .expect("Enemy")
            .status_flags[0] = false;
        // for (int i = 0; i < statuses.size(); i++) { if (((StatusIcon) statuses.elementAt(i)).kind == 0) { removeElementAt(i); return; } }
        let mut i: i32 = 0;
        loop {
            let len = g.entity_arena[id]
                .as_enemy()
                .expect("Enemy")
                .battler
                .statuses
                .len() as i32;
            if i >= len {
                break;
            }
            let kind = g.entity_arena[id]
                .as_enemy()
                .expect("Enemy")
                .battler
                .statuses[i as usize]
                .as_status_icon()
                .expect("ClassCastException: statuses element is not a StatusIcon")
                .kind;
            if kind == 0 {
                g.entity_arena[id]
                    .as_enemy_mut()
                    .expect("Enemy")
                    .battler
                    .statuses
                    .remove(i as usize);
                return;
            }
            i = i.wrapping_add(1);
        }
    }
}

/// `public final void damage(int amount)` (`al.b:(I)V`) — subtracts `amount` HP, floats
/// the number, and starts death if lethal.
pub fn damage(g: &mut Game, id: EntityId, amount: i32) {
    let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
    // this.hp = (short) (this.hp - amount);
    e.hp = (e.hp as i32).wrapping_sub(amount) as i16;
    // floaters.addElement(new Floater((byte) 7, (short) 4, (short) amount));
    battler::add_floater(&mut e.battler, floater::new(7, 4, amount as i16));
    // if (this.hp <= 0) { setState((byte) 5); this.animFrame = -1; this.deathTimer = 3; }
    if e.hp <= 0 {
        set_state(e, 5);
        e.battler.anim_frame = -1;
        e.death_timer = 3;
    }
}

/// `public final void heal(int amount)` (`al.c:(I)V`) — heals `amount` HP, clamped to
/// the template maximum.
pub fn heal(g: &mut Game, id: EntityId, amount: i32) {
    let e = g.entity_arena[id].as_enemy_mut().expect("Enemy");
    // this.hp = (short) (this.hp + amount);
    e.hp = (e.hp as i32).wrapping_add(amount) as i16;
    // if (this.hp > this.stats.maxHp) this.hp = this.stats.maxHp;
    if e.hp > e.stats.max_hp {
        e.hp = e.stats.max_hp;
    }
}

/// `public final void slay(byte unused)` (`al.c:(B)V`) — instantly kills a living enemy
/// by dealing its full max HP as damage.
pub fn slay(g: &mut Game, id: EntityId, _unused: i8) {
    // this.hidden = false;
    g.entity_arena[id].as_enemy_mut().expect("Enemy").hidden = false;
    // if (this.state == 6 || this.state == 5) return;
    let state = g.entity_arena[id].as_enemy().expect("Enemy").battler.state;
    if state == 6 || state == 5 {
        return;
    }
    // damage(this.stats.maxHp);
    let max_hp = g.entity_arena[id].as_enemy().expect("Enemy").stats.max_hp as i32;
    damage(g, id, max_hp);
}

/// `private final Entity pickTarget(Hero hero, Guardian guardian)` (`al.a:(Lao;Lbl;)Lck;`)
/// — chooses this enemy's AI target: the guardian (DEFERRED — none in this slice) when
/// nearby and casting, otherwise the hero when within (aggro-widened) search range, else
/// null.
pub fn pick_target(g: &mut Game, id: EntityId, hero: EntityId) -> Option<EntityId> {
    // byte guardianDistX = tileDistX(guardian); byte guardianDistY = tileDistY(guardian);
    //   DEFERRED: the guardian companion (Guardian unported; activeGuardian null here).
    // byte searchRange = aggroed ? (relentless ? 100 : 8) : sightRange;
    let (aggroed, relentless, sight_range) = {
        let e = g.entity_arena[id].as_enemy().expect("Enemy");
        (e.aggroed, e.stats.relentless, e.stats.sight_range)
    };
    let search_range: i8 = if aggroed {
        if relentless {
            100
        } else {
            8
        }
    } else {
        sight_range
    };
    // byte reach = searchRange;
    let reach = search_range;
    // boolean heroInRange = tileDistX(hero) <= reach && tileDistY(hero) <= reach;
    let hero_in_range = tile_dist_x(g, id, hero) as i32 <= reach as i32
        && tile_dist_y(g, id, hero) as i32 <= reach as i32;
    // if ((guardianDistX <= reach && guardianDistY <= reach) && guardian.castState == 2 && randRange(0,9) < 7) return guardian;
    //   DEFERRED: the guardian-preference branch (Guardian.castState). No guardian exists
    //   in this slice, so in Java the earlier guardian-distance/castState conditions would
    //   already be false — the gated randRange(0,9) is not consumed here, matching Java.
    // if (heroInRange) return hero; return null;
    if hero_in_range {
        Some(hero)
    } else {
        None
    }
}

// --- Entity/Battler occupancy scans, inlined in the Enemy lane -----------------------
// `Entity.neighbor` (ck) and `Battler.entityInDir` (o) operate purely on Entity-base
// fields + the map occupancy grid; they are reproduced here (battler.rs is read-only in
// this lane, and the Enemy is their sole caller in this batch).

/// `public final Entity neighbor(byte direction, byte distance)` (`ck.a:(BB)Lck;`) —
/// the entity `distance` tiles away in `direction` (1 up, 2 down, 3 left, 4 right), or
/// `None` when off-map / empty.
fn neighbor(g: &Game, id: EntityId, direction: i8, distance: i8) -> Option<EntityId> {
    // GameMap map = GameState.map;
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
        // Debug.assertTrue(scanX >= 0); Debug.assertTrue(scanX < widthTiles);
        // Debug.assertTrue(scanY >= 0); Debug.assertTrue(scanY < heightTiles);
        debug::assert_true(scan_x >= 0);
        debug::assert_true(scan_x < width_tiles);
        debug::assert_true(scan_y >= 0);
        debug::assert_true(scan_y < height_tiles);
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
