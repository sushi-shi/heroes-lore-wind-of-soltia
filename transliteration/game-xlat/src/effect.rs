//! Transliterated from `java/src/main/java/defpackage/Effect.java`
//! (original `y.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A transient animated visual effect entity (hit sparks, cast flashes, summon
//! puffs, explosions) — an [`crate::entity::EntityNode`] that advances one frame
//! each time it is painted and unlinks itself from the map once
//! [`is_finished`](base_is_finished) is reached. The effect [`EffectData::type_`]
//! selects both the lifetime (from [`FRAME_COUNTS`]) and the draw routine.
//! [`on_frame`] is the overridable per-frame hook (base no-op; [`crate::projectile`]
//! overrides it to travel and deal damage). `Effect extends Entity`.
//!
//! ## Flattened subclass ([`EffectData`])
//!
//! Modelled exactly as [`crate::hero`] flattens `Hero`: an [`EffectData`] lives in
//! [`crate::entity::EntityData::Effect`]; [`crate::projectile::ProjectileData`]
//! embeds an [`EffectData`] as its "super". A Java `((Effect) this)` field access on
//! a `Projectile` becomes [`crate::entity::EntityNode::as_effect`], which returns the
//! `Effect` base for both an `Effect` and a `Projectile` node (the `instanceof
//! Effect` accessor, mirroring [`crate::entity::EntityNode::as_battler`]).
//!
//! ## DEFERRED draws
//!
//! [`draw_sprite`] (types 1/6/8/9/100) blits `this.spriteScript` through the ported
//! [`crate::game_screen::draw_frame_group`], which no-ops a null script — so a
//! script-fed effect (type 100) draws for real, while the guardian types (1/6/…)
//! whose `spriteScript` comes from the DEFERRED `AssetCache.guardianFrames` no-op.
//! The other draw arms bind unported `AssetCache` guardian banks / `GameScreen`
//! helpers and are DEFERRED: `drawHitSpark` (2 → `AssetCache.guardianSpriteScript`),
//! `drawRisingCast` (4/5/7 → `GameScreen.clipToWorld` + the guardian `frameBank`),
//! `drawSummonPuff` (10 → the guardian `frameBank`). The per-frame lifetime advance
//! and the `unlinkEntity`/`removeEntity` bookkeeping — the observable state — are
//! fully ported.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `y.<clinit>:()V => []`,
//! `y.<init>:(SSB)V => []` (the type ctor — pure assignment + `AssetCache` reads),
//! `y.<init>:(BB[B)V => ["ishl","i2s","ishl","i2s","i2s"]` (`tileX<<4`/`tileY<<4`
//! narrowed to `short`, then `frameCount = spriteScript[0]` byte→short),
//! `y.a:()V (onFrame) => []`, `y.a:()Z (isFinished) => []`,
//! `y.a:(…Graphics;II)V (paint) => ["iadd"×5,"i2s"]` (`screenX`/`screenY` + the
//! `frame + 1`), `y.b:(…Graphics;II)V (drawSprite) => ["i2b"]` (`(byte) this.frame`).

use crate::asset_cache::AssetCacheState;
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityKind, EntityNode};
use crate::game::Game;
use crate::game_map;
use crate::game_screen;
use j2me_jvm::ishl;
use j2me_me::Image;

/// `private static final short[] FRAME_COUNTS` (`y.a:[S`). Lifetime (frame count)
/// per built-in effect type. A `static final` constant table reproduced as a `const`
/// (the `directions`/`Floater::DEFAULT_LIFETIME` precedent).
pub const FRAME_COUNTS: [i16; 11] = [-1, 4, 8, 6, 10, 11, 7, 9, 6, 4, 3];

/// `private static final byte[] TYPE_SPRITE_INDEX` (`y.i:[B`). Sprite-script index
/// (into `AssetCache.guardianFrames`) per type; `-1` = none. A `static final`
/// constant table reproduced as a `const`.
pub const TYPE_SPRITE_INDEX: [i8; 11] = [-1, 0, -1, -1, 0, 0, 1, 0, 1, 1, -1];

/// The `Effect` (`y`) instance data — the subclass fields beyond the [`EntityNode`]
/// base. Embedded in [`EntityData::Effect`] and, as the "super", in
/// [`crate::projectile::ProjectileData`].
#[derive(Debug)]
pub struct EffectData {
    /// `public short frameCount;` (`y.a`) — animation frames before the effect
    /// finishes.
    pub frame_count: i16,
    /// `public short frame;` (`y.b`) — current animation frame.
    pub frame: i16,
    /// `private byte type;` (`y.f`) — effect variant selecting lifetime + draw
    /// routine (`type` is a Rust keyword → `type_`).
    pub type_: i8,
    /// `private Image[] frameBank;` (`y.a`, collision) — guardian cast/summon image
    /// bank (`AssetCache.spriteBanks[12]`). `Image[]` (nullable) →
    /// `Option<Vec<Option<Image>>>`; the read is faithful but the bank is unloaded
    /// (None) on this slice's path.
    pub frame_bank: Option<Vec<Option<Image>>>,
    /// `public byte[] spriteScript;` (`y.h`) — sprite-cell script blitted for simple
    /// frame-based effects. `byte[]` (nullable) → `Option<Vec<i8>>`.
    pub sprite_script: Option<Vec<i8>>,
}

/// The shared body of the `Effect(byte tileX, byte tileY, byte[] spriteScript)`
/// constructor (`y.<init>:(BB[B)V`), reused as [`crate::projectile`]'s `super(...)`.
/// `frameCount = spriteScript[0]` (byte→short), `frame = 0`, `type = 100`,
/// `spriteScript = spriteScript`. (The `super(...)` positioning is applied by the
/// caller via [`entity::init_base`].)
pub(crate) fn effect_data_from_script(sprite_script: Vec<i8>) -> EffectData {
    // this.frameCount = spriteScript[0];   (i2s: the byte sign-extends to short)
    let frame_count = sprite_script[0] as i16;
    EffectData {
        frame_count,
        // this.frame = (short) 0;
        frame: 0,
        // this.type = (byte) 100;
        type_: 100,
        // (the script ctor leaves frameBank null.)
        frame_bank: None,
        // this.spriteScript = spriteScript;
        sprite_script: Some(sprite_script),
    }
}

/// `public Effect(short pixelX, short pixelY, byte type)` (`y.<init>:(SSB)V => []`).
/// Allocates a type-selected effect node in `arena` and returns its [`EntityId`].
///
/// **DEFERRED bank binding.** `frameBank = AssetCache.spriteBanks[12]` is read
/// faithfully (unloaded → `None` here); `spriteScript = AssetCache.guardianFrames[
/// TYPE_SPRITE_INDEX[type]]` reaches the unported `AssetCache.guardianFrames` bank
/// and is DEFERRED (leaves `spriteScript` `None`).
pub fn new_effect(
    arena: &mut EntityArena,
    asset_cache: &AssetCacheState,
    pixel_x: i16,
    pixel_y: i16,
    type_: i8,
) -> EntityId {
    let effect = EffectData {
        // this.frameCount = FRAME_COUNTS[type];
        frame_count: FRAME_COUNTS[type_ as usize],
        // this.frame = (short) 0;
        frame: 0,
        // this.type = type;
        type_,
        // this.frameBank = AssetCache.spriteBanks[12];   (unloaded here → None)
        frame_bank: asset_cache.sprite_banks[12].clone(),
        // if (TYPE_SPRITE_INDEX[type] != -1)
        //   this.spriteScript = (byte[]) AssetCache.guardianFrames[TYPE_SPRITE_INDEX[type]];
        //   DEFERRED: AssetCache.guardianFrames (unported guardian sprite-script bank);
        //   leaves spriteScript null.
        sprite_script: None,
    };
    // super(pixelX, pixelY, (byte) 8, (byte) 9);
    let mut node = EntityNode {
        data: EntityData::Effect(Box::new(effect)),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 9);
    arena.alloc(node)
}

/// `public Effect(byte tileX, byte tileY, byte[] spriteScript)`
/// (`y.<init>:(BB[B)V => [ishl,i2s,ishl,i2s,i2s]`). Allocates a script-fed effect
/// (`type = 100`) at the tile centre and returns its [`EntityId`].
pub fn new_effect_from_script(
    arena: &mut EntityArena,
    tile_x: i8,
    tile_y: i8,
    sprite_script: Vec<i8>,
) -> EntityId {
    // super((short) (tileX << 4), (short) (tileY << 4), (byte) 8, (byte) 9);
    let pixel_x = ishl(tile_x as i32, 4) as i16;
    let pixel_y = ishl(tile_y as i32, 4) as i16;
    let effect = effect_data_from_script(sprite_script);
    let mut node = EntityNode {
        data: EntityData::Effect(Box::new(effect)),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 9);
    arena.alloc(node)
}

/// `public boolean isFinished()` (`y.a:()Z => []`) — the base `Effect` predicate:
/// `frame >= frameCount`. [`crate::projectile::is_finished`] overrides it.
pub fn base_is_finished(effect: &EffectData) -> bool {
    // return this.frame >= this.frameCount;
    effect.frame >= effect.frame_count
}

/// The virtual `Effect.isFinished()` dispatch: a `Projectile` node routes to
/// [`crate::projectile::is_finished`]; any other `Effect` uses [`base_is_finished`].
pub fn is_finished(g: &Game, id: EntityId) -> bool {
    match g.entity_arena[id].kind() {
        EntityKind::Projectile => crate::projectile::is_finished(g, id),
        _ => base_is_finished(g.entity_arena[id].as_effect().expect("Effect base")),
    }
}

/// The virtual `Effect.onFrame()` dispatch (`y.a:()V => []` base no-op): a
/// `Projectile` node routes to [`crate::projectile::on_frame`]; the base `Effect`
/// does nothing.
pub fn on_frame(g: &mut Game, id: EntityId) {
    // Projectile overrides onFrame (travel + damage); base Effect.onFrame is a no-op
    // (`public void onFrame() {}`).
    if g.entity_arena[id].kind() == EntityKind::Projectile {
        crate::projectile::on_frame(g, id);
    }
}

/// `public void paint(Graphics graphics, int originX, int originY)`
/// (`y.a:(…Graphics;II)V => [iadd×5, i2s]`) — the `Effect` render: unlink from the
/// z-list, draw the type's routine, advance the frame, and reap the effect once
/// finished. (`Projectile` overrides this with [`crate::projectile::paint`].)
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // GameState.map.unlinkEntity(this);
    game_map::unlink_entity(g, id);
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
    let screen_x = origin_x.wrapping_add(pixel_x).wrapping_add(half_w);
    let screen_y = origin_y.wrapping_add(pixel_y).wrapping_add(half_h);
    // switch (this.type)
    let type_ = g.entity_arena[id].as_effect().expect("Effect base").type_;
    match type_ {
        // case 1: case 6: case 8: case 9: case 100: drawSprite(graphics, screenX, screenY);
        1 | 6 | 8 | 9 | 100 => draw_sprite(g, id, screen_x, screen_y),
        // case 2: drawHitSpark(graphics, screenX, screenY);
        //   DEFERRED: AssetCache.guardianSpriteScript (unported bank).
        2 => {}
        // case 4: drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[4], frameBank[8]);
        // case 5: drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[5], frameBank[11]);
        // case 7: drawRisingCast(graphics, screenX, screenY, FRAME_COUNTS[7], frameBank[11]);
        //   DEFERRED: GameScreen.clipToWorld (unported) + the guardian frameBank.
        4 | 5 | 7 => {}
        // case 10: drawSummonPuff(graphics, screenX, screenY);
        //   DEFERRED: the guardian frameBank (spriteBanks[12], unloaded here).
        10 => {}
        _ => {}
    }
    // this.frame = (short) (this.frame + 1);
    {
        let effect = g.entity_arena[id].as_effect_mut().expect("Effect base");
        effect.frame = (effect.frame as i32).wrapping_add(1) as i16;
    }
    // if (isFinished()) GameState.map.removeEntity(this);
    if is_finished(g, id) {
        game_map::remove_entity(g, id);
    }
}

/// `public final void drawSprite(Graphics graphics, int x, int y)`
/// (`y.b:(…Graphics;II)V => ["i2b"]`). Blits the current frame of the effect's
/// sprite script. A null `spriteScript` (the guardian types on this slice's path)
/// no-ops inside [`game_screen::draw_frame_group`]; a script-fed effect (type 100)
/// draws for real.
pub fn draw_sprite(g: &mut Game, id: EntityId, x: i32, y: i32) {
    // if (this.frame < 0 || this.frame >= this.frameCount) return;
    let (frame, frame_count) = {
        let e = g.entity_arena[id].as_effect().expect("Effect base");
        (e.frame, e.frame_count)
    };
    if frame < 0 || frame >= frame_count {
        return;
    }
    // (re-establish GameMap.paint's persistent world clip on this fresh Graphics.)
    let width = g.game_screen.width;
    let world_height = g.game_screen.world_height;
    let Game {
        screen,
        asset_cache,
        entity_arena,
        ..
    } = &mut *g;
    // GameScreen.drawFrameGroup(graphics, this.spriteScript, (byte) this.frame, x, y);
    let sprite_script = entity_arena[id]
        .as_effect()
        .expect("Effect base")
        .sprite_script
        .as_deref();
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);
    graphics.set_clip(0, 0, width, world_height);
    game_screen::draw_frame_group(&mut graphics, asset_cache, sprite_script, frame as i8, x, y);
}
