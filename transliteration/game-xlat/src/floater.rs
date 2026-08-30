//! Transliterated from `java/src/main/java/defpackage/Floater.java`
//! (original `aw.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! A short-lived visual "floater" overlaid on a [`crate::battler::BattlerData`]:
//! damage/heal numbers, hit sparks, level-up and pickup icons, status flashes, etc.
//! Extends [`crate::overlay::Overlay`]; `kind` selects both which sprite source
//! [`load_sprites`] binds and how [`paint`] renders and offsets it each frame. The
//! inherited `frame` counter advances in [`paint`], and the overlay finishes once it
//! reaches its lifetime (a lifetime of `-1` loops forever). `value` doubles as the
//! number to draw (kind 7) or a sprite index (kinds 9/10).
//!
//! ## DEFERRED drawing
//!
//! [`load_sprites`] and the asset-backed [`paint`] cases bind `AssetCache` static
//! image/script banks the `asset_cache` slice does not yet model (it covers only the
//! title/logo render path). So `frames`/`spriteScript` stay unbound and the draws
//! that read `AssetCache` directly (`floaterIcon2`/`floaterIcon3`/`emoticonBubble`),
//! `GameScreen.drawFrameGroup` (over the DEFERRED script) and `BaseCanvas.drawNumber`
//! are DEFERRED. The `this.frames[this.frame]` image cases (1/5/6/8/default) and the
//! per-frame `frame`-advance lifetime logic — the observable overlay state — are
//! ported.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `aw.<clinit>:()V => []`,
//! `aw.<init>:(B)V => ["i2s"]` (the `DEFAULT_LIFETIME[kind]` byte→short),
//! `aw.<init>:(BSS)V => []`, `aw.a:()V (loadSprites) => ["i2s"]` (the case-9
//! `lifetime = spriteScript[0]`), `aw.a:(Ljavax/microedition/lcdui/Graphics;II)V
//! (paint) => ["isub","imul","isub","irem","iadd","i2b","ineg","iadd","isub","imul",
//! "isub","iinc"×?,"iadd"×?,"isub"×2,"irem","iadd"×3,"i2s"]` (the full multi-case
//! draw; the DEFERRED arms account for the arithmetic not reproduced here).

use crate::overlay::{Overlay, OverlayData};

/// `public static final byte[] DEFAULT_LIFETIME` (`aw.h:[B`). Default lifetime in
/// frames per `kind` (`-1` = none / looping). A `static final` constant table
/// reproduced as a `const` (the `directions`/`ItemBag.QUICK_TYPES` precedent).
pub const DEFAULT_LIFETIME: [i8; 11] = [-1, 3, 4, 11, 9, 3, 3, -1, 8, -1, -1];

/// The `Floater` (`aw`) instance data — the subclass fields beyond the [`Overlay`]
/// base. Embedded in [`OverlayData::Floater`].
#[derive(Debug)]
pub struct FloaterData {
    /// `private Image[] frames;` (`aw.a`) — sprite frames for image-based kinds,
    /// bound from `AssetCache` in [`load_sprites`] (DEFERRED, so `None` until those
    /// banks land). `Image[]` (nullable) → `Option<Vec<Image>>`.
    pub frames: Option<Vec<j2me_me::Image>>,
    /// `private byte[] spriteScript;` (`aw.i`) — frame-group script for kinds 4/9,
    /// bound from `AssetCache` in [`load_sprites`] (DEFERRED → `None`). `byte[]`
    /// (nullable) → `Option<Vec<i8>>`.
    pub sprite_script: Option<Vec<i8>>,
    /// `private byte kind;` (`aw.a`) — which floater kind (selects sprite source and
    /// paint behaviour).
    pub kind: i8,
    /// `private short value;` (`aw.c`) — number to display (kind 7) or sprite index
    /// (kinds 9/10).
    pub value: i16,
}

/// `public Floater(byte kind)` (`aw.<init>:(B)V => ["i2s"]`). The convenience
/// constructor: `this(kind, DEFAULT_LIFETIME[kind], (short) 0);`.
pub fn new_default(kind: i8) -> Overlay {
    // this(kind, DEFAULT_LIFETIME[kind], (short) 0);  — the byte→short is the `i2s`.
    new(kind, DEFAULT_LIFETIME[kind as usize] as i16, 0)
}

/// `public Floater(byte kind, short lifetime, short value)` (`aw.<init>:(BSS)V => []`).
/// The primary constructor: `super(lifetime); this.kind = kind; this.value = value;
/// loadSprites();`.
pub fn new(kind: i8, lifetime: i16, value: i16) -> Overlay {
    // super(lifetime);  +  this.kind = kind; this.value = value;
    let mut o = Overlay::new(
        lifetime,
        OverlayData::Floater(FloaterData {
            frames: None,
            sprite_script: None,
            kind,
            value,
        }),
    );
    // loadSprites();
    load_sprites(&mut o);
    o
}

/// `private final void loadSprites()` (`aw.a:()V => ["i2s"]`). Binds the sprite
/// frames / script for `this.kind` from `AssetCache`.
///
/// **DEFERRED.** Every arm binds an `AssetCache` static image/script bank not yet
/// ported (`asset_cache` models only the title/logo render path), so `frames` /
/// `spriteScript` stay `None` and the case-9 side effect `lifetime = spriteScript[0]`
/// (the `i2s`) is not applied. The paint draws that read those bindings are DEFERRED
/// to match (see [`paint`]).
pub fn load_sprites(_o: &mut Overlay) {
    // switch (this.kind) {
    //   case 1:  this.frames = AssetCache.attackFx1;
    //   case 4:  this.spriteScript = AssetCache.levelUpScript;
    //   case 5:  this.frames = AssetCache.attackFx2;
    //   case 6:  this.frames = AssetCache.attackFx3;
    //   case 9:  this.spriteScript = (byte[]) AssetCache.attackEffectScripts[this.value];
    //            ((Overlay) this).lifetime = this.spriteScript[0];
    //   case 10: this.frames = AssetCache.emoticons;
    // }
    // DEFERRED: AssetCache.attackFx1 / attackFx2 / attackFx3 / levelUpScript /
    //   attackEffectScripts / emoticons (unported image/script banks).
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`aw.a:(Ljavax/microedition/lcdui/Graphics;II)V`). Renders the floater for its
/// `kind`, then advances the `frame` counter and finishes at `lifetime` (a `-1`
/// lifetime loops forever).
///
/// **PARTIAL — DEFERRED draws.** The `this.frames[this.frame]` image cases
/// (1 / 5 / 6 / 8 / default) draw through [`j2me_me::Graphics`]. The cases that bind
/// unported `AssetCache` statics (2 → `floaterIcon2`, 3 → `floaterIcon3`,
/// 10 → `emoticonBubble` + `frames`=`emoticons`), the frame-group cases
/// (4 / 9 → `GameScreen.drawFrameGroup` over the DEFERRED `spriteScript`) and the
/// number case (7 → `BaseCanvas.drawNumber`, an unported method) are DEFERRED. The
/// `frame`-advance lifetime tail is fully ported, so a floater's lifetime is
/// advanced by painting it exactly as in Java.
pub fn paint(o: &mut Overlay, graphics: &mut j2me_me::Graphics, x: i32, y: i32) {
    let mut x = x;
    let mut y = y;
    // `this.frame` — captured before the post-switch increment (every switch read is
    // the pre-increment value).
    let frame = o.frame;
    let lifetime = o.lifetime;

    // Downcast to the Floater data (the paint dispatch guarantees this variant).
    let f = match &o.data {
        OverlayData::Floater(f) => f,
        _ => unreachable!("floater::paint on a non-Floater overlay"),
    };

    // switch (this.kind)
    match f.kind {
        // case 1:
        1 => {
            // if (frame == 0) { y -= 10; x -= 3; } else if (frame == 1) { y -= 8; }
            if frame == 0 {
                y = y.wrapping_sub(10);
                x = x.wrapping_sub(3);
            } else if frame == 1 {
                y = y.wrapping_sub(8);
            }
            // graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
            let frames = f
                .frames
                .as_ref()
                .expect("NullPointerException: Floater.frames null (loadSprites DEFERRED)");
            graphics
                .draw_image(&frames[frame as usize], x, y.wrapping_add(3), 33)
                .expect("drawImage(Floater frame)");
        }
        // case 2: DEFERRED — AssetCache.floaterIcon2 (unported image bank).
        //   graphics.drawImage(AssetCache.floaterIcon2, x, (y - 30) - (this.frame * 4), 17);
        2 => {}
        // case 3: DEFERRED — AssetCache.floaterIcon3 (unported image bank).
        //   if (this.frame % 4 < 3) graphics.drawImage(AssetCache.floaterIcon3, x, y + 5, 33);
        3 => {}
        // case 4: case 9: DEFERRED — GameScreen.drawFrameGroup over this.spriteScript,
        //   whose binding (AssetCache.levelUpScript / attackEffectScripts) is DEFERRED
        //   in loadSprites, so spriteScript is never populated.
        //   GameScreen.drawFrameGroup(graphics, this.spriteScript, (byte) this.frame, x, y);
        4 | 9 => {}
        // case 5:
        5 => {
            // if (frame == 2) { y -= 5; }
            if frame == 2 {
                y = y.wrapping_sub(5);
            }
            // graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
            let frames = f
                .frames
                .as_ref()
                .expect("NullPointerException: Floater.frames null (loadSprites DEFERRED)");
            graphics
                .draw_image(&frames[frame as usize], x, y.wrapping_add(3), 33)
                .expect("drawImage(Floater frame)");
        }
        // case 6:
        6 => {
            // if (frame == 1) { y -= 2; } else if (frame == 2) { y -= 6; }
            if frame == 1 {
                y = y.wrapping_sub(2);
            } else if frame == 2 {
                y = y.wrapping_sub(6);
            }
            // graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
            let frames = f
                .frames
                .as_ref()
                .expect("NullPointerException: Floater.frames null (loadSprites DEFERRED)");
            graphics
                .draw_image(&frames[frame as usize], x, y.wrapping_add(3), 33)
                .expect("drawImage(Floater frame)");
        }
        // case 7: DEFERRED — BaseCanvas.drawNumber (unported method).
        //   BaseCanvas.drawNumber(graphics, this.value < 0 ? -this.value : this.value,
        //     x + 1, (y - 30) - (this.frame * 4), 1,
        //     this.frame < 2 ? (this.value < 0 ? 4 : 3) : (this.value < 0 ? 2 : 1));
        7 => {}
        // case 10: DEFERRED — AssetCache.emoticonBubble + this.frames (= AssetCache.emoticons,
        //   DEFERRED in loadSprites).
        //   if (this.value != 8 && this.value != 9)
        //     graphics.drawImage(AssetCache.emoticonBubble, x, y - 40, 17);
        //   graphics.drawImage(this.frames[this.value], x, (y - 39) + (this.frame % 2), 17);
        10 => {}
        // case 8: default:
        _ => {
            // graphics.drawImage(this.frames[this.frame], x, y + 3, 33);
            let frames = f
                .frames
                .as_ref()
                .expect("NullPointerException: Floater.frames null (loadSprites DEFERRED)");
            graphics
                .draw_image(&frames[frame as usize], x, y.wrapping_add(3), 33)
                .expect("drawImage(Floater frame)");
        }
    }

    // this.frame = (short) (this.frame + 1);
    o.frame = (frame as i32).wrapping_add(1) as i16;
    // if (this.frame < ((Overlay) this).lifetime || ((Overlay) this).lifetime == -1) return;
    if (o.frame as i32) < (lifetime as i32) || lifetime == -1 {
        return;
    }
    // ((Overlay) this).finished = true;
    o.finished = true;
}
