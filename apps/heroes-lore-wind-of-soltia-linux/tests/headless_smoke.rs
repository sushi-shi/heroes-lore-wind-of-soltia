//! The headless LIVE-loop smoke — the native host's gate.
//!
//! Construct [`GameHost`] from the real `_originals` build and drive it through the
//! transliteration's single frame-drive entry (`game_loop::run_one_frame`, `bs.run`
//! one tick) with NO display, asserting the loop is genuinely LIVE:
//!
//!   * boot reaches a real, non-blank frame — the state-10 Hands-On Mobile publisher
//!     splash the port currently renders;
//!   * driving frames ADVANCES the logo animation (the painted frame changes as the
//!     splash slides to centre), no panic;
//!   * proven RED by a *frozen* host (constructed, never driven): its frame never
//!     changes — exactly the predicate the live test asserts — and by a blank
//!     framebuffer, which fails the non-blank predicate the live frame passes.
//!
//! A separate check runs the `--exit-after-frames` binary and asserts it exits
//! cleanly (0). No window is opened (a `winit` window needs a display the CI/agent
//! lacks): the windowless [`GameHost`] is driven directly.
//!
//! Reads content from `_originals`; FAILS loudly if absent (R1/R2).

use std::path::PathBuf;
use std::process::Command;

use heroes_lore_wind_of_soltia_linux::frame::analyze;
use heroes_lore_wind_of_soltia_linux::host::{GameHost, H, W};
use j2me_me::Image;

/// Baseline v207 JAR filename under `_originals/`.
const BASELINE_JAR: &str = "Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar";

/// Walk up from this crate to `_originals/` and return the baseline JAR.
fn baseline_jar() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = dir.join("_originals").join(BASELINE_JAR);
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            panic!(
                "could not locate `_originals/{BASELINE_JAR}` above {} — run `just bootstrap`",
                env!("CARGO_MANIFEST_DIR")
            );
        }
    }
}

/// A host booted to the publisher splash (state 10) with the default deterministic
/// seams.
fn host() -> GameHost {
    GameHost::new(&baseline_jar()).expect("construct the host (boot reaches the splash)")
}

/// Count of pixels that differ between two same-length frames.
fn pixel_diff(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// THE HEADLINE — boot paints a real, non-blank publisher-splash frame, and driving
/// the loop ADVANCES the animation (the painted frame changes as the logo slides),
/// with no panic.
#[test]
fn the_live_loop_paints_and_advances_the_publisher_splash() {
    let mut host = host();

    // Boot seated a real frame on the logo screen (state 10).
    assert_eq!(
        host.title_state(),
        10,
        "boots to the logo/publisher-splash state"
    );
    let s0 = analyze(host.frame());
    assert!(
        s0.is_real_frame(),
        "the initial splash frame is blank/degenerate: {s0:?}"
    );

    let initial = host.frame().pixels().to_vec();

    // Frozen control (R3): sampling the frame again WITHOUT driving the loop yields
    // an identical frame — the change below is caused by the loop, not by time.
    assert_eq!(
        pixel_diff(&initial, host.frame().pixels()),
        0,
        "an un-driven frame changed on its own"
    );

    // Drive live frames: the logo slides down (`animTick` converges to `halfH`), so
    // the painted frame changes and every frame stays real.
    let mut max_frame_diff = 0usize;
    for f in 0..6u32 {
        host.tick(&[]);
        assert!(
            analyze(host.frame()).is_real_frame(),
            "splash frame {f} went blank/degenerate"
        );
        max_frame_diff = max_frame_diff.max(pixel_diff(&initial, host.frame().pixels()));
    }

    assert!(
        max_frame_diff > 0,
        "the painted frame never changed across the live loop — the animation is frozen"
    );
    assert_eq!(
        host.title_state(),
        10,
        "still on the logo screen after the slide"
    );
}

/// R3 caused-by control: a *frozen* host (constructed, never driven) does not
/// repaint — so the very assertion the live test makes (the frame changes) is RED
/// without the loop. The change is caused by driving `run_one_frame`, not by
/// construction.
#[test]
fn a_frozen_host_never_repaints() {
    let host = host();
    let frozen = host.frame().pixels().to_vec();
    // Wall time passes, but nothing is driven: the frame is fixed.
    assert_eq!(
        pixel_diff(&frozen, host.frame().pixels()),
        0,
        "an un-driven frame changed on its own — the world advanced without the loop"
    );
}

/// R3 proven-red: a blank framebuffer FAILS the same `is_real_frame` predicate the
/// live frame passes — so the gate genuinely rejects a blank frame.
#[test]
fn a_blank_framebuffer_is_rejected() {
    let blank = Image::create_mutable(W, H).expect("blank framebuffer");
    let blank_stats = analyze(&blank);
    assert!(
        !blank_stats.is_real_frame(),
        "a blank (all-white) framebuffer wrongly passed as a real frame: {blank_stats:?}"
    );
    assert_eq!(
        blank_stats.white, blank_stats.total,
        "blank must be all white"
    );

    // Contrast: the live splash frame the gate accepts.
    let host = host();
    assert!(
        analyze(host.frame()).is_real_frame(),
        "the live splash frame must pass the predicate the blank one fails"
    );
}

/// The process-exit half of the gate: the `--exit-after-frames` binary runs the
/// headless loop and exits 0 (proving the host drives the game and tears down
/// cleanly without a display).
#[test]
fn the_headless_binary_exits_cleanly() {
    let jar = baseline_jar();
    let exe = env!("CARGO_BIN_EXE_heroes-lore-wind-of-soltia-linux");
    let status = Command::new(exe)
        .args(["--jar"])
        .arg(&jar)
        .args(["--exit-after-frames", "12"])
        .status()
        .expect("run the heroes-lore-wind-of-soltia-linux binary");
    assert!(
        status.success(),
        "the headless binary exited with {status} (expected 0)"
    );
}
