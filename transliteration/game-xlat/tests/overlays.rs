//! Unit gate for the on-battler OVERLAY FAMILY: the `Overlay` base (`f`) and its
//! two leaf subclasses — `Floater` (`aw`, floating damage/heal popups) and
//! `StatusIcon` (`cf`, buff/debuff icons) — plus the `Battler` methods that create,
//! apply, tick and draw them.
//!
//! These prove the ported lifetime state machine directly, with no boot/render
//! drive:
//!
//! * a `StatusIcon` advances one frame per `tick()` and finishes exactly when
//!   `frame` reaches its per-kind duration (`DURATION_BY_KIND`); `expire()` finishes
//!   it at once, `reset()` rewinds it, `elapsed()` reads the counter.
//! * a `Floater` advances its `frame` in `paint()` and finishes at its lifetime.
//! * `Battler.applyStatus` refreshes an existing same-kind icon (no stacking) or
//!   adds a new one; `addFloater`/`clearFloaters` push/drop; `drawFloaters` reaps a
//!   finished floater; `drawStatusIcons` lays a row out without panicking.
//!
//! Drawing bottoms out in DEFERRED `AssetCache` overlay banks, so most `paint`
//! draws are no-ops (see `floater`/`status_icon`). The one exception exercised for
//! real pixels is a `this.frames[frame]` image kind (kind 1): we inject synthetic
//! sprite frames (standing in for the DEFERRED `AssetCache.attackFx1` binding
//! `loadSprites` would do) and show the paint blits them onto the framebuffer.

use heroes_lore_wind_of_soltia_game_xlat::overlay::OverlayData;
use heroes_lore_wind_of_soltia_game_xlat::{
    battler::{self, BattlerData},
    floater, status_icon,
};
use j2me_me::{Graphics, Image};

/// `StatusIcon(kind=0)` has `DURATION_BY_KIND[0] == 40`: it ticks 40 frames and
/// finishes on the 40th (`frame >= lifetime`), not before.
#[test]
fn status_icon_ticks_to_its_per_kind_lifetime() {
    let mut icon = status_icon::new(0);

    // Constructor: super(DURATION_BY_KIND[0]) + frame 0, not finished.
    assert_eq!(
        icon.lifetime, 40,
        "kind-0 status lifetime is DURATION_BY_KIND[0]"
    );
    assert_eq!(icon.frame, 0);
    assert!(!icon.finished);
    assert_eq!(status_icon::elapsed(&icon), 0);
    assert!(matches!(icon.data, OverlayData::StatusIcon(_)));
    assert_eq!(icon.as_status_icon().unwrap().kind, 0);

    // 39 ticks: advanced but not yet finished (frame 39 < lifetime 40).
    for _ in 0..39 {
        status_icon::tick(&mut icon);
    }
    assert_eq!(icon.frame, 39);
    assert_eq!(status_icon::elapsed(&icon), 39);
    assert!(!icon.finished, "not finished before the lifetime elapses");

    // The 40th tick reaches the lifetime and finishes it.
    status_icon::tick(&mut icon);
    assert_eq!(icon.frame, 40);
    assert!(icon.finished, "finished once frame >= lifetime");

    // reset() rewinds the frame counter (a re-applied status).
    status_icon::reset(&mut icon);
    assert_eq!(icon.frame, 0);
    assert_eq!(status_icon::elapsed(&icon), 0);
}

/// `expire()` finishes a status icon immediately, regardless of frame.
#[test]
fn status_icon_expire_finishes_immediately() {
    let mut icon = status_icon::new(5); // DURATION_BY_KIND[5] == 140
    assert_eq!(icon.lifetime, 140);
    assert!(!icon.finished);
    status_icon::expire(&mut icon);
    assert!(icon.finished, "expire() finishes the icon at once");
    assert_eq!(icon.frame, 0, "expire() does not advance the frame");
}

/// A `Floater` advances its `frame` inside `paint()` and finishes at `lifetime`.
/// Kind 2 (`DEFAULT_LIFETIME[2] == 4`) has a DEFERRED draw, so `paint` is a pure
/// frame-advance here — a floater's lifetime is ticked by painting it, as in Java.
#[test]
fn floater_paint_advances_frame_and_finishes_at_lifetime() {
    let mut fb = Image::create_mutable(120, 120).unwrap();
    let mut g = Graphics::new(&mut fb);

    let mut fl = floater::new_default(2);
    assert_eq!(
        fl.lifetime, 4,
        "kind-2 floater lifetime is DEFAULT_LIFETIME[2]"
    );
    assert_eq!(fl.frame, 0);
    assert!(!fl.finished);
    assert!(matches!(fl.data, OverlayData::Floater(_)));

    // Three paints: frame 3, still short of the lifetime.
    for _ in 0..3 {
        floater::paint(&mut fl, &mut g, 60, 60);
    }
    assert_eq!(fl.frame, 3);
    assert!(!fl.finished, "not finished before frame reaches lifetime");

    // The fourth paint reaches the lifetime and finishes it.
    floater::paint(&mut fl, &mut g, 60, 60);
    assert_eq!(fl.frame, 4);
    assert!(
        fl.finished,
        "finished once frame >= lifetime (lifetime != -1)"
    );
}

/// A `-1` lifetime loops forever: kind 7 (`DEFAULT_LIFETIME[7] == -1`) never
/// finishes, however many times it is painted (its draw is DEFERRED).
#[test]
fn floater_with_negative_lifetime_never_finishes() {
    let mut fb = Image::create_mutable(120, 120).unwrap();
    let mut g = Graphics::new(&mut fb);

    let mut fl = floater::new_default(7);
    assert_eq!(
        fl.lifetime, -1,
        "kind-7 floater loops forever (lifetime -1)"
    );
    for _ in 0..50 {
        floater::paint(&mut fl, &mut g, 60, 60);
    }
    assert_eq!(fl.frame, 50);
    assert!(!fl.finished, "a -1 lifetime never finishes");
}

/// A `this.frames[frame]` image floater (kind 1) actually blits onto the frame.
/// `loadSprites` is DEFERRED (it would bind `AssetCache.attackFx1`), so we inject
/// synthetic opaque sprite frames in its place and show the paint draws pixels.
#[test]
fn floater_paint_draws_pixels_for_an_image_kind() {
    let mut fb = Image::create_mutable(120, 120).unwrap();
    let before = fb.pixels().to_vec();

    let mut fl = floater::new_default(1); // kind 1, lifetime DEFAULT_LIFETIME[1] == 3
    assert_eq!(fl.lifetime, 3);

    // Inject three opaque sprite frames (standing in for the DEFERRED
    // AssetCache.attackFx1 binding). Kind 1 reads frames[frame] for frame 0..2.
    let sprite = Image::from_argb(4, 4, vec![0xff00_0000u32; 16]).unwrap();
    if let OverlayData::Floater(ref mut d) = fl.data {
        d.frames = Some(vec![sprite.clone(), sprite.clone(), sprite.clone()]);
    } else {
        panic!("floater data is not a Floater");
    }

    {
        let mut g = Graphics::new(&mut fb);
        floater::paint(&mut fl, &mut g, 60, 60); // frame 0 → draws frames[0]
    }

    assert_eq!(fl.frame, 1, "paint advanced the frame counter");
    assert_ne!(
        before,
        fb.pixels().to_vec(),
        "kind-1 floater paint blitted its sprite onto the framebuffer"
    );
}

/// `Battler.applyStatus` refreshes an existing same-kind icon (no stacking) or adds
/// a new one, and reports which; `addFloater`/`clearFloaters` push and drop.
#[test]
fn battler_apply_status_refreshes_or_adds() {
    let mut b = BattlerData::new();
    assert!(b.floaters.is_empty());
    assert!(b.statuses.is_empty());

    // First apply of kind 3: a NEW icon (not a refresh).
    assert!(
        !battler::apply_status(&mut b, 3),
        "first apply(3) adds a new icon"
    );
    assert_eq!(b.statuses.len(), 1);
    assert_eq!(b.statuses[0].as_status_icon().unwrap().kind, 3);

    // Advance the icon so a refresh (frame → 0) is observable.
    status_icon::tick(&mut b.statuses[0]);
    status_icon::tick(&mut b.statuses[0]);
    assert_eq!(b.statuses[0].frame, 2);

    // Re-apply kind 3: a REFRESH — reset in place, no new element.
    assert!(
        battler::apply_status(&mut b, 3),
        "re-apply(3) refreshes the icon"
    );
    assert_eq!(b.statuses.len(), 1, "refresh does not stack a second icon");
    assert_eq!(b.statuses[0].frame, 0, "refresh reset the icon to frame 0");

    // A different kind adds a second icon.
    assert!(
        !battler::apply_status(&mut b, 5),
        "apply(5) adds a new icon"
    );
    assert_eq!(b.statuses.len(), 2);

    // addFloater / clearFloaters.
    battler::add_floater(&mut b, floater::new_default(2));
    assert_eq!(b.floaters.len(), 1);
    battler::clear_floaters(&mut b);
    assert!(b.floaters.is_empty(), "clearFloaters drops every floater");
}

/// `Battler.drawFloaters` paints each floater and reaps the finished ones;
/// `Battler.drawStatusIcons` lays out the status row without panicking (its draw is
/// DEFERRED).
#[test]
fn battler_draw_floaters_reaps_finished_and_status_row_lays_out() {
    let mut b = BattlerData::new();
    battler::apply_status(&mut b, 3);
    battler::apply_status(&mut b, 5);
    // A kind-2 floater (lifetime 4, DEFERRED draw).
    battler::add_floater(&mut b, floater::new_default(2));

    let mut fb = Image::create_mutable(64, 64).unwrap();
    let mut g = Graphics::new(&mut fb);

    // Three draws: the floater ticks to frame 3, not yet finished → not reaped.
    for _ in 0..3 {
        battler::draw_floaters(&mut b, &mut g, 10, 10);
    }
    assert_eq!(
        b.floaters.len(),
        1,
        "floater not reaped before its lifetime"
    );

    // The fourth draw finishes it (frame 4) and reaps it in place.
    battler::draw_floaters(&mut b, &mut g, 10, 10);
    assert!(
        b.floaters.is_empty(),
        "drawFloaters reaped the finished floater"
    );

    // The status row (two icons) draws without panicking (DEFERRED no-op paints).
    assert_eq!(b.statuses.len(), 2);
    battler::draw_status_icons(&b, &mut g, 10, 10);
}
