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
//! ## In-game UI / HUD image banks (this increment)
//!
//! The shared in-game UI and HUD image banks the world/menu render lanes deferred
//! are now ported here (each faithfully mirrors the `getResourceAsStream` /
//! `PngMerger` / `Image` conventions the title-path banks already use):
//!
//! - [`load_in_game_ui`]/[`unload_in_game_ui`] (`ce.g`/`ce.h`) — `/img/uifrm`
//!   → `hudFrame` (7) + `dialogBorder` (4); `/img/etcui` → `entityShadow`,
//!   `numberFont1..4`, `dropItemMarker`/`dropGoldMarker`/`skillChargeFill`; the
//!   `/char/lvup` level-up effect (`levelUpScript` + `spriteBanks[13]` via
//!   [`assemble_sprites`]);
//! - [`load_item_icons`]/[`unload_item_icons`] (`ce.p`/`ce.q`) — `/img/icoitm`;
//! - [`load_guardian_icons`]/[`unload_guardian_icons`] (`ce.t`/`ce.u`) —
//!   `/grd/grdico` → `guardianIcons` (6) + `guardianSkillIcons` (24);
//! - [`load_global_ui`] (`ce.o`) — `/img/glb` single images (`numberFont0`,
//!   `cursorArrow`, `slotFrame`, scroll arrows, `goldIcon`, `portraitFrame`,
//!   `statusPanelIcon`, `fractionSlash`) + `helpText` (`/sgui/help` `TextTable`);
//! - [`load_game_menu_icons`] (`ce.n`) — `/sgui/gmico` → `menuTabIcons` (6) +
//!   `equipSlotIcons` (5);
//! - [`load_shop_ui`]/[`unload_shop_ui`] (`ce.r`/`ce.s`) — `/sgui/shop`;
//! - [`load_status_effect_icons`]/[`unload_status_effect_icons`] (`ce.i`/`ce.j`) —
//!   `/img/keepst` + `/img/emoti`;
//! - [`assemble_sprites`] (`ce.a:(Z[BIBBLbr;)V`) — the frame-script → sprite-bank
//!   decoder [`load_in_game_ui`]'s level-up assembly (and the DEFERRED enemy/boss/
//!   guardian sprite loaders) drive.
//!
//! **DEFERRED sub-banks (a still-unported class/seam):** every `FontManager.loadLocaleImage`
//! feed — `floaterIcon2`/`floaterIcon3`/`statPointAlert` (in `load_in_game_ui`),
//! `statLabel1..5` (in `load_global_ui`), `shopBuyIcon`/`shopSellIcon` (in
//! `load_shop_ui`) — because `FontManager.loadLocaleImage` is DEFERRED (see
//! [`crate::font_manager`]). The fields are modelled (so the `unload*` clears stay
//! faithful) but never filled; a null bank safely no-ops when drawn. `commonText`
//! (`ce.g:Lz;`) is modelled but its only assignment lives in `TitleScreen.loadLanguage`
//! (DEFERRED there).
//!
//! Still **DEFERRED** (other lanes / unmodelled fields): the hero/enemy/boss/npc/
//! guardian sprite-script loaders (`loadEnemySprite`/`loadBossSprite`/`loadNpcSprite`/
//! `loadGuardianSprite`/`loadGuardian*`, `loadDeathEffects`, `loadAttackEffects`,
//! `resetEnemyBossBanks`, `bossSlot`, `unloadMapObjects`/`unloadMapNpcImages`) and
//! their `*Frames`/`mapObjects`/`attackFx*` banks — owned by the entity/enemy/boss/
//! guardian/map lanes. The string tables `heroText`/`guardianText`/`classSkillText`/…
//! stay DEFERRED except `helpText`/`commonText` above.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `ce.a:(Ljava/lang/String;)[B => []` (readResource — the byte accumulation is
//! stream/collection calls, no arithmetic opcodes), `ce.w:()V => []` (loadLogo),
//! `ce.g:()V => []` (loadInGameUi), `ce.h:()V => []` (unloadInGameUi),
//! `ce.p:()V => []` (loadItemIcons), `ce.q:()V => []` (unloadItemIcons),
//! `ce.o:()V => []` (loadGlobalUi), `ce.n:()V => []` (loadGameMenuIcons),
//! `ce.r:()V => []` (loadShopUi), `ce.s:()V => []` (unloadShopUi),
//! `ce.i:()V => []` (loadStatusEffectIcons), `ce.j:()V => []` (unloadStatusEffectIcons),
//! `ce.t:()V` / `ce.u:()V` (loadGuardianIcons/unload — the small-int index math
//! `(guardian*4)+skill`), `ce.a:(Z[BIBBLbr;)V` (assembleSprites — index math only).

use crate::base_canvas;
use crate::debug;
use crate::game::Game;
use crate::png_merger;
use crate::text_table::{self, TextTableState};
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

    // ---- Enemy/boss sprite-script banks (appended for the Enemy lane) ----
    // Each is `new Object[N]` in `<clinit>` (a non-null array of null elements),
    // read by [`crate::enemy_type`] (bind*) and [`crate::enemy::paint`] /
    // [`crate::enemy::resolve_attack`] / [`crate::enemy::death_effect`]. The
    // element load (`AssetLoader.loadSpriteBank`) is DEFERRED, so every element
    // stays null (`None`) in this slice; the array itself is allocated to match the
    // `<clinit>`, so an `enemyFrames[i]` read yields null (a no-op draw), not an NPE.
    /// `public static Object[] enemyFrames = new Object[60];` (obf `ce.e`) — enemy
    /// sprite frame scripts, keyed `(slot*12)+group` (0 walk, 4 attack, 8 cast). Each
    /// element is a `byte[]` draw script or Java null.
    pub enemy_frames: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] attackEffectScripts = new Object[5];` (obf `ce.f`) —
    /// enemy/boss attack-effect frame scripts, indexed by enemy/boss slot.
    pub attack_effect_scripts: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] deathFxScripts = new Object[3];` (obf `ce.g`) — enemy
    /// death/explosion frame scripts, indexed by `stats.size`.
    pub death_fx_scripts: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] bossFrames = new Object[80];` (obf `ce.h`) — boss
    /// sprite frame scripts, keyed `(slot*16)+(group*4)+dir`.
    pub boss_frames: Option<Vec<Option<Vec<i8>>>>,
    /// `public static Object[] guardianFrames = new Object[3];` (obf `ce.d`) — guardian
    /// pose frame-group scripts, indexed by skill slot. Captured by `GuardianCastFx`;
    /// the per-element load (`AssetCache.loadGuardianSprites`) is DEFERRED, so every
    /// element stays null (`None`) in this slice, matching the `<clinit>` allocation.
    pub guardian_frames: Option<Vec<Option<Vec<i8>>>>,

    // ---- In-game UI / HUD image banks (loadInGameUi + siblings) ----
    /// `public static Image[] hudFrame;` (obf `ce.q`) — the in-game HUD window-frame
    /// pieces (`/img/uifrm`, 7). Filled by [`load_in_game_ui`], drawn by
    /// `GameScreen.drawHudFrame`. `None` == Java null.
    pub hud_frame: Option<Vec<Image>>,
    /// `public static Image[] dialogBorder;` (obf `ce.r`) — the dialogue-border corner
    /// images (`/img/uifrm` 7/8 + mirrors, 4). Filled by [`load_in_game_ui`]. `None` == null.
    pub dialog_border: Option<Vec<Image>>,
    /// `public static Image floaterIcon3;` (obf `ce.s`) — kind-3 floater icon
    /// (`_img_etcui__0.png`). **DEFERRED feed**: assigned via `FontManager.loadLocaleImage`
    /// (see [`crate::font_manager`]); modelled so [`unload_in_game_ui`]'s clear stays
    /// faithful, but never filled. `None` == null.
    pub floater_icon3: Option<Image>,
    /// `public static Image floaterIcon2;` (obf `ce.t`) — kind-2 floater icon
    /// (`_img_etcui__1.png`). **DEFERRED feed** (`FontManager.loadLocaleImage`).
    pub floater_icon2: Option<Image>,
    /// `public static Image statPointAlert;` (obf `ce.v`) — the blinking stat-point/
    /// level-up alert on the HUD (`_img_etcui__4.png`). **DEFERRED feed**
    /// (`FontManager.loadLocaleImage`); modelled but never filled.
    pub stat_point_alert: Option<Image>,
    /// `public static Image numberFont1;` (obf `ce.w`) — digit glyph sheet style 1
    /// (`/img/etcui` img 5). Filled by [`load_in_game_ui`]. `None` == null.
    pub number_font1: Option<Image>,
    /// `public static Image numberFont2;` (obf `ce.x`) — digit glyph sheet style 2
    /// (`/img/etcui` img 6). Filled by [`load_in_game_ui`].
    pub number_font2: Option<Image>,
    /// `public static Image numberFont3;` (obf `ce.y`) — digit glyph sheet style 3
    /// (`/img/etcui` img 7). Filled by [`load_in_game_ui`].
    pub number_font3: Option<Image>,
    /// `public static Image numberFont4;` (obf `ce.z`) — digit glyph sheet style 4
    /// (`/img/etcui` img 8). Filled by [`load_in_game_ui`].
    pub number_font4: Option<Image>,
    /// `public static Image dropItemMarker;` (obf `ce.A`) — map item-drop marker
    /// (`/img/etcui` img 9). Filled by [`load_in_game_ui`].
    pub drop_item_marker: Option<Image>,
    /// `public static Image dropGoldMarker;` (obf `ce.B`) — map gold-drop marker
    /// (`/img/etcui` img 10). Filled by [`load_in_game_ui`].
    pub drop_gold_marker: Option<Image>,
    /// `public static Image skillChargeFill;` (obf `ce.C`) — HUD guardian-skill
    /// charge-fill overlay (`/img/etcui` img 11). Filled by [`load_in_game_ui`].
    pub skill_charge_fill: Option<Image>,
    /// `public static byte[] levelUpScript;` (obf `ce.h:[B`) — the level-up effect
    /// frame script (`/char/lvup.eif`); its flag bytes are rewritten in place by
    /// [`assemble_sprites`] to reference `spriteBanks[13]`. Filled by [`load_in_game_ui`].
    pub level_up_script: Option<Vec<i8>>,

    /// `public static Image[] itemIcons;` (obf `ce.d:[LImage`) — item-type icons
    /// (`/img/icoitm`). Filled by [`load_item_icons`], drawn by the bag/shop lists.
    pub item_icons: Option<Vec<Image>>,

    /// `public static Image[] guardianIcons;` (obf `ce.b:[LImage`) — guardian portrait
    /// icons (`/grd/grdico`, 6). Filled by [`load_guardian_icons`]. `None` == null.
    pub guardian_icons: Option<Vec<Image>>,
    /// `public static Image[] guardianSkillIcons;` (obf `ce.c:[LImage`) — guardian
    /// skill icons (`/grd/grdico`, 24 = 6 guardians x 4 skills). Filled by
    /// [`load_guardian_icons`], drawn by the guardian HUD/skill panels.
    pub guardian_skill_icons: Option<Vec<Image>>,

    // ---- Global UI single images (loadGlobalUi, `/img/glb`) ----
    /// `public static Image scrollUpArrow;` (obf `ce.k`) — scroll-up arrow (`/img/glb` 0).
    pub scroll_up_arrow: Option<Image>,
    /// `public static Image scrollDownArrow;` (obf `ce.n`) — scroll-down arrow (`/img/glb` 1).
    pub scroll_down_arrow: Option<Image>,
    /// `public static Image slotFrame;` (obf `ce.o`) — item-slot box frame (`/img/glb` 2).
    pub slot_frame: Option<Image>,
    /// `public static Image cursorArrow;` (obf `ce.d`) — menu/list cursor arrow (`/img/glb` 3).
    pub cursor_arrow: Option<Image>,
    /// `public static Image numberFont0;` (obf `ce.r`) — digit glyph sheet style 0
    /// (`/img/glb` 5). Filled by [`load_global_ui`], drawn by the HUD number blitter.
    pub number_font0: Option<Image>,
    /// `public static Image portraitFrame;` (obf `ce.l`) — portrait frame (`/img/glb` 6).
    pub portrait_frame: Option<Image>,
    /// `public static Image goldIcon;` (obf `ce.m`) — gold/currency icon (`/img/glb` 7).
    pub gold_icon: Option<Image>,
    /// `public static Image statusPanelIcon;` (obf `ce.a:LImage`) — status-page panel
    /// icon (`/img/glb` 8).
    pub status_panel_icon: Option<Image>,
    /// `public static Image fractionSlash;` (obf `ce.i:LImage`) — the "/" separator
    /// drawn by `BaseCanvas.drawFraction` (`/img/glb` 14).
    pub fraction_slash: Option<Image>,
    /// `public static TextTable helpText;` (obf `ce.e:Lz;`) — the help string table
    /// (`/sgui/help`). Filled by [`load_global_ui`]. `None` == Java null.
    pub help_text: Option<TextTableState>,

    // ---- Character-menu / equip icons (loadGameMenuIcons, `/sgui/gmico`) ----
    /// `public static Image[] menuTabIcons;` (obf `ce.n:[LImage`) — character-menu tab
    /// icons (`/sgui/gmico`, 6). Filled by [`load_game_menu_icons`].
    pub menu_tab_icons: Option<Vec<Image>>,
    /// `public static Image[] equipSlotIcons;` (obf `ce.o:[LImage`) — equip-slot icons
    /// (`/sgui/gmico`, 5). Filled by [`load_game_menu_icons`].
    pub equip_slot_icons: Option<Vec<Image>>,

    // ---- Shop UI (loadShopUi, `/sgui/shop`) ----
    /// `public static Image[] shopCategoryIcons;` (obf `ce.p:[LImage`) — shop-category
    /// icons (`/sgui/shop`, 6). Filled by [`load_shop_ui`]. `None` == null.
    pub shop_category_icons: Option<Vec<Image>>,
    /// `public static Image shopCoinIcon;` (obf `ce.q:LImage`) — shop coin icon (`/sgui/shop` 6).
    pub shop_coin_icon: Option<Image>,
    /// `public static Image shopSelectBox;` (obf `ce.p:LImage`) — shop cursor/selection
    /// box (`/sgui/shop` 7).
    pub shop_select_box: Option<Image>,
    /// `public static Image shopBuyIcon;` (obf `ce.b:LImage`) — shop buy icon
    /// (`_sgui_shop__8.png`). **DEFERRED feed** (`FontManager.loadLocaleImage`); modelled
    /// so [`unload_shop_ui`]'s clear stays faithful, but never filled.
    pub shop_buy_icon: Option<Image>,
    /// `public static Image shopSellIcon;` (obf `ce.c:LImage`) — shop sell icon
    /// (`_sgui_shop__9.png`). **DEFERRED feed** (`FontManager.loadLocaleImage`).
    pub shop_sell_icon: Option<Image>,

    // ---- Status-effect icons / emoticons (loadStatusEffectIcons) ----
    /// `public static Image emoticonBubble;` (obf `ce.D:LImage`) — emoticon bubble
    /// background (`/img/keepst` 0). Filled by [`load_status_effect_icons`].
    pub emoticon_bubble: Option<Image>,
    /// `public static Image[] statusIcons;` (obf `ce.v:[LImage`) — status-effect icons
    /// (`/img/keepst`, 8). Filled by [`load_status_effect_icons`].
    pub status_icons: Option<Vec<Image>>,
    /// `public static Image[] emoticons;` (obf `ce.w:[LImage`) — emoticon frames
    /// (`/img/emoti`). Filled by [`load_status_effect_icons`].
    pub emoticons: Option<Vec<Image>>,

    /// `public static TextTable commonText;` (obf `ce.g:Lz;`) — the common UI string
    /// table (`/sgui/com`). Modelled here, but its only assignment lives in
    /// `TitleScreen.loadLanguage` (DEFERRED there); stays `None` in this slice.
    pub common_text: Option<TextTableState>,
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
            // enemyFrames = new Object[60]; attackEffectScripts = new Object[5];
            // deathFxScripts = new Object[3]; bossFrames = new Object[80];  (<clinit>
            // allocations; every element null until the DEFERRED sprite load).
            enemy_frames: Some((0..60).map(|_| None).collect()),
            attack_effect_scripts: Some((0..5).map(|_| None).collect()),
            death_fx_scripts: Some((0..3).map(|_| None).collect()),
            boss_frames: Some((0..80).map(|_| None).collect()),
            // guardianFrames = new Object[3];  (<clinit>; every element null until the
            // DEFERRED guardian-sprite load).
            guardian_frames: Some((0..3).map(|_| None).collect()),
            // In-game UI / HUD banks — all declared null (`None`) until their load*.
            hud_frame: None,
            dialog_border: None,
            floater_icon3: None,
            floater_icon2: None,
            stat_point_alert: None,
            number_font1: None,
            number_font2: None,
            number_font3: None,
            number_font4: None,
            drop_item_marker: None,
            drop_gold_marker: None,
            skill_charge_fill: None,
            level_up_script: None,
            item_icons: None,
            guardian_icons: None,
            guardian_skill_icons: None,
            scroll_up_arrow: None,
            scroll_down_arrow: None,
            slot_frame: None,
            cursor_arrow: None,
            number_font0: None,
            portrait_frame: None,
            gold_icon: None,
            status_panel_icon: None,
            fraction_slash: None,
            help_text: None,
            menu_tab_icons: None,
            equip_slot_icons: None,
            shop_category_icons: None,
            shop_coin_icon: None,
            shop_select_box: None,
            shop_buy_icon: None,
            shop_sell_icon: None,
            emoticon_bubble: None,
            status_icons: None,
            emoticons: None,
            common_text: None,
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

/// `public static final void assembleSprites(boolean hasCounts, byte[] script,
/// int offset, byte bank, byte mirrorBank, PngMerger merger)`
/// (`ce.a:(Z[BIBBLbr;)V`).
///
/// Decodes an animation `script`: reads a frame count from `script[offset]` then,
/// for each frame's sub-entries, resolves each atlas image index into
/// `spriteBanks[bank]` (or `[mirrorBank]` for mirrored frames) via `merger`,
/// rewriting the script's flag byte **in place** to the resolved bank id. When
/// `hasCounts` is false each frame has exactly one sub-entry. A `None` `merger`
/// only rewrites the flag bytes (the DEFERRED guardian-sprite callers use that
/// form; [`load_in_game_ui`] passes the `/char/lvup` merger).
pub fn assemble_sprites(
    g: &mut Game,
    has_counts: bool,
    script: &mut [i8],
    offset: i32,
    bank: i8,
    mirror_bank: i8,
    mut merger: Option<&mut png_merger::PngMergerState>,
) {
    // int p = offset + 1;
    let mut p: i32 = offset.wrapping_add(1);
    // byte frameCount = script[offset];
    let frame_count: i8 = script[offset as usize];
    // if (merger != null) { merger.preloadAll = true; allocate spriteBanks[bank]/[mirrorBank] }
    if let Some(m) = merger.as_deref_mut() {
        // merger.preloadAll = true;
        m.preload_all = true;
        let fc = png_merger::frame_count(m);
        // if (spriteBanks[bank] == null) spriteBanks[bank] = new Image[merger.frameCount()];
        if g.asset_cache.sprite_banks[bank as usize].is_none() {
            g.asset_cache.sprite_banks[bank as usize] = Some((0..fc).map(|_| None).collect());
        }
        // if (mirrorBank != -1 && spriteBanks[mirrorBank] == null) same;
        if mirror_bank != -1 && g.asset_cache.sprite_banks[mirror_bank as usize].is_none() {
            g.asset_cache.sprite_banks[mirror_bank as usize] =
                Some((0..fc).map(|_| None).collect());
        }
    }
    // BaseCanvas.yieldTick();
    base_canvas::yield_tick(g);
    // for (int frame = 0; frame < frameCount; frame++) {
    let mut frame: i32 = 0;
    while frame < frame_count as i32 {
        // byte subCount = hasCounts ? script[p++] : 1;
        let sub_count: i8 = if has_counts {
            let count_pos = p;
            p = p.wrapping_add(1);
            script[count_pos as usize]
        } else {
            1
        };
        // for (int sub = 0; sub < subCount; sub++) {
        let mut sub: i32 = 0;
        while sub < sub_count as i32 {
            // int flagPos = p + 1 + 1; int imagePos = flagPos + 1;
            let flag_pos = p.wrapping_add(1).wrapping_add(1);
            let image_pos = flag_pos.wrapping_add(1);
            // boolean mirrored = script[flagPos] != 0;
            let mirrored = script[flag_pos as usize] != 0;
            // p = imagePos + 1;
            p = image_pos.wrapping_add(1);
            // byte imageIndex = script[imagePos];
            let image_index = script[image_pos as usize];
            // int flagWriteBack = p - 2;
            let flag_write_back = p.wrapping_sub(2);
            // byte destBank = mirrored ? mirrorBank : bank;
            let dest_bank: i8 = if mirrored { mirror_bank } else { bank };
            // script[flagWriteBack] = destBank;
            script[flag_write_back as usize] = dest_bank;
            // Debug.assertTrue(destBank > 0);
            debug::assert_true(dest_bank > 0);
            // if (merger != null) { Image[] bankImages = spriteBanks[destBank];
            //   if (bankImages[imageIndex] == null) { bankImages[imageIndex] =
            //     mirrored ? merger.imageMirrored(imageIndex) : merger.image(imageIndex);
            //     BaseCanvas.yieldTick(); } }
            if let Some(m) = merger.as_deref_mut() {
                let need = g.asset_cache.sprite_banks[dest_bank as usize]
                    .as_ref()
                    .expect("assembleSprites: destBank sprite bank null")
                    [image_index as usize]
                    .is_none();
                if need {
                    let img = if mirrored {
                        png_merger::image_mirrored(g, m, image_index as i32)
                    } else {
                        png_merger::image(g, m, image_index as i32)
                    };
                    g.asset_cache.sprite_banks[dest_bank as usize]
                        .as_mut()
                        .expect("assembleSprites: destBank sprite bank null")
                        [image_index as usize] = Some(img);
                    base_canvas::yield_tick(g);
                }
            }
            sub = sub.wrapping_add(1);
        }
        frame = frame.wrapping_add(1);
    }
}

/// `public static final void loadInGameUi()` (`ce.g:()V`): loads the shared in-game
/// UI — the `/img/uifrm` HUD window frame + dialogue border, the `/img/etcui`
/// glyph/marker set, and the `/char/lvup` level-up effect (assembled into
/// `spriteBanks[13]` via [`assemble_sprites`]).
///
/// The atlases are present on the classpath, so the body runs straight-line; the
/// original `catch (Exception e) { System.out.println(e); }` is a swallowed log
/// (a no-op on this path). **DEFERRED sub-banks** (`FontManager.loadLocaleImage`,
/// see [`crate::font_manager`]): `floaterIcon3`/`floaterIcon2`/`statPointAlert`
/// (`_img_etcui__0/1/4.png`) — never filled; a null bank safely no-ops when drawn.
/// The discarded `etcui.image(2)` probe is preserved (decode + discard).
pub fn load_in_game_ui(g: &mut Game) {
    // PngMerger uiframe = new PngMerger("/img/uifrm"); uiframe.preloadAll = true;
    let mut uiframe = png_merger::construct(g, "/img/uifrm");
    uiframe.preload_all = true;
    // hudFrame = new Image[7]; for (i=0;i<7;i++) { hudFrame[i]=uiframe.image(i); yieldTick(); }
    let mut hud_frame: Vec<Image> = Vec::with_capacity(7);
    let mut i: i32 = 0;
    while i < 7 {
        hud_frame.push(png_merger::image(g, &mut uiframe, i));
        base_canvas::yield_tick(g);
        i = i.wrapping_add(1);
    }
    g.asset_cache.hud_frame = Some(hud_frame);
    // dialogBorder = new Image[4];
    // dialogBorder[0]=uiframe.image(7); dialogBorder[1]=uiframe.imageMirrored(7); yieldTick();
    let d0 = png_merger::image(g, &mut uiframe, 7);
    let d1 = png_merger::image_mirrored(g, &mut uiframe, 7);
    base_canvas::yield_tick(g);
    // dialogBorder[2]=uiframe.image(8); dialogBorder[3]=uiframe.imageMirrored(8); yieldTick();
    let d2 = png_merger::image(g, &mut uiframe, 8);
    let d3 = png_merger::image_mirrored(g, &mut uiframe, 8);
    base_canvas::yield_tick(g);
    g.asset_cache.dialog_border = Some(vec![d0, d1, d2, d3]);
    // PngMerger etcui = new PngMerger("/img/etcui"); etcui.preloadAll = true;
    let mut etcui = png_merger::construct(g, "/img/etcui");
    etcui.preload_all = true;
    // floaterIcon3 = FontManager.loadLocaleImage("_img_etcui__0.png");  — DEFERRED (loadLocaleImage).
    // floaterIcon2 = FontManager.loadLocaleImage("_img_etcui__1.png");  — DEFERRED (loadLocaleImage).
    // etcui.image(2);   (discarded probe — decode + discard, a faithful side effect)
    let _ = png_merger::image(g, &mut etcui, 2);
    base_canvas::yield_tick(g);
    // entityShadow = etcui.image(3);
    let shadow = png_merger::image(g, &mut etcui, 3);
    g.asset_cache.entity_shadow = Some(shadow);
    // statPointAlert = FontManager.loadLocaleImage("_img_etcui__4.png");  — DEFERRED (loadLocaleImage).
    // numberFont1 = etcui.image(5); numberFont2 = etcui.image(6); yieldTick();
    let nf1 = png_merger::image(g, &mut etcui, 5);
    g.asset_cache.number_font1 = Some(nf1);
    let nf2 = png_merger::image(g, &mut etcui, 6);
    g.asset_cache.number_font2 = Some(nf2);
    base_canvas::yield_tick(g);
    // numberFont3 = etcui.image(7); numberFont4 = etcui.image(8);
    let nf3 = png_merger::image(g, &mut etcui, 7);
    g.asset_cache.number_font3 = Some(nf3);
    let nf4 = png_merger::image(g, &mut etcui, 8);
    g.asset_cache.number_font4 = Some(nf4);
    // dropItemMarker = etcui.image(9); dropGoldMarker = etcui.image(10);
    // skillChargeFill = etcui.image(11); yieldTick();
    let dim = png_merger::image(g, &mut etcui, 9);
    g.asset_cache.drop_item_marker = Some(dim);
    let dgm = png_merger::image(g, &mut etcui, 10);
    g.asset_cache.drop_gold_marker = Some(dgm);
    let scf = png_merger::image(g, &mut etcui, 11);
    g.asset_cache.skill_charge_fill = Some(scf);
    base_canvas::yield_tick(g);
    // PngMerger levelUp = new PngMerger("/char/lvup");
    let mut level_up = png_merger::construct(g, "/char/lvup");
    // levelUpScript = readResource("/char/lvup.eif");
    g.asset_cache.level_up_script = Some(
        read_resource(g, "/char/lvup.eif").expect("readResource(/char/lvup.eif) returned null"),
    );
    // BaseCanvas.yieldTick();
    base_canvas::yield_tick(g);
    // assembleSprites(true, levelUpScript, 0, (byte) 13, (byte) -1, levelUp);
    //   The static holds the same array assembleSprites rewrites in place; it is
    //   aliased out here and restored — net-identical, nothing reads it mid-assembly
    //   (cf. loadSpriteBank's spriteBanks alias, docs/TRANSLITERATION.md).
    let mut level_up_script = g
        .asset_cache
        .level_up_script
        .take()
        .expect("levelUpScript null");
    assemble_sprites(
        g,
        true,
        &mut level_up_script,
        0,
        13,
        -1,
        Some(&mut level_up),
    );
    g.asset_cache.level_up_script = Some(level_up_script);
}

/// `public static final void unloadInGameUi()` (`ce.h:()V => []`): drops the shared
/// in-game UI set. The `floaterIcon3`/`floaterIcon2`/`statPointAlert` fields are the
/// DEFERRED-feed banks (always `None`); clearing them is faithful and a no-op.
pub fn unload_in_game_ui(g: &mut Game) {
    // hudFrame = null; dialogBorder = null;
    g.asset_cache.hud_frame = None;
    g.asset_cache.dialog_border = None;
    // floaterIcon3 = null; floaterIcon2 = null;
    g.asset_cache.floater_icon3 = None;
    g.asset_cache.floater_icon2 = None;
    // entityShadow = null; statPointAlert = null;
    g.asset_cache.entity_shadow = None;
    g.asset_cache.stat_point_alert = None;
    // numberFont1..4 = null;
    g.asset_cache.number_font1 = None;
    g.asset_cache.number_font2 = None;
    g.asset_cache.number_font3 = None;
    g.asset_cache.number_font4 = None;
    // dropItemMarker = null; dropGoldMarker = null; skillChargeFill = null;
    g.asset_cache.drop_item_marker = None;
    g.asset_cache.drop_gold_marker = None;
    g.asset_cache.skill_charge_fill = None;
    // levelUpScript = null; spriteBanks[13] = null;
    g.asset_cache.level_up_script = None;
    g.asset_cache.sprite_banks[13] = None;
}

/// `public static final void loadItemIcons()` (`ce.p:()V => []`):
/// `itemIcons = new PngMerger("/img/icoitm").allImages()`. The `catch (Exception)`
/// is subsumed (the atlas is present).
pub fn load_item_icons(g: &mut Game) {
    // itemIcons = new PngMerger("/img/icoitm").allImages();
    let mut icoitm = png_merger::construct(g, "/img/icoitm");
    let frames = png_merger::all_images(g, &mut icoitm);
    g.asset_cache.item_icons = Some(frames);
}

/// `public static final void unloadItemIcons()` (`ce.q:()V => []`): `itemIcons = null`.
pub fn unload_item_icons(g: &mut Game) {
    // itemIcons = null;
    g.asset_cache.item_icons = None;
}

/// `public static final void loadGuardianIcons()` (`ce.t:()V`): loads the guardian
/// portrait icons (`guardianIcons`, 6) and per-skill icons (`guardianSkillIcons`,
/// 24 = 6 guardians x 4 skills) from `/grd/grdico`. The `catch (Exception)` is
/// subsumed (the atlas is present).
pub fn load_guardian_icons(g: &mut Game) {
    // guardianIcons = new Image[6]; guardianSkillIcons = new Image[24];
    let mut guardian_icons: Vec<Option<Image>> = (0..6).map(|_| None).collect();
    let mut guardian_skill_icons: Vec<Option<Image>> = (0..24).map(|_| None).collect();
    // PngMerger grdico = new PngMerger("/grd/grdico"); grdico.preloadAll = true;
    let mut grdico = png_merger::construct(g, "/grd/grdico");
    grdico.preload_all = true;
    // for (byte guardian = 0; guardian < 6; guardian++) {
    let mut guardian: i8 = 0;
    while guardian < 6 {
        // guardianIcons[guardian] = grdico.image((int) guardian);
        let icon = png_merger::image(g, &mut grdico, guardian as i32);
        guardian_icons[guardian as usize] = Some(icon);
        // for (byte skill = 0; skill < 4; skill++) {
        let mut skill: i8 = 0;
        while skill < 4 {
            // guardianSkillIcons[(guardian*4)+skill] = grdico.image(6 + (guardian*4) + skill);
            let dst = (guardian as i32).wrapping_mul(4).wrapping_add(skill as i32);
            let src = 6i32
                .wrapping_add((guardian as i32).wrapping_mul(4))
                .wrapping_add(skill as i32);
            let img = png_merger::image(g, &mut grdico, src);
            guardian_skill_icons[dst as usize] = Some(img);
            skill = (skill as i32).wrapping_add(1) as i8;
        }
        // BaseCanvas.yieldTick();
        base_canvas::yield_tick(g);
        guardian = (guardian as i32).wrapping_add(1) as i8;
    }
    g.asset_cache.guardian_icons = Some(
        guardian_icons
            .into_iter()
            .map(|o| o.expect("guardianIcons"))
            .collect(),
    );
    g.asset_cache.guardian_skill_icons = Some(
        guardian_skill_icons
            .into_iter()
            .map(|o| o.expect("guardianSkillIcons"))
            .collect(),
    );
}

/// `public static final void unloadGuardianIcons()` (`ce.u:()V => []`): drops the
/// guardian icons and skill icons.
pub fn unload_guardian_icons(g: &mut Game) {
    // guardianIcons = null; guardianSkillIcons = null;
    g.asset_cache.guardian_icons = None;
    g.asset_cache.guardian_skill_icons = None;
}

/// `public static final void loadGlobalUi()` (`ce.o:()V`): loads the global UI single
/// images (`/img/glb`) and the help string table (`helpText`, `/sgui/help`).
///
/// The atlas/table are present, so the body runs straight-line (the `catch (Exception)`
/// is a swallowed log). **DEFERRED sub-banks** (`FontManager.loadLocaleImage`, see
/// [`crate::font_manager`]): `statLabel1..5` (`_img_glb__9/10/11/12/15.png`) and the
/// discarded `_img_glb__13.png` probe — never filled. `glb.image(16)` is a discarded
/// decode probe (preserved); `glb.image(4)` is never referenced by the original.
pub fn load_global_ui(g: &mut Game) {
    // PngMerger glb = new PngMerger("/img/glb"); glb.preloadAll = true;
    let mut glb = png_merger::construct(g, "/img/glb");
    glb.preload_all = true;
    // scrollUpArrow = glb.image(0); scrollDownArrow = glb.image(1);
    let s0 = png_merger::image(g, &mut glb, 0);
    g.asset_cache.scroll_up_arrow = Some(s0);
    let s1 = png_merger::image(g, &mut glb, 1);
    g.asset_cache.scroll_down_arrow = Some(s1);
    // slotFrame = glb.image(2); cursorArrow = glb.image(3);
    let s2 = png_merger::image(g, &mut glb, 2);
    g.asset_cache.slot_frame = Some(s2);
    let s3 = png_merger::image(g, &mut glb, 3);
    g.asset_cache.cursor_arrow = Some(s3);
    // numberFont0 = glb.image(5); portraitFrame = glb.image(6);
    let n0 = png_merger::image(g, &mut glb, 5);
    g.asset_cache.number_font0 = Some(n0);
    let pf = png_merger::image(g, &mut glb, 6);
    g.asset_cache.portrait_frame = Some(pf);
    // goldIcon = glb.image(7); statusPanelIcon = glb.image(8);
    let gi = png_merger::image(g, &mut glb, 7);
    g.asset_cache.gold_icon = Some(gi);
    let sp = png_merger::image(g, &mut glb, 8);
    g.asset_cache.status_panel_icon = Some(sp);
    // statLabel1..4 = FontManager.loadLocaleImage("_img_glb__9/10/11/12.png");  — DEFERRED.
    // FontManager.loadLocaleImage("_img_glb__13.png");   — DEFERRED (discarded).
    // fractionSlash = glb.image(14);
    let fs = png_merger::image(g, &mut glb, 14);
    g.asset_cache.fraction_slash = Some(fs);
    // statLabel5 = FontManager.loadLocaleImage("_img_glb__15.png");  — DEFERRED.
    // glb.image(16);   (discarded probe — decode + discard)
    let _ = png_merger::image(g, &mut glb, 16);
    // helpText = new TextTable("/sgui/help");
    let help = text_table::construct(g, "/sgui/help");
    g.asset_cache.help_text = Some(help);
}

/// `public static final void loadGameMenuIcons()` (`ce.n:()V`): loads the
/// character-menu tab icons (`menuTabIcons`, 6) and equip-slot icons
/// (`equipSlotIcons`, 5) from `/sgui/gmico`.
pub fn load_game_menu_icons(g: &mut Game) {
    // menuTabIcons = new Image[6]; equipSlotIcons = new Image[5];
    let mut menu_tab_icons: Vec<Option<Image>> = (0..6).map(|_| None).collect();
    let mut equip_slot_icons: Vec<Option<Image>> = (0..5).map(|_| None).collect();
    // PngMerger gmico = new PngMerger("/sgui/gmico"); gmico.preloadAll = true;
    let mut gmico = png_merger::construct(g, "/sgui/gmico");
    gmico.preload_all = true;
    // byte tab = 0; while (tab < 6) { menuTabIcons[tab] = gmico.image(tab==5 ? 6 : tab); tab++; }
    let mut tab: i8 = 0;
    while tab < 6 {
        let src: i32 = if tab == 5 { 6 } else { tab as i32 };
        let img = png_merger::image(g, &mut gmico, src);
        menu_tab_icons[tab as usize] = Some(img);
        tab = (tab as i32).wrapping_add(1) as i8;
    }
    // for (byte slot = 0; slot < 5; slot++) equipSlotIcons[slot] = gmico.image(slot + 7);
    let mut slot: i8 = 0;
    while slot < 5 {
        let img = png_merger::image(g, &mut gmico, (slot as i32).wrapping_add(7));
        equip_slot_icons[slot as usize] = Some(img);
        slot = (slot as i32).wrapping_add(1) as i8;
    }
    g.asset_cache.menu_tab_icons = Some(
        menu_tab_icons
            .into_iter()
            .map(|o| o.expect("menuTabIcons"))
            .collect(),
    );
    g.asset_cache.equip_slot_icons = Some(
        equip_slot_icons
            .into_iter()
            .map(|o| o.expect("equipSlotIcons"))
            .collect(),
    );
}

/// `public static final void loadShopUi()` (`ce.r:()V`): loads the shop UI
/// (`shopCategoryIcons` 6, `shopCoinIcon`, `shopSelectBox`) from `/sgui/shop`.
///
/// **DEFERRED sub-banks** (`FontManager.loadLocaleImage`, see [`crate::font_manager`]):
/// `shopBuyIcon`/`shopSellIcon` (`_sgui_shop__8/9.png`) — never filled.
pub fn load_shop_ui(g: &mut Game) {
    // BaseCanvas.yieldTick();
    base_canvas::yield_tick(g);
    // shopCategoryIcons = new Image[6];
    let mut shop_category_icons: Vec<Option<Image>> = (0..6).map(|_| None).collect();
    // PngMerger shop = new PngMerger("/sgui/shop"); shop.preloadAll = true; yieldTick();
    let mut shop = png_merger::construct(g, "/sgui/shop");
    shop.preload_all = true;
    base_canvas::yield_tick(g);
    // for (byte category = 0; category < 6; category++) shopCategoryIcons[category] = shop.image(category);
    let mut category: i8 = 0;
    while category < 6 {
        let img = png_merger::image(g, &mut shop, category as i32);
        shop_category_icons[category as usize] = Some(img);
        category = (category as i32).wrapping_add(1) as i8;
    }
    // shopCoinIcon = shop.image(6); shopSelectBox = shop.image(7); yieldTick();
    let coin = png_merger::image(g, &mut shop, 6);
    g.asset_cache.shop_coin_icon = Some(coin);
    let sel = png_merger::image(g, &mut shop, 7);
    g.asset_cache.shop_select_box = Some(sel);
    base_canvas::yield_tick(g);
    g.asset_cache.shop_category_icons = Some(
        shop_category_icons
            .into_iter()
            .map(|o| o.expect("shopCategoryIcons"))
            .collect(),
    );
    // shopBuyIcon = FontManager.loadLocaleImage("_sgui_shop__8.png");   — DEFERRED (loadLocaleImage).
    // shopSellIcon = FontManager.loadLocaleImage("_sgui_shop__9.png");  — DEFERRED (loadLocaleImage).
}

/// `public static final void unloadShopUi()` (`ce.s:()V => []`): drops the shop UI.
/// `shopBuyIcon`/`shopSellIcon` are the DEFERRED-feed banks (always `None`).
pub fn unload_shop_ui(g: &mut Game) {
    // shopCategoryIcons = null; shopCoinIcon = null; shopSelectBox = null;
    g.asset_cache.shop_category_icons = None;
    g.asset_cache.shop_coin_icon = None;
    g.asset_cache.shop_select_box = None;
    // shopBuyIcon = null; shopSellIcon = null;
    g.asset_cache.shop_buy_icon = None;
    g.asset_cache.shop_sell_icon = None;
}

/// `public static final void loadStatusEffectIcons()` (`ce.i:()V`): loads the
/// emoticon bubble + status-effect icons (`/img/keepst`) and the emoticon frames
/// (`/img/emoti`). The `catch (Exception)` is subsumed (the atlases are present).
pub fn load_status_effect_icons(g: &mut Game) {
    // PngMerger keepst = new PngMerger("/img/keepst"); keepst.preloadAll = true;
    let mut keepst = png_merger::construct(g, "/img/keepst");
    keepst.preload_all = true;
    // emoticonBubble = keepst.image(0); yieldTick();
    let bubble = png_merger::image(g, &mut keepst, 0);
    g.asset_cache.emoticon_bubble = Some(bubble);
    base_canvas::yield_tick(g);
    // statusIcons = new Image[8]; for (i=0;i<8;i++) statusIcons[i] = keepst.image(i + 1); yieldTick();
    let mut status_icons: Vec<Option<Image>> = (0..8).map(|_| None).collect();
    let mut i: i32 = 0;
    while i < 8 {
        let img = png_merger::image(g, &mut keepst, i.wrapping_add(1));
        status_icons[i as usize] = Some(img);
        i = i.wrapping_add(1);
    }
    base_canvas::yield_tick(g);
    g.asset_cache.status_icons = Some(
        status_icons
            .into_iter()
            .map(|o| o.expect("statusIcons"))
            .collect(),
    );
    // emoticons = new PngMerger("/img/emoti").allImages(); yieldTick();
    let mut emoti = png_merger::construct(g, "/img/emoti");
    let frames = png_merger::all_images(g, &mut emoti);
    g.asset_cache.emoticons = Some(frames);
    base_canvas::yield_tick(g);
}

/// `public static final void unloadStatusEffectIcons()` (`ce.j:()V => []`): drops the
/// status-effect icons and emoticons.
pub fn unload_status_effect_icons(g: &mut Game) {
    // emoticonBubble = null; statusIcons = null; emoticons = null;
    g.asset_cache.emoticon_bubble = None;
    g.asset_cache.status_icons = None;
    g.asset_cache.emoticons = None;
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
