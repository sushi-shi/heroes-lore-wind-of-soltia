//! Boot-entry gate for the stateful transliteration.
//!
//! Constructs the [`Game`], runs the BOOT-ENTRY lifecycle
//! (`new GameMIDlet()` → `startApp()` → `GameLoop.create` → the private
//! constructor + `applyDifficultyFps`/`setFps` → `start`), and asserts it
//! completes without panic in the expected init state — the constructed loop,
//! before the first rendered frame. A negative control proves the gate turns red
//! on a one-unit init error.
//!
//! This mirrors the leaf-decoder oracles' discipline (a real run + a proven-red
//! negative control), scaled to a lifecycle gate rather than a corpus cross-check.

use heroes_lore_wind_of_soltia_game_xlat::{game_midlet, game_state, Game};

/// Runs `new GameMIDlet()` then the one-shot `startApp()`.
fn run_boot() -> Game {
    let mut g = Game::new();
    game_midlet::construct(&mut g); // new GameMIDlet()
    game_midlet::start_app(&mut g); // startApp()
    g
}

#[test]
fn boot_reaches_the_constructed_loop_before_first_paint() {
    let g = run_boot();

    // --- GameMIDlet lifecycle ---
    assert!(g.application.instance, "GameMIDlet singleton registered");
    assert!(g.application.started, "startApp ran the one-shot startup");
    assert!(
        g.application.display,
        "Display.getDisplay acquired a display"
    );
    assert!(
        !g.display.has_current(),
        "no Displayable set yet (TitleScreen deferred)"
    );

    // --- GameLoop constructed as the singleton ---
    assert!(
        g.game_loop.instance,
        "GameLoop.instance created by create()"
    );
    assert!(g.game_loop.display, "the loop holds the display reference");
    assert!(g.game_loop.boot_pending, "start() set bootPending");
    assert!(!g.game_loop.stopped);

    // --- GameLoop.<init> field values ---
    //   volume     = AudioManager.maxVolume (snapshot: bw.<clinit> => 10)
    //   soundEnabled = !Debug.fullVersion   (snapshot: x.<clinit>  => false)
    assert_eq!(g.game_loop.volume, 10);
    assert!(g.game_loop.sound_enabled);
    assert!(!g.game_loop.has_created_character);
    assert!(!g.game_loop.auto_text_advance);
    assert!(g.game_loop.camera_follow);
    assert_eq!(g.game_loop.difficulty, 2);
    assert_eq!(g.game_loop.frame_delay, 14, "frameDelayTable[difficulty=2]");
    assert_eq!(
        g.game_loop.frame_target_ms,
        1000 / 14,
        "setFps(14) => 1000/14 == 71"
    );
    assert_eq!(g.game_loop.progress_flags, 8, "progressFlags |= 8");
    assert_eq!(g.game_loop.progress_data, 0);
    assert_eq!(g.game_loop.frame_delay_table, vec![8, 10, 14, 18]);
    assert!(
        g.game_loop.lock,
        "the frame monitor was allocated in bs.<clinit>"
    );

    // --- <clinit> trigger order (lazy JVM class init) ---
    // bs.<clinit> fired at the static create() call...
    assert!(
        g.game_loop_class_initialized,
        "GameLoop <clinit> fired at create()"
    );
    // ...n.<clinit> did NOT: the boot-entry never touches GameState.
    assert!(
        !g.game_state_class_initialized,
        "GameState <clinit> is lazy: not reached on the boot-entry path"
    );

    // --- the TitleScreen render path is deferred: no Canvas yet ---
    assert!(g.canvas.is_none());

    // --- GameState static-init values (post-<clinit>, eagerly encoded by Default) ---
    assert_eq!(g.game_state.clear_bonus_table, vec![60, 30, 10]);
    assert_eq!(g.game_state.save_key, vec![5, 11, 8, 81, 3, 20]);
    assert_eq!(
        g.game_state.class_start_table,
        vec![0, 22, 4, 60, 5, 36, 77, 10, 18]
    );
    assert_eq!(g.game_state.save_slots, vec!["/k", "/s", "/w"]);
    assert_eq!(g.game_state.screen, 0);
    assert_eq!(g.game_state.next_state, 0);
    assert_eq!(g.game_state.switches.len(), 128);
    assert_eq!(g.game_state.flags.len(), 128);
    assert_eq!(g.game_state.class_start_flags.len(), 3);
    assert!(g
        .game_state
        .class_start_flags
        .iter()
        .all(|row| row.len() == 15));
    // The three per-class start rows (classes 6..8).
    assert!(g.game_state.class_start_flags[1][7]);
    assert!(!g.game_state.class_start_flags[1][1]);
}

#[test]
fn startapp_early_returns_on_resume() {
    let mut g = Game::new();
    game_midlet::construct(&mut g);
    game_midlet::start_app(&mut g);
    // Perturb a field a second construct() would reset...
    g.game_loop.volume = 3;
    // ...then resume: startApp must early-return on the `started` guard, not
    // rebuild the loop.
    game_midlet::start_app(&mut g);
    assert_eq!(
        g.game_loop.volume, 3,
        "resume must not rebuild the GameLoop (started guard)"
    );
}

#[test]
fn game_state_clinit_trigger_is_lazy_and_idempotent() {
    let mut g = run_boot();
    // Not triggered by boot...
    assert!(!g.game_state_class_initialized);
    // ...its first active use fires n.<clinit> once...
    game_state::class_init(&mut g);
    assert!(g.game_state_class_initialized);
    let save_key = g.game_state.save_key.clone();
    // ...and a second trigger is a no-op.
    game_state::class_init(&mut g);
    assert_eq!(g.game_state.save_key, save_key);
}

#[test]
fn pause_and_destroy_run_without_panic() {
    let mut g = run_boot();
    game_midlet::pause_app(&mut g);
    game_midlet::destroy_app(&mut g, true); // exit() -> notifyDestroyed()
}

/// Negative control (GATES.md R3): a one-unit perturbation of the expected init
/// state must turn the gate red. The difficulty-2 `frameTargetMs` is
/// `1000 / 14 == 71`, NOT 70; asserting 70 must fail, proving the assertions bite.
#[test]
#[should_panic(expected = "negative control")]
fn boot_negative_control_wrong_frame_target_rejected() {
    let g = run_boot();
    assert_eq!(g.game_loop.frame_target_ms, 70, "negative control");
}
