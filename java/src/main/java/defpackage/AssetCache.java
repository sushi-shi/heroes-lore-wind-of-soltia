package defpackage;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import javax.microedition.lcdui.Image;

/* renamed from: ce */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ce.class */
/**
 * Central resource cache and JAR loader. All game art, sprite-frame scripts,
 * string tables and read buffers live here as {@code static} banks; the
 * background {@link AssetLoader} worker and the various screens fill and drain
 * them through the {@code load*}/{@code unload*} entry points below. Nothing is
 * instanceable ({@link #AssetCache()} is private) — the class is a global bag of
 * lazily-loaded banks.
 *
 * <p>The raw byte gateway is {@link #readResource(String)}: it slurps a JAR
 * resource into a {@code byte[]} through the shared {@link #readBuffer}. Sprite
 * atlases are decoded by {@link PngMerger}; the per-frame animation scripts that
 * index those atlases are assembled by {@link #assembleSprites}. The decoded
 * atlas images are held per sprite-bank slot in {@link #spriteBanks}
 * (0-11 hero equipment, 12 guardian element, 13 level-up, 15-24 enemy,
 * 25-26 boss, 27-36 npc, 37 death), while the frame scripts that walk them are
 * split across the {@code *Frames}/{@code *Scripts} banks:
 *
 * <ul>
 *   <li>{@link #heroFrames} — hero equipment animation scripts, keyed
 *       {@code (row*36)+(col*9)+layer}.</li>
 *   <li>{@link #enemyFrames}/{@link #npcFrames} — enemy/npc scripts, keyed
 *       {@code (slot*12)+(group*4)+dir}; {@link #bossFrames} keyed
 *       {@code (slot*16)+(group*4)+dir}.</li>
 *   <li>{@link #guardianFrames}, {@link #mageAuraScripts},
 *       {@link #weaponPreviewFrames}, {@link #attackEffectScripts},
 *       {@link #deathFxScripts}, {@link #bossExtraFrames} — the smaller effect
 *       banks.</li>
 *   <li>{@link #enemySpriteIds}/{@link #bossSpriteIds} — per-map slot→sprite-id
 *       registries (filled by {@link GameMap}); {@link #bossSlot} reverse-maps a
 *       boss id back to its slot.</li>
 * </ul>
 *
 * <p>The remaining fields are decoded {@link Image}s (HUD, menu and map UI) and
 * {@link TextTable}s (localized strings), each named for what it holds.
 */
public final class AssetCache implements Directions {

    // ---- Sprite-frame script banks (byte[] entries decoded by assembleSprites) ----

    /* renamed from: a */
    /** Hero equipment animation scripts, keyed {@code (row*36)+(col*9)+layer} (9 layers/cell). */
    public static Object[] heroFrames;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Mage (class 8) extra shield/aura frame scripts (ea2/ea3/ea4); fired as projectiles by {@link Hero}. */
    public static Object[] mageAuraScripts = null;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Weapon-preview frame scripts, keyed {@code (row*4)+col}. */
    public static Object[] weaponPreviewFrames;

    /* renamed from: d */
    /** Guardian sprite frame scripts (indexed via {@code Effect.TYPE_SPRITE_INDEX}). */
    public static Object[] guardianFrames = new Object[3];

    /* renamed from: e */
    /** Enemy sprite frame scripts, keyed {@code (slot*12)+group} (0 walk, 4 attack, 8 cast). */
    public static Object[] enemyFrames = new Object[60];

    /* renamed from: f */
    /** Enemy/boss attack-effect frame scripts, indexed by enemy/boss slot. */
    public static Object[] attackEffectScripts = new Object[5];

    /* renamed from: g */
    /** Enemy death/explosion frame scripts, indexed by {@code stats.size}. */
    public static Object[] deathFxScripts = new Object[3];

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    /** Boss sprite frame scripts, keyed {@code (slot*16)+(group*4)+dir}. */
    public static Object[] bossFrames = new Object[80];

    /* renamed from: i */
    /** Boss-type-1 extra frame scripts (copied into {@link #bossFrames} by {@link RockyBoss}). */
    public static Object[] bossExtraFrames = new Object[12];

    /* renamed from: j */
    /** Npc sprite frame scripts, keyed {@code (slot*12)+group+dir}. */
    public static Object[] npcFrames = new Object[60];

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    /** Per-npc-slot frame count of animation group 0 (walk); compared against {@code Npc.animFrame}. */
    public static byte[] npcAnimFrames0 = new byte[5];

    /* JADX INFO: renamed from: j, reason: collision with other field name */
    /** Per-npc-slot frame count of animation group 1 (action); compared against {@code Npc.animFrame}. */
    public static byte[] npcAnimFrames1 = new byte[5];

    /* renamed from: k */
    /** Boss slot → boss sprite id registry (filled per-map by {@link GameMap}). */
    public static byte[] bossSpriteIds = new byte[5];

    /* renamed from: l */
    /** Enemy slot → enemy sprite id registry (filled per-map by {@link GameMap}). */
    public static byte[] enemySpriteIds = new byte[5];

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Decoded atlas images per sprite-bank slot (see class doc for the slot map). */
    public static Image[][] spriteBanks = new Image[38][];

    // ---- Localized string tables ----

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Hero string table ({@code /char/hero}). */
    public static TextTable heroText;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Guardian string table ({@code /grd/grd}). */
    public static TextTable guardianText;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Guardian skill string table ({@code /grd/grdsk}). */
    public static TextTable guardianSkillText;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Map-name string table ({@code /m/name}). */
    public static TextTable mapNameText;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /** Help string table ({@code /sgui/help}). */
    public static TextTable helpText;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    /** Per-class skill/quest string table ({@code /sgui/q<classId>}). */
    public static TextTable classSkillText;

    /* JADX INFO: renamed from: g, reason: collision with other field name */
    /** Common UI string table ({@code /sgui/com}) — menu/option labels. */
    public static TextTable commonText;

    // ---- Image-array banks ----

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Guardian summon-beam images ({@code /grd/<n>}, 2 frames). */
    public static Image[] guardianBeam;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Guardian portrait icons ({@code /grd/grdico}, 6). */
    public static Image[] guardianIcons;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Guardian skill icons ({@code /grd/grdico}, 24 = 6 guardians x 4 skills). */
    public static Image[] guardianSkillIcons;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Item-type icons ({@code /img/icoitm}). */
    public static Image[] itemIcons;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /** Map tileset images ({@code /m/t/t<NN>}); loaded lazily by {@link GameMap}. */
    public static Image[] mapTiles;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    /** Map object images; loaded per-map by {@link GameMap}. */
    public static Image[] mapObjects;

    /* JADX INFO: renamed from: g, reason: collision with other field name */
    /** Simple map-npc images (drawn by {@link Npc} as {@code [kind-18]}). */
    public static Image[] mapNpcImages;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    /** Cutscene/event dialogue portraits (drawn by {@link EventScript}). */
    public static Image[] dialoguePortraits;

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    /** Title logo frames ({@code /img/logo}). */
    public static Image[] logoFrames;

    /* JADX INFO: renamed from: j, reason: collision with other field name */
    /** Title-screen background frames ({@code /img/title1}). */
    public static Image[] titleBgFrames;

    /* JADX INFO: renamed from: k, reason: collision with other field name */
    /** Title-screen menu frames ({@code /img/title2}, 10). */
    public static Image[] titleMenuFrames;

    /* JADX INFO: renamed from: l, reason: collision with other field name */
    /** Main-menu frame/border images ({@code /sgui/mm/etc}). */
    public static Image[] menuFrames;

    /** Class portrait faces ({@code /sgui/mm/face}, 6 = 3 colour + 3 gray). */
    public static Image[] classFaces;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Main-menu guardian preview images ({@code /grd/0..2}, [3][2]). */
    public static Image[][] menuGuardianPreview;

    /** Character-menu tab icons ({@code /sgui/gmico}, 6). */
    public static Image[] menuTabIcons;

    /** Equip-slot icons ({@code /sgui/gmico}, 5). */
    public static Image[] equipSlotIcons;

    /* JADX INFO: renamed from: p, reason: collision with other field name */
    /** Shop-category icons ({@code /sgui/shop}, 6). */
    public static Image[] shopCategoryIcons;

    /* JADX INFO: renamed from: q, reason: collision with other field name */
    /** In-game HUD frame pieces ({@code /img/uifrm}, 7). */
    public static Image[] hudFrame;

    /* JADX INFO: renamed from: r, reason: collision with other field name */
    /** Dialogue-border corner images ({@code /img/uifrm}, 4). */
    public static Image[] dialogBorder;

    /* JADX INFO: renamed from: s, reason: collision with other field name */
    /** Attack-effect animation frames 1 ({@code /img/atteff1}, 3), element-tinted. */
    public static Image[] attackFx1;

    /* JADX INFO: renamed from: t, reason: collision with other field name */
    /** Attack-effect animation frames 2 ({@code /img/atteff2}, 3), element-tinted. */
    public static Image[] attackFx2;

    /* JADX INFO: renamed from: u, reason: collision with other field name */
    /** Attack-effect animation frames 3 ({@code /img/atteff3}, 3), element-tinted. */
    public static Image[] attackFx3;

    /* JADX INFO: renamed from: v, reason: collision with other field name */
    /** Status-effect icons ({@code /img/keepst}, 8). */
    public static Image[] statusIcons;

    /* JADX INFO: renamed from: w, reason: collision with other field name */
    /** Emoticon frames ({@code /img/emoti}). */
    public static Image[] emoticons;

    // ---- Single images: HUD, menus, map UI ----

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Status-page panel icon ({@code /img/glb} img 8). */
    public static Image statusPanelIcon;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Shop buy icon ({@code /sgui/shop} img 8). */
    public static Image shopBuyIcon;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Shop sell icon ({@code /sgui/shop} img 9). */
    public static Image shopSellIcon;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Menu/list cursor arrow ({@code /img/glb} img 3). */
    public static Image cursorArrow;

    /* JADX INFO: renamed from: e, reason: collision with other field name */
    /** Stat-name label image 1 ({@code /img/glb} img 9). */
    public static Image statLabel1;

    /* JADX INFO: renamed from: f, reason: collision with other field name */
    /** Stat-name label image 2 ({@code /img/glb} img 10). */
    public static Image statLabel2;

    /* JADX INFO: renamed from: g, reason: collision with other field name */
    /** Stat-name label image 3 ({@code /img/glb} img 11). */
    public static Image statLabel3;

    /* JADX INFO: renamed from: h, reason: collision with other field name */
    /** Stat-name label image 4 ({@code /img/glb} img 12). */
    public static Image statLabel4;

    /* JADX INFO: renamed from: i, reason: collision with other field name */
    /** Fraction "/" separator drawn by {@code BaseCanvas.drawFraction} ({@code /img/glb} img 14). */
    public static Image fractionSlash;

    /* JADX INFO: renamed from: j, reason: collision with other field name */
    /** Stat-name label image 5 ({@code /img/glb} img 15). */
    public static Image statLabel5;

    /* JADX INFO: renamed from: k, reason: collision with other field name */
    /** Scroll-up arrow ({@code /img/glb} img 0). */
    public static Image scrollUpArrow;

    /* JADX INFO: renamed from: l, reason: collision with other field name */
    /** Portrait frame ({@code /img/glb} img 6). */
    public static Image portraitFrame;

    /* JADX INFO: renamed from: m, reason: collision with other field name */
    /** Gold/currency icon drawn before an amount ({@code /img/glb} img 7). */
    public static Image goldIcon;

    /* JADX INFO: renamed from: n, reason: collision with other field name */
    /** Scroll-down arrow ({@code /img/glb} img 1). */
    public static Image scrollDownArrow;

    /* JADX INFO: renamed from: o, reason: collision with other field name */
    /** Item-slot box frame ({@code /img/glb} img 2). */
    public static Image slotFrame;

    /* JADX INFO: renamed from: p, reason: collision with other field name */
    /** Shop cursor/selection box ({@code /sgui/shop} img 7). */
    public static Image shopSelectBox;

    /** Shop coin icon ({@code /sgui/shop} img 6). */
    public static Image shopCoinIcon;

    /** Digit glyph sheet, style 0 (5x7, {@code /img/glb} img 5). */
    public static Image numberFont0;

    /** Blinking stat-point/level-up alert on the HUD ({@code /img/etcui} img 4). */
    public static Image statPointAlert;

    /** Digit glyph sheet, style 1 (7x9, {@code /img/etcui} img 5). */
    public static Image numberFont1;

    /** Digit glyph sheet, style 2 (7x9, {@code /img/etcui} img 6). */
    public static Image numberFont2;

    /** Digit glyph sheet, style 3 (9x14, {@code /img/etcui} img 7). */
    public static Image numberFont3;

    /** Digit glyph sheet, style 4 (9x14, {@code /img/etcui} img 8). */
    public static Image numberFont4;

    /** Map item-drop marker ({@code /img/etcui} img 9). */
    public static Image dropItemMarker;

    /** Map gold-drop marker ({@code /img/etcui} img 10). */
    public static Image dropGoldMarker;

    /** HUD guardian-skill charge-fill overlay ({@code /img/etcui} img 11). */
    public static Image skillChargeFill;

    /** Emoticon bubble background ({@code /img/keepst} img 0). */
    public static Image emoticonBubble;

    /** Floater icon for kind-3 floaters ({@code /img/etcui} img 0). */
    public static Image floaterIcon3;

    /** Floater icon for kind-2 floaters ({@code /img/etcui} img 1). */
    public static Image floaterIcon2;

    /** Entity ground shadow drawn under hero/enemy/npc ({@code /img/etcui} img 3). */
    public static Image entityShadow;

    // ---- byte-array banks ----

    /** Level-up effect frame script ({@code /char/lvup.eif}). */
    public static byte[] levelUpScript;

    /** Guardian sprite script for the element-0 base pose ({@code /grd/spr/0_02.eif}). */
    public static byte[] guardianSpriteScript;

    /** Shared 512-byte read buffer for {@link #readResource(String)}. */
    public static byte[] readBuffer = new byte[512];

    private AssetCache() {
    }

    /* renamed from: a */
    /** Loads the three element-tinted attack-effect frame sets ({@code /img/atteff1..3}). */
    public static final void loadAttackEffects(byte element) {
        try {
            PngMerger merger = new PngMerger("/img/atteff1");
            merger.preloadAll = true;
            tintAttackEffect(merger, element);
            attackFx1 = new Image[3];
            attackFx1[0] = merger.image(0);
            attackFx1[1] = merger.image(1);
            attackFx1[2] = merger.image(2);
            defpackage.BaseCanvas.yieldTick();
            merger.load("/img/atteff2");
            tintAttackEffect(merger, element);
            attackFx2 = new Image[3];
            attackFx2[0] = merger.image(0);
            attackFx2[1] = merger.image(1);
            attackFx2[2] = merger.image(2);
            defpackage.BaseCanvas.yieldTick();
            merger.load("/img/atteff3");
            attackFx3 = new Image[3];
            attackFx3[0] = merger.image(0);
            attackFx3[1] = merger.image(1);
            attackFx3[2] = merger.image(2);
            defpackage.BaseCanvas.yieldTick();
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: a */
    /** Drops the attack-effect frame sets. */
    public static final void unloadAttackEffects() {
        attackFx1 = null;
        attackFx2 = null;
        attackFx3 = null;
    }

    /* renamed from: a */
    /** Recolours the attack-effect atlas palette for the given guardian element. */
    private static final void tintAttackEffect(PngMerger merger, byte element) {
        switch (element) {
            case 1:
                merger.remapPalette(12574719, 16777152);
                merger.remapPalette(10469375, 16760703);
                merger.remapPalette(6258623, 16744255);
                break;
            case 3:
                merger.remapPalette(12574719, 14679999);
                merger.remapPalette(10469375, 12574655);
                merger.remapPalette(6258623, 10469247);
                break;
        }
    }

    /* renamed from: b */
    /** Drops the map tileset images. */
    public static final void unloadMapTiles() {
        mapTiles = null;
    }

    /* renamed from: c */
    /** Drops the map object images. */
    public static final void unloadMapObjects() {
        mapObjects = null;
    }

    /* renamed from: d */
    /** Drops the simple map-npc images. */
    public static final void unloadMapNpcImages() {
        mapNpcImages = null;
    }

    /* renamed from: e */
    /** Resets the enemy/boss frame banks and their sprite-bank slots (15-24). */
    public static final void resetEnemyBossBanks() {
        enemyFrames = new Object[60];
        bossFrames = new Object[80];
        bossSpriteIds = new byte[5];
        for (int slot = 0; slot < 10; slot++) {
            spriteBanks[15 + slot] = null;
        }
    }

    /* renamed from: a */
    /** Loads one guardian sprite script ({@code /grd/spr/<element>_<index>.eif}) into {@link #guardianFrames}. */
    public static final void loadGuardianSprite(byte element, byte index) {
        if (element == 0 && index == 0) {
            guardianFrames[index] = readResource("/grd/spr/0_01.eif");
            assembleSprites(true, (byte[]) guardianFrames[index], 0, (byte) 12, (byte) -1, null);
            guardianSpriteScript = readResource("/grd/spr/0_02.eif");
            assembleSprites(true, guardianSpriteScript, 0, (byte) 12, (byte) -1, null);
        } else {
            guardianFrames[index] = readResource(new StringBuffer().append("/grd/spr/").append((int) element).append("_").append((int) index).append(".eif").toString());
            assembleSprites(true, (byte[]) guardianFrames[index], 0, (byte) 12, (byte) -1, null);
        }
        new StringBuffer().append("GuardianSprite : ").append((int) element).append(", ").append((int) index).toString();
    }

    /* renamed from: f */
    /** Drops the guardian sprite frame scripts. */
    public static final void unloadGuardianSprites() {
        guardianFrames = new Object[3];
        guardianSpriteScript = null;
    }

    /* renamed from: a */
    /**
     * Decodes an animation script: reads a frame count from {@code script[offset]}
     * then, for each frame's sub-entries, resolves each atlas image index into
     * {@link #spriteBanks}{@code [bank]} (or {@code [mirrorBank]} for mirrored
     * frames) via {@code merger}, rewriting the script's flag byte in place to the
     * resolved bank id. When {@code hasCounts} is false each frame has exactly one
     * sub-entry. A {@code null} {@code merger} only rewrites the flag bytes.
     */
    public static final void assembleSprites(boolean hasCounts, byte[] script, int offset, byte bank, byte mirrorBank, PngMerger merger) {
        byte subCount;
        int p = offset + 1;
        byte frameCount = script[offset];
        if (merger != null) {
            merger.preloadAll = true;
            if (spriteBanks[bank] == null) {
                spriteBanks[bank] = new Image[merger.frameCount()];
            }
            if (mirrorBank != -1 && spriteBanks[mirrorBank] == null) {
                spriteBanks[mirrorBank] = new Image[merger.frameCount()];
            }
        }
        defpackage.BaseCanvas.yieldTick();
        for (int frame = 0; frame < frameCount; frame++) {
            if (hasCounts) {
                int countPos = p;
                p++;
                subCount = script[countPos];
            } else {
                subCount = 1;
            }
            for (int sub = 0; sub < subCount; sub++) {
                int flagPos = p + 1 + 1;
                int imagePos = flagPos + 1;
                boolean mirrored = script[flagPos] != 0;
                p = imagePos + 1;
                byte imageIndex = script[imagePos];
                int flagWriteBack = p - 2;
                byte destBank = mirrored ? mirrorBank : bank;
                script[flagWriteBack] = destBank;
                defpackage.Debug.assertTrue(destBank > 0);
                if (merger != null) {
                    Image[] bankImages = spriteBanks[destBank];
                    if (bankImages[imageIndex] == null) {
                        bankImages[imageIndex] = mirrored ? merger.imageMirrored((int) imageIndex) : merger.image((int) imageIndex);
                        defpackage.BaseCanvas.yieldTick();
                    }
                }
            }
        }
    }

    /* renamed from: g */
    /** Loads the shared in-game UI: window frame, HUD glyph/icon set and level-up effect. */
    public static final void loadInGameUi() {
        try {
            PngMerger uiframe = new PngMerger("/img/uifrm");
            uiframe.preloadAll = true;
            hudFrame = new Image[7];
            for (int i = 0; i < 7; i++) {
                hudFrame[i] = uiframe.image(i);
                defpackage.BaseCanvas.yieldTick();
            }
            dialogBorder = new Image[4];
            dialogBorder[0] = uiframe.image(7);
            dialogBorder[1] = uiframe.imageMirrored(7);
            defpackage.BaseCanvas.yieldTick();
            dialogBorder[2] = uiframe.image(8);
            dialogBorder[3] = uiframe.imageMirrored(8);
            defpackage.BaseCanvas.yieldTick();
            PngMerger etcui = new PngMerger("/img/etcui");
            etcui.preloadAll = true;
            floaterIcon3 = FontManager.loadLocaleImage("_img_etcui__0.png");
            floaterIcon2 = FontManager.loadLocaleImage("_img_etcui__1.png");
            etcui.image(2);
            defpackage.BaseCanvas.yieldTick();
            entityShadow = etcui.image(3);
            statPointAlert = FontManager.loadLocaleImage("_img_etcui__4.png");
            numberFont1 = etcui.image(5);
            numberFont2 = etcui.image(6);
            defpackage.BaseCanvas.yieldTick();
            numberFont3 = etcui.image(7);
            numberFont4 = etcui.image(8);
            dropItemMarker = etcui.image(9);
            dropGoldMarker = etcui.image(10);
            skillChargeFill = etcui.image(11);
            defpackage.BaseCanvas.yieldTick();
            PngMerger levelUp = new PngMerger("/char/lvup");
            levelUpScript = readResource("/char/lvup.eif");
            defpackage.BaseCanvas.yieldTick();
            assembleSprites(true, levelUpScript, 0, (byte) 13, (byte) -1, levelUp);
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: h */
    /** Drops the shared in-game UI set. */
    public static final void unloadInGameUi() {
        hudFrame = null;
        dialogBorder = null;
        floaterIcon3 = null;
        floaterIcon2 = null;
        entityShadow = null;
        statPointAlert = null;
        numberFont1 = null;
        numberFont2 = null;
        numberFont3 = null;
        numberFont4 = null;
        dropItemMarker = null;
        dropGoldMarker = null;
        skillChargeFill = null;
        levelUpScript = null;
        spriteBanks[13] = null;
    }

    /* renamed from: i */
    /** Loads the status-effect icons ({@code /img/keepst}) and emoticons ({@code /img/emoti}). */
    public static final void loadStatusEffectIcons() {
        try {
            PngMerger keepst = new PngMerger("/img/keepst");
            keepst.preloadAll = true;
            emoticonBubble = keepst.image(0);
            defpackage.BaseCanvas.yieldTick();
            statusIcons = new Image[8];
            for (int i = 0; i < 8; i++) {
                statusIcons[i] = keepst.image(i + 1);
            }
            defpackage.BaseCanvas.yieldTick();
            emoticons = new PngMerger("/img/emoti").allImages();
            defpackage.BaseCanvas.yieldTick();
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: j */
    /** Drops the status-effect icons and emoticons. */
    public static final void unloadStatusEffectIcons() {
        emoticonBubble = null;
        statusIcons = null;
        emoticons = null;
    }

    /* renamed from: b */
    /** Loads the guardian element atlas (bank 12) and the summon-beam images for {@code element}. */
    public static final void loadGuardianElement(byte element) {
        guardianBeam = new Image[2];
        PngMerger elementAtlas = null;
        try {
            switch (element) {
                case 0:
                case 3:
                    elementAtlas = new PngMerger("/grd/fi");
                    break;
                case 1:
                case 4:
                    elementAtlas = new PngMerger("/grd/wa");
                    break;
                case 2:
                case 5:
                    elementAtlas = new PngMerger("/grd/gr");
                    break;
            }
            spriteBanks[12] = elementAtlas.allImages();
            PngMerger beam = new PngMerger(new StringBuffer().append("/grd/").append((int) element).toString());
            beam.preloadAll = true;
            guardianBeam[0] = beam.image(0);
            guardianBeam[1] = beam.image(1);
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: k */
    /** Drops the guardian element atlas and summon beam. */
    public static final void unloadGuardianElement() {
        guardianBeam = null;
        spriteBanks[12] = null;
    }

    /* renamed from: a */
    /**
     * Loads an enemy (or, when {@code npcMode}, an npc-style) sprite set for
     * {@code spriteId} into slot {@code slot}: decodes {@code /enm/spr/<NN>} into
     * {@link #enemyFrames}/{@link #npcFrames} against the {@code /enm/<NN>} atlas,
     * and, for AI-capable enemies, its attack effect ({@code /enm/atef/<NN>}) into
     * {@link #attackEffectScripts}.
     */
    public static final void loadEnemySprite(short spriteId, byte slot, boolean npcMode) {
        try {
            PngMerger atlas = new PngMerger(new StringBuffer().append("/enm/").append(spriteId < 10 ? "0" : "").append((int) spriteId).toString());
            atlas.preloadAll = true;
            defpackage.BaseCanvas.yieldTick();
            byte[] script = readResource(new StringBuffer().append("/enm/spr/").append(spriteId < 10 ? "0" : "").append((int) spriteId).toString());
            defpackage.BaseCanvas.yieldTick();
            int pos = 0;
            while (pos < script.length) {
                int groupPos = pos;
                int dirPos = pos + 1;
                byte group = script[groupPos];
                int lengthPos = dirPos + 1;
                byte dir = script[dirPos];
                int dataStart = lengthPos + 1;
                int length = script[lengthPos];
                if (npcMode) {
                    if (group == 0) {
                        npcAnimFrames0[slot] = script[dataStart];
                    } else if (group == 1) {
                        npcAnimFrames1[slot] = script[dataStart];
                    }
                    npcFrames[(slot * 12) + (group * 4) + dir] = new byte[length];
                    assembleSprites(true, script, dataStart, (byte) (27 + slot), (byte) (27 + slot + 5), atlas);
                    System.arraycopy(script, dataStart, npcFrames[(slot * 12) + (group * 4) + dir], 0, length);
                } else {
                    enemyFrames[(slot * 12) + (group * 4) + dir] = new byte[length];
                    assembleSprites(true, script, dataStart, (byte) (15 + slot), (byte) (15 + slot + 5), atlas);
                    System.arraycopy(script, dataStart, enemyFrames[(slot * 12) + (group * 4) + dir], 0, length);
                }
                pos = dataStart + length;
                defpackage.BaseCanvas.yieldTick();
            }
            if (!npcMode && defpackage.EnemyType.types[slot].aiType >= 2) {
                byte[] effect = readResource(new StringBuffer().append("/enm/atef/").append(spriteId < 10 ? "0" : "").append((int) spriteId).toString());
                assembleSprites(true, effect, 0, (byte) (15 + slot), (byte) (15 + slot + 5), atlas);
                attackEffectScripts[slot] = effect;
            }
            defpackage.BaseCanvas.yieldTick();
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: c */
    /** Drops enemy slot {@code slot}: its sprite banks (15+slot, 20+slot) and frame scripts. */
    public static final void unloadEnemySprite(byte slot) {
        spriteBanks[15 + slot] = null;
        spriteBanks[15 + slot + 5] = null;
        for (int i = 0; i < 12; i++) {
            enemyFrames[(slot * 12) + i] = null;
        }
    }

    /* renamed from: l */
    /** Loads the enemy death/explosion frame scripts ({@code /enm/die/0..2}) into bank 37. */
    public static final void loadDeathEffects() {
        try {
            PngMerger bang = new PngMerger("/enm/die/bang");
            bang.preloadAll = true;
            for (int i = 0; i < 3; i++) {
                byte[] script = readResource(new StringBuffer().append("/enm/die/").append(i).toString());
                assembleSprites(true, script, 0, (byte) 37, (byte) -1, bang);
                deathFxScripts[i] = script;
            }
        } catch (IOException unused) {
        }
    }

    /* renamed from: b */
    /**
     * Loads npc sprite id {@code spriteId} into npc slot {@code slot}: decodes
     * {@code /npc/spr/<NN>} into {@link #npcFrames} (banks 27+slot, 32+slot)
     * against the {@code /npc/<NN>} atlas, recording group-0/1 frame counts.
     */
    public static final void loadNpcSprite(byte spriteId, byte slot) {
        try {
            PngMerger atlas = new PngMerger(new StringBuffer().append("/npc/").append(spriteId < 10 ? "0" : "").append((int) spriteId).toString());
            atlas.preloadAll = true;
            defpackage.BaseCanvas.yieldTick();
            byte[] script = readResource(new StringBuffer().append("/npc/spr/").append(spriteId < 10 ? "0" : "").append((int) spriteId).toString());
            defpackage.BaseCanvas.yieldTick();
            int pos = 0;
            while (true) {
                if (pos >= script.length) {
                    return;
                }
                int groupPos = pos;
                int dirPos = pos + 1;
                byte group = script[groupPos];
                int lengthPos = dirPos + 1;
                byte dir = script[dirPos];
                int dataStart = lengthPos + 1;
                int length = script[lengthPos];
                npcFrames[(slot * 12) + (group * 4) + dir] = new byte[length];
                if (group == 0) {
                    npcAnimFrames0[slot] = script[dataStart];
                } else if (group == 1) {
                    npcAnimFrames1[slot] = script[dataStart];
                }
                assembleSprites(true, script, dataStart, (byte) (27 + slot), (byte) (27 + slot + 5), atlas);
                System.arraycopy(script, dataStart, npcFrames[(slot * 12) + (group * 4) + dir], 0, length);
                pos = dataStart + length;
                defpackage.BaseCanvas.yieldTick();
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: d */
    /** Drops npc slot {@code slot}: its sprite banks (27+slot, 32+slot) and frame scripts. */
    public static final void unloadNpcSprite(byte slot) {
        spriteBanks[27 + slot] = null;
        spriteBanks[27 + slot + 5] = null;
        for (int i = 0; i < 12; i++) {
            npcFrames[(slot * 12) + i] = null;
        }
    }

    /* renamed from: e */
    /**
     * Loads all sprites for boss {@code bossType} (id range per type) into
     * {@link #bossFrames} (banks 25-26), with boss-type-1 extras mirrored into
     * {@link #bossExtraFrames} and per-id attack effects into
     * {@link #attackEffectScripts}.
     */
    public static final void loadBossSprite(byte bossType) {
        byte rangeStart;
        byte rangeEnd;
        try {
            PngMerger atlas = new PngMerger(new StringBuffer().append("/boss/").append((int) bossType).toString());
            atlas.preloadAll = true;
            switch (bossType) {
                case 1:
                    rangeStart = 32;
                    rangeEnd = 32;
                    break;
                case 2:
                    rangeStart = 35;
                    rangeEnd = 38;
                    break;
                case 3:
                    rangeStart = 39;
                    rangeEnd = 41;
                    break;
                case 4:
                    rangeStart = 42;
                    rangeEnd = 42;
                    break;
                default:
                    defpackage.Debug.assertTrue(false);
                    rangeStart = -1;
                    rangeEnd = -1;
                    break;
            }
            byte bossId = rangeStart;
            while (bossId <= rangeEnd) {
                byte[] script = readResource(new StringBuffer().append("/boss/spr/").append((int) bossType).append("_").append((int) bossId).toString());
                int pos = 0;
                while (pos < script.length) {
                    byte slot = bossSlot(bossId);
                    int groupPos = pos;
                    int dirPos = pos + 1;
                    byte group = script[groupPos];
                    int lengthPos = dirPos + 1;
                    byte dir = script[dirPos];
                    int dataStart = lengthPos + 1;
                    byte[] frameScript = new byte[script[lengthPos] & 255];
                    assembleSprites(true, script, dataStart, (byte) 25, (byte) 26, atlas);
                    System.arraycopy(script, dataStart, frameScript, 0, frameScript.length);
                    pos = dataStart + frameScript.length;
                    if (group <= 3) {
                        bossFrames[(slot * 16) + (group * 4) + dir] = frameScript;
                    }
                    if (bossType == 1 && group >= 3) {
                        bossExtraFrames[((group - 3) * 4) + dir] = frameScript;
                    }
                }
                byte[] effect = readResource(new StringBuffer().append("/boss/atef/").append(bossId < 10 ? "0" : "").append((int) bossId).toString());
                if (effect != null) {
                    assembleSprites(true, effect, 0, (byte) 25, (byte) 26, atlas);
                    attackEffectScripts[bossSlot(bossId)] = effect;
                }
                bossId = (byte) (bossId + 1);
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: m */
    /** Drops the boss sprite banks (25-26) and extra frame scripts. */
    public static final void unloadBossSprite() {
        spriteBanks[25] = null;
        spriteBanks[26] = null;
        bossExtraFrames = new Object[12];
    }

    /* renamed from: n */
    /** Loads the character-menu tab icons and equip-slot icons ({@code /sgui/gmico}). */
    public static final void loadGameMenuIcons() {
        menuTabIcons = new Image[6];
        equipSlotIcons = new Image[5];
        try {
            PngMerger gmico = new PngMerger("/sgui/gmico");
            gmico.preloadAll = true;
            byte tab = 0;
            while (tab < 6) {
                menuTabIcons[tab] = gmico.image((int) (tab == 5 ? (byte) 6 : tab));
                tab = (byte) (tab + 1);
            }
            for (byte slot = 0; slot < 5; slot = (byte) (slot + 1)) {
                equipSlotIcons[slot] = gmico.image(slot + 7);
            }
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: o */
    /** Loads the global UI images ({@code /img/glb}) and the help string table. */
    public static final void loadGlobalUi() {
        try {
            PngMerger glb = new PngMerger("/img/glb");
            glb.preloadAll = true;
            scrollUpArrow = glb.image(0);
            scrollDownArrow = glb.image(1);
            slotFrame = glb.image(2);
            cursorArrow = glb.image(3);
            numberFont0 = glb.image(5);
            portraitFrame = glb.image(6);
            goldIcon = glb.image(7);
            statusPanelIcon = glb.image(8);
            statLabel1 = FontManager.loadLocaleImage("_img_glb__9.png");
            statLabel2 = FontManager.loadLocaleImage("_img_glb__10.png");
            statLabel3 = FontManager.loadLocaleImage("_img_glb__11.png");
            statLabel4 = FontManager.loadLocaleImage("_img_glb__12.png");
            FontManager.loadLocaleImage("_img_glb__13.png");
            fractionSlash = glb.image(14);
            statLabel5 = FontManager.loadLocaleImage("_img_glb__15.png");
            glb.image(16);
            helpText = new TextTable("/sgui/help");
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: p */
    /** Loads the item-type icons ({@code /img/icoitm}). */
    public static final void loadItemIcons() {
        try {
            itemIcons = new PngMerger("/img/icoitm").allImages();
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    /* renamed from: q */
    /** Drops the item-type icons. */
    public static final void unloadItemIcons() {
        itemIcons = null;
    }

    /* renamed from: r */
    /** Loads the shop UI ({@code /sgui/shop}): category icons, cursor and buy/sell icons. */
    public static final void loadShopUi() {
        defpackage.BaseCanvas.yieldTick();
        shopCategoryIcons = new Image[6];
        try {
            PngMerger shop = new PngMerger("/sgui/shop");
            shop.preloadAll = true;
            defpackage.BaseCanvas.yieldTick();
            for (byte category = 0; category < 6; category = (byte) (category + 1)) {
                shopCategoryIcons[category] = shop.image((int) category);
            }
            shopCoinIcon = shop.image(6);
            shopSelectBox = shop.image(7);
            defpackage.BaseCanvas.yieldTick();
            shopBuyIcon = FontManager.loadLocaleImage("_sgui_shop__8.png");
            shopSellIcon = FontManager.loadLocaleImage("_sgui_shop__9.png");
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: s */
    /** Drops the shop UI. */
    public static final void unloadShopUi() {
        shopCategoryIcons = null;
        shopCoinIcon = null;
        shopSelectBox = null;
        shopBuyIcon = null;
        shopSellIcon = null;
    }

    /* renamed from: t */
    /** Loads the guardian icons and per-skill icons ({@code /grd/grdico}). */
    public static final void loadGuardianIcons() {
        guardianIcons = new Image[6];
        guardianSkillIcons = new Image[24];
        try {
            PngMerger grdico = new PngMerger("/grd/grdico");
            grdico.preloadAll = true;
            for (byte guardian = 0; guardian < 6; guardian = (byte) (guardian + 1)) {
                guardianIcons[guardian] = grdico.image((int) guardian);
                for (byte skill = 0; skill < 4; skill = (byte) (skill + 1)) {
                    guardianSkillIcons[(guardian * 4) + skill] = grdico.image(6 + (guardian * 4) + skill);
                }
                defpackage.BaseCanvas.yieldTick();
            }
        } catch (Exception e) {
            System.out.println(e);
        }
    }

    /* renamed from: u */
    /** Drops the guardian icons and skill icons. */
    public static final void unloadGuardianIcons() {
        guardianIcons = null;
        guardianSkillIcons = null;
    }

    /* renamed from: a */
    /** Reads record {@code record} of item file {@code itemId} from {@code /itm/<NN>}. */
    public static final byte[] loadItemRecord(byte itemId, byte record) {
        byte[] result = null;
        String idText = String.valueOf((int) itemId);
        if (itemId < 10) {
            idText = new StringBuffer().append("0").append(idText).toString();
        }
        try {
            InputStream in = new Object().getClass().getResourceAsStream(new StringBuffer().append("/itm/").append(idText).toString());
            for (int i = 0; i < record; i++) {
                in.skip(in.read());
            }
            result = new byte[in.read()];
            in.read(result);
            in.close();
        } catch (IOException e) {
            e.printStackTrace();
        }
        return result;
    }

    /* renamed from: a */
    /** Returns the shop item-list data ({@code /itm/forshop}). */
    public static final byte[] loadShopItemData() {
        return readResource("/itm/forshop");
    }

    /* renamed from: f */
    /** Loads a full guardian of {@code type}: element atlas plus its element/pose sprite scripts. */
    public static final void loadGuardian(byte type) {
        defpackage.BaseCanvas.yieldTick();
        loadGuardianElement(type);
        switch (type) {
            case 0:
                loadGuardianSprite(type, (byte) 0);
                loadGuardianSprite(type, (byte) 1);
                break;
            case 1:
                loadGuardianSprite(type, (byte) 0);
                loadGuardianSprite(type, (byte) 1);
                break;
            case 2:
            case 3:
            case 4:
            case 5:
                loadGuardianSprite(type, (byte) 0);
                loadGuardianSprite(type, (byte) 1);
                loadGuardianSprite(type, (byte) 2);
                break;
        }
    }

    /* renamed from: v */
    /** Drops the guardian element atlas and sprite frame scripts. */
    public static final void unloadGuardian() {
        unloadGuardianElement();
        unloadGuardianSprites();
    }

    /* renamed from: w */
    /** Loads the title logo frames ({@code /img/logo}). */
    public static final void loadLogo() {
        try {
            logoFrames = new PngMerger("/img/logo").allImages();
        } catch (IOException unused) {
        }
    }

    /* renamed from: x */
    /** Drops the title logo frames. */
    public static final void unloadLogo() {
        logoFrames = null;
    }

    /* renamed from: y */
    /** Loads the title screen: background frames, menu frames and its title jingle. */
    public static final void loadTitleScreen() {
        // Reconstructed from the JADX smali dump for defpackage.AssetCache.y() (JADX
        // could not lift it: "Unexpected instance arg in invoke"); verified
        // against the CFR decompile.
        try {
            PngMerger title = new PngMerger("/img/title1");
            titleBgFrames = title.allImages();
            defpackage.BaseCanvas.yieldTick();
            title = new PngMerger("/img/title2");
            title.preloadAll = true;
            titleMenuFrames = new Image[10];
            for (int i = 0; i < 5; i++) {
                titleMenuFrames[i] = title.image(i);
                titleMenuFrames[i + 5] = title.imageMirrored(i);
                defpackage.BaseCanvas.yieldTick();
            }
            defpackage.BaseCanvas.yieldTick();
            AudioManager.loadClip((byte) 22);
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: z */
    /** Drops the title screen frames and its jingle. */
    public static final void unloadTitleScreen() {
        titleBgFrames = null;
        titleMenuFrames = null;
        AudioManager.unloadClip((byte) 22);
    }

    /* renamed from: A */
    /** Loads the main-menu assets: class faces, menu frame images and guardian previews. */
    public static final void loadMainMenuAssets() {
        try {
            PngMerger faces = new PngMerger("/sgui/mm/face");
            faces.preloadAll = true;
            classFaces = new Image[6];
            classFaces[0] = faces.image(0);
            classFaces[1] = faces.image(1);
            classFaces[2] = faces.image(2);
            defpackage.BaseCanvas.yieldTick();
            classFaces[3] = faces.imageGray(0);
            classFaces[4] = faces.imageGray(1);
            classFaces[5] = faces.imageGray(2);
            defpackage.BaseCanvas.yieldTick();
            menuFrames = new PngMerger("/sgui/mm/etc").allImages();
            menuGuardianPreview = new Image[3][2];
            for (int i = 0; i < 3; i++) {
                PngMerger preview = new PngMerger(new StringBuffer().append("/grd/").append(i).toString());
                preview.preloadAll = true;
                menuGuardianPreview[i][0] = preview.image(0);
                menuGuardianPreview[i][1] = preview.image(1);
                defpackage.BaseCanvas.yieldTick();
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: B */
    /** Drops the main-menu assets. */
    public static final void unloadMainMenuAssets() {
        menuFrames = null;
        classFaces = null;
        menuGuardianPreview = (Image[][]) null;
    }

    /* renamed from: a */
    /**
     * Reads a JAR resource {@code path} fully into a {@code byte[]} through
     * {@link #readBuffer}, or {@code null} if missing. Blocks while the game sits
     * on the loading screen ({@code GameState.screen == 15}).
     */
    public static final byte[] readResource(String path) {
        System.gc();
        byte[] result = null;
        try {
            InputStream in = new Object().getClass().getResourceAsStream(path);
            if (in == null) {
                return null;
            }
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            while (true) {
                int read = in.read(readBuffer);
                if (read == -1) {
                    break;
                }
                out.write(readBuffer, 0, read);
            }
            result = out.toByteArray();
            out.close();
        } catch (Exception e) {
            new StringBuffer().append("miss - ").append(path).toString();
            e.printStackTrace();
        }
        while (defpackage.GameState.screen == 15) {
            try {
                Thread.sleep(100L);
            } catch (InterruptedException e) {
                e.printStackTrace();
            }
        }
        return result;
    }

    /* renamed from: a */
    /** Reverse-maps a boss sprite id to its slot via {@link #bossSpriteIds}, or {@code -1}. */
    public static final byte bossSlot(byte bossId) {
        byte slot = 0;
        while (true) {
            if (slot >= bossSpriteIds.length) {
                return (byte) -1;
            }
            if (bossSpriteIds[slot] == bossId) {
                return slot;
            }
            slot = (byte) (slot + 1);
        }
    }
}
