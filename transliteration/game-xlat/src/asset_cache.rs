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
//! The world tile-render path adds one more: `mapTiles` (`ce.e`) + its
//! `unloadMapTiles`/`unloadMainMenuAssets` companions, filled by
//! [`crate::game_map::load`] and drawn by `GameMap.drawTiles`.
//!
//! Everything else — the sprite banks (`heroFrames`/`enemyFrames`/…,
//! `spriteBanks`), the string tables, `loadGlobalUi`/`loadCommonAssets`/
//! `assembleSprites`/… and the HUD/menu image banks — is **DEFERRED**.
//! Correspondingly [`AssetCacheState`] carries only the handful of statics these
//! paths read (see `java/reconstruction/ownership.tsv`); the rest are not modelled yet.
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
    /// `public static Image[] menuFrames;` (obf `ce.l`) — the main-menu frame/border
    /// atlas (`/sgui/mm/etc`), filled by [`load_main_menu_assets`] and drawn by
    /// `MainMenu.paint`/`drawMenuPanel`. `None` == Java null.
    pub menu_frames: Option<Vec<Image>>,
    /// `public static Image[] mapTiles;` (obf `ce.e`) — the map tileset frames
    /// (`/m/t/t<NN>`), loaded lazily by [`crate::game_map::load`] and drawn by
    /// `GameMap.drawTiles`. `None` == Java null (also the reload-guard sentinel).
    pub map_tiles: Option<Vec<Image>>,

    // ---- Hero sprite system (byte-script frame tables + decoded atlas banks) ----
    /// `public static Object[] heroFrames;` (obf `ce.a`) — the hero equipment
    /// animation scripts, keyed `(pose*36)+((dir-1)*9)+layer` (9 layers/cell, 11
    /// poses → `Object[396]`). Each element is a decoded per-frame draw script
    /// (`byte[]`) or Java null. Filled by
    /// [`crate::asset_loader::load_sprite_bank`], read by
    /// [`crate::game_screen::draw_frame`]/[`crate::game_screen::draw_frame_group`].
    /// `None` == Java null (the whole array unallocated until `loadHeroEquipSprites`).
    pub hero_frames: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] weaponPreviewFrames;` (obf `ce.c`) — weapon-preview
    /// frame scripts, keyed `(row*4)+col`. Written only by `loadSpriteBank`'s
    /// `weaponPreview` branch (the class-select preview, DEFERRED); `None` == null.
    pub weapon_preview_frames: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] mageAuraScripts = null;` (obf `ce.b`) — mage (class 8)
    /// extra shield/aura frame scripts. Set only by `loadMageShieldFrames` (DEFERRED);
    /// `None` == null.
    pub mage_aura_scripts: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Image[][] spriteBanks = new Image[38][];` (obf `ce.a`) — the
    /// decoded atlas images per sprite-bank slot (0 armor, 1 body, 2 head, 3 weapon,
    /// 4 aura, 5 shield; +6 = the mirrored twin). Each slot is either null (`None`)
    /// or a lazily-filled `Image[]` (`Vec<Option<Image>>`). Indexed by
    /// [`crate::game_screen::draw_frame`] via a script's bank byte.
    pub sprite_banks: Vec<Option<Vec<Option<Image>>>>,
    /// `public static Image entityShadow;` (obf `ce.u`) — the ground shadow drawn under
    /// the hero/enemy/npc (`/img/etcui` frame 3). Filled by [`load_in_game_ui`], drawn
    /// by [`crate::hero::paint`]. `None` == Java null.
    pub entity_shadow: Option<Image>,

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
            menu_frames: None,
            map_tiles: None,
            // heroFrames / weaponPreviewFrames / mageAuraScripts are declared null.
            hero_frames: None,
            weapon_preview_frames: None,
            mage_aura_scripts: None,
            // static Image[][] spriteBanks = new Image[38][];  (38 null bank slots)
            sprite_banks: (0..38).map(|_| None).collect(),
            entity_shadow: None,
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

/// `public static final void unloadLogo()` (`ce.x:()V => []`): `logoFrames = null`.
/// Called by `TitleScreen.keyPressed` (state 1) on the title → main-menu key.
pub fn unload_logo(g: &mut Game) {
    // logoFrames = null;
    g.asset_cache.logo_frames = None;
}

/// `public static final void unloadTitleScreen()` (`ce.z:()V => []`): drops the
/// title frames. `AudioManager.unloadClip((byte) 22)` is DEFERRED (audio not
/// ported). Called by `TitleScreen.keyPressed` (state 1).
pub fn unload_title_screen(g: &mut Game) {
    // titleBgFrames = null;
    g.asset_cache.title_bg_frames = None;
    // titleMenuFrames = null;
    g.asset_cache.title_menu_frames = None;
    // AudioManager.unloadClip((byte) 22);   — DEFERRED (audio not ported).
}

/// `public static final void loadMainMenuAssets()` (`ce.A:()V => []`).
///
/// ANTI-BOG: only `menuFrames = new PngMerger("/sgui/mm/etc").allImages()` — the
/// frame/border atlas the main-menu render (`MainMenu.paint` / `drawMenuPanel` /
/// the selection sprite) draws — is ported. The class-portrait faces (`classFaces`,
/// `/sgui/mm/face` + gray variants) and the guardian previews (`menuGuardianPreview`,
/// `/grd/0..2`) are read only by the class-select / preview screens (not the
/// main-menu render) and are DEFERRED.
pub fn load_main_menu_assets(g: &mut Game) {
    // (DEFERRED: classFaces = new PngMerger("/sgui/mm/face") ... image/imageGray)
    // menuFrames = new PngMerger("/sgui/mm/etc").allImages();
    let mut etc = png_merger::construct(g, "/sgui/mm/etc");
    let frames = png_merger::all_images(g, &mut etc);
    g.asset_cache.menu_frames = Some(frames);
    // (DEFERRED: menuGuardianPreview = new Image[3][2] ... /grd/0..2)
}

/// `public static final void unloadMainMenuAssets()` (`ce.B:()V => []`): drops the
/// main-menu assets. Only `menuFrames` is modelled here; `classFaces` and
/// `menuGuardianPreview` are DEFERRED banks (not read on the world-render path).
/// Called by `GameState.newGame` when leaving the menu for a new game.
pub fn unload_main_menu_assets(g: &mut Game) {
    // menuFrames = null;
    g.asset_cache.menu_frames = None;
    // classFaces = null;  menuGuardianPreview = (Image[][]) null;  — DEFERRED banks.
}

/// `public static final void loadInGameUi()` (`ce.g:()V`) — **PARTIAL** (anti-bog).
///
/// The full method decodes the shared in-game UI atlas (`/img/uifrm` HUD frame +
/// dialog border, `/img/etcui` glyph/marker set, the `/char/lvup` level-up effect).
/// This milestone slice ports **only** `entityShadow = new PngMerger("/img/etcui").image(3)`
/// — the ground shadow [`crate::hero::paint`] draws under the hero. The HUD frame,
/// dialog border, floater/number/marker icons and the level-up sprite assembly are
/// DEFERRED (read only by the DEFERRED `drawHud` / dialogue / floater lanes). The
/// discarded `etcui.image(2)` probe is a no-op here.
pub fn load_in_game_ui(g: &mut Game) {
    // PngMerger etcui = new PngMerger("/img/etcui"); etcui.preloadAll = true;
    let mut etcui = png_merger::construct(g, "/img/etcui");
    etcui.preload_all = true;
    // (DEFERRED: uifrm hudFrame/dialogBorder; floaterIcon2/3; numberFont1..4;
    //  dropItemMarker/dropGoldMarker; skillChargeFill; statPointAlert; lvup assembly.)
    // entityShadow = etcui.image(3);
    let shadow = png_merger::image(g, &mut etcui, 3);
    g.asset_cache.entity_shadow = Some(shadow);
}

/// `public static final void unloadMapTiles()` (`ce.b:()V => []`): `mapTiles = null`.
/// [`crate::game_map::load`] calls it before reloading a differing tileset.
pub fn unload_map_tiles(g: &mut Game) {
    // mapTiles = null;
    g.asset_cache.map_tiles = None;
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
