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

pub mod asset_cache;
pub mod asset_loader;
pub mod audio_manager;
pub mod base_canvas;
pub mod bitmap_font;
pub mod class_confirm_menu;
pub mod class_select_menu;
pub mod font_manager;
pub mod game;
pub mod game_loop;
pub mod game_midlet;
pub mod game_screen;
pub mod game_state;
pub mod main_menu;
pub mod menu;
pub mod resources;
pub mod sound_player;
pub mod start_trait_menu;
pub mod string_table;
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
pub mod entity;
pub mod game_map;
pub mod hero;
pub mod map_object;

pub use game::Game;
