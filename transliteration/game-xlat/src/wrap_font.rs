//! Transliterated from `java/src/main/java/defpackage/WrapFont.java`
//! (original `b.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! `WrapFont extends BitmapFont`, adding only greedy word-wrapping (`wrap` /
//! `wrapInto`) and the `create` factories `FontManager` builds its fonts through.
//! The subclass declares **no fields** — a `WrapFont` instance is structurally a
//! [`BitmapFontState`], so the factories construct one directly.
//!
//! ANTI-BOG: `wrap` / `wrapInto` are the wrapped-text-block path; the title paint
//! draws `versionText` / `titleFooter` through `FontManager.drawChars` /
//! `drawCharsCentered` (no wrapping), so they are **DEFERRED**.
//!
//! Opcode shapes (R8): `b.<init>:(Ljava/lang/String;IIZ)V => []`,
//! `b.a:(Ljava/lang/String;IIZ)Laz; => []` (create),
//! `b.a:(Ljava/lang/String;IZ)Laz; => []` (create single-colour).

use crate::bitmap_font::{self, BitmapFontState};
use crate::game::Game;

/// `public static final BitmapFont create(String name, int primaryColor, int secondaryColor, boolean forceUppercase)`
/// — `return new WrapFont(name, primaryColor, secondaryColor, forceUppercase);`.
/// The private `WrapFont(...)` constructor is just `super(...)` (BitmapFont.load).
pub fn create(
    g: &mut Game,
    name: &str,
    primary_color: i32,
    secondary_color: i32,
    force_uppercase: bool,
) -> BitmapFontState {
    bitmap_font::construct(g, name, primary_color, secondary_color, force_uppercase)
}

/// `public static final BitmapFont create(String name, int color, boolean forceUppercase)`
/// — `return create(name, color, -1, forceUppercase);`.
pub fn create_single(
    g: &mut Game,
    name: &str,
    color: i32,
    force_uppercase: bool,
) -> BitmapFontState {
    create(g, name, color, -1, force_uppercase)
}
