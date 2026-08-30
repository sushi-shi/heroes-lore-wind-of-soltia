//! The strict transliteration of Heroes Lore: Wind of Soltia (J2ME).
//!
//! This crate is *provably the same program* as the recovered Java: the
//! executable spec, not idiomatic Rust. Every integer op routes through
//! `j2me-jvm`; every device call goes through `j2me-me`. Do not refactor it.
//!
//! Phase-3: the leaf decoder classes land first (pure integer utilities with no
//! `Game` state yet), each gated by an independent cross-check oracle under
//! `tests/`. The stateful gameplay classes land in later increments.
//!
//! Phase-4 (stateful): the [`Game`] aggregator + one `*State` per ported class
//! (`docs/TRANSLITERATION.md`, *Statics and ownership*;
//! `java/reconstruction/ownership.tsv`). The BOOT-ENTRY lifecycle lands first —
//! [`game_midlet`] (`rpg.GameMIDlet`) → [`game_loop`] (`bs`) construction →
//! [`game_state`] (`n`) `<clinit>` — up to the constructed loop, before the first
//! rendered frame. Gated by `tests/boot.rs`.

pub mod adler32;
pub mod byte_util;
pub mod crc32;
pub mod png_merger;

// Independent leaf/util/save classes (this increment): the save cipher, the RMS
// "XFile" wrapper, the direction/element constant tables, the JAD app-property
// config, the assertion/build-flag helper, and the z-ordered entity linked-list.
// None sit on the title/render path. See `docs/TRANSLITERATION.md`.
pub mod app_config;
pub mod debug;
pub mod directions;
pub mod entity_list;
pub mod rms_file;
pub mod save_cipher;

pub mod about_screen;
pub mod asset_cache;
pub mod asset_loader;
pub mod audio_manager;
pub mod base_canvas;
pub mod bitmap_font;
pub mod buy_sell_dialog;
pub mod character_menu;
pub mod class_confirm_menu;
pub mod class_select_menu;
pub mod combine_menu;
pub mod confirm_dialog;
pub mod continue_menu;
pub mod cost_confirm_dialog;
pub mod enchant_menu;
pub mod equip_tab;
pub mod font_manager;
pub mod game;
pub mod game_loop;
pub mod game_midlet;
pub mod game_screen;
pub mod game_state;
pub mod guardian_tab;
pub mod item_picker_list;
pub mod items_tab;
pub mod main_menu;
pub mod menu;
pub mod options_menu;
pub mod popup_menu;
pub mod refine_menu;
pub mod resources;
pub mod scroll_caption;
pub mod sell_list;
pub mod shop_item_list;
pub mod shop_menu;
pub mod skill_tab;
pub mod sound_player;
pub mod start_trait_menu;
pub mod stat_alloc_menu;
pub mod status_page;
pub mod string_table;
pub mod system_tab;
pub mod text_table;
pub mod title_screen;
pub mod wrap_font;

// The item hierarchy (`Item -> Equipment -> Armor -> Weapon`, `ad -> e -> t -> l`)
// plus the carried-inventory store (`ItemBag`, `g`). Flattened onto one `Item`
// struct with an `ItemClass` discriminant (see `item`); gated by
// `tests/item_oracle.rs` (record cross-check) and `tests/item_bag.rs`.
pub mod armor;
pub mod equipment;
pub mod item;
pub mod item_bag;
pub mod weapon;

// The on-map entity/world data model: the `Entity -> Battler -> Hero` and
// `Entity -> MapObject` hierarchy (flattened onto the shared `EntityArena` that
// extends `entity_list`), plus the per-level `GameMap`. Field-layer foundation for
// the render + world-logic lanes.
pub mod battler;
pub mod boss;
pub mod enemy;
pub mod enemy_type;
pub mod entity;
pub mod game_map;
pub mod hero;
pub mod map_object;

// The on-battler overlay family: the abstract `Overlay` base (`f`) and its three leaf
// subclasses — the floating damage/heal popups (`Floater`, `aw`), the buff/debuff
// status icons (`StatusIcon`, `cf`), and the guardian summon/cast animation
// (`GuardianCastFx`, `bj`). These are the real tagged union backing a `Battler`'s
// `floaters`/`statuses` lists. Their drawing bottoms out in DEFERRED `AssetCache`
// overlay banks.
pub mod floater;
pub mod guardian_cast_fx;
pub mod overlay;
pub mod status_icon;

// The transient/world entity leaves layered on the `Entity`/`Battler` hierarchy: the
// animated visual `Effect` (`y`) and its moving `Projectile` (`i`) subclass, plus the
// town-folk `Npc` (`ac`). Each adds an `EntityData` variant (see `entity`) and, where
// it renders on the map, a `game_map::draw_entities` / entity-update dispatch arm.
// Their guardian sprite/anim banks bottom out in DEFERRED `AssetCache` banks.
pub mod effect;
pub mod npc;
pub mod projectile;

pub use game::Game;
