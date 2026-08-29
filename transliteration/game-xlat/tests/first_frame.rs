//! First-frame render gate for the strict transliteration.
//!
//! Milestone (gothic `first_frame.rs` style): drive the boot entry, then the
//! title (logo) render path — construct the `TitleScreen` (its `Canvas` +
//! framebuffer), load the `/img/logo` atlas through the reused `PngMerger`
//! (`AssetCache.loadLogo`), arm the state-10 logo animation (`startLogo`),
//! `Display.setCurrent`, and drive `GameLoop.run_one_frame` (the frame loop's
//! synchronized section → one `TitleScreen.paint`). The paint routes through
//! `j2me-me` `Graphics`, which rasterises into the ARGB `Image` that IS the frame.
//!
//! The state-10 branch is the genuine first rendered frame after boot (captured
//! as the reference `title-logo.png`, 240×320). The async boot loader that
//! reaches state 10 on device (`boot()` with `FontManager`/`AppConfig`/
//! `StringTable` and the sprite/string banks) is DEFERRED (anti-bog); this driver
//! stands in for it by porting only the loaders the paint touches.
//!
//! The gate proves a REAL frame (pixel-richness: distinct colours ≥ 8 AND the
//! dominant colour does not fill the frame AND it is not all-white), backed by a
//! proven-red negative control (the same assertions on a fresh blank framebuffer
//! must FAIL) and a liveness check (the paint actually wrote pixels). The EXACT
//! pixel diff vs the captured reference is a FOLLOW-UP (needs a `--script` capture
//! harness); this increment uses the pixel-richness gate.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, game_loop, game_midlet, title_screen, Game,
};
use j2me_me::Image;
use std::collections::HashMap;

/// Populate the classpath seam from the baseline JAR (every entry, by its zip
/// name). Loud failure if the JAR is missing is inherited from `common::jar()`.
fn load_resources(g: &mut Game) {
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
}

/// Drive boot → title (logo) render setup → one frame; return the whole `Game`.
fn drive_first_frame(frames: u32) -> Game {
    let mut g = Game::new();
    load_resources(&mut g);

    // --- boot entry (already ported): GameMIDlet -> GameLoop.create/start ---
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);

    // --- title render setup (stands in for the deferred boot()+async loader) ---
    // new TitleScreen(): materialise the Canvas + framebuffer + BaseCanvas geometry.
    title_screen::construct(&mut g);
    // AssetCache.loadLogo(): logoFrames = new PngMerger("/img/logo").allImages().
    asset_cache::load_logo(&mut g);
    // TitleScreen.startLogo(): state=10, animTick=-20, glyph1X=1, setFps(20).
    title_screen::start_logo(&mut g);
    // Display.setCurrent(titleScreen).
    {
        let Game {
            display, canvas, ..
        } = &mut g;
        display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
    }

    // --- drive the frame loop: each run_one_frame yields one rendered frame ---
    for _ in 0..frames {
        game_loop::run_one_frame(&mut g);
    }
    g
}

/// (distinct colour count, most-common colour's pixel count, total pixels).
fn frame_stats(img: &Image) -> (usize, usize, usize) {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for y in 0..img.height() {
        for x in 0..img.width() {
            if let Some(px) = img.get(x, y) {
                *counts.entry(px).or_insert(0) += 1;
            }
        }
    }
    let total = (img.width() * img.height()) as usize;
    let dominant = counts.values().copied().max().unwrap_or(0);
    (counts.len(), dominant, total)
}

/// The gate assertions applied to a candidate frame (shared by the real-frame
/// test and the proven-red negative control).
fn assert_real_frame(img: &Image) {
    let (distinct, dominant, total) = frame_stats(img);
    let white = 0xffff_ffffu32;
    let all_white = img.pixels().iter().all(|&p| p == white);
    assert!(
        distinct >= 8,
        "a real frame has variety: distinct colours = {distinct} (< 8)"
    );
    assert!(
        dominant < total,
        "a real frame is not one flat colour: dominant {dominant} == total {total}"
    );
    assert!(!all_white, "a real frame is not blank (all-white)");
}

#[test]
fn first_frame_renders_a_real_title_logo_frame() {
    // The logo slides down over the first frames (animTick converges to halfH);
    // drive enough frames to seat it, exactly as the device loop would.
    let g = drive_first_frame(12);
    let img = g.screen.as_ref().expect("framebuffer");

    // Correct device geometry made it onto BaseCanvas.
    assert_eq!((img.width(), img.height()), (240, 320));
    assert!(
        g.display.has_current(),
        "Display.setCurrent showed the title"
    );
    assert_eq!(g.title_screen.state, 10, "TitleScreen is on the logo state");
    assert!(
        g.asset_cache
            .logo_frames
            .as_ref()
            .is_some_and(|f| f.len() == 5),
        "loadLogo decoded the 5-frame /img/logo atlas"
    );

    let (distinct, dominant, total) = frame_stats(img);
    eprintln!("first_frame: {distinct} distinct colours, dominant {dominant}/{total} px, 240x320");
    assert_real_frame(img);
}

/// A single `run_one_frame`/paint already produces a real (non-blank) frame — the
/// literal milestone ("one TitleScreen paint produces a non-blank frame").
#[test]
fn a_single_paint_already_produces_a_non_blank_frame() {
    let g = drive_first_frame(1);
    let img = g.screen.as_ref().expect("framebuffer");
    let (distinct, dominant, total) = frame_stats(img);
    eprintln!("single_paint: {distinct} distinct colours, dominant {dominant}/{total} px");
    assert_real_frame(img);
}

/// Liveness (GATES.md): the paint actually WROTE pixels — the framebuffer differs
/// from a fresh blank (all-white) surface of the same size. A no-op paint fails.
#[test]
fn paint_actually_wrote_pixels() {
    let g = drive_first_frame(12);
    let painted = g.screen.as_ref().expect("framebuffer");
    let blank = Image::create_mutable(painted.width(), painted.height()).unwrap();
    assert_ne!(
        painted.pixels(),
        blank.pixels(),
        "the paint left the framebuffer identical to a blank surface"
    );
    // And a concrete non-white pixel exists.
    let non_white = (0..painted.height())
        .any(|y| (0..painted.width()).any(|x| painted.get(x, y).is_some_and(|p| p != 0xffff_ffff)));
    assert!(non_white, "no non-white pixel — nothing was drawn");
}

/// Negative control (GATES.md R3): the SAME gate assertions on a fresh unpainted
/// (blank, all-white) framebuffer must FAIL — proving the pixel-richness gate
/// actually bites and an all-white surface cannot read as a rendered frame.
#[test]
#[should_panic(expected = "distinct colours")]
fn negative_control_blank_framebuffer_is_rejected() {
    // A fresh mutable Image is uniformly white: distinct == 1, dominant == total,
    // all-white — every clause of the gate is violated. The first to fire is the
    // distinct-colour floor, proving the richness gate bites (R3).
    let blank = Image::create_mutable(240, 320).unwrap();
    assert_real_frame(&blank);
}
