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

use crate::about_screen::AboutScreenState;
use crate::app_config::AppConfigState;
use crate::asset_cache::AssetCacheState;
use crate::asset_loader::AssetLoaderState;
use crate::audio_manager::AudioManagerState;
use crate::base_canvas::BaseCanvasState;
use crate::buy_sell_dialog::BuySellDialogState;
use crate::byte_util::ByteUtilState;
use crate::character_menu::CharacterMenuState;
use crate::class_confirm_menu::ClassConfirmMenuState;
use crate::class_select_menu::ClassSelectMenuState;
use crate::combine_menu::CombineMenuState;
use crate::confirm_dialog::ConfirmDialogState;
use crate::continue_menu::ContinueMenuState;
use crate::cost_confirm_dialog::CostConfirmDialogState;
use crate::debug::DebugState;
use crate::enchant_menu::EnchantMenuState;
use crate::enemy_type::EnemyTypeState;
use crate::entity::{EntityArena, EntityState};
use crate::font_manager::FontManagerState;
use crate::game_loop::GameLoopState;
use crate::game_map::GameMapClassState;
use crate::game_midlet::ApplicationState;
use crate::game_screen::GameScreenState;
use crate::game_state::GameStateData;
use crate::item_picker_list::ItemPickerListState;
use crate::main_menu::MainMenuState;
use crate::options_menu::OptionsMenuState;
use crate::popup_menu::PopupMenuState;
use crate::refine_menu::RefineMenuState;
use crate::resources::ResourceBank;
use crate::sell_list::SellListState;
use crate::shop_item_list::ShopItemListState;
use crate::shop_menu::ShopMenuState;
use crate::start_trait_menu::StartTraitMenuState;
use crate::stat_alloc_menu::StatAllocMenuState;
use crate::status_page::StatusPageState;
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
    /// `c` / `ClassSelectMenu` state (the starting-class picker + its `cb`/`Menu`
    /// base fields). Not a Java static/singleton — a per-instance child of
    /// `MainMenu`; the flat model carries the reusable child slot as a `Game` field,
    /// linked/unlinked by `MainMenu`'s [`MenuChild`](crate::menu::MenuChild).
    pub class_select_menu: ClassSelectMenuState,
    /// `by` / `ClassConfirmMenu` state (the class Yes/No confirm + its `cb`/`Menu`
    /// base fields + the chosen `classId`). Not a Java static — a per-instance child
    /// of `ClassSelectMenu`; the flat model carries the reusable child slot as a
    /// `Game` field, linked/unlinked by the [`MenuChild`](crate::menu::MenuChild).
    pub class_confirm_menu: ClassConfirmMenuState,
    /// `bk` / `StartTraitMenu` state (the starting-guardian picker + its `cb`/`Menu`
    /// base fields + the guardian/confirmation state; its `startGame` launches
    /// `GameState.newGame`). Not a Java static — a per-instance child of
    /// `ClassConfirmMenu`; the flat model carries the reusable child slot as a `Game`
    /// field, linked/unlinked by the [`MenuChild`](crate::menu::MenuChild).
    pub start_trait_menu: StartTraitMenuState,
    /// `af` / `PopupMenu` state (the general-purpose dialog + its `cb`/`Menu` base
    /// fields). Not a Java static — a per-instance child pushed by any menu's
    /// `showPopup`/`showMessage`; the flat model carries the reusable child slot as a
    /// `Game` field, linked/unlinked by the [`MenuChild`](crate::menu::MenuChild).
    pub popup_menu: PopupMenuState,
    /// `am` / `ConfirmDialog` state (the two-line Yes/No dialog + its `cb`/`Menu` base
    /// fields). Not a Java static — a per-instance child (creator `SkillTab`); the flat
    /// model carries the reusable child slot as a `Game` field, linked by the
    /// [`MenuChild`](crate::menu::MenuChild).
    pub confirm_dialog: ConfirmDialogState,
    /// `a` / `ContinueMenu` state (the load-game slot picker + its `cb`/`Menu` base
    /// fields + the save blob). Not a Java static — a per-instance child of `MainMenu`;
    /// the flat model carries the reusable child slot as a `Game` field, linked by the
    /// [`MenuChild`](crate::menu::MenuChild).
    pub continue_menu: ContinueMenuState,
    /// `be` / `OptionsMenu` state (the options screen + its `cb`/`Menu` base fields).
    /// Not a Java static — a per-instance child of `MainMenu` (or `SystemTab`); the flat
    /// model carries the reusable child slot as a `Game` field, linked by the
    /// [`MenuChild`](crate::menu::MenuChild).
    pub options_menu: OptionsMenuState,
    /// `bl` / `AboutScreen` state (the credits/about screen + its `cb`/`Menu` base
    /// fields). Not a Java static — a per-instance child of `MainMenu` (FIRE case 4);
    /// the flat model carries the reusable child slot as a `Game` field, linked by the
    /// [`MenuChild`](crate::menu::MenuChild).
    pub about_screen: AboutScreenState,
    /// `m` / `ItemPickerList` state (the generic scrollable item-slot picker + its
    /// `cb`/`Menu` base fields). Not a Java static — a per-instance child pushed by the
    /// (unported) equip/craft menus; the flat model carries the reusable child slot as
    /// a `Game` field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub item_picker_list: ItemPickerListState,
    /// `bb` / `SellList` state (the shop's sell-from-bag list; extends
    /// `ItemPickerList`). Not a Java static — a per-instance child pushed by the
    /// (unported) shop menu; the flat model carries the reusable child slot as a `Game`
    /// field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub sell_list: SellListState,
    /// `bp` / `ShopMenu` state (the merchant shop singleton — its `Menu` base +
    /// `shopStock` instance field + the `panelX`/`panelY`/`text`/`singleton` statics).
    /// A separate menu root from `MainMenu`, reached via the world's screen-6 dispatch
    /// (that wiring is DEFERRED to the game-state lane).
    pub shop_menu: ShopMenuState,
    /// `v` / `ShopItemList` state (the shop's per-category stock list + its `cb`/`Menu`
    /// base fields). Not a Java static — a per-instance child of `ShopMenu`; the flat
    /// model carries the reusable child slot as a `Game` field, linked by the
    /// [`MenuChild`](crate::menu::MenuChild).
    pub shop_item_list: ShopItemListState,
    /// `ab` / `BuySellDialog` state (the buy/sell confirm dialog + its `cb`/`Menu` base
    /// fields). Not a Java static — a per-instance child pushed by `ShopItemList`/
    /// `SellList`; the flat model carries the reusable child slot as a `Game` field,
    /// linked by the [`MenuChild`](crate::menu::MenuChild).
    pub buy_sell_dialog: BuySellDialogState,
    /// `ai` / `CharacterMenu` state (the six-tab character-menu singleton — its `Menu`
    /// base + the per-instance equip/guardian snapshots + the
    /// `panelX`/`panelY`/`text`/`singleton` statics). A separate menu root from
    /// `MainMenu`, reached via the world's screen-5 dispatch (that wiring is DEFERRED to
    /// the game-state lane).
    pub character_menu: CharacterMenuState,
    /// `q` / `StatusPage` state (the character menu's status tab + its `cb`/`Menu` base
    /// fields). Not a Java static — a per-instance child of `CharacterMenu` (or the
    /// level-up flow); the flat model carries the reusable child slot as a `Game` field,
    /// linked by the [`MenuChild`](crate::menu::MenuChild).
    pub status_page: StatusPageState,
    /// `bi` / `StatAllocMenu` state (the level-up stat-allocation dialog + its `cb`/`Menu`
    /// base fields + the pending/remaining allocation). Not a Java static — a per-instance
    /// child of `StatusPage`; the flat model carries the reusable child slot as a `Game`
    /// field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub stat_alloc_menu: StatAllocMenuState,
    /// `ax` / `RefineMenu` state (the item-refinery hub singleton — its `Menu` base +
    /// the `panelX`/`panelY`/`text`/`singleton` statics). A separate menu root from
    /// `MainMenu`, reached via the world's screen-7 dispatch (that wiring is DEFERRED to
    /// the game-state lane).
    pub refine_menu: RefineMenuState,
    /// `ap` / `EnchantMenu` state (the refinery's armor-enchant screen + its `cb`/`Menu`
    /// base fields + the per-instance `armor`/`material` picks). Not a Java static — a
    /// per-instance child of `RefineMenu`; the flat model carries the reusable child slot
    /// as a `Game` field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub enchant_menu: EnchantMenuState,
    /// `k` / `CombineMenu` state (the refinery's item-combine screen + its `cb`/`Menu`
    /// base fields + the per-instance `craftSlots`). Not a Java static — a per-instance
    /// child of `RefineMenu`; the flat model carries the reusable child slot as a `Game`
    /// field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub combine_menu: CombineMenuState,
    /// `bo` / `CostConfirmDialog` state (the combine fee confirm dialog + its `cb`/`Menu`
    /// base fields + the per-instance title/lines/cost/tag). Not a Java static — a
    /// per-instance child of `CombineMenu`; the flat model carries the reusable child slot
    /// as a `Game` field, linked by the [`MenuChild`](crate::menu::MenuChild).
    pub cost_confirm_dialog: CostConfirmDialogState,
    /// `bw` / `AudioManager` state (the `snd/` clip pool + volume/mixer state).
    pub audio: AudioManagerState,
    /// `ce` / `AssetCache` state (PARTIAL — only the title/logo render-path banks
    /// are modelled; see `asset_cache`).
    pub asset_cache: AssetCacheState,
    /// `bu` / `AssetLoader` state (PARTIAL — only the `phase` static; the sprite
    /// script tables + `commonLoaded` are DEFERRED; see `asset_loader`).
    pub asset_loader: AssetLoaderState,
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
    /// `j` / `EnemyType` class state — the `static EnemyType[] types` template array
    /// (`attackHitFrame` is a `static final` const in `enemy_type`).
    pub enemy_type: EnemyTypeState,

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
            class_select_menu: ClassSelectMenuState::default(),
            class_confirm_menu: ClassConfirmMenuState::default(),
            start_trait_menu: StartTraitMenuState::default(),
            popup_menu: PopupMenuState::default(),
            confirm_dialog: ConfirmDialogState::default(),
            continue_menu: ContinueMenuState::default(),
            options_menu: OptionsMenuState::default(),
            about_screen: AboutScreenState::default(),
            item_picker_list: ItemPickerListState::default(),
            sell_list: SellListState::default(),
            shop_menu: ShopMenuState::default(),
            shop_item_list: ShopItemListState::default(),
            buy_sell_dialog: BuySellDialogState::default(),
            character_menu: CharacterMenuState::default(),
            status_page: StatusPageState::default(),
            stat_alloc_menu: StatAllocMenuState::default(),
            refine_menu: RefineMenuState::default(),
            enchant_menu: EnchantMenuState::default(),
            combine_menu: CombineMenuState::default(),
            cost_confirm_dialog: CostConfirmDialogState::default(),
            asset_cache: AssetCacheState::new(),
            asset_loader: AssetLoaderState::default(),
            font_manager: FontManagerState::default(),
            string_table: StringTableData::default(),
            byte_util: ByteUtilState::seeded(0),
            entity: EntityState::default(),
            game_map_class: GameMapClassState::default(),
            enemy_type: EnemyTypeState::default(),
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
