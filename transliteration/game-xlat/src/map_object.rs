//! Transliterated from `java/src/main/java/defpackage/MapObject.java`
//! (original `aj.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A static placed-image map decoration/prop — an [`crate::entity::EntityNode`]
//! with no AI or stats. It holds one [`MapObjectData::image`] and precomputed cull
//! bounds; its only behaviour is to draw itself at its world position when on
//! screen. `MapObject` has no `static` fields (no `ownership.tsv` rows).
//!
//! `paint` (`aj.a:(Graphics;II)V`) is DEFERRED to the render lane.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`):
//! `aj.<init>:(SSBBLjavax/microedition/lcdui/Image;)V =>
//! [ishr,ineg,i2s,ishr,iadd,i2s,iadd,i2s]` — `-(w>>1)`, `GameScreen.width + (w>>1)`,
//! `GameScreen.worldHeight + h`, each narrowed to `short`.

use crate::entity::{self, EntityArena, EntityData, EntityId, EntityNode};
use crate::game_screen::GameScreenState;
use j2me_jvm::ishr;
use j2me_me::Image;

/// `MapObject` (`aj`) instance data — the prop bitmap and its screen-cull bounds.
#[derive(Debug)]
pub struct MapObjectData {
    /// `public Image image;` (`aj.a`) — the prop bitmap (`None` == Java `null`).
    pub image: Option<Image>,
    /// `private short minX;` (`aj.a`) — left cull bound.
    pub min_x: i16,
    /// `private short maxX;` (`aj.b`) — right cull bound.
    pub max_x: i16,
    /// `private short maxY;` (`aj.e`) — bottom cull bound.
    pub max_y: i16,
}

/// `public MapObject(short pixelX, short pixelY, byte halfWidth, byte halfHeight,
/// Image image)` — `super(...)` then the image + cull bounds. Allocates the node in
/// `arena` and returns its [`EntityId`].
pub fn new_map_object(
    arena: &mut EntityArena,
    game_screen: &GameScreenState,
    pixel_x: i16,
    pixel_y: i16,
    half_width: i8,
    half_height: i8,
    image: Option<Image>,
) -> EntityId {
    // this.minX/maxX/maxY default to 0 (only assigned when image != null).
    let mut data = MapObjectData {
        image,
        min_x: 0,
        max_x: 0,
        max_y: 0,
    };
    // if (image != null) { ... }
    if let Some(img) = &data.image {
        // this.minX = (short) (-(image.getWidth() >> 1));
        data.min_x = ishr(img.width(), 1).wrapping_neg() as i16;
        // this.maxX = (short) (GameScreen.width + (image.getWidth() >> 1));
        data.max_x = game_screen.width.wrapping_add(ishr(img.width(), 1)) as i16;
        // this.maxY = (short) (GameScreen.worldHeight + image.getHeight());
        data.max_y = game_screen.world_height.wrapping_add(img.height()) as i16;
    }
    // super(pixelX, pixelY, halfWidth, halfHeight);
    let mut node = EntityNode {
        data: EntityData::MapObject(data),
        ..EntityNode::default()
    };
    entity::init_base(&mut node, pixel_x, pixel_y, half_width, half_height);
    arena.alloc(node)
}
