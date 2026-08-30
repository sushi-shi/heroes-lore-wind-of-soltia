//! Transliterated from `java/src/main/java/defpackage/GuardianCastFx.java`
//! (original `bj.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! On-map animation for a guardian summon/cast, carried in an entity's floater list
//! as an [`Overlay`] (the third `Overlay` subclass alongside [`crate::floater`] /
//! [`crate::status_icon`]). After an initial `startDelay` it advances the inherited
//! `frame` counter each [`paint`], choosing one of three looks by
//! (`guardianType`, `skillSlot`):
//!
//! - both zero — the base summon pose drawn straight from the element atlas
//!   (`elementSprites` slots 7-9);
//! - guardian 0 or 1 with skill 2 — the two-frame descending beam (`beamFrames`)
//!   cycled every four frames;
//! - otherwise — the guardian's pose frame-group script (`guardianFrames[skillSlot]`).
//!
//! The effect marks itself `finished` once `frame` reaches its `lifetime`.
//!
//! ## The overlay union
//!
//! `GuardianCastFx extends Overlay`; it joins the flattened [`crate::overlay`] union
//! as [`OverlayData::GuardianCastFx`] exactly as `StatusIcon`/`Floater` do. The
//! virtual `paint` dispatch is [`crate::overlay::paint`]; the concrete data is reached
//! through [`crate::overlay::Overlay::as_guardian_cast_fx`]. `beamFrames` /
//! `elementSprites` (both `Image[]`) and `guardianFrames` (`Object[]`, the byte[] pose
//! scripts) are captured from `AssetCache` in the constructor and modelled as owned
//! clones (the immutable-after-load "capture the reference now" model, the
//! `enemy_type` stat-clone precedent).
//!
//! ## DEFERRED
//!
//! - **Spawn call site.** A `GuardianCastFx` is created by the unported `Guardian`
//!   (`bl`); the Guardian-spawn call site is DEFERRED. The constructor + paint here
//!   are ported and driven directly by the oracle.
//! - **The `guardianFrames`-case draw.** The third look calls
//!   `GameScreen.drawFrameGroup(graphics, guardianFrames[skillSlot], (byte) frame, x, y)`,
//!   which reads `AssetCache.spriteBanks[bank]` for the images the pose script names.
//!   The overlay `paint` dispatch is `(&mut Overlay, &mut Graphics)` — it threads no
//!   `AssetCache` (adding it would ripple into the read-only `floater`/`status_icon`
//!   paint signatures) — so this draw is DEFERRED, exactly like `Floater`'s
//!   frame-group cases. The `frame`-advance / `finished` tail and the two
//!   self-owned-image looks (the base pose and the beam) ARE ported, so the effect's
//!   observable lifetime state machine is complete.
//! - **The element atlas + pose scripts.** `AssetCache.spriteBanks[12]` and
//!   `AssetCache.guardianFrames` are DEFERRED-loaded (see [`crate::asset_cache`]).
//!   The constructor reads them exactly as Java does (a null `spriteBanks[12]` NPEs,
//!   faithfully — in real play the element atlas is loaded before any cast).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bj.<init>:(SSBB)V => []`
//! (pure assignment + array reads), `bj.a:(…Graphics;II)V (paint) => ["isub","i2s",
//! "isub","isub","isub","isub","irem","iadd","iadd","iadd","iadd","i2b","iadd","i2s"]`
//! (the `startDelay - 1` narrow, the four base-pose `y - k`, the `frame % 4`, the four
//! beam `y + k`, the `(byte) frame` for the DEFERRED frame-group draw, and the
//! `frame + 1` narrow).

use crate::game::Game;
use crate::overlay::{Overlay, OverlayData};
use j2me_jvm::java_rem;

/// The `GuardianCastFx` (`bj`) instance data — the subclass fields beyond the
/// [`Overlay`] base. Embedded in [`OverlayData::GuardianCastFx`].
#[derive(Debug)]
pub struct GuardianCastFxData {
    /// `private byte skillSlot;` (`bj.a`) — guardian skill slot; also indexes
    /// `guardianFrames`.
    pub skill_slot: i8,
    /// `private byte guardianType;` (`bj.b`) — guardian type (selects the beam
    /// special-case).
    pub guardian_type: i8,
    /// `private short startDelay;` (`bj.c`) — frames to wait before the animation
    /// starts playing.
    pub start_delay: i16,
    /// `private Image[] beamFrames;` (`bj.a`) — the two descending-beam frames
    /// (element atlas slots 0-1). `Image[]` (elements nullable) → `Vec<Option<Image>>`.
    pub beam_frames: Vec<Option<j2me_me::Image>>,
    /// `private Image[] elementSprites;` (`bj.b`) — the guardian element atlas
    /// (`spriteBanks[12]`); slots 7-9 are the base pose. `Vec<Option<Image>>`.
    pub element_sprites: Vec<Option<j2me_me::Image>>,
    /// `private Object[] guardianFrames;` (`bj.a`) — guardian pose frame-group scripts
    /// (a capture of `AssetCache.guardianFrames`). `Object[]` (byte[] or null) →
    /// `Vec<Option<Vec<i8>>>`. Read only by the DEFERRED `guardianFrames`-case draw.
    pub guardian_frames: Vec<Option<Vec<i8>>>,
}

/// `public GuardianCastFx(short startDelay, short lifetime, byte guardianType,
/// byte skillSlot)` (`bj.<init>:(SSBB)V => []`). Builds the effect, capturing the
/// element atlas + pose scripts from `AssetCache`.
pub fn new(
    g: &Game,
    start_delay: i16,
    lifetime: i16,
    guardian_type: i8,
    skill_slot: i8,
) -> Overlay {
    // this.beamFrames = new Image[2];
    // Image[] elementAtlas = AssetCache.spriteBanks[12];
    let element_atlas = g.asset_cache.sprite_banks[12]
        .as_ref()
        .expect("NullPointerException: AssetCache.spriteBanks[12] null in GuardianCastFx");
    // this.beamFrames[0] = elementAtlas[0]; this.beamFrames[1] = elementAtlas[1];
    let beam_frames = vec![element_atlas[0].clone(), element_atlas[1].clone()];
    // this.elementSprites = AssetCache.spriteBanks[12];
    let element_sprites = element_atlas.clone();
    // this.guardianFrames = AssetCache.guardianFrames;
    let guardian_frames = g
        .asset_cache
        .guardian_frames
        .clone()
        .expect("AssetCache.guardianFrames null in GuardianCastFx");
    // super(lifetime);  +  this.startDelay/guardianType/skillSlot = ...;
    Overlay::new(
        lifetime,
        OverlayData::GuardianCastFx(GuardianCastFxData {
            skill_slot,
            guardian_type,
            start_delay,
            beam_frames,
            element_sprites,
            guardian_frames,
        }),
    )
}

/// `graphics.drawImage(image, x, y, 33)` for a nullable overlay image — a null image
/// NPEs exactly as `Graphics.drawImage(null, …)` does (the element atlas is loaded
/// before any cast in real play). All of `GuardianCastFx`'s blits use anchor `33`.
fn draw_image(graphics: &mut j2me_me::Graphics, image: &Option<j2me_me::Image>, x: i32, y: i32) {
    let image = image
        .as_ref()
        .expect("NullPointerException: GuardianCastFx image slot null");
    graphics
        .draw_image(image, x, y, 33)
        .expect("drawImage(GuardianCastFx)");
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bj.a:(…Graphics;II)V`). Renders the effect for its `(guardianType, skillSlot)`,
/// then advances the `frame` counter and finishes at `lifetime`.
///
/// **PARTIAL — one DEFERRED draw.** The base-pose look (`elementSprites[7-9]`) and the
/// beam look (`beamFrames[0-1]`) blit self-owned images and are ported. The
/// `guardianFrames`-case draw (`GameScreen.drawFrameGroup`) needs `AssetCache`, which
/// the overlay paint dispatch does not thread, so it is DEFERRED (see the module
/// header). The `frame`-advance / `finished` tail is fully ported.
pub fn paint(o: &mut Overlay, graphics: &mut j2me_me::Graphics, x: i32, y: i32) {
    // if (this.startDelay > 0) { this.startDelay = (short) (this.startDelay - 1); return; }
    {
        let d = o
            .as_guardian_cast_fx_mut()
            .expect("guardian_cast_fx::paint on a non-GuardianCastFx overlay");
        if d.start_delay > 0 {
            d.start_delay = (d.start_delay as i32).wrapping_sub(1) as i16;
            return;
        }
    }
    // `this.frame` / `this.lifetime` — captured before the post-switch increment (every
    // switch read is the pre-increment value).
    let frame = o.frame;
    let lifetime = o.lifetime;

    {
        let d = o
            .as_guardian_cast_fx()
            .expect("guardian_cast_fx::paint on a non-GuardianCastFx overlay");
        // if (this.guardianType != 0 || this.skillSlot != 0) {
        if d.guardian_type != 0 || d.skill_slot != 0 {
            // if ((guardianType == 0 && skillSlot == 2) || (guardianType == 1 && skillSlot == 2)) {
            if (d.guardian_type == 0 && d.skill_slot == 2)
                || (d.guardian_type == 1 && d.skill_slot == 2)
            {
                // switch (((Overlay) this).frame % 4)
                match java_rem(frame as i32, 4).expect("frame % 4") {
                    // case 1: graphics.drawImage(beamFrames[0], x, y + 9, 33);
                    1 => draw_image(graphics, &d.beam_frames[0], x, y.wrapping_add(9)),
                    // case 2: drawImage(beamFrames[0], x, y + 9, 33); drawImage(beamFrames[1], x, y + 12, 33);
                    2 => {
                        draw_image(graphics, &d.beam_frames[0], x, y.wrapping_add(9));
                        draw_image(graphics, &d.beam_frames[1], x, y.wrapping_add(12));
                    }
                    // case 3: graphics.drawImage(beamFrames[1], x, y + 12, 33);
                    3 => draw_image(graphics, &d.beam_frames[1], x, y.wrapping_add(12)),
                    _ => {}
                }
            } else {
                // GameScreen.drawFrameGroup(graphics, (byte[]) this.guardianFrames[this.skillSlot],
                //   (byte) ((Overlay) this).frame, x, y);
                // DEFERRED: needs AssetCache.spriteBanks (drawFrameGroup reads the image bank the
                //   captured this.guardianFrames[skillSlot] script names); the overlay paint
                //   dispatch threads no AssetCache. Like Floater's frame-group cases, the draw is
                //   DEFERRED; the frame-advance tail below is ported.
            }
        } else {
            // switch (((Overlay) this).frame) { base summon pose from the element atlas }
            match frame {
                // case 0: graphics.drawImage(elementSprites[7], x, y, 33);
                0 => draw_image(graphics, &d.element_sprites[7], x, y),
                // case 1: graphics.drawImage(elementSprites[8], x, y - 1, 33);
                1 => draw_image(graphics, &d.element_sprites[8], x, y.wrapping_sub(1)),
                // case 2: graphics.drawImage(elementSprites[7], x, y - 2, 33);
                2 => draw_image(graphics, &d.element_sprites[7], x, y.wrapping_sub(2)),
                // case 3: graphics.drawImage(elementSprites[8], x, y - 3, 33);
                3 => draw_image(graphics, &d.element_sprites[8], x, y.wrapping_sub(3)),
                // case 4: case 5: graphics.drawImage(elementSprites[9], x, y - 4, 33);
                4 | 5 => draw_image(graphics, &d.element_sprites[9], x, y.wrapping_sub(4)),
                _ => {}
            }
        }
    }

    // ((Overlay) this).frame = (short) (((Overlay) this).frame + 1);
    o.frame = (frame as i32).wrapping_add(1) as i16;
    // if (((Overlay) this).frame >= ((Overlay) this).lifetime) ((Overlay) this).finished = true;
    if o.frame as i32 >= lifetime as i32 {
        o.finished = true;
    }
}
