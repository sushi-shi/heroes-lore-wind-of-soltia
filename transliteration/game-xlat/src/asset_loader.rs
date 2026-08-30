//! Transliterated from `java/src/main/java/defpackage/AssetLoader.java`
//! (original `bu.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The background asset loader. On device each entry point (`loadResources`,
//! `loadMap`, `loadGuardian`, `loadMainMenu`) sets [`AssetLoaderState::phase`],
//! shows a loading screen, and spawns a worker `Thread`; `run` then does the
//! phase's heavy work off the UI thread.
//!
//! ## ANTI-BOG boundary + the thread collapse
//!
//! This slice ports **only what the first in-game frame needs** — the phase-1
//! ("- RESOURCE") → phase-2 ("- MAP") chain that drives a New Game to the world.
//! The worker `Thread` is **collapsed into the frame loop**: `load_resources` /
//! `load_map` run their phase synchronously (each is followed immediately by the
//! next queued `GameState` request), which keeps the observable `screen` sequence
//! `1 → 2` faithful without modelling threads.
//!
//! DEFERRED on the phase-1 path: `loadCommonAssets`, `classSkillText`,
//! `loadHeroEquipSprites`, `loadGuardianSprites` (the sprite/UI/text banks + the
//! per-class byte-script sprite system) and `beginLoading`/`keepLoadingProgress`.
//! DEFERRED elsewhere: `loadGuardian` (phase 3), `loadMainMenu` (phase 5), the
//! sprite-bank assembler (`loadSpriteBank` and its `*Anim` tables), and
//! `drawLoadingOverlay`'s real loading art.
//!
//! `AssetLoader` has one mutable static this slice models — `phase` (`bu.a:B`);
//! `commonLoaded` (`bu.a:Z`) and the script/atlas/anim tables are DEFERRED.

use crate::asset_cache;
use crate::game::Game;
use crate::game_map;
use crate::game_state;
use crate::png_merger;

/// `private static final String[] scriptSuffixes` (`bu.g`) — animation-script file
/// suffixes per sprite-bank kind (indexed by `bankKind`).
const SCRIPT_SUFFIXES: [&str; 7] = ["a", "b", "e", "hA", "hB", "w", "s"];
/// `public static final String[] scriptDirs` (`bu.a`) — per-class animation-script
/// dirs (`c1/s` .. `c3/s`), indexed by `classId - 6`.
const SCRIPT_DIRS: [&str; 3] = ["/c1/s/", "/c2/s/", "/c3/s/"];
/// `public static final String[] atlasDirs` (`bu.a`) — per-class sprite-atlas dirs
/// (`c1/i` .. `c3/i`), indexed by `classId - 6`.
const ATLAS_DIRS: [&str; 3] = ["/c1/i/", "/c2/i/", "/c3/i/"];
/// `public static final String[] armorFiles` (`bu.a`) — armor atlas file names.
const ARMOR_FILES: [&str; 6] = ["a1", "a2", "a3", "a4", "a5", "a6"];
/// `public static final String[] headFiles` (`bu.a`) — head atlas file names.
const HEAD_FILES: [&str; 7] = ["h1", "h2", "h3", "h4", "h5", "h6", "h7"];
/// `public static final String[] weaponFiles` (`bu.a`) — weapon atlas file names.
const WEAPON_FILES: [&str; 5] = ["w1", "w2", "w3", "w4", "w5"];
/// `public static final String[] shieldFiles` (`bu.a`) — shield atlas file names.
const SHIELD_FILES: [&str; 5] = ["s1", "s2", "s3", "s4", "s5"];
/// `public static final byte[] headAnim` (`bu.a`) — head-subId → atlas-file index.
const HEAD_ANIM: [i8; 16] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6];
/// `public static final byte[][] armorAnim` (`bu.a`) — `[classId-6][subId]` → armor
/// atlas-file index, or `-1` (no armor overlay → unload bank 0). Classes 6/7 share
/// the leading `-1`; class 8 starts at `0`.
const ARMOR_ANIM: [[i8; 19]; 3] = [
    [-1, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3],
    [-1, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3],
    [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3],
];

/// Java `bu` / `AssetLoader` state — **partial** (anti-bog). Only the `phase`
/// static (which entry point a worker is running) is modelled; `commonLoaded` and
/// the sprite-script tables are DEFERRED (see the module header and
/// `java/reconstruction/ownership.tsv`).
#[derive(Debug, Default)]
pub struct AssetLoaderState {
    /// `private static byte phase = 0;` (obf `bu.a:B`) — which load phase a worker
    /// runs (1 resources, 2 map, 3 guardian, 5 menu). Read by [`draw_loading_overlay`].
    pub phase: i8,
}

/// `public static final void loadResources()` (`bu.a:()V`): starts the
/// "- RESOURCE" load. The worker `Thread` is collapsed into the frame loop, so the
/// phase-1 work ([`run`]) runs synchronously here.
pub fn load_resources(g: &mut Game) {
    // phase = (byte) 1;
    g.asset_loader.phase = 1;
    // BaseCanvas.beginLoading("- RESOURCE", 500);  — DEFERRED (loading overlay art).
    // new Thread(new AssetLoader()).start();  — collapsed: run the phase now.
    run(g);
}

/// `public static final void loadMap()` (`bu.b:()V`): starts the "- MAP" (map warp)
/// load. Collapsed like [`load_resources`].
pub fn load_map(g: &mut Game) {
    // GameLoop.gameScreen.resetHudState();  — DEFERRED (HUD state, not on the tile path).
    // phase = (byte) 2;
    g.asset_loader.phase = 2;
    // BaseCanvas.beginLoading("- MAP", 200);  — DEFERRED.
    // new Thread(new AssetLoader()).start();  — collapsed: run the phase now.
    run(g);
}

/// `public final void run()` (`bu.run:()V`) — the worker body, dispatched by
/// [`AssetLoaderState::phase`]. `Thread.sleep`/`yieldTick` are no-ops. Only phases 1
/// and 2 are ported (3 = guardian, 5 = main-menu are DEFERRED).
fn run(g: &mut Game) {
    // try { Thread.sleep(100L); } catch { }   — no-op.
    match g.asset_loader.phase {
        1 => run_phase_resources(g),
        2 => run_phase_map(g),
        // (DEFERRED: case 3 guardian summon, case 5 return-to-main-menu.)
        _ => {}
    }
}

/// `run`'s `case 1` (phase 1, "- RESOURCE").
///
/// The decompiled Java has a JADX control-flow artifact — the `classSkillText`
/// `TextTable` load sits in a `try { … break; } catch { }` that, as rendered, would
/// skip the sprite loads when the table parses. The intended behaviour (and the
/// only one that produces a visible hero) is that the hero sprites load; this slice
/// reproduces that intent: `loadCommonAssets` (PARTIAL — only the `entityShadow` the
/// hero paint needs; see [`asset_cache::load_in_game_ui`]) then
/// [`load_hero_equip_sprites`], then the map-warp request that advances the New Game
/// sequence. Still DEFERRED here: `classSkillText`, `loadGuardianSprites` (needs the
/// DEFERRED Guardian), and `beginLoading`/`keepLoadingProgress`.
fn run_phase_resources(g: &mut Game) {
    // if (!commonLoaded) loadCommonAssets();  — PARTIAL: only loadInGameUi → entityShadow.
    asset_cache::load_in_game_ui(g);
    // (DEFERRED: the rest of loadCommonAssets — icons/UI/status/shop/death fx/text tables/audio.)
    // (DEFERRED: AssetCache.classSkillText = new TextTable("/sgui/q" + classId).)
    // loadHeroEquipSprites();
    load_hero_equip_sprites(g);
    // loadGuardianSprites();  — DEFERRED (derefs the unported Guardian).
    // GameState.requestMapWarp(GameState.storyMapId, (byte) 1, GameState.arg0, GameState.arg1);
    let map_id = g.game_state.story_map_id;
    let a1 = g.game_state.arg0;
    let a2 = g.game_state.arg1;
    game_state::request_map_warp(g, map_id, 1, a1, a2);
    // BaseCanvas.keepLoadingProgress = true;  — DEFERRED.
}

/// `private final void loadHeroEquipSprites()` (`bu.j:()V`) — allocates the hero
/// frame table and loads the body/armor/head/shield sprite banks from the current
/// equipment.
///
/// The accessory slots are now populated by `Hero.initClass` (`Item.create` gear
/// setup ported — see [`crate::hero::init_class`]), so the armor (`getAccessory1`)
/// and head (`getAccessory2`) branches take the non-null side: [`load_armor_sprite`]
/// (bank 0, or an `unloadSpriteBank(0)` when the class/subId maps to `-1`) and
/// [`load_head_sprite`] for the equipped head sub-index. The class-8 shield load
/// (`getArmor` → `loadShieldSprite`) stays DEFERRED (the drive is class 6, and the
/// shield loader is not yet ported).
pub fn load_hero_equip_sprites(g: &mut Game) {
    // AssetCache.heroFrames = new Object[396];
    g.asset_cache.hero_frames = Some((0..396).map(|_| None).collect());
    let class_id = g.game_state.class_id;
    // Hero hero = GameState.hero();
    let id = g
        .game_state
        .hero
        .expect("GameState.hero null in loadHeroEquipSprites");
    // if (hero.getAccessory1() != null) loadArmorSprite(classId, hero.getEquip(2).subId);
    let accessory1_sub_id = g.entity_arena[id].as_hero().expect("Hero node").equipment[2]
        .as_ref()
        .map(|it| it.borrow().sub_id);
    if let Some(sub_id) = accessory1_sub_id {
        // (the StringBuffer log + BaseCanvas.yieldTick are no-ops.)
        load_armor_sprite(g, class_id, sub_id);
    }
    // loadSpriteBank(classId, (byte) 1, (byte) 0, false, (byte) 0);  — the body layer.
    load_sprite_bank(g, class_id, 1, 0, false, 0);
    // if (hero.getAccessory2() != null) loadHeadSprite(classId, hero.getAccessory2().subId);
    // else loadHeadSprite(classId, (byte) 0);
    let accessory2_sub_id = g.entity_arena[id].as_hero().expect("Hero node").equipment[3]
        .as_ref()
        .map(|it| it.borrow().sub_id);
    if let Some(sub_id) = accessory2_sub_id {
        load_head_sprite(g, class_id, sub_id);
    } else {
        load_head_sprite(g, class_id, 0);
    }
    // if (classId == 8 && hero.getArmor() != null) loadShieldSprite(classId, getArmor().subId);
    //   — DEFERRED (drive is class 6; loadShieldSprite/getArmor path not ported).
}

/// `public static final void loadArmorSprite(byte classId, byte subId)` (`bu.a:(BB)V`)
/// — loads the armor overlay for armor sub-index `subId` (sprite bank 0), or clears
/// bank 0 when `armorAnim[classId-6][subId] == -1` (no overlay). Class 6 / subId 0
/// (the warrior's starting accessory) hits the `-1` arm → [`unload_sprite_bank`]`(0)`.
pub fn load_armor_sprite(g: &mut Game, class_id: i8, sub_id: i8) {
    let class_index = (class_id as i32).wrapping_sub(6) as usize;
    // if (armorAnim[classId-6][subId] == -1) unloadSpriteBank(0);
    if ARMOR_ANIM[class_index][sub_id as usize] == -1 {
        unload_sprite_bank(g, 0);
    } else {
        // else loadSpriteBank(classId, (byte) 0, armorAnim[classId-6][subId], false, (byte) 0);
        load_sprite_bank(
            g,
            class_id,
            0,
            ARMOR_ANIM[class_index][sub_id as usize],
            false,
            0,
        );
    }
}

/// `public static final void unloadSpriteBank(int bank)` (`bu.a:(I)V`) — clears sprite
/// bank `bank` and its mirror (`bank + 6`), the weapon-preview frames for bank 3, and
/// the per-layer `heroFrames` draw scripts the bank contributes (across the 11×4
/// pose/direction grid). `bank`'s layer offsets: 0 → armor layers 2..5, 1 → body 0,
/// 2 → aura 1, 3 → head 6, 4 → head-equip 7, 5 → weapon 8.
pub fn unload_sprite_bank(g: &mut Game, bank: i32) {
    // AssetCache.spriteBanks[bank] = null; AssetCache.spriteBanks[bank + 6] = null;
    g.asset_cache.sprite_banks[bank as usize] = None;
    g.asset_cache.sprite_banks[bank.wrapping_add(6) as usize] = None;
    // if (bank == 3) AssetCache.weaponPreviewFrames = null;
    if bank == 3 {
        g.asset_cache.weapon_preview_frames = None;
    }
    let hero_frames = g
        .asset_cache
        .hero_frames
        .as_mut()
        .expect("heroFrames null in unloadSpriteBank");
    // for (row = 0; row < 11; row++) for (col = 0; col < 4; col++) switch (bank) { ... }
    let mut row: i32 = 0;
    while row < 11 {
        let mut col: i32 = 0;
        while col < 4 {
            let base = (row.wrapping_mul(36)).wrapping_add(col.wrapping_mul(9));
            match bank {
                0 => {
                    hero_frames[base.wrapping_add(2) as usize] = None;
                    hero_frames[base.wrapping_add(3) as usize] = None;
                    hero_frames[base.wrapping_add(4) as usize] = None;
                    hero_frames[base.wrapping_add(5) as usize] = None;
                }
                1 => hero_frames[base as usize] = None,
                2 => hero_frames[base.wrapping_add(1) as usize] = None,
                3 => hero_frames[base.wrapping_add(6) as usize] = None,
                4 => hero_frames[base.wrapping_add(7) as usize] = None,
                5 => hero_frames[base.wrapping_add(8) as usize] = None,
                _ => {}
            }
            col = col.wrapping_add(1);
        }
        row = row.wrapping_add(1);
    }
}

/// `public static final void loadHeadSprite(byte classId, byte subId)` (`bu.b:(BB)V`)
/// — loads the head sprite for head sub-index `subId` (sprite bank 3 or 4). Class-6
/// heads with `subId` in `0..=3` use script bank 3 (`hA`); everything else bank 4 (`hB`).
pub fn load_head_sprite(g: &mut Game, class_id: i8, sub_id: i8) {
    // byte bank = 4; if (classId == 6 && subId >= 0 && subId <= 3) bank = 3;
    let mut bank: i8 = 4;
    // (verbatim `subId >= 0 && subId <= 3`; kept as two comparisons, not a range.)
    #[allow(clippy::manual_range_contains)]
    if class_id == 6 && sub_id >= 0 && sub_id <= 3 {
        bank = 3;
    }
    // loadSpriteBank(classId, bank, headAnim[subId], false, (byte) 0);
    load_sprite_bank(g, class_id, bank, HEAD_ANIM[sub_id as usize], false, 0);
}

/// `private static final void loadSpriteBank(byte classId, byte bankKind, byte
/// fileIndex, boolean weaponPreview, byte element)` (`bu.a:(BBBZB)V`) — opens the
/// atlas for `fileIndex` via [`png_merger`], reads the per-class animation script for
/// `bankKind` from the `c*/s` dir, and assembles the per-frame draw scripts into
/// [`asset_cache::AssetCacheState::hero_frames`] while lazily decoding the referenced
/// atlas frames into the paired sprite banks.
///
/// **This slice ports the non-aura decode (branch `bankKind != 2`)** — the path every
/// body/armor/head/weapon/shield bank takes (bank kinds 0/1/3/4/5/6). The **aura**
/// arm (`bankKind == 2`, its inline group-script decode + `loadMageShieldFrames`) is
/// DEFERRED: it is reached only by the guardian-summon load, and its layer (7) is
/// gated on `map.combatEnabled`, false on the class-6 start map. The `weaponPreview`
/// branch (class-select preview) keeps its frames local (never stored into
/// `spriteBanks`), reproduced faithfully though not driven here.
fn load_sprite_bank(
    g: &mut Game,
    class_id: i8,
    bank_kind: i8,
    file_index: i8,
    weapon_preview: bool,
    element: i8,
) {
    // byte classIndex = (byte) (classId - 6);
    let class_index = (class_id as i32).wrapping_sub(6) as i8;
    let atlas_dir = ATLAS_DIRS[class_index as usize];
    // switch (bankKind): construct the atlas merger + pick the destination sprite bank.
    let (mut merger, sprite_bank): (png_merger::PngMergerState, i8) = match bank_kind {
        // case 0: armor → spriteBanks[0]/[6].
        0 => (
            png_merger::construct(
                g,
                &format!("{atlas_dir}{}", ARMOR_FILES[file_index as usize]),
            ),
            0,
        ),
        // case 1: body → spriteBanks[1]/[7].
        1 => (png_merger::construct(g, &format!("{atlas_dir}b")), 1),
        // case 3/4: head → spriteBanks[2]/[8].
        3 | 4 => (
            png_merger::construct(
                g,
                &format!("{atlas_dir}{}", HEAD_FILES[file_index as usize]),
            ),
            2,
        ),
        // case 5: weapon → spriteBanks[3]/[9].
        5 => (
            png_merger::construct(
                g,
                &format!("{atlas_dir}{}", WEAPON_FILES[file_index as usize]),
            ),
            3,
        ),
        // case 6: shield → spriteBanks[5]/[11].
        6 => (
            png_merger::construct(
                g,
                &format!("{atlas_dir}{}", SHIELD_FILES[file_index as usize]),
            ),
            5,
        ),
        // case 2: guardian aura (spriteBanks[4]/[10] + loadMageShieldFrames) — DEFERRED.
        _ => unimplemented!(
            "DEFERRED: loadSpriteBank bankKind 2 (guardian aura) — not on the milestone path"
        ),
    };
    // Image[] frames = new Image[merger.frameCount()];  Image[] mirroredFrames = same;
    let frame_count = png_merger::frame_count(&merger);
    let mut frames: Vec<Option<j2me_me::Image>> = (0..frame_count).map(|_| None).collect();
    let mut mirrored_frames: Vec<Option<j2me_me::Image>> = (0..frame_count).map(|_| None).collect();
    // case 5 weapon element remap (element != 0): recolour the weapon palette.
    if bank_kind == 5 && element != 0 {
        match element {
            1 => png_merger::remap_palette(&mut merger, 255, 16744255),
            2 => png_merger::remap_palette(&mut merger, 255, 6258623),
            3 => png_merger::remap_palette(&mut merger, 255, 8388479),
            _ => {}
        }
    }
    // merger.preloadAll = true;
    merger.preload_all = true;
    // byte[] script = AssetCache.readResource(scriptDirs[classIndex] + scriptSuffixes[bankKind]);
    let script = asset_cache::read_resource(
        g,
        &format!(
            "{}{}",
            SCRIPT_DIRS[class_index as usize], SCRIPT_SUFFIXES[bank_kind as usize]
        ),
    )
    .expect("readResource(sprite script) returned null");
    // int pos = 0; while (pos < script.length) { ...  (bankKind != 2 branch) ... }
    let mut pos: i32 = 0;
    while pos < script.len() as i32 {
        // byte action = script[pos]; byte row = script[pos+1]; byte col = script[pos+2];
        // byte frameCount = script[pos+3]; pos += 4;
        let action = script[pos as usize];
        let row = script[(pos + 1) as usize];
        let col = script[(pos + 2) as usize];
        let frame_count_hdr = script[(pos + 3) as usize];
        pos = pos.wrapping_add(4);
        // byte[] entry = new byte[1 + (frameCount * 4)]; int w = 1; entry[0] = frameCount;
        let entry_len = 1i32.wrapping_add((frame_count_hdr as i32).wrapping_mul(4));
        let mut entry: Vec<i8> = vec![0i8; entry_len as usize];
        let mut w: i32 = 1;
        entry[0] = frame_count_hdr;
        // for (int i = 0; i < frameCount; i++)  (each frame: dx, dy, mirrorFlag, imageIndex)
        let mut i: i32 = 0;
        while i < frame_count_hdr as i32 {
            // entry[w] = script[pos]  (dx);  entry[w+1] = script[pos+1]  (dy).
            entry[w as usize] = script[pos as usize];
            entry[(w + 1) as usize] = script[(pos + 1) as usize];
            // boolean mirrored = script[pos+2] != 0;  entry[w+2] = mirrored ? bank+6 : bank.
            let mirrored = script[(pos + 2) as usize] != 0;
            entry[(w + 2) as usize] = if mirrored {
                (sprite_bank as i32).wrapping_add(6) as i8
            } else {
                sprite_bank
            };
            // byte imageIndex = script[pos+3];  entry[w+3] = imageIndex.
            let image_index = script[(pos + 3) as usize];
            entry[(w + 3) as usize] = image_index;
            // w += 4; pos += 4;
            w = w.wrapping_add(4);
            pos = pos.wrapping_add(4);
            // lazily decode the referenced atlas frame into the base / mirror bank.
            if image_index != -1 {
                if !mirrored && frames[image_index as usize].is_none() {
                    let img = png_merger::image(g, &mut merger, image_index as i32);
                    frames[image_index as usize] = Some(img);
                } else if mirrored && mirrored_frames[image_index as usize].is_none() {
                    let img = png_merger::image_mirrored(g, &mut merger, image_index as i32);
                    mirrored_frames[image_index as usize] = Some(img);
                }
            }
            i = i.wrapping_add(1);
        }
        // if (weaponPreview) weaponPreviewFrames[(row*4)+col] = entry;
        // else heroFrames[(row*36)+(col*9)+action] = entry;
        if weapon_preview {
            let idx = (row as i32).wrapping_mul(4).wrapping_add(col as i32);
            g.asset_cache
                .weapon_preview_frames
                .as_mut()
                .expect("weaponPreviewFrames null")[idx as usize] = Some(entry);
        } else {
            let idx = (row as i32)
                .wrapping_mul(36)
                .wrapping_add((col as i32).wrapping_mul(9))
                .wrapping_add(action as i32);
            g.asset_cache.hero_frames.as_mut().expect("heroFrames null")[idx as usize] =
                Some(entry);
        }
    }
    // merger.unloadAllMpd();
    png_merger::unload_all_mpd(&mut merger);
    // Publish the filled banks (Java aliases spriteBanks[dest]/[mirror] up front and
    // fills them via the alias; net-identical since nothing reads them mid-decode).
    if !weapon_preview {
        g.asset_cache.sprite_banks[sprite_bank as usize] = Some(frames);
        g.asset_cache.sprite_banks[(sprite_bank as i32).wrapping_add(6) as usize] =
            Some(mirrored_frames);
    }
}

/// `run`'s `case 2` (phase 2, "- MAP"): drops the old map, loads the destination
/// map, places the hero, seeds spawns, and requests the warp (state 15).
fn run_phase_map(g: &mut Game) {
    // swapMap();
    swap_map(g);
    // GameState.setHeroTile((int) GameState.arg1, (int) GameState.arg2);
    let tile_x = g.game_state.arg1 as i32;
    let tile_y = g.game_state.arg2 as i32;
    game_state::set_hero_tile(g, tile_x, tile_y);
    // GameState.map.fadeStep();
    game_map::fade_step(g);
    // GameState.requestState((byte) 15, GameState.arg0);
    let a0 = g.game_state.arg0;
    game_state::request_state_a0(g, 15, a0);
}

/// `private final GameMap swapMap()` (`bu`): unlinks the hero/guardian from the old
/// map, drops it, and loads the destination `new GameMap(storyMapId)`.
///
/// On a fresh New Game there is no prior map (`oldMap == null`), so the
/// `removeEntity`/`next`/`prev`/guardian cleanup is a DEFERRED no-op here.
fn swap_map(g: &mut Game) {
    // GameMap oldMap = GameState.map; Hero hero = GameState.hero();
    // if (oldMap != null) { oldMap.removeEntity(hero); ...; guardian cleanup }
    //   — DEFERRED (oldMap null on the first map; no guardian).
    // GameState.map = null; GameMap newMap = new GameMap(GameState.storyMapId);
    let story = g.game_state.story_map_id;
    let new_map = {
        let Game {
            game_map_class,
            base_canvas,
            ..
        } = &mut *g;
        game_map::new_game_map(game_map_class, base_canvas, story)
    };
    // GameState.setMap(newMap);
    g.game_state.map = Some(new_map);
    // newMap.load();
    game_map::load(g);
}

/// `public static final void drawLoadingOverlay(Graphics graphics)` (`bu`): draws
/// the phase-appropriate loading overlay. The real overlay art
/// (`BaseCanvas.drawLoadingScreen` / `GameScreen.drawLoadBox`) is DEFERRED; this
/// minimal body keeps the loading frame from panicking (it leaves the framebuffer
/// as-is — the world frame overwrites it once `screen` reaches 2).
pub fn draw_loading_overlay(g: &mut Game) {
    // switch (phase) { case 1/2: BaseCanvas.drawLoadingScreen(graphics); case 3: ... }
    //   — DEFERRED (loading overlay art not ported in this slice).
    let _ = g;
}
