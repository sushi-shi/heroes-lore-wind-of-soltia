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
//!   `logoFrames` bank that `TitleScreen.paint` (state 10) draws;
//! - `loadTitleScreen` (`ce.y:()V`) — `/img/title1` → `titleBgFrames` (the title
//!   art) and `/img/title2` → `titleMenuFrames` (the fluttering birds) that
//!   `TitleScreen.paint` (state 1) draws.
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

use crate::base_canvas;
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
    /// `public static Image[] titleBgFrames;` (obf `ce.j`) — the state-1 title art
    /// (`/img/title1`), filled by [`load_title_screen`]. `None` == Java null.
    pub title_bg_frames: Option<Vec<Image>>,
    /// `public static Image[] titleMenuFrames;` (obf `ce.k`) — the fluttering-bird
    /// sprites (`/img/title2`, 10 = 5 base + 5 mirrored), filled by
    /// [`load_title_screen`]. `None` == Java null.
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

/// `public static final void loadTitleScreen()` (`ce.y:()V`): loads the state-1
/// title art (`/img/title1` → `titleBgFrames`) and the fluttering-bird sprites
/// (`/img/title2` → `titleMenuFrames`, base frames 0..4 plus their mirrors 5..9).
/// The `catch (IOException)` is subsumed (the atlases are present); the trailing
/// `AudioManager.loadClip((byte) 22)` (the title jingle) is DEFERRED (audio not
/// ported).
pub fn load_title_screen(g: &mut Game) {
    // PngMerger title = new PngMerger("/img/title1");
    let mut title = png_merger::construct(g, "/img/title1");
    // titleBgFrames = title.allImages();
    let frames = png_merger::all_images(g, &mut title);
    g.asset_cache.title_bg_frames = Some(frames);
    // BaseCanvas.yieldTick();
    base_canvas::yield_tick(g);
    // title = new PngMerger("/img/title2");
    let mut title = png_merger::construct(g, "/img/title2");
    // title.preloadAll = true;
    title.preload_all = true;
    // titleMenuFrames = new Image[10];
    let mut menu: Vec<Option<Image>> = (0..10).map(|_| None).collect();
    // for (int i = 0; i < 5; i++) { titleMenuFrames[i] = title.image(i);
    //   titleMenuFrames[i + 5] = title.imageMirrored(i); BaseCanvas.yieldTick(); }
    let mut i: i32 = 0;
    while i < 5 {
        let img = png_merger::image(g, &mut title, i);
        menu[i as usize] = Some(img);
        let mirrored = png_merger::image_mirrored(g, &mut title, i);
        menu[i.wrapping_add(5) as usize] = Some(mirrored);
        base_canvas::yield_tick(g);
        i = i.wrapping_add(1);
    }
    // BaseCanvas.yieldTick();
    base_canvas::yield_tick(g);
    // AudioManager.loadClip((byte) 22);   — DEFERRED (audio not ported).
    g.asset_cache.title_menu_frames =
        Some(menu.into_iter().map(|o| o.expect("title2 frame")).collect());
}

/// `public static final byte[] loadItemRecord(byte itemId, byte record)`. Opens
/// `/itm/<zero-padded itemId>`, skips `record` length-framed records, and returns
/// the `record`-th record's content (the `[u8 recLen][recLen bytes]` framing read
/// through `InputStream.read`/`skip`). `null` (`None`) when the resource is absent
/// (the `catch (IOException)` path). Drives `Item.load` (see `crate::item`).
pub fn load_item_record(g: &mut Game, item_id: i8, record: i8) -> Option<Vec<i8>> {
    // String idText = String.valueOf((int) itemId);
    let mut id_text = format!("{}", item_id as i32);
    // if (itemId < 10) idText = "0" + idText;
    if item_id < 10 {
        id_text = format!("0{id_text}");
    }
    // InputStream in = getResourceAsStream("/itm/" + idText);
    let src: Vec<i8> = g.resources.get(&format!("/itm/{id_text}"))?.to_vec();
    let mut pos: usize = 0;
    // for (int i = 0; i < record; i++) in.skip(in.read());   (in.read() = unsigned recLen)
    let mut i: i32 = 0;
    while i < record as i32 {
        let len = (src[pos] as i32) & 255;
        pos += 1;
        pos = pos.wrapping_add(len as usize);
        i = i.wrapping_add(1);
    }
    // result = new byte[in.read()]; in.read(result);
    let rec_len = ((src[pos] as i32) & 255) as usize;
    pos += 1;
    let mut result = vec![0i8; rec_len];
    result.copy_from_slice(&src[pos..pos + rec_len]);
    Some(result)
}

/// `public static final byte[] loadShopItemData()` — `readResource("/itm/forshop")`.
pub fn load_shop_item_data(g: &mut Game) -> Option<Vec<i8>> {
    read_resource(g, "/itm/forshop")
}
