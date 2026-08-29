//! Transliterated from `java/src/main/java/defpackage/AssetCache.java`
//! (original `ce.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! ## ANTI-BOG boundary
//!
//! `AssetCache` (`ce`) is a global bag of ~79 `static` banks with ~40 `load*`/
//! `unload*` entry points. This increment ports **only** what the first-frame
//! title (logo) render path touches, per the milestone's anti-bog rule:
//!
//! - the raw byte gateway `readResource` (`ce.a:(Ljava/lang/String;)[B`) + its
//!   shared `readBuffer` static;
//! - `loadLogo` (`ce.w:()V`) — `new PngMerger("/img/logo").allImages()` → the
//!   `logoFrames` bank that `TitleScreen.paint` (state 10) draws.
//!
//! Everything else — the sprite banks (`heroFrames`/`enemyFrames`/…,
//! `spriteBanks`), the string tables, `loadGlobalUi`/`loadTitleScreen`/
//! `loadMainMenuAssets`/`assembleSprites`/… and the HUD/menu image banks — is
//! **DEFERRED**. Correspondingly [`AssetCacheState`] carries only the handful of
//! statics this path reads (see `java/reconstruction/ownership.tsv`); the rest are
//! not modelled yet.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `ce.a:(Ljava/lang/String;)[B => []` (readResource — the byte accumulation is
//! stream/collection calls, no arithmetic opcodes), `ce.w:()V => []` (loadLogo).

use crate::game::Game;
use crate::png_merger;
use j2me_me::Image;

/// Java `ce` / `AssetCache` state — **partial** (anti-bog). Only the statics the
/// first-frame title (logo) render path reads are modelled; the ~75 other banks
/// are deferred (see the module header and `ownership.tsv`).
#[derive(Debug, Default)]
pub struct AssetCacheState {
    /// `public static Image[] logoFrames;` (obf `ce.i`) — title logo frames,
    /// filled by [`load_logo`], drawn by `TitleScreen.paint`. `None` == Java null.
    pub logo_frames: Option<Vec<Image>>,
    /// `public static Image[] titleBgFrames;` (obf `ce.j`) — DEFERRED
    /// (`loadTitleScreen`, state-1 title draw); `None` until then.
    pub title_bg_frames: Option<Vec<Image>>,
    /// `public static Image[] titleMenuFrames;` (obf `ce.k`) — DEFERRED; `None`.
    pub title_menu_frames: Option<Vec<Image>>,
    /// `public static byte[] readBuffer = new byte[512];` (obf `ce.n`) — the
    /// shared 512-byte scratch [`read_resource`] slurps through.
    pub read_buffer: Vec<i8>,
}

impl AssetCacheState {
    /// Post-`<clinit>` state: `readBuffer = new byte[512]`, every ported bank at
    /// its JVM default (null → `None`).
    pub fn new() -> Self {
        AssetCacheState {
            logo_frames: None,
            title_bg_frames: None,
            title_menu_frames: None,
            // static byte[] readBuffer = new byte[512];
            read_buffer: vec![0i8; 512],
        }
    }
}

/// `public static final byte[] readResource(String path)`
/// (`ce.a:(Ljava/lang/String;)[B => []`). Slurps a JAR resource fully into a
/// `byte[]` through the shared [`AssetCacheState::read_buffer`], or `null`
/// (`None`) when the resource is absent. `System.gc()` is a no-op; the trailing
/// `while (GameState.screen == 15) Thread.sleep(100)` loading-screen spin is
/// preserved (never entered here — `screen != 15`).
// The trailing `while (GameState.screen == 15)` spin waits for ANOTHER thread to
// leave the loading screen; in the single-threaded transliteration `screen` is
// not mutated in the (empty) body, which clippy flags. The loop is preserved
// verbatim (faithful to the Java) — it is never entered on this path
// (`screen != 15`).
#[allow(clippy::while_immutable_condition)]
pub fn read_resource(g: &mut Game, path: &str) -> Option<Vec<i8>> {
    // System.gc();   — no-op.
    // InputStream in = getResourceAsStream(path); if (in == null) return null;
    //   The classpath is the host resource seam; a copy is taken so the shared
    //   readBuffer below can be written without aliasing the bank.
    let source: Vec<i8> = g.resources.get(path)?.to_vec();
    // ByteArrayOutputStream out = new ByteArrayOutputStream();
    let mut out: Vec<i8> = Vec::new();
    // while ((read = in.read(readBuffer)) != -1) out.write(readBuffer, 0, read);
    let mut pos: usize = 0;
    while pos < source.len() {
        let read: usize = core::cmp::min(g.asset_cache.read_buffer.len(), source.len() - pos);
        g.asset_cache.read_buffer[..read].copy_from_slice(&source[pos..pos + read]);
        out.extend_from_slice(&g.asset_cache.read_buffer[..read]);
        pos += read;
    }
    // result = out.toByteArray(); out.close();
    let result: Option<Vec<i8>> = Some(out);
    // while (GameState.screen == 15) { Thread.sleep(100L); }   — screen != 15 here.
    while g.game_state.screen == 15 {
        // Thread.sleep(100L) — the loading-screen spin; not entered on this path.
    }
    result
}

/// `public static final void loadLogo()` (`ce.w:()V => []`): loads the title logo
/// frames (`/img/logo`). The `catch (IOException)` is subsumed — the atlas is
/// present on the classpath.
pub fn load_logo(g: &mut Game) {
    // logoFrames = new PngMerger("/img/logo").allImages();
    let mut merger = png_merger::construct(g, "/img/logo");
    let frames = png_merger::all_images(g, &mut merger);
    g.asset_cache.logo_frames = Some(frames);
}
