//! Transliterated from `java/src/main/java/defpackage/ScrollCaption.java`
//! (original `bc.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! One line of the end-credits staff roll: a pre-rendered caption [`image`] plus its
//! current vertical [`y`]. `GameScreen` spawns these into its (DEFERRED)
//! `creditCaptions` vector as the ending text advances, scrolls each up two pixels
//! per frame, and drops it once it leaves the top of the screen. A leaf data holder
//! — no methods beyond the constructor.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `bc.<init>:(Ljavax/microedition/lcdui/Image;I)V => []` (plain field stores).
//!
//! [`image`]: ScrollCaption::image
//! [`y`]: ScrollCaption::y

use j2me_me::Image;

/// Java `bc` / `ScrollCaption` — one credits-roll line: a pre-rendered caption image
/// and its scroll position. Owned per-instance by `GameScreen`'s (deferred)
/// `creditCaptions` vector; no statics.
#[derive(Debug, Default)]
pub struct ScrollCaption {
    /// `public Image image;` (`bc.a`) — the pre-rendered caption bitmap.
    /// `None` == Java `null` (the reference-nullable model, as `MapObject.image`).
    pub image: Option<Image>,
    /// `public int y;` (`bc.a`, `I`) — current y position (screen pixels),
    /// decremented as the roll scrolls up.
    pub y: i32,
}

impl ScrollCaption {
    /// `public ScrollCaption(Image image, int y)`
    /// (`bc.<init>:(Ljavax/microedition/lcdui/Image;I)V => []`):
    /// `this.image = image; this.y = y;`.
    pub fn new(image: Option<Image>, y: i32) -> Self {
        // this.image = image;
        // this.y = y;
        ScrollCaption { image, y }
    }
}
