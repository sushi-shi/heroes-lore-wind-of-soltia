//! Transliterated from `java/src/main/java/defpackage/Overlay.java`
//! (original `f.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Abstract base of the short-lived on-battler visual effects carried in a
//! [`crate::battler::BattlerData`]'s overlay lists: the floating damage/heal
//! numbers ([`crate::floater`]), the buff/debuff status icons
//! ([`crate::status_icon`]), and the guardian-cast animation `GuardianCastFx`
//! (DEFERRED — not in this batch). Each effect advances a `frame` counter and
//! marks itself `finished` once it has run for its `lifetime`; the owning
//! `Battler` then drops finished effects from its list.
//!
//! ## The overlay union ([`Overlay`] / [`OverlayData`])
//!
//! Java references each effect through the abstract `Overlay` supertype (the
//! `Vector floaters` holds `Overlay`s; `Vector statuses` holds `StatusIcon`s cast
//! to `Overlay`). Java single-inheritance is flattened exactly as [`crate::entity`]
//! flattens `Entity`: an [`Overlay`] struct carries the `Overlay` **base** fields
//! plus an [`OverlayData`] tagged union of the concrete subclass data
//! ([`crate::status_icon::StatusIconData`] / [`crate::floater::FloaterData`]). A
//! Java `(StatusIcon)` downcast becomes [`Overlay::as_status_icon`]; the virtual
//! `paint` dispatch becomes [`paint`], matching on [`OverlayData`].
//!
//! `Overlay implements Directions` (its subclasses read the shared direction/element
//! constant tables as inherited interface constants); those constants live in
//! [`crate::directions`] as module `const`s, so nothing is inherited in code here.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`): `f.<init>:(S)V => []`
//! (`Overlay(short lifetime)` is `this.lifetime = lifetime;` — pure assignment;
//! `paint` is abstract, no body).

use crate::floater::{self, FloaterData};
use crate::guardian_cast_fx::{self, GuardianCastFxData};
use crate::status_icon::{self, StatusIconData};

/// The per-subclass data of an overlay — the flattened tagged union standing in for
/// the concrete runtime type (a `StatusIcon` or a `Floater`), modelled on
/// [`crate::entity::EntityData`].
///
/// The three concrete `Overlay` subclasses are all modelled: `StatusIcon`, `Floater`,
/// and `GuardianCastFx` (the guardian summon/cast animation, carried in a battler's
/// `floaters` list like a `Floater`).
#[derive(Debug)]
pub enum OverlayData {
    /// `StatusIcon` (`cf`) instance data — a buff/debuff icon.
    StatusIcon(StatusIconData),
    /// `Floater` (`aw`) instance data — a floating damage/heal/effect popup.
    Floater(FloaterData),
    /// `GuardianCastFx` (`bj`) instance data — a guardian summon/cast animation.
    GuardianCastFx(GuardianCastFxData),
}

/// The `Overlay` (`f`) base record: the fields every effect subclass inherits, plus
/// the [`OverlayData`] tagged union of the concrete subclass data. The base fields
/// keep their names (`frame`/`finished` are read directly by the owning `Battler`'s
/// [`crate::battler::draw_floaters`] reap).
#[derive(Debug)]
pub struct Overlay {
    /// `public short lifetime;` (`f.a`) — frames this effect lives for (or `-1` for
    /// open-ended / looping).
    pub lifetime: i16,
    /// `public short frame = 0;` (`f.b`) — frames elapsed since the effect started.
    pub frame: i16,
    /// `public boolean finished = false;` (`f.a`) — set once the effect has finished
    /// so the owner can drop it.
    pub finished: bool,
    /// The concrete subclass data (flattened tagged union).
    pub data: OverlayData,
}

impl Overlay {
    /// The `Overlay(short lifetime)` constructor (`f.<init>:(S)V => []`), reproduced
    /// onto a freshly-built [`Overlay`] carrying `data`. `super(lifetime)` sets
    /// `lifetime = lifetime`; the field initializers `frame = 0` and
    /// `finished = false` supply the rest.
    pub fn new(lifetime: i16, data: OverlayData) -> Overlay {
        Overlay {
            // this.lifetime = lifetime;
            lifetime,
            // public short frame = 0;
            frame: 0,
            // public boolean finished = false;
            finished: false,
            data,
        }
    }

    /// `this instanceof StatusIcon` → the borrowed [`StatusIconData`], or `None`
    /// (a Java `(StatusIcon)` downcast — an `elementAt` cast in `applyStatus` /
    /// `drawStatusIcons`).
    pub fn as_status_icon(&self) -> Option<&StatusIconData> {
        match &self.data {
            OverlayData::StatusIcon(s) => Some(s),
            _ => None,
        }
    }

    /// Mutable [`Self::as_status_icon`].
    pub fn as_status_icon_mut(&mut self) -> Option<&mut StatusIconData> {
        match &mut self.data {
            OverlayData::StatusIcon(s) => Some(s),
            _ => None,
        }
    }

    /// `this instanceof Floater` → the borrowed [`FloaterData`], or `None`.
    pub fn as_floater(&self) -> Option<&FloaterData> {
        match &self.data {
            OverlayData::Floater(f) => Some(f),
            _ => None,
        }
    }

    /// `this instanceof GuardianCastFx` → the borrowed [`GuardianCastFxData`], or
    /// `None` (the concrete-type access used by [`guardian_cast_fx::paint`]).
    pub fn as_guardian_cast_fx(&self) -> Option<&GuardianCastFxData> {
        match &self.data {
            OverlayData::GuardianCastFx(g) => Some(g),
            _ => None,
        }
    }

    /// Mutable [`Self::as_guardian_cast_fx`].
    pub fn as_guardian_cast_fx_mut(&mut self) -> Option<&mut GuardianCastFxData> {
        match &mut self.data {
            OverlayData::GuardianCastFx(g) => Some(g),
            _ => None,
        }
    }
}

/// The virtual `Overlay.paint(Graphics, int, int)` dispatch: routes to the concrete
/// subclass `paint` on the effect's runtime type. Used by
/// [`crate::battler::draw_floaters`], which paints each overlay through the abstract
/// supertype.
pub fn paint(o: &mut Overlay, graphics: &mut j2me_me::Graphics, x: i32, y: i32) {
    match o.data {
        OverlayData::Floater(_) => floater::paint(o, graphics, x, y),
        OverlayData::StatusIcon(_) => status_icon::paint(o, graphics, x, y),
        OverlayData::GuardianCastFx(_) => guardian_cast_fx::paint(o, graphics, x, y),
    }
}
