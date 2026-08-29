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

pub mod asset_cache;
pub mod base_canvas;
pub mod game;
pub mod game_loop;
pub mod game_midlet;
pub mod game_state;
pub mod resources;
pub mod title_screen;

pub use game::Game;
