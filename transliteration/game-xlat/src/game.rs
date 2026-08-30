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

use crate::app_config::AppConfigState;
use crate::asset_cache::AssetCacheState;
use crate::audio_manager::AudioManagerState;
use crate::base_canvas::BaseCanvasState;
use crate::byte_util::ByteUtilState;
use crate::debug::DebugState;
use crate::entity::{EntityArena, EntityState};
use crate::font_manager::FontManagerState;
use crate::game_loop::GameLoopState;
use crate::game_map::GameMapClassState;
use crate::game_midlet::ApplicationState;
use crate::game_screen::GameScreenState;
use crate::game_state::GameStateData;
use crate::main_menu::MainMenuState;
use crate::resources::ResourceBank;
use crate::string_table::StringTableData;
use crate::title_screen::TitleScreenState;

/// Which `BaseCanvas` is currently shown — the concrete type of `GameLoop.current`.
/// The transliteration models the two reachable screens (`TitleScreen`,
/// `GameScreen`) with one shared `j2me-me` [`Canvas`](j2me_me::Canvas) +
/// framebuffer; this discriminator selects which screen's `paint` / `keyPressed`
/// the frame loop dispatches. `Title` until `GameLoop.showGameScreen` swaps
/// `current` to a `GameScreen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrentScreen {
    /// `GameLoop.current instanceof TitleScreen`.
    #[default]
    Title,
    /// `GameLoop.current instanceof GameScreen`.
    Game,
}

/// The whole transliterated program's state: the ported classes' `*State`s, the
/// device runtime they reference, and the JVM class-init trigger guards.
#[derive(Debug)]
pub struct Game {
    /// The LCDUI display (owned here; `GameMIDlet.display` / `GameLoop.display`
    /// are references to it). Acquired by `GameMIDlet.startApp` via
    /// `Display.getDisplay(this)`.
    pub display: j2me_me::Display,
    /// The current MIDP surface (a `BaseCanvas`). `None` until
    /// `TitleScreen`'s constructor materialises it on the render path.
    pub canvas: Option<j2me_me::Canvas>,
    /// Which screen `GameLoop.current` points at (see [`CurrentScreen`]) — selects
    /// the frame loop's `paint`/`keyPressed` dispatch target.
    pub current_screen: CurrentScreen,
    /// The ARGB framebuffer that IS the rendered frame — the `Image` the paint
    /// `Graphics` rasterises into. `None` until the `TitleScreen`/`Canvas` exists.
    pub screen: Option<j2me_me::Image>,
    /// `System.currentTimeMillis()` source. Read by `GameLoop.markFrameStart` /
    /// `throttle` / `sleepFor`; deterministic for tests.
    pub clock: j2me_jvm::VirtualClock,
    /// The JAR classpath seam behind `AssetCache.readResource` /
    /// `getResourceAsStream` (a host boundary; see [`ResourceBank`]).
    pub resources: ResourceBank,
    /// The `javax.microedition.media` (MMAPI) runtime — the `Player` arena +
    /// ordered host-audio op sink that `SoundPlayer` (`ci`) drives. A device
    /// runtime owned here, like [`display`](Self::display).
    pub media: j2me_me::MediaRuntime,
    /// The shared `Entity` (`ck`) heap — the arena every entity record lives in;
    /// `GameState.hero` and `GameMap.entities` hold handles into it. A host/heap
    /// seam like [`resources`](Self::resources) / [`rms`](Self::rms); not a Java
    /// static (no `ownership.tsv` row).
    pub entity_arena: EntityArena,

    /// `rpg.GameMIDlet` state.
    pub application: ApplicationState,
    /// `bs` / `GameLoop` state.
    pub game_loop: GameLoopState,
    /// `n` / `GameState` state.
    pub game_state: GameStateData,
    /// `r` / `BaseCanvas` state (geometry + loading counters + shown-canvas
    /// instance fields).
    pub base_canvas: BaseCanvasState,
    /// `bg` / `TitleScreen` state.
    pub title_screen: TitleScreenState,
    /// `as` / `GameScreen` state (PARTIAL — only the geometry + `case 9` main-menu
    /// dispatch; see `game_screen`).
    pub game_screen: GameScreenState,
    /// `bf` / `MainMenu` state (the front menu + its `cb`/`Menu` base fields).
    pub main_menu: MainMenuState,
    /// `bw` / `AudioManager` state (the `snd/` clip pool + volume/mixer state).
    pub audio: AudioManagerState,
    /// `ce` / `AssetCache` state (PARTIAL — only the title/logo render-path banks
    /// are modelled; see `asset_cache`).
    pub asset_cache: AssetCacheState,
    /// `bh` / `FontManager` state (PARTIAL — only the title text path's fonts +
    /// `versionText`/`titleFooter` labels; see `font_manager`).
    pub font_manager: FontManagerState,
    /// `cj` / `StringTable` singleton state (the loaded lang blob + `get`).
    pub string_table: StringTableData,
    /// `h` / `ByteUtil` state (the shared `Random`). `new Random()` is time-seeded
    /// on device; a fixed seed is used here for reproducibility (a determinism
    /// seam — `randRange` is not exercised on the single first-frame drive).
    pub byte_util: ByteUtilState,

    // --- World data model (field-layer foundation; see `entity`, `game_map`). The
    //     active `GameMap` instance and the hero handle live in `game_state`
    //     (`map` / `hero`); these carry the two classes' persistent statics. ---
    /// `ck` / `Entity` class state — the shared `static Random rng`.
    pub entity: EntityState,
    /// `ae` / `GameMap` mutable class statics (`minimapScale` / `lastTilesetId`).
    pub game_map_class: GameMapClassState,

    // --- Independent leaf/util/save classes increment (parallel lane: merge these
    //     three fields into the aggregator). ---
    /// The host-owned MIDP record-store namespace behind `au`/`RmsFile` (a host
    /// seam like `display`; no `ownership.tsv` row — not a Java static).
    pub rms: j2me_me::RmsRuntime,
    /// `w` / `AppConfig` state (the JAD app-property/demo config).
    pub app_config: AppConfigState,
    /// `x` / `Debug` state (the mirrored full-version flag).
    pub debug: DebugState,

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
            current_screen: CurrentScreen::default(),
            screen: None,
            clock: j2me_jvm::VirtualClock::new(0),
            resources: ResourceBank::new(),
            media: j2me_me::MediaRuntime::new(),
            entity_arena: EntityArena::new(),
            application: ApplicationState::default(),
            game_loop: GameLoopState::default(),
            game_state: GameStateData::default(),
            audio: AudioManagerState::default(),
            base_canvas: BaseCanvasState::default(),
            title_screen: TitleScreenState::default(),
            game_screen: GameScreenState::default(),
            main_menu: MainMenuState::default(),
            asset_cache: AssetCacheState::new(),
            font_manager: FontManagerState::default(),
            string_table: StringTableData::default(),
            byte_util: ByteUtilState::seeded(0),
            entity: EntityState::default(),
            game_map_class: GameMapClassState::default(),
            rms: j2me_me::RmsRuntime::new(),
            app_config: AppConfigState::default(),
            debug: DebugState::default(),
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
