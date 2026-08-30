//! Transliterated from `java/src/main/java/defpackage/StatusIcon.java`
//! (original `cf.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A single status-effect icon bobbing above a [`crate::battler::BattlerData`]
//! (poison, stun, buffs, etc.). Extends [`crate::overlay::Overlay`]: the inherited
//! `frame` counter advances each [`tick`] and, once it reaches the per-kind duration
//! ([`DURATION_BY_KIND`]), the overlay marks itself finished so the owner reaps it.
//! [`crate::battler::apply_status`] reuses an existing icon of the same `kind` by
//! calling [`reset`] instead of stacking a new one.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `cf.<clinit>:()V => []`,
//! `cf.<init>:(B)V => []`, `cf.a:()V (tick) => ["iadd","i2s"]`,
//! `cf.b:()V (expire) => []`, `cf.a:(Ljavax/microedition/lcdui/Graphics;II)V (paint)
//! => ["isub","isub","irem","iadd"]`, `cf.c:()V (reset) => []`,
//! `cf.a:()S (elapsed) => []`.

use crate::overlay::{Overlay, OverlayData};

/// `private static final short[] DURATION_BY_KIND` (`cf.a:[S`). Lifetime in frames
/// per status kind (indexes 0..7). A `static final` constant table reproduced as a
/// `const` (the `directions`/`ItemBag.QUICK_TYPES` precedent).
pub const DURATION_BY_KIND: [i16; 8] = [40, 40, 40, 40, 40, 140, 160, 80];

/// The `StatusIcon` (`cf`) instance data — the subclass fields beyond the
/// [`Overlay`] base. Embedded in [`OverlayData::StatusIcon`].
#[derive(Debug)]
pub struct StatusIconData {
    /// `public byte kind;` (`cf.a`) — status kind this icon represents (indexes
    /// `AssetCache.statusIcons`).
    pub kind: i8,
}

/// `public StatusIcon(byte kind)` (`cf.<init>:(B)V => []`). Builds the icon:
/// `super(DURATION_BY_KIND[kind]); this.kind = kind;`.
pub fn new(kind: i8) -> Overlay {
    // super(DURATION_BY_KIND[kind]);
    Overlay::new(
        DURATION_BY_KIND[kind as usize],
        // this.kind = kind;
        OverlayData::StatusIcon(StatusIconData { kind }),
    )
}

/// `public final void tick()` (`cf.a:()V => ["iadd","i2s"]`). Advances the animation
/// one frame, finishing when the lifetime elapses.
pub fn tick(o: &mut Overlay) {
    // this.frame = (short) (this.frame + 1);
    o.frame = (o.frame as i32).wrapping_add(1) as i16;
    // if (this.frame >= this.lifetime) this.finished = true;
    if o.frame as i32 >= o.lifetime as i32 {
        o.finished = true;
    }
}

/// `public final void expire()` (`cf.b:()V => []`). Ends this icon immediately
/// (e.g. a cleansed status).
pub fn expire(o: &mut Overlay) {
    // this.finished = true;
    o.finished = true;
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`cf.a:(Ljavax/microedition/lcdui/Graphics;II)V => ["isub","isub","irem","iadd"]`).
///
/// **DEFERRED drawing.** Both draws bind `AssetCache` static image banks that are
/// not yet ported (`asset_cache` models only the title/logo render path):
/// `AssetCache.emoticonBubble` and `AssetCache.statusIcons[this.kind]`. The method
/// mutates no overlay state (the `frame` advance lives in [`tick`], not here), so
/// the DEFERRED body is a faithful no-op; the two blits and their arithmetic
/// (`y - 30`, `(y - 29) + (this.frame % 2)`) are recorded for when those banks land.
pub fn paint(_o: &Overlay, _graphics: &mut j2me_me::Graphics, _x: i32, _y: i32) {
    // graphics.drawImage(AssetCache.emoticonBubble, x, y - 30, 17);
    // graphics.drawImage(AssetCache.statusIcons[this.kind], x, (y - 29) + (this.frame % 2), 17);
    // DEFERRED: AssetCache.emoticonBubble / AssetCache.statusIcons (unported image
    //   banks). No overlay-state mutation; nothing else to reproduce.
}

/// `public final void reset()` (`cf.c:()V => []`). Restarts the icon's animation from
/// frame 0 (refreshes a re-applied status).
pub fn reset(o: &mut Overlay) {
    // this.frame = (short) 0;
    o.frame = 0;
}

/// `public final short elapsed()` (`cf.a:()S => []`). Returns the current frame
/// counter (used for periodic poison ticks).
pub fn elapsed(o: &Overlay) -> i16 {
    // return this.frame;
    o.frame
}
