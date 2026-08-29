//! The strict transliteration of Heroes Lore: Wind of Soltia (J2ME).
//!
//! This crate is *provably the same program* as the recovered Java: the
//! executable spec, not idiomatic Rust. Every integer op routes through
//! `j2me-jvm`; every device call goes through `j2me-me`. Do not refactor it.
//!
//! Phase-3: the leaf decoder classes land first (pure integer utilities with no
//! `Game` state yet), each gated by an independent cross-check oracle under
//! `tests/`. The stateful gameplay classes land in later increments.

pub mod adler32;
pub mod byte_util;
pub mod crc32;
pub mod png_merger;
