//! The `Game` aggregator — the top-level owner of the strict transliteration's
//! stateful classes.
//!
//! Java `static` fields become fields on one `*State` struct per class (see
//! `docs/TRANSLITERATION.md`, *Statics and ownership*, and
//! `java/reconstruction/ownership.tsv`); this struct aggregates one `*State` per
//! ported class plus the device runtime it references (the `j2me-me`
//! [`Display`](j2me_me::Display) / [`Canvas`](j2me_me::Canvas)) and the
//! [`Clock`](j2me_jvm::Clock). Methods are free functions taking `&mut Game` and
//! threading the owners — never `self`-methods, which would conflict the moment a
//! method needs two sub-structs at once.
//!
//! ## `<clinit>` machinery
//!
//! Each `*State`'s [`Default`] eagerly reproduces the class's field initializers
//! and `static{}` block in JVM order (a ready-to-use post-class-load value). A
//! `class_init(&mut Game)` per class reproduces the `<clinit>` at its JVM
//! *trigger* point — the first active use of the class — guarded one-shot by the
//! `*_class_initialized` flags here so the lazy initialization *order* stays
//! observable. On the boot-entry path only `GameLoop`'s `<clinit>` fires (at the
//! static `create()` call); `GameState`'s is not reached until a later
//! menu/world use, exactly as the JVM would defer it.

use crate::game_loop::GameLoopState;
use crate::game_midlet::ApplicationState;
use crate::game_state::GameStateData;

/// The whole transliterated program's state: the ported classes' `*State`s, the
/// device runtime they reference, and the JVM class-init trigger guards.
#[derive(Debug)]
pub struct Game {
    /// The LCDUI display (owned here; `GameMIDlet.display` / `GameLoop.display`
    /// are references to it). Acquired by `GameMIDlet.startApp` via
    /// `Display.getDisplay(this)`.
    pub display: j2me_me::Display,
    /// The current MIDP surface. `None` until `GameLoop.run()` constructs the
    /// `TitleScreen` (a `BaseCanvas`) — DEFERRED to the render increment.
    pub canvas: Option<j2me_me::Canvas>,
    /// `System.currentTimeMillis()` source. Read by `GameLoop.markFrameStart` /
    /// `throttle` in the deferred run-loop; deterministic for tests.
    pub clock: j2me_jvm::VirtualClock,

    /// `rpg.GameMIDlet` state.
    pub application: ApplicationState,
    /// `bs` / `GameLoop` state.
    pub game_loop: GameLoopState,
    /// `n` / `GameState` state.
    pub game_state: GameStateData,

    /// One-shot guard: has `bs.<clinit>` fired on the executed path?
    pub game_loop_class_initialized: bool,
    /// One-shot guard: has `n.<clinit>` fired on the executed path?
    pub game_state_class_initialized: bool,
}

impl Game {
    /// Constructs the initial program state: a fresh device Display, no current
    /// Canvas yet, and every `*State` at its post-class-load [`Default`]. No
    /// `<clinit>` trigger has fired yet (the guards start `false`).
    pub fn new() -> Self {
        Game {
            display: j2me_me::Display::default(),
            canvas: None,
            clock: j2me_jvm::VirtualClock::new(0),
            application: ApplicationState::default(),
            game_loop: GameLoopState::default(),
            game_state: GameStateData::default(),
            game_loop_class_initialized: false,
            game_state_class_initialized: false,
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
