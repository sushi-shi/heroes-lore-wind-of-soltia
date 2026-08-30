//! Transliterated from `java/src/main/java/defpackage/Projectile.java`
//! (original `i.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A moving [`crate::effect::EffectData`] that travels tile-by-tile and deals
//! damage. Each frame [`on_frame`] chains a fresh segment one tile forward
//! (decrementing [`ProjectileData::range`]) and, on landing, resolves a hit by owner
//! polarity: an enemy-owned bolt strikes the hero, a hero-owned bolt strikes an
//! enemy. The two constructors distinguish enemy-fired ([`new_projectile_enemy`])
//! from hero-fired shots ([`new_projectile_hero`], which carry a `damage` payload,
//! an inflicted `statusKind`, and a `crit` flag). `Projectile extends Effect`.
//!
//! ## Flattened subclass ([`ProjectileData`])
//!
//! Modelled as [`crate::hero`] flattens `Hero`: a [`ProjectileData`] embeds an
//! [`EffectData`] as its "super" and lives in
//! [`crate::entity::EntityData::Projectile`]. A Java `((Effect) this).frame` /
//! `super.spriteScript` becomes [`crate::entity::EntityNode::as_effect`] (the
//! `instanceof Effect` super accessor); the concrete-type access is
//! [`crate::entity::EntityNode::as_projectile`].
//!
//! ## DEFERRED combat (Enemy / Hero.takeHit)
//!
//! The chain-spawn is ported for the **hero-owned** branch (the reachable one in
//! this slice); the **enemy-owned** spawn and the whole `frame == 1` hit-resolution
//! block reach the unported `Enemy` (`al`, `takeHeroHit`) and `Hero.takeHit` (the
//! DEFERRED combat FSM), so they are DEFERRED with named comments. No `Enemy` nodes
//! exist in this slice, so those branches are unreachable and deferring them changes
//! nothing observable (`hasHit` stays `false`). [`paint`], [`is_finished`] and the
//! hero-owned tile-chaining — the observable travel/lifecycle — are fully ported.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `i.<init>:(BB[BLo;BBB)V => ["isub","i2b"]` (enemy ctor: `range - 1` → byte),
//! `i.<init>:(BB[BLo;ZBBBIBZ)V => ["isub","i2b"]` (hero ctor: `range - 1`),
//! `i.a:()V (onFrame) => ["iadd","i2b","iadd","i2b"]` (`nextX = tileX + dirDx[dir]`,
//! `nextY = tileY + dirDy[dir]`, each narrowed to byte), `i.a:()Z (isFinished) => []`,
//! `i.a:(…Graphics;II)V (paint) =>
//! ["iadd"×4,"imul","iadd","imul","iadd","iadd","i2s"]` (`screenX`/`screenY`, the
//! `dirDx[dir] * 8`/`dirDy[dir] * 8` nudge, the `frame + 1`).

use crate::directions::{DIR_DX, DIR_DY};
use crate::effect::{self, effect_data_from_script, EffectData};
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game::Game;
use crate::game_map;
use j2me_jvm::ishl;

/// The `Projectile` (`i`) instance data — the subclass fields beyond the embedded
/// [`EffectData`] "super". Boxed in [`EntityData::Projectile`].
#[derive(Debug)]
pub struct ProjectileData {
    /// The `Effect` "super" (`super(tileX, tileY, spriteScript)`).
    pub effect: EffectData,
    /// `private Battler owner;` (`i.a`) — the battler that fired this projectile
    /// (an [`EntityId`] into the arena; always set by the constructors).
    pub owner: EntityId,
    /// `private boolean piercing;` (`i.d`) — keeps travelling and hitting after the
    /// first target.
    pub piercing: bool,
    /// `private byte dir;` (`i.f`) — travel direction (1..4).
    pub dir: i8,
    /// `private byte range;` (`i.g`) — tiles of range remaining (each chained
    /// segment decrements it).
    pub range: i8,
    /// `private byte chainFrame;` (`i.h`) — frame at which the projectile chains its
    /// next segment.
    pub chain_frame: i8,
    /// `private boolean hasHit;` (`i.e`) — true once this segment has already applied
    /// damage.
    pub has_hit: bool,
    /// `private int damage;` (`i.a`, collision) — damage payload (hero-fired shots).
    pub damage: i32,
    /// `private byte statusKind;` (`i.i`) — status effect inflicted on hit
    /// (hero-fired shots).
    pub status_kind: i8,
    /// `private boolean crit;` (`i.f`, collision) — whether this is a critical hit
    /// (hero-fired shots).
    pub crit: bool,
}

/// Allocates a fully-built projectile node at the tile centre and returns its
/// [`EntityId`] (the shared `super((short)(tileX<<4),(short)(tileY<<4),(byte)8,
/// (byte)9)` positioning both constructors run via their `super(...)` chain).
fn alloc_projectile(
    arena: &mut EntityArena,
    tile_x: i8,
    tile_y: i8,
    proj: ProjectileData,
) -> EntityId {
    // super((short) (tileX << 4), (short) (tileY << 4), (byte) 8, (byte) 9);
    let pixel_x = ishl(tile_x as i32, 4) as i16;
    let pixel_y = ishl(tile_y as i32, 4) as i16;
    let mut node = EntityNode {
        data: EntityData::Projectile(Box::new(proj)),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 9);
    arena.alloc(node)
}

/// `public Projectile(byte tileX, byte tileY, byte[] spriteScript, Battler owner,
/// byte dir, byte range, byte chainFrame)` (`i.<init>:(BB[BLo;BBB)V => [isub,i2b]`) —
/// the enemy-fired shot: no damage payload.
#[allow(clippy::too_many_arguments)]
pub fn new_projectile_enemy(
    arena: &mut EntityArena,
    tile_x: i8,
    tile_y: i8,
    sprite_script: Vec<i8>,
    owner: EntityId,
    dir: i8,
    range: i8,
    chain_frame: i8,
) -> EntityId {
    // super(tileX, tileY, spriteScript);
    let effect = effect_data_from_script(sprite_script);
    let proj = ProjectileData {
        effect,
        // this.owner = owner;
        owner,
        // this.piercing = false;
        piercing: false,
        // this.dir = dir;
        dir,
        // this.range = (byte) (range - 1);
        range: (range as i32).wrapping_sub(1) as i8,
        // this.chainFrame = chainFrame;
        chain_frame,
        // (damage/statusKind/crit/hasHit at JVM defaults.)
        has_hit: false,
        damage: 0,
        status_kind: 0,
        crit: false,
    };
    alloc_projectile(arena, tile_x, tile_y, proj)
}

/// `public Projectile(byte tileX, byte tileY, byte[] spriteScript, Battler owner,
/// boolean piercing, byte dir, byte range, byte chainFrame, int damage,
/// byte statusKind, boolean crit)` (`i.<init>:(BB[BLo;ZBBBIBZ)V => [isub,i2b]`) — the
/// hero-fired shot, carrying the damage payload.
#[allow(clippy::too_many_arguments)]
pub fn new_projectile_hero(
    arena: &mut EntityArena,
    tile_x: i8,
    tile_y: i8,
    sprite_script: Vec<i8>,
    owner: EntityId,
    piercing: bool,
    dir: i8,
    range: i8,
    chain_frame: i8,
    damage: i32,
    status_kind: i8,
    crit: bool,
) -> EntityId {
    // super(tileX, tileY, spriteScript);
    let effect = effect_data_from_script(sprite_script);
    let proj = ProjectileData {
        effect,
        // this.owner = owner; this.piercing = piercing; this.dir = dir;
        owner,
        piercing,
        dir,
        // this.range = (byte) (range - 1);
        range: (range as i32).wrapping_sub(1) as i8,
        // this.chainFrame = chainFrame; this.damage = damage; this.statusKind = statusKind;
        // this.crit = crit;
        chain_frame,
        has_hit: false,
        damage,
        status_kind,
        crit,
    };
    alloc_projectile(arena, tile_x, tile_y, proj)
}

/// `public final void onFrame()` (`i.a:()V => [iadd,i2b,iadd,i2b]`) — chains the next
/// travel segment and resolves a landing hit. Overrides the base no-op
/// [`crate::effect::on_frame`].
pub fn on_frame(g: &mut Game, id: EntityId) {
    // if (((Effect) this).frame == this.chainFrame && this.range > 0 && (this.piercing || !this.hasHit)) {
    let frame = g.entity_arena[id].as_effect().expect("Effect base").frame;
    let (chain_frame, range, piercing, has_hit, dir) = {
        let p = g.entity_arena[id].as_projectile().expect("Projectile");
        (p.chain_frame, p.range, p.piercing, p.has_hit, p.dir)
    };
    if frame as i32 == chain_frame as i32 && range > 0 && (piercing || !has_hit) {
        // GameMap map = GameState.map;
        // byte nextX = (byte) (tileX + Directions.dirDx[dir]);
        // byte nextY = (byte) (tileY + Directions.dirDy[dir]);
        let tile_x = g.entity_arena[id].tile_x as i32;
        let tile_y = g.entity_arena[id].tile_y as i32;
        let next_x = tile_x.wrapping_add(DIR_DX[dir as usize] as i32) as i8;
        let next_y = tile_y.wrapping_add(DIR_DY[dir as usize] as i32) as i8;
        // if (nextX >= 0 && nextX < map.widthTiles && nextY >= 0 && nextY < map.heightTiles) {
        let (width_tiles, height_tiles) = {
            let m = g
                .game_state
                .map
                .as_ref()
                .expect("GameState.map null in Projectile.onFrame");
            (m.width_tiles, m.height_tiles)
        };
        if next_x as i32 >= 0
            && (next_x as i32) < width_tiles
            && next_y as i32 >= 0
            && (next_y as i32) < height_tiles
        {
            let owner = g.entity_arena[id]
                .as_projectile()
                .expect("Projectile")
                .owner;
            let owner_is_hero = g.entity_arena[owner].as_hero().is_some();
            // if (this.owner instanceof Enemy) {
            //   map.addEntity(new Projectile(nextX, nextY, super.spriteScript, this.owner,
            //     this.dir, this.range, this.chainFrame));
            // }  DEFERRED: Enemy (owner) — no Enemy variant; enemy-fired bolts do not spawn here.
            // else if (this.owner instanceof Hero) {
            if owner_is_hero {
                let sprite_script = g.entity_arena[id]
                    .as_effect()
                    .expect("Effect base")
                    .sprite_script
                    .clone()
                    .expect("super.spriteScript null in Projectile.onFrame");
                let (piercing2, dir2, range2, chain2, damage2, status2, crit2) = {
                    let p = g.entity_arena[id].as_projectile().expect("Projectile");
                    (
                        p.piercing,
                        p.dir,
                        p.range,
                        p.chain_frame,
                        p.damage,
                        p.status_kind,
                        p.crit,
                    )
                };
                //   map.addEntity(new Projectile(nextX, nextY, super.spriteScript, this.owner,
                //     this.piercing, this.dir, this.range, this.chainFrame, this.damage,
                //     this.statusKind, this.crit));
                let new_id = new_projectile_hero(
                    &mut g.entity_arena,
                    next_x,
                    next_y,
                    sprite_script,
                    owner,
                    piercing2,
                    dir2,
                    range2,
                    chain2,
                    damage2,
                    status2,
                    crit2,
                );
                game_map::add_entity(g, new_id);
            }
        }
    }
    // if ((this.piercing || !this.hasHit) && ((Effect) this).frame == 1) {
    //   Entity cell = GameState.map.occupancy[tileY][tileX];
    //   if (this.owner instanceof Enemy) {
    //     if (cell == null || !(cell instanceof Hero)) return;
    //     ((Hero) cell).takeHit((Enemy) this.owner, this.dir); this.hasHit = true; return;
    //   }
    //   if ((this.owner instanceof Hero) && cell != null && (cell instanceof Enemy)) {
    //     ((Enemy) cell).takeHeroHit(this.damage, false, this.dir, this.crit, (byte) 1,
    //       this.statusKind, (Hero) this.owner);
    //     this.hasHit = true;
    //   }
    // }
    //   DEFERRED: the hit-resolution reaches Hero.takeHit (the DEFERRED combat FSM)
    //   and Enemy.takeHeroHit (Enemy unported). No Enemy nodes exist in this slice, so
    //   neither branch can fire (an enemy owner never exists; `cell instanceof Enemy`
    //   is never true), and hasHit stays false — deferring the block is observably
    //   identical.
}

/// `public final boolean isFinished()` (`i.a:()Z => []`) — overrides
/// [`crate::effect::base_is_finished`]: finished at the frame count, or early for a
/// spent 2-frame non-piercing bolt that has already hit.
pub fn is_finished(g: &Game, id: EntityId) -> bool {
    let frame = g.entity_arena[id].as_effect().expect("Effect base").frame;
    let frame_count = g.entity_arena[id]
        .as_effect()
        .expect("Effect base")
        .frame_count;
    let (piercing, has_hit) = {
        let p = g.entity_arena[id].as_projectile().expect("Projectile");
        (p.piercing, p.has_hit)
    };
    // if (((Effect) this).frame != ((Effect) this).frameCount) {
    //   return ((Effect) this).frameCount == 2 && !this.piercing && this.hasHit
    //          && ((Effect) this).frame >= 1;
    // }
    if frame != frame_count {
        return frame_count == 2 && !piercing && has_hit && frame >= 1;
    }
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`i.a:(…Graphics;II)V`) — draws the projectile sprite (nudged forward mid-flight
/// for a 2-frame bolt) and advances its frame. Overrides [`crate::effect::paint`].
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
    let (frame, frame_count) = {
        let e = g.entity_arena[id].as_effect().expect("Effect base");
        (e.frame, e.frame_count)
    };
    let dir = g.entity_arena[id].as_projectile().expect("Projectile").dir;
    // if (((Effect) this).frameCount == 2 && ((Effect) this).frame == 1) {
    //   screenX += Directions.dirDx[dir] * 8; screenY += Directions.dirDy[dir] * 8;
    // }
    if frame_count == 2 && frame == 1 {
        screen_x = screen_x.wrapping_add((DIR_DX[dir as usize] as i32).wrapping_mul(8));
        screen_y = screen_y.wrapping_add((DIR_DY[dir as usize] as i32).wrapping_mul(8));
    }
    // drawSprite(graphics, screenX, screenY);   — inherited Effect.drawSprite.
    effect::draw_sprite(g, id, screen_x, screen_y);
    // ((Effect) this).frame = (short) (((Effect) this).frame + 1);
    let e = g.entity_arena[id].as_effect_mut().expect("Effect base");
    e.frame = (frame as i32).wrapping_add(1) as i16;
}
