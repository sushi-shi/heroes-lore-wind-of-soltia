//! Transliterated from `java/src/main/java/defpackage/Npc.java`
//! (original `ac.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A town / quest NPC actor (a [`crate::battler::BattlerData`] that never fights).
//! Kinds `< 18` are animated character sprites (walk/talk frame banks); kinds
//! `>= 18` draw a single static object image. NPCs move free-form in their facing
//! direction and never auto-path, and only register in the occupancy grid while
//! idle. `Npc extends Battler` (a leaf); it overrides `setPixelPos`, `move`,
//! `tryStepForward` and `paint`.
//!
//! ## Flattened subclass ([`NpcData`])
//!
//! Modelled as [`crate::hero`] flattens `Hero`: an [`NpcData`] embeds a
//! [`BattlerData`] as its "super" and is boxed in [`crate::entity::EntityData::Npc`].
//! `Battler` generic access is [`crate::entity::EntityNode::as_battler`] (now
//! matching an `Npc` node too); the concrete access is
//! [`crate::entity::EntityNode::as_npc`].
//!
//! ## DEFERRED sprite draw
//!
//! [`paint`] draws the ground shadow (`AssetCache.entityShadow`, ported) and the
//! (empty here) floaters, but the NPC sprite itself is DEFERRED: the `kind >= 18`
//! object image reaches the unported `AssetCache.mapNpcImages`, and the `kind < 18`
//! character branch reaches `AssetCache.npcFrames` / `npcAnimFrames0` /
//! `npcAnimFrames1` (unported banks) via `GameScreen.drawFrameGroup`. The visible
//! check, the on-screen cull, the shadow blit and the floater draw — the observable,
//! bank-independent parts — are ported.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ac.<init>:(SSBB)V => []`,
//! `ac.a:(SS)V (setPixelPos) => []`, `ac.a:(I)V (move) =>
//! ["imul","iadd","i2s","imul","iadd","i2s"]` (`pixelX + stepPixels * dirDx[facing]`,
//! `pixelY + stepPixels * dirDy[facing]`, each narrowed to short),
//! `ac.a:()Z (tryStepForward) => []` (the `? false : false` ternary),
//! `ac.a:(…Graphics;II)V (paint)` — the shadow/cull/floater arithmetic plus the
//! DEFERRED sprite branch.

use crate::battler::{self, BattlerData, STATE_IDLE, STATE_KNOCKBACK, STATE_STEPPING};
use crate::directions::{DIR_DX, DIR_DY};
use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game::Game;

/// The `Npc` (`ac`) instance data — the subclass fields beyond the embedded
/// [`BattlerData`] "super". Boxed in [`EntityData::Npc`].
#[derive(Debug)]
pub struct NpcData {
    /// The `Battler` "super" (`super(pixelX, pixelY, (byte) 8, (byte) 8)`).
    pub battler: BattlerData,
    /// `public byte kind;` (`ac.f`) — NPC kind; `>= 18` selects a static object image
    /// instead of a character.
    pub kind: i8,
    /// `public byte spriteSet;` (`ac.g`) — sprite/animation-bank index for animated
    /// NPCs.
    pub sprite_set: i8,
    /// `public boolean visible;` (`ac.d`) — whether this NPC is currently drawn.
    pub visible: bool,
}

/// `public Npc(short pixelX, short pixelY, byte kind, byte spriteSet)`
/// (`ac.<init>:(SSBB)V => []`). Allocates the NPC node in `arena` and returns its
/// [`EntityId`].
pub fn new_npc(
    arena: &mut EntityArena,
    pixel_x: i16,
    pixel_y: i16,
    kind: i8,
    sprite_set: i8,
) -> EntityId {
    let npc = NpcData {
        // super(pixelX, pixelY, (byte) 8, (byte) 8);  → Battler init.
        battler: BattlerData::new(),
        // this.kind = kind;
        kind,
        // this.visible = true;
        visible: true,
        // this.spriteSet = spriteSet;
        sprite_set,
    };
    let mut node = EntityNode {
        data: EntityData::Npc(Box::new(npc)),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, 8, 8);
    arena.alloc(node)
}

/// `public final void setPixelPos(short pixelX, short pixelY)` (`ac.a:(SS)V => []`) —
/// overrides `Entity.setPixelPos`: clear the footprint, move, re-derive the tile, and
/// re-register occupancy only while idle.
pub fn set_pixel_pos(g: &mut Game, id: EntityId, pixel_x: i16, pixel_y: i16) {
    // clearOccupancy();
    battler::clear_occupancy(g, id);
    // super.setPixelPos(pixelX, pixelY);
    entity::set_pixel_pos(&mut g.entity_arena[id], pixel_x, pixel_y);
    // syncTile();
    entity::sync_tile(&mut g.entity_arena[id]);
    // if (this.state == 1) setOccupancy();
    let state = g.entity_arena[id].as_battler().expect("Npc battler").state;
    if state == STATE_IDLE {
        battler::set_occupancy(g, id);
    }
}

/// `public final void move(int stepPixels)`
/// (`ac.a:(I)V => [imul,iadd,i2s,imul,iadd,i2s]`) — overrides `Battler.move`: a
/// free-form facing-direction slide (no off-grid half-tile phase), re-registering
/// occupancy only while idle.
pub fn move_(g: &mut Game, id: EntityId, step_pixels: i32) {
    // clearOccupancy();
    battler::clear_occupancy(g, id);
    // (fields read off the Battler base up front.)
    let facing = g.entity_arena[id].as_battler().expect("Npc battler").facing;
    {
        let node = &mut g.entity_arena[id];
        // this.pixelX = (short) (this.pixelX + (stepPixels * Directions.dirDx[this.facing]));
        node.pixel_x = (node.pixel_x as i32)
            .wrapping_add(step_pixels.wrapping_mul(DIR_DX[facing as usize] as i32))
            as i16;
        // this.pixelY = (short) (this.pixelY + (stepPixels * Directions.dirDy[this.facing]));
        node.pixel_y = (node.pixel_y as i32)
            .wrapping_add(step_pixels.wrapping_mul(DIR_DY[facing as usize] as i32))
            as i16;
        // syncTile();
        entity::sync_tile(node);
    }
    // if (this.state == 1) setOccupancy();
    let state = g.entity_arena[id].as_battler().expect("Npc battler").state;
    if state == STATE_IDLE {
        battler::set_occupancy(g, id);
    }
}

/// `public final boolean tryStepForward()` (`ac.a:()Z => []`) — overrides
/// `Battler.tryStepForward`: `return (offGridX || offGridY) ? false : false;` (both
/// ternary arms are `false`, so NPCs never halt/re-align here).
pub fn try_step_forward(_g: &mut Game, _id: EntityId) -> bool {
    // return (offGridX || offGridY) ? false : false;   (the condition is side-effect
    //   free and both arms are false — always false.)
    false
}

/// `public final void update()` — inherited `Battler.update()` (`Npc` does not
/// override it): `stepIfMoving()`, which — while stepping (state 2/4) — runs `Npc`'s
/// overridden [`try_step_forward`] then [`move_`]. Inlined here so the virtual
/// `tryStepForward`/`move` dispatch reaches the `Npc` overrides without touching the
/// read-only `Battler` FSM.
pub fn update(g: &mut Game, id: EntityId) {
    // Battler.update(): stepIfMoving();
    // Battler.stepIfMoving(): if (state == 2 || state == 4) tryStepForward();
    let state = g.entity_arena[id].as_battler().expect("Npc battler").state;
    if state == STATE_STEPPING || state == STATE_KNOCKBACK {
        try_step_forward(g, id);
    }
    //   if (state == 2 || state == 4) move(8);
    let state = g.entity_arena[id].as_battler().expect("Npc battler").state;
    if state == STATE_STEPPING || state == STATE_KNOCKBACK {
        move_(g, id, 8);
    }
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ac.a:(…Graphics;II)V`) — overrides `Entity.paint`: when visible and on-screen,
/// blits the ground shadow, the NPC sprite (DEFERRED), then the floaters.
pub fn paint(g: &mut Game, id: EntityId, origin_x: i32, origin_y: i32) {
    // if (this.visible) {
    let visible = g.entity_arena[id].as_npc().expect("Npc").visible;
    if !visible {
        return;
    }
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
    let width = g.game_screen.width;
    let world_height = g.game_screen.world_height;
    // if (screenX + 16 < 0 || screenY < 0 || screenX - 16 > GameScreen.width
    //     || screenY > GameScreen.worldHeight + 32) return;
    if screen_x.wrapping_add(16) < 0
        || screen_y < 0
        || screen_x.wrapping_sub(16) > width
        || screen_y > world_height.wrapping_add(32)
    {
        return;
    }
    // (kind read for the DEFERRED sprite branch below.)
    let kind = g.entity_arena[id].as_npc().expect("Npc").kind;
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
    // graphics.drawImage(AssetCache.entityShadow, screenX, screenY - 3, 17);
    let shadow = asset_cache
        .entity_shadow
        .as_ref()
        .expect("NullPointerException: entityShadow null");
    graphics
        .draw_image(shadow, screen_x, screen_y.wrapping_sub(3), 17)
        .expect("drawImage(entityShadow)");
    // if (this.kind >= 18) {
    //   graphics.drawImage(AssetCache.mapNpcImages[this.kind - 18], screenX, screenY, 33);
    // } else {
    //   GameScreen.drawFrameGroup(graphics, (byte[]) AssetCache.npcFrames[
    //     this.state == 2 ? (spriteSet * 12) + 4 + (moveDir - 1) : (spriteSet * 12) + (moveDir - 1)],
    //     this.animFrame, screenX, screenY);
    //   this.animFrame = (byte) (this.animFrame + 1);
    //   if (this.state == 1 && AssetCache.npcAnimFrames0[this.spriteSet] <= this.animFrame)
    //     this.animFrame = (byte) 0;
    //   else if (this.state == 2 && AssetCache.npcAnimFrames1[this.spriteSet] <= this.animFrame)
    //     this.animFrame = (byte) 0;
    // }
    //   DEFERRED: AssetCache.mapNpcImages (kind >= 18) / AssetCache.npcFrames +
    //   npcAnimFrames0 + npcAnimFrames1 (kind < 18) — unported sprite/anim banks. The
    //   animFrame advance/reset is gated on those bank reads, so it is DEFERRED as one
    //   unit.
    let _ = kind;
    // drawFloaters(graphics, screenX, screenY);
    let npc = entity_arena[id].as_npc_mut().expect("Npc");
    battler::draw_floaters(&mut npc.battler, &mut graphics, screen_x, screen_y);
}
