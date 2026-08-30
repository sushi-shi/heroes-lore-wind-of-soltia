//! The headless MENU-NAVIGATION smoke — proves keyboard input reaches the ported
//! New-Game menu chain.
//!
//! The windowed [`shell`](heroes_lore_wind_of_soltia_linux::shell) maps each `winit`
//! key to a raw Nokia code via [`keymap::nokia_code`] and feeds it to
//! [`GameHost::tick`], which enqueues it on the `j2me-me` serial queue (R9);
//! `run_one_frame` then drains it into the current screen's `keyPressed`. This test
//! drives that EXACT path with no display: it injects the SAME codes the live window
//! produces (derived here through `keymap::nokia_code`, so the winit→Nokia mapping is
//! part of the proof) and walks
//!
//!   splash → title → any-key → MainMenu
//!          → FIRE (New Game)        → ClassSelectMenu
//!          → FIRE (select class)    → ClassConfirmMenu
//!          → LEFT (Yes) + FIRE      → StartTraitMenu
//!
//! ## Two witnesses, because the deep art is deferred
//!
//! Where the port renders genuinely distinct art the test asserts the **frame
//! changes**: the title, the parchment main menu, and the class-select screen are
//! three visibly different frames, so a change there can only come from the injected
//! key reaching `keyPressed`. But `ClassSelectMenu`/`ClassConfirmMenu`/`StartTraitMenu`
//! currently share ONE partial paint (parchment + title plate + heading + panel) —
//! their distinguishing art (class names, portraits, Yes/No, guardian previews) is
//! DEFERRED — so those three do NOT differ pixel-for-pixel yet. For the deeper pushes
//! the witness is therefore [`GameHost::menu_depth`], which walks the real `Menu.child`
//! chain: it advances 0 → 1 → 2 → 3 only if each key actually reached the menu and
//! pushed the next child. Together they prove the whole chain is driven by input.
//!
//! Reads content from `_originals`; FAILS loudly if absent (R1/R2).

use std::path::PathBuf;

use heroes_lore_wind_of_soltia_linux::frame::analyze;
use heroes_lore_wind_of_soltia_linux::host::{GameHost, InputEvent};
use heroes_lore_wind_of_soltia_linux::keymap::nokia_code;
use winit::keyboard::KeyCode;

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

fn host() -> GameHost {
    GameHost::new(&baseline_jar()).expect("construct the host (boot reaches the splash)")
}

/// The raw Nokia code the live window emits for a physical key — the mapping under
/// test. Every key this navigation uses maps to a code (never `None`).
fn code(key: KeyCode) -> i32 {
    nokia_code(key).unwrap_or_else(|| panic!("keymap has no Nokia code for {key:?}"))
}

/// A stable hash of the current framebuffer (FNV-1a over the ARGB pixels), so two
/// screens can be compared for "changed" without keeping every buffer around.
fn frame_hash(host: &GameHost) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in host.frame().pixels() {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn assert_real(host: &GameHost, what: &str) {
    assert!(
        analyze(host.frame()).is_real_frame(),
        "the {what} frame is blank/degenerate: {:?}",
        analyze(host.frame())
    );
}

/// Frames run after each key so the menu's `logoFrame`/child render settles (the
/// child menu is painted on the frame AFTER the key is dispatched, since the owed
/// repaint drains before the key). Matches `menu_chain.rs`'s settle budget.
const SETTLE: u32 = 16;

/// Press a key (the live window's Press/Release pair) and settle. The press is
/// consumed on the first tick's serialized dispatch; the release is a no-op on the
/// menu route (`keyReleased` is not consumed there). Advances the injected clock a
/// frame period each tick, as the shell does.
fn press_and_settle(host: &mut GameHost, key: KeyCode) {
    let c = code(key);
    host.tick(&[InputEvent::Press(c), InputEvent::Release(c)]);
    for _ in 0..SETTLE {
        host.tick(&[]);
    }
}

/// Drive the state-10 publisher splash until it flips to the state-1 title on its
/// own (no input needed), then paint a couple of settled title frames.
fn drive_to_title(host: &mut GameHost) {
    let mut guard = 0u32;
    while host.title_state() != 1 {
        host.tick(&[]);
        guard += 1;
        assert!(
            guard < 10_000,
            "the state-10 splash never transitioned to the state-1 title"
        );
    }
    for _ in 0..3 {
        host.tick(&[]);
    }
}

/// THE MENU-NAV GATE. Boot → title → any-key → the full New-Game chain, asserting a
/// real frame at every stop, a CHANGED frame where the art is real, and the menu
/// child chain advancing 0 → 1 → 2 → 3 the whole way down.
#[test]
fn keys_drive_the_new_game_menu_chain() {
    let mut host = host();

    // Boot seated the splash (state 10); let it slide to the title.
    assert_eq!(host.title_state(), 10, "boots to the logo/publisher splash");
    drive_to_title(&mut host);
    assert_eq!(host.title_state(), 1, "reached the state-1 title");
    assert_eq!(host.menu_depth(), 0, "no menu open at the title");
    assert_real(&host, "title");
    let title = frame_hash(&host);

    // Any-key at the title → enterStoryMode → the parchment main menu (NEW GAME).
    // Enter is the natural "select"; at the state-1 title ANY key leaves.
    press_and_settle(&mut host, KeyCode::Enter);
    assert_real(&host, "main-menu");
    let main_menu = frame_hash(&host);
    assert_ne!(
        main_menu, title,
        "any-key at the title did not change the frame — input never reached keyPressed"
    );
    assert_eq!(
        host.menu_depth(),
        0,
        "at the main menu, no child pushed yet"
    );

    // FIRE on NEW GAME (cursorIndex 0, no save) → ClassSelectMenu (a distinct screen).
    press_and_settle(&mut host, KeyCode::Enter);
    assert_real(&host, "class-select");
    let class_select = frame_hash(&host);
    assert_ne!(
        class_select, main_menu,
        "FIRE on NEW GAME did not change the frame — the class picker was not shown"
    );
    assert_eq!(
        host.menu_depth(),
        1,
        "FIRE on NEW GAME must push ClassSelectMenu"
    );

    // FIRE on the default class → ClassConfirmMenu. Its distinguishing art (Yes/No,
    // portrait) is deferred, so the frame need not change; the child push is the
    // witness that FIRE reached ClassSelectMenu.handleKey.
    press_and_settle(&mut host, KeyCode::Enter);
    assert_real(&host, "class-confirm");
    assert_eq!(
        host.menu_depth(),
        2,
        "FIRE on the class must push ClassConfirmMenu"
    );

    // LEFT moves the confirm cursor 1 (No) → 0 (Yes); FIRE on Yes → StartTraitMenu.
    press_and_settle(&mut host, KeyCode::ArrowLeft);
    press_and_settle(&mut host, KeyCode::Enter);
    assert_real(&host, "start-trait");
    assert_eq!(
        host.menu_depth(),
        3,
        "confirm-Yes must push StartTraitMenu — the full chain is reached"
    );

    // The three screens that DO render distinct art are pairwise distinct.
    assert_ne!(title, main_menu, "title vs main-menu");
    assert_ne!(main_menu, class_select, "main-menu vs class-select");
    assert_ne!(title, class_select, "title vs class-select");
}
