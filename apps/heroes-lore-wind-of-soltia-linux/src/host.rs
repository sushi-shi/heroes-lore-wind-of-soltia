//! [`GameHost`] — the game-driving core, with NO windowing.
//!
//! It owns the transliterated [`Game`], which itself owns the `j2me-me` device
//! [`Canvas`](j2me_me::Canvas) (the MIDP serial paint/input queue, R9), the ARGB
//! framebuffer [`Image`] (`Game::screen`), and the [`VirtualClock`] the frame loop
//! reads. [`GameHost::tick`] advances one frame through the transliteration's single
//! frame-drive entry [`run_one_frame`](game_loop::run_one_frame) (the `bs.run`
//! synchronized section → one `TitleScreen.paint`), and [`GameHost::frame`] hands
//! back the painted framebuffer. Because it carries no window, the headless smoke
//! drives it directly with no display.
//!
//! ## The injected clock
//! Gothic's host threads a `Box<dyn Clock>` into its loop entry; here the `Game`
//! OWNS a concrete [`VirtualClock`] that `run_one_frame` reads (`markFrameStart` /
//! `sleepFor`), so "injecting the clock" means the host seeds that owned clock's
//! starting value ([`GameHost::with_clock_start`]) and advances it per route command
//! ([`GameHost::advance_clock`]) — the same deterministic time base the reference
//! capture's substituted clock uses.
//!
//! ## What actually runs — and where the port stops
//! The transliteration currently ports the FIRST-FRAME render path: boot entry
//! (`GameMIDlet` → `GameLoop`), the `TitleScreen` constructor (its `Canvas` +
//! framebuffer), `AssetCache.loadLogo` (`/img/logo`), `startLogo` (arms the state-10
//! logo animation), and `paint`'s **state-10** branch — the Hands-On Mobile
//! publisher splash sliding to centre. The **state-1** title draw (the "Heroes Lore"
//! logo, birds, PRESS-ANY-KEY, version) and `keyPressed`/`enterStoryMode` are
//! DEFERRED (the other lane). So this host boots straight to the publisher splash
//! (no async loader, no sound prompt — the declared boot deviation, mirroring the
//! reference route's `port=skip`), animates the splash, and — because `keyPressed`
//! is not yet ported — does NOT respond to input: input is enqueued on the serial
//! queue (the real path is exercised) but not yet consumed. That input gap, and the
//! deferred state-1 title, are limits of the port, not of the host, and are exactly
//! what the frame oracle reports honestly against the reference.

use std::path::Path;

use heroes_lore_wind_of_soltia_game_xlat::byte_util::ByteUtilState;
use heroes_lore_wind_of_soltia_game_xlat::{
    asset_cache, base_canvas, game_loop, game_midlet, title_screen, Game,
};
use j2me_me::Image;

use crate::jar::{self, JarError};

/// The MIDP `Canvas`/framebuffer dimensions (the v207 build's captured frames are
/// 240×320; the device fact lives in `base_canvas`).
pub const W: i32 = base_canvas::DEVICE_WIDTH;
/// See [`W`].
pub const H: i32 = base_canvas::DEVICE_HEIGHT;

/// The shared-RNG seed the reference route pins (`seed 305419896`, this game's
/// determinism knob). Seeding `ByteUtil.rng` to it makes any RNG-driven frame (the
/// state-1 title's fluttering birds, once ported) reproducible; the current
/// state-10 publisher-splash render is RNG-free, so it matches regardless.
pub const RNG_SEED: i64 = 305_419_896;

/// The game clock's starting value under capture. A nonzero base keeps the
/// frame-pacing reads (`markFrameStart` / `sleepFor`) away from any `== 0` boot
/// sentinel and mirrors the reference driver's substituted-clock base.
pub const CLOCK_START_MS: i64 = 1_000_000;

/// One host input event, as the raw Nokia code the game's `keyPressed(int)` sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// `keyPressed(code)`.
    Press(i32),
    /// `keyReleased(code)`.
    Release(i32),
}

/// Fatal host construction errors (loud; never a silent empty result, R1/R2).
#[derive(Debug)]
pub enum HostError {
    /// The `_originals` JAR could not be materialized into the classpath seam.
    Jar(JarError),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::Jar(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<JarError> for HostError {
    fn from(e: JarError) -> Self {
        HostError::Jar(e)
    }
}

/// The windowless game driver.
pub struct GameHost {
    game: Game,
}

impl GameHost {
    /// Construct the game from a real `_originals` JAR and boot it to the publisher
    /// splash, using the default deterministic clock base and RNG seed. See
    /// [`GameHost::with_clock_start`].
    pub fn new(jar_path: &Path) -> Result<GameHost, HostError> {
        GameHost::with_clock_start(jar_path, CLOCK_START_MS, RNG_SEED)
    }

    /// Construct the game with an explicit clock base and RNG seed, boot it to the
    /// state-10 publisher splash, and paint the initial frame so [`GameHost::frame`]
    /// is valid before the first [`GameHost::tick`].
    pub fn with_clock_start(
        jar_path: &Path,
        clock_start_ms: i64,
        rng_seed: i64,
    ) -> Result<GameHost, HostError> {
        let mut game = Game::new();

        // Deterministic seams the reference capture also pins. `Game` OWNS its
        // `VirtualClock`, so seeding it here IS the clock injection.
        game.clock.set(clock_start_ms);
        game.byte_util = ByteUtilState::seeded(rng_seed);

        // Classpath seam <- every JAR entry (mirrors tests/first_frame.rs).
        jar::load_into_bank(jar_path, &mut game.resources)?;

        boot_to_logo(&mut game);

        let mut host = GameHost { game };
        // Seat frame 0: one loop iteration services the owed paint (`set_current`
        // scheduled it) into the framebuffer, so `frame()` is valid immediately.
        host.run_one();
        Ok(host)
    }

    /// Advance one frame. Deliver `inputs` into the R9 serial queue, then run the
    /// single frame-drive entry [`run_one_frame`](game_loop::run_one_frame) once: it
    /// runs the `synchronized (lock)` critical section (`markFrameStart` →
    /// `flushKey` → `requestRepaint`) and MIDP's serialized dispatch of the owed
    /// repaint into one `TitleScreen.paint`.
    ///
    /// The game's `keyPressed` is DEFERRED in the current transliteration, so the
    /// enqueued keys are not yet consumed (the recorded input gap) — enqueuing keeps
    /// the real input PATH exercised for when it lands.
    pub fn tick(&mut self, inputs: &[InputEvent]) {
        if let Some(canvas) = self.game.canvas.as_mut() {
            for ev in inputs {
                match ev {
                    InputEvent::Press(code) => canvas.key_pressed(*code),
                    InputEvent::Release(code) => canvas.key_released(*code),
                }
            }
        }
        self.run_one();
    }

    /// The most recently painted framebuffer (ARGB `0xAARRGGBB`). Boot seats it, so
    /// it is always present.
    pub fn frame(&self) -> &Image {
        self.game
            .screen
            .as_ref()
            .expect("framebuffer (seated at construction)")
    }

    /// The `TitleScreen` state machine (`bg.state`): 10 = the logo/publisher splash,
    /// 1 = the (deferred) title. Surfaced so the smoke can assert which screen the
    /// port is on without reading pixels.
    pub fn title_state(&self) -> i8 {
        self.game.title_screen.state
    }

    /// Advance the injected game clock by `ms`. The route driver calls this per
    /// command so the port's game-time tracks the reference's deterministic clock.
    pub fn advance_clock(&self, ms: i64) {
        self.game.clock.advance(ms);
    }

    /// Reseed the shared RNG (`ByteUtil.rng`) — the route's `seed <n>` command,
    /// which the reference capture uses to pin RNG-driven animation just before a
    /// shot. Reseeds `new Random(seed)` in place, exactly like the reference driver.
    pub fn reseed_rng(&mut self, seed: i64) {
        self.game.byte_util = ByteUtilState::seeded(seed);
    }

    /// One iteration of the frame loop against the owned `Game`.
    fn run_one(&mut self) {
        game_loop::run_one_frame(&mut self.game);
    }
}

/// Boot the transliteration to the state-10 publisher splash, ready to paint —
/// exactly the sequence `tests/first_frame.rs` drives (which stands in for the
/// deferred async `boot()` loader by porting only the loaders the splash paint
/// touches).
fn boot_to_logo(game: &mut Game) {
    // Boot entry (ported): GameMIDlet -> GameLoop.create/start.
    game_midlet::construct(game);
    game_midlet::start_app(game);

    // Title render setup: materialise the Canvas + framebuffer + BaseCanvas
    // geometry, load the /img/logo atlas, arm the state-10 logo animation.
    title_screen::construct(game);
    asset_cache::load_logo(game);
    title_screen::start_logo(game);

    // Display.setCurrent(titleScreen) — schedules the first (owed) paint.
    let Game {
        display, canvas, ..
    } = game;
    display.set_current(None, canvas.as_mut().expect("TitleScreen canvas"));
}
