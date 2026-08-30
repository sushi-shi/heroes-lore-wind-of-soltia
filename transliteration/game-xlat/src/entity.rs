//! Transliterated from `java/src/main/java/defpackage/Entity.java`
//! (original `ck.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Root of the on-map object hierarchy: `Battler`/`Hero`/`Enemy`, `MapObject`,
//! `Effect`/`Projectile`, `Guardian` all `extend Entity`. It holds a pixel
//! position, the derived tile coordinate, a collision half-size, a draw layer, and
//! the intrusive doubly-linked-list pointers used by [`crate::entity_list`]. Tiles
//! are 16 pixels (`pixel >> 4` yields the tile index).
//!
//! ## The shared-heap seam ([`EntityArena`])
//!
//! Java references entity objects from several places at once (the per-map
//! [`crate::entity_list::EntityListState`], the map occupancy grid, and
//! `GameState.hero`). The transliteration models the Java heap as one
//! [`EntityArena`] slab of [`EntityNode`] records addressed by an [`EntityId`]
//! index; a Java reference is that index, and `a.equals(b)` (Object identity —
//! `Entity` overrides nothing) becomes index equality. Java single-inheritance is
//! modelled by composition: [`EntityNode`] carries the `Entity` **base** fields
//! plus an [`EntityData`] tagged union of the concrete subclass data, and each
//! `Battler` subclass embeds a [`crate::battler::BattlerData`] as its "super". This
//! is the arena shape [`crate::entity_list`] anticipated; that module keeps the
//! endpoint/relocation logic and re-exports these types.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `ck.<init>:(SSBB)V => []` (the ctor body is pure assignment + calls),
//! `ck.a:(SS)V (setPixelPos) => []`,
//! `ck.b:()V (syncTile) => ["ishr","i2b","ishr","i2b","iand","iand"]`
//! (`pixelY >> 4`, `pixelX >> 4`, `pixelY & 15`, `pixelX & 15`).

use crate::byte_util::JavaRandom;
use crate::effect::EffectData;
use crate::hero::HeroData;
use crate::map_object::MapObjectData;
use crate::npc::NpcData;
use crate::projectile::ProjectileData;
use j2me_jvm::ishr;

/// A node in the [`EntityArena`] — the slab index that stands in for an `Entity`
/// (`ck`) reference. Identity is the index (`a.equals(b)` → `a == b`).
pub type EntityId = usize;

/// Which concrete `Entity` subclass a node is — the discriminant of the flattened
/// hierarchy, used where the Java did `instanceof`. Computed from [`EntityData`];
/// `Bare` is the abstract `Entity` itself (never live — used only by the
/// EntityList oracle's synthetic test nodes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// Abstract `Entity` (`ck`) — no concrete subclass (test node).
    Bare,
    /// `MapObject` (`aj`) — a static placed-image prop.
    MapObject,
    /// `Hero` (`ao`) — the player character (a `Battler`).
    Hero,
    /// `Npc` (`ac`) — a town/quest actor (a `Battler`).
    Npc,
    /// `Effect` (`y`) — a transient animated visual effect.
    Effect,
    /// `Projectile` (`i`) — a moving ranged-attack effect (an `Effect`).
    Projectile,
}

/// The per-subclass data of a node — the flattened tagged union standing in for the
/// concrete runtime type. `Battler` subclasses embed a
/// [`crate::battler::BattlerData`] (their "super"); only `Hero` is modelled in this
/// slice (Npc `ac`, Enemy `al` and the boss leaves are DEFERRED for a later lane).
#[derive(Debug, Default)]
pub enum EntityData {
    /// Abstract `Entity` with no concrete subclass — the bare node the EntityList
    /// oracle allocates (`Entity` is abstract, never instantiated on a live path).
    #[default]
    Bare,
    /// `MapObject` (`aj`) instance data.
    MapObject(MapObjectData),
    /// `Hero` (`ao`) instance data (embeds the `Battler` base). Boxed: a Java `Hero`
    /// is itself a heap object, and boxing keeps the (large) leaf off the shared
    /// [`EntityNode`] size.
    Hero(Box<HeroData>),
    /// `Npc` (`ac`) instance data (embeds the `Battler` base). Boxed like `Hero`.
    Npc(Box<NpcData>),
    /// `Effect` (`y`) instance data. Boxed: the effect carries an (unloaded here)
    /// image bank + sprite script, kept off the shared [`EntityNode`] size.
    Effect(Box<EffectData>),
    /// `Projectile` (`i`) instance data (embeds the `Effect` base). Boxed like `Hero`.
    Projectile(Box<ProjectileData>),
}

/// The `Entity` (`ck`) base record: the fields every subclass inherits, plus the
/// arena links and the [`EntityData`] tagged union of subclass data. `next`/`prev`/
/// `half_h`/`pixel_y`/`removed` keep their names because [`crate::entity_list`]'s
/// list logic reads them directly.
#[derive(Debug)]
pub struct EntityNode {
    /// `public byte tileX;` — tile column (`pixelX >> 4`).
    pub tile_x: i8,
    /// `public byte tileY;` — tile row (`pixelY >> 4`).
    pub tile_y: i8,
    /// `public boolean offGridX;` (`ck.a`) — mid-tile horizontally (low 4 px bits set).
    pub off_grid_x: bool,
    /// `public boolean offGridY;` (`ck.b`) — mid-tile vertically.
    pub off_grid_y: bool,
    /// `public short pixelX;` — pixel X of the reference point.
    pub pixel_x: i16,
    /// `public short pixelY;` — pixel Y (also the depth term with `halfH`).
    pub pixel_y: i16,
    /// `public byte halfW;` (`ck.c`) — collision box half-width, px.
    pub half_w: i8,
    /// `public byte halfH;` (`ck.d`) — collision box half-height (depth term).
    pub half_h: i8,
    /// `public Entity next;` (`ck.a`) — next node in the owning `EntityList`.
    pub next: Option<EntityId>,
    /// `public Entity prev;` (`ck.b`) — previous node.
    pub prev: Option<EntityId>,
    /// `public byte layer = 1;` — draw/sort layer (defaults to 1).
    pub layer: i8,
    /// `public boolean removed = false;` (`ck.c`) — set once unlinked from its list.
    pub removed: bool,
    /// The concrete subclass data (flattened tagged union).
    pub data: EntityData,
}

impl Default for EntityNode {
    /// A node at the JVM field defaults, with the `Entity` field initializer
    /// `layer = 1` applied and no concrete subclass ([`EntityData::Bare`]).
    fn default() -> Self {
        EntityNode {
            tile_x: 0,
            tile_y: 0,
            off_grid_x: false,
            off_grid_y: false,
            pixel_x: 0,
            pixel_y: 0,
            half_w: 0,
            half_h: 0,
            next: None,
            prev: None,
            layer: 1,
            removed: false,
            data: EntityData::Bare,
        }
    }
}

impl EntityNode {
    /// The node's concrete [`EntityKind`] (the `instanceof` discriminant).
    pub fn kind(&self) -> EntityKind {
        match self.data {
            EntityData::Bare => EntityKind::Bare,
            EntityData::MapObject(_) => EntityKind::MapObject,
            EntityData::Hero(_) => EntityKind::Hero,
            EntityData::Npc(_) => EntityKind::Npc,
            EntityData::Effect(_) => EntityKind::Effect,
            EntityData::Projectile(_) => EntityKind::Projectile,
        }
    }

    /// `this instanceof Hero` → the borrowed [`HeroData`], or `None`.
    pub fn as_hero(&self) -> Option<&HeroData> {
        match &self.data {
            EntityData::Hero(h) => Some(h.as_ref()),
            _ => None,
        }
    }

    /// Mutable [`Self::as_hero`].
    pub fn as_hero_mut(&mut self) -> Option<&mut HeroData> {
        match &mut self.data {
            EntityData::Hero(h) => Some(h.as_mut()),
            _ => None,
        }
    }

    /// `this instanceof Battler` → the borrowed embedded [`crate::battler::BattlerData`]
    /// ("super"), or `None`. The uniform accessor for `Battler`'s generic methods
    /// (`move`/`setState`/`setFacing`/`stepIfMoving`), which operate on any concrete
    /// combatant. Only `Hero` is a live `Battler` in this slice; Enemy/Npc are added
    /// when those subclasses land.
    pub fn as_battler(&self) -> Option<&crate::battler::BattlerData> {
        match &self.data {
            EntityData::Hero(h) => Some(&h.battler),
            EntityData::Npc(n) => Some(&n.battler),
            _ => None,
        }
    }

    /// Mutable [`Self::as_battler`].
    pub fn as_battler_mut(&mut self) -> Option<&mut crate::battler::BattlerData> {
        match &mut self.data {
            EntityData::Hero(h) => Some(&mut h.battler),
            EntityData::Npc(n) => Some(&mut n.battler),
            _ => None,
        }
    }

    /// `this instanceof MapObject` → the borrowed [`MapObjectData`], or `None`.
    pub fn as_map_object(&self) -> Option<&MapObjectData> {
        match &self.data {
            EntityData::MapObject(m) => Some(m),
            _ => None,
        }
    }

    /// `this instanceof Npc` → the borrowed [`NpcData`], or `None`.
    pub fn as_npc(&self) -> Option<&NpcData> {
        match &self.data {
            EntityData::Npc(n) => Some(n.as_ref()),
            _ => None,
        }
    }

    /// Mutable [`Self::as_npc`].
    pub fn as_npc_mut(&mut self) -> Option<&mut NpcData> {
        match &mut self.data {
            EntityData::Npc(n) => Some(n.as_mut()),
            _ => None,
        }
    }

    /// `this instanceof Effect` → the borrowed `Effect` **base** [`EffectData`], or
    /// `None`. Returns the base for both an `Effect` and a `Projectile` node (the
    /// `instanceof Effect` accessor, mirroring [`Self::as_battler`]); a Java
    /// `((Effect) this).frame` / `super.spriteScript` on a `Projectile` routes here.
    pub fn as_effect(&self) -> Option<&EffectData> {
        match &self.data {
            EntityData::Effect(e) => Some(e.as_ref()),
            EntityData::Projectile(p) => Some(&p.effect),
            _ => None,
        }
    }

    /// Mutable [`Self::as_effect`].
    pub fn as_effect_mut(&mut self) -> Option<&mut EffectData> {
        match &mut self.data {
            EntityData::Effect(e) => Some(e.as_mut()),
            EntityData::Projectile(p) => Some(&mut p.effect),
            _ => None,
        }
    }

    /// `this instanceof Projectile` → the borrowed [`ProjectileData`], or `None`.
    pub fn as_projectile(&self) -> Option<&ProjectileData> {
        match &self.data {
            EntityData::Projectile(p) => Some(p.as_ref()),
            _ => None,
        }
    }

    /// Mutable [`Self::as_projectile`].
    pub fn as_projectile_mut(&mut self) -> Option<&mut ProjectileData> {
        match &mut self.data {
            EntityData::Projectile(p) => Some(p.as_mut()),
            _ => None,
        }
    }
}

/// The shared `Entity` heap seam: a slab of nodes addressed by [`EntityId`].
#[derive(Debug, Default)]
pub struct EntityArena {
    nodes: Vec<EntityNode>,
}

impl EntityArena {
    /// A fresh, empty arena.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Allocates a bare node with the given depth terms and returns its
    /// [`EntityId`] — the abstract `Entity` used by the EntityList oracle. Concrete
    /// entities are allocated by [`Self::alloc`] via the per-subclass constructors.
    pub fn spawn(&mut self, half_h: i8, pixel_y: i16) -> EntityId {
        self.alloc(EntityNode {
            half_h,
            pixel_y,
            ..EntityNode::default()
        })
    }

    /// Allocates a fully-built node (a subclass constructor's product) and returns
    /// its [`EntityId`] (like `new Hero(...)` handing back a reference).
    pub fn alloc(&mut self, node: EntityNode) -> EntityId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Read access to a node (`arena.node(id).next`, etc.).
    pub fn node(&self, id: EntityId) -> &EntityNode {
        &self.nodes[id]
    }

    /// Mutable access to a node.
    pub fn node_mut(&mut self, id: EntityId) -> &mut EntityNode {
        &mut self.nodes[id]
    }
}

impl std::ops::Index<EntityId> for EntityArena {
    type Output = EntityNode;
    fn index(&self, id: EntityId) -> &EntityNode {
        &self.nodes[id]
    }
}

impl std::ops::IndexMut<EntityId> for EntityArena {
    fn index_mut(&mut self, id: EntityId) -> &mut EntityNode {
        &mut self.nodes[id]
    }
}

/// Java `Entity` (`ck`) class state — its one `static` field
/// (`java/reconstruction/ownership.tsv`).
#[derive(Debug)]
pub struct EntityState {
    /// `public static Random rng = new Random();` (`ck.a`). Time-seeded on device;
    /// a fixed seed here for reproducibility (a determinism seam, like
    /// [`crate::byte_util::ByteUtilState`]) — read only by DEFERRED combat rolls
    /// (`Hero.rollDamage`/`rollCrit`, `Battler.approach`), not exercised in this
    /// slice.
    pub rng: JavaRandom,
}

impl Default for EntityState {
    fn default() -> Self {
        // ck.<clinit>: rng = new Random();
        EntityState {
            rng: JavaRandom::new(0),
        }
    }
}

/// `public void setPixelPos(short pixelX, short pixelY)` (`ck.a:(SS)V => []`).
/// Moves the entity to an absolute pixel position (the tile is not re-derived).
pub fn set_pixel_pos(node: &mut EntityNode, pixel_x: i16, pixel_y: i16) {
    // this.pixelX = pixelX; this.pixelY = pixelY;
    node.pixel_x = pixel_x;
    node.pixel_y = pixel_y;
}

/// `public final void syncTile()` (`ck.b:()V => [ishr,i2b,ishr,i2b,iand,iand]`).
/// Recomputes `tileX`/`tileY` and the off-grid flags from the pixel position.
/// `pixelX`/`pixelY` are `short`; each `>> 4` / `& 15` sign-extends to `int` first.
pub fn sync_tile(node: &mut EntityNode) {
    // this.tileY = (byte) (this.pixelY >> 4);
    node.tile_y = ishr(node.pixel_y as i32, 4) as i8;
    // this.tileX = (byte) (this.pixelX >> 4);
    node.tile_x = ishr(node.pixel_x as i32, 4) as i8;
    // this.offGridY = (this.pixelY & 15) != 0;
    node.off_grid_y = (node.pixel_y as i32 & 15) != 0;
    // this.offGridX = (this.pixelX & 15) != 0;
    node.off_grid_x = (node.pixel_x as i32 & 15) != 0;
}

/// `public Entity(short pixelX, short pixelY, byte halfWidth, byte halfHeight)`
/// (`ck.<init>:(SSBB)V => []`), reproduced onto a freshly-built [`EntityNode`]
/// carrying `data`. The `Entity` field initializer `layer = 1` is already applied
/// by [`EntityNode::default`], from which the subclass constructors build.
pub fn init_base(node: &mut EntityNode, pixel_x: i16, pixel_y: i16, half_w: i8, half_h: i8) {
    // setPixelPos(pixelX, pixelY);
    set_pixel_pos(node, pixel_x, pixel_y);
    // syncTile();
    sync_tile(node);
    // this.halfW = halfWidth; this.halfH = halfHeight;
    node.half_w = half_w;
    node.half_h = half_h;
}
