package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: bu */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bu.class */
/**
 * Background asset loader. Each of the entry points ({@link #loadResources},
 * {@link #loadMap}, {@link #loadGuardian}, {@link #loadMainMenu}) sets a
 * {@link #phase}, shows a loading screen and spawns a worker {@link Thread};
 * {@link #run} then does the phase's heavy work (feeding {@link AssetCache}) off
 * the UI thread. The "- RESOURCE" and "- MAP" phases are the two big loads (game
 * resources and a map warp).
 *
 * <p>The bulk of the work is decoding the hero/guardian sprite atlases:
 * {@link #loadSpriteBank} reads a per-class animation script from the
 * {@code c1/s}..{@code c3/s} script dirs, pulls frames out of the paired
 * {@code c1/i}..{@code c3/i} atlas via
 * {@link PngMerger}, and writes the assembled frame tables into
 * {@link AssetCache}. The {@code *Anim} tables and file-name arrays map a
 * class/equipment sub-index to the atlas file and sprite bank to use.
 */
public final class AssetLoader implements Runnable {
    /* renamed from: a */
    /** Which load phase this worker runs (1 resources, 2 map, 3 guardian, 5 menu). */
    private static byte phase = 0;

    /* renamed from: g */
    /** Animation-script file suffixes per sprite bank kind. */
    private static final String[] scriptSuffixes = {"a", "b", "e", "hA", "hB", "w", "s"};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Per-class animation-script directories ({@code c1/s} .. {@code c3/s}). */
    public static final String[] scriptDirs = {"/c1/s/", "/c2/s/", "/c3/s/"};
    /** Per-class sprite-atlas directories ({@code c1/i} .. {@code c3/i}). */
    public static final String[] atlasDirs = {"/c1/i/", "/c2/i/", "/c3/i/"};
    /** Armor atlas file names. */
    public static final String[] armorFiles = {"a1", "a2", "a3", "a4", "a5", "a6"};
    /** Head atlas file names. */
    public static final String[] headFiles = {"h1", "h2", "h3", "h4", "h5", "h6", "h7"};
    /** Weapon atlas file names. */
    public static final String[] weaponFiles = {"w1", "w2", "w3", "w4", "w5"};
    /** Shield atlas file names. */
    public static final String[] shieldFiles = {"s1", "s2", "s3", "s4", "s5"};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Weapon-subId to atlas-file index, per class. */
    public static final byte[][] weaponAnim = {new byte[]{0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 0, 3, 3, 4, 1, 2, 3}, new byte[]{0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 0, 1, 2, 4, 3, 4, 1}, new byte[]{0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 0, 2, 4, 1, 2, 4, -1}};

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Armor-subId to atlas-file index, per class ({@code -1} = none). */
    public static final byte[][] armorAnim = {new byte[]{-1, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3}, new byte[]{-1, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3}, new byte[]{0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 2, 0, 4, 5, 4, 3}};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Head-subId to atlas-file index. */
    public static final byte[] headAnim = {0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6};

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Shield-subId to atlas-file index. */
    public static final byte[] shieldAnim = {0, 0, 0, 1, 1, 1, 2, 2, 2, 4, 4, 4, 2, 0, 4};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Whether the shared in-game resources have been loaded once. */
    private static boolean commonLoaded = false;

    /* renamed from: a */
    /** Starts the "- RESOURCE" load on a worker thread. */
    public static final void loadResources() {
        phase = (byte) 1;
        BaseCanvas.beginLoading("- RESOURCE", 500);
        new Thread(new AssetLoader()).start();
    }

    /* renamed from: b */
    /** Starts the "- MAP" (map warp) load on a worker thread. */
    public static final void loadMap() {
        GameLoop.gameScreen.resetHudState();
        phase = (byte) 2;
        BaseCanvas.beginLoading("- MAP", 200);
        new Thread(new AssetLoader()).start();
    }

    /* renamed from: c */
    /** Starts the guardian-summon load on a worker thread. */
    public static final void loadGuardian() {
        phase = (byte) 3;
        BaseCanvas.beginLoading("가디언 소환중..", 120);
        new Thread(new AssetLoader()).start();
    }

    /* renamed from: d */
    /** Starts the return-to-main-menu unload/load on a worker thread. */
    public static final void loadMainMenu() {
        phase = (byte) 5;
        BaseCanvas.beginLoading("- MAIN MENU", 100);
        new Thread(new AssetLoader()).start();
    }

    @Override // java.lang.Runnable
    public final void run() {
        try {
            Thread.sleep(100L);
        } catch (InterruptedException unused) {
        }
        switch (phase) {
            case 1:
                try {
                    Thread.sleep(1000L);
                    System.out.println("sleep");
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
                BaseCanvas.yieldTick();
                if (!commonLoaded) {
                    loadCommonAssets();
                }
                try {
                    AssetCache.classSkillText = new TextTable(new StringBuffer().append("/sgui/q").append((int) GameState.classId).toString());
                    BaseCanvas.yieldTick();
                    break;
                } catch (Exception unused2) {
                }
                loadHeroEquipSprites();
                loadGuardianSprites();
                GameState.requestMapWarp(GameState.storyMapId, (byte) 1, GameState.arg0, GameState.arg1);
                BaseCanvas.keepLoadingProgress = true;
                break;
            case 2:
                swapMap();
                GameState.setHeroTile((int) GameState.arg1, (int) GameState.arg2);
                GameState.map.fadeStep();
                GameState.requestState((byte) 15, GameState.arg0);
                break;
            case 3:
                unloadGuardianSprites();
                loadGuardianSprites();
                GameState.requestState((byte) 2, (byte) 2, (byte) 1);
                break;
            case 5:
                unloadHeroSprites();
                unloadGuardianSprites();
                AssetCache.loadMainMenuAssets();
                GameState.buildLoadMenu();
                break;
        }
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Unlinks the hero/guardian, drops the old map and loads the destination map. */
    private final GameMap swapMap() {
        BaseCanvas.yieldTick();
        GameMap oldMap = GameState.map;
        Hero hero = GameState.hero();
        if (oldMap != null) {
            oldMap.removeEntity(hero);
            ((Entity) hero).next = null;
            ((Entity) hero).prev = null;
            Guardian guardian = hero.getActiveGuardian();
            if (guardian != null) {
                oldMap.removeEntity(guardian);
                ((Entity) guardian).next = null;
                ((Entity) guardian).prev = null;
            }
        }
        BaseCanvas.yieldTick();
        GameState.map = null;
        GameMap newMap = new GameMap(GameState.storyMapId);
        GameState.setMap(newMap);
        BaseCanvas.yieldTick();
        newMap.load();
        BaseCanvas.yieldTick();
        return newMap;
    }

    /* renamed from: e */
    /** Loads the guardian and hero string tables. */
    public static final void loadStringTables() {
        try {
            AssetCache.guardianText = new TextTable("/grd/grd");
            BaseCanvas.yieldTick();
            AssetCache.heroText = new TextTable("/char/hero");
            BaseCanvas.yieldTick();
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: i */
    /** Loads the shared in-game resources (tiles, UI, tables, sounds) once. */
    private final void loadCommonAssets() {
        commonLoaded = true;
        BaseCanvas.yieldTick();
        AssetCache.loadGameMenuIcons();
        BaseCanvas.yieldTick();
        AssetCache.loadItemIcons();
        BaseCanvas.yieldTick();
        AssetCache.loadGuardianIcons();
        BaseCanvas.yieldTick();
        AssetCache.loadInGameUi();
        BaseCanvas.yieldTick();
        AssetCache.loadStatusEffectIcons();
        BaseCanvas.yieldTick();
        AssetCache.loadShopUi();
        BaseCanvas.yieldTick();
        AssetCache.loadDeathEffects();
        BaseCanvas.yieldTick();
        try {
            AssetCache.guardianSkillText = new TextTable("/grd/grdsk");
            BaseCanvas.yieldTick();
            AssetCache.mapNameText = new TextTable("/m/name");
            BaseCanvas.yieldTick();
        } catch (IOException e) {
            e.printStackTrace();
        }
        for (byte clip = 5; clip <= 8; clip = (byte) (clip + 1)) {
            AudioManager.loadClip(clip);
        }
        for (byte clip = 12; clip <= 15; clip = (byte) (clip + 1)) {
            AudioManager.loadClip(clip);
        }
    }

    /* renamed from: f */
    /** Unloads the shared in-game resources and sounds. */
    public static final void unloadInGame() {
        commonLoaded = false;
        AssetCache.unloadItemIcons();
        AssetCache.unloadGuardianIcons();
        AssetCache.unloadInGameUi();
        AssetCache.unloadStatusEffectIcons();
        AssetCache.unloadShopUi();
        AssetCache.guardianSkillText = null;
        AssetCache.mapNameText = null;
        for (byte clip = 5; clip <= 8; clip = (byte) (clip + 1)) {
            AudioManager.unloadClip(clip);
        }
        for (byte clip = 12; clip <= 15; clip = (byte) (clip + 1)) {
            AudioManager.unloadClip(clip);
        }
    }

    /* renamed from: j */
    /** Loads the hero's body/armor/head/shield sprites from the current equipment. */
    private final void loadHeroEquipSprites() {
        AssetCache.heroFrames = new Object[396];
        Hero hero = GameState.hero();
        if (hero.getAccessory1() != null) {
            new StringBuffer().append("HERO ARMOR ").append((int) armorAnim[GameState.classId - 6][hero.getAccessory1().subId]).toString();
            BaseCanvas.yieldTick();
            loadArmorSprite(GameState.classId, hero.getEquip(2).subId);
        }
        BaseCanvas.yieldTick();
        loadSpriteBank(GameState.classId, (byte) 1, (byte) 0, false, (byte) 0);
        if (hero.getAccessory2() != null) {
            new StringBuffer().append("HERO HEAD ").append((int) headAnim[hero.getAccessory2().subId]).toString();
            BaseCanvas.yieldTick();
            loadHeadSprite(GameState.classId, hero.getAccessory2().subId);
        } else {
            BaseCanvas.yieldTick();
            loadHeadSprite(GameState.classId, (byte) 0);
        }
        if (GameState.classId != 8 || hero.getArmor() == null) {
            return;
        }
        new StringBuffer().append("HERO SHIELD ").append((int) shieldAnim[((Item) hero.getArmor()).subId]).toString();
        BaseCanvas.yieldTick();
        loadShieldSprite(GameState.classId, ((Item) hero.getArmor()).subId);
    }

    /* renamed from: g */
    /** Unloads the hero body/armor/head/shield sprite banks. */
    public static final void unloadHeroSprites() {
        unloadSpriteBank(0);
        unloadSpriteBank(1);
        unloadSpriteBank(2);
        unloadSpriteBank(5);
    }

    /* renamed from: k */
    /** Loads the active guardian's sprites (and the hero's weapon aura). */
    private final void loadGuardianSprites() {
        Hero hero = GameState.hero();
        Guardian guardian = hero.getActiveGuardian();
        BaseCanvas.yieldTick();
        BaseCanvas.yieldTick();
        AssetCache.loadGuardian(guardian.type);
        BaseCanvas.yieldTick();
        if (hero.getEquip(0) != null) {
            loadWeaponSprite(GameState.classId, (Weapon) hero.getEquip(0), false, guardian.element());
        }
        BaseCanvas.yieldTick();
        loadAuraSprite(GameState.classId, guardian.element());
        BaseCanvas.yieldTick();
        AssetCache.loadAttackEffects(guardian.element());
        BaseCanvas.yieldTick();
        switch (guardian.type) {
            case 0:
                AudioManager.loadClip((byte) 16);
                AudioManager.loadClip((byte) 21);
                break;
            case 1:
                AudioManager.loadClip((byte) 20);
                break;
            case 2:
                AudioManager.loadClip((byte) 17);
                AudioManager.loadClip((byte) 21);
                break;
            case 3:
                AudioManager.loadClip((byte) 16);
                break;
            case 4:
                AudioManager.loadClip((byte) 18);
                AudioManager.loadClip((byte) 20);
                break;
            case 5:
                AudioManager.loadClip((byte) 17);
                break;
        }
    }

    /* renamed from: h */
    /** Unloads the guardian sprites, aura banks and guardian sounds. */
    public static final void unloadGuardianSprites() {
        AssetCache.unloadGuardian();
        unloadSpriteBank(3);
        unloadSpriteBank(4);
        AssetCache.unloadAttackEffects();
        for (byte clip = 16; clip <= 21; clip = (byte) (clip + 1)) {
            AudioManager.unloadClip(clip);
        }
    }

    /* renamed from: a */
    /** Loads the weapon sprite for {@code weapon} (bank 5). */
    public static final void loadWeaponSprite(byte classId, Weapon weapon, boolean weaponPreview, byte element) {
        loadSpriteBank(classId, (byte) 5, weaponAnim[GameState.classId - 6][((Item) weapon).subId], weaponPreview, element);
    }

    /* renamed from: a */
    /** Loads the armor sprite for armor sub-index {@code subId} (bank 0). */
    public static final void loadArmorSprite(byte classId, byte subId) {
        if (armorAnim[GameState.classId - 6][subId] == -1) {
            unloadSpriteBank(0);
        } else {
            loadSpriteBank(classId, (byte) 0, armorAnim[GameState.classId - 6][subId], false, (byte) 0);
        }
    }

    /* renamed from: b */
    /** Loads the head sprite for head sub-index {@code subId} (bank 3 or 4). */
    public static final void loadHeadSprite(byte classId, byte subId) {
        byte bank = 4;
        if (classId == 6 && subId >= 0 && subId <= 3) {
            bank = 3;
        }
        loadSpriteBank(classId, bank, headAnim[subId], false, (byte) 0);
    }

    /* renamed from: c */
    /** Loads the shield sprite for shield sub-index {@code subId} (bank 6). */
    public static final void loadShieldSprite(byte classId, byte subId) {
        loadSpriteBank(classId, (byte) 6, shieldAnim[subId], false, (byte) 0);
    }

    /* renamed from: d */
    /** Loads the guardian aura sprite tinted for {@code element} (bank 2). */
    public static final void loadAuraSprite(byte classId, byte element) {
        loadSpriteBank(classId, (byte) 2, (byte) 0, false, element);
    }

    /* renamed from: a */
    /**
     * Loads one sprite bank: opens the atlas for {@code fileIndex} via
     * {@link PngMerger}, reads the animation script for {@code bankKind} from
     * the {@code c*}{@code /s} script dir, and assembles the per-frame image tables into
     * {@link AssetCache}. {@code weaponPreview} routes a weapon into the preview
     * tables, and {@code element} recolours guardian aura/weapon palettes.
     */
    private static final void loadSpriteBank(byte classId, byte bankKind, byte fileIndex, boolean weaponPreview, byte element) {
        int normalDest;
        int mirrorDest;
        byte classIndex = (byte) (classId - 6);
        PngMerger merger = null;
        Image[] frames = null;
        Image[] mirroredFrames = null;
        byte spriteBank = -1;
        try {
            switch (bankKind) {
                case 0:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append(armorFiles[fileIndex]).toString());
                    Image[][] armorBanks = AssetCache.spriteBanks;
                    Image[] armorFrames = new Image[merger.frameCount()];
                    frames = armorFrames;
                    armorBanks[0] = armorFrames;
                    Image[][] armorMirrorBanks = AssetCache.spriteBanks;
                    Image[] armorMirror = new Image[merger.frameCount()];
                    mirroredFrames = armorMirror;
                    armorMirrorBanks[6] = armorMirror;
                    spriteBank = 0;
                    break;
                case 1:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append("b").toString());
                    Image[][] bodyBanks = AssetCache.spriteBanks;
                    Image[] bodyFrames = new Image[merger.frameCount()];
                    frames = bodyFrames;
                    bodyBanks[1] = bodyFrames;
                    Image[][] bodyMirrorBanks = AssetCache.spriteBanks;
                    Image[] bodyMirror = new Image[merger.frameCount()];
                    mirroredFrames = bodyMirror;
                    bodyMirrorBanks[7] = bodyMirror;
                    spriteBank = 1;
                    break;
                case 2:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append("e").toString());
                    Image[][] auraBanks = AssetCache.spriteBanks;
                    Image[] auraFrames = new Image[merger.frameCount()];
                    frames = auraFrames;
                    auraBanks[4] = auraFrames;
                    Image[][] auraMirrorBanks = AssetCache.spriteBanks;
                    Image[] auraMirror = new Image[merger.frameCount()];
                    mirroredFrames = auraMirror;
                    auraMirrorBanks[10] = auraMirror;
                    spriteBank = 4;
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
                    if (classId == 8) {
                        loadMageShieldFrames(merger, (byte) 4, (byte) 10);
                    }
                    break;
                case 3:
                case 4:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append(headFiles[fileIndex]).toString());
                    Image[][] headBanks = AssetCache.spriteBanks;
                    Image[] headFrames = new Image[merger.frameCount()];
                    frames = headFrames;
                    headBanks[2] = headFrames;
                    Image[][] headMirrorBanks = AssetCache.spriteBanks;
                    Image[] headMirror = new Image[merger.frameCount()];
                    mirroredFrames = headMirror;
                    headMirrorBanks[8] = headMirror;
                    spriteBank = 2;
                    break;
                case 5:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append(weaponFiles[fileIndex]).toString());
                    if (weaponPreview) {
                        frames = new Image[merger.frameCount()];
                        mirroredFrames = new Image[merger.frameCount()];
                    } else {
                        Image[][] weaponBanks = AssetCache.spriteBanks;
                        Image[] weaponFrames = new Image[merger.frameCount()];
                        frames = weaponFrames;
                        weaponBanks[3] = weaponFrames;
                        Image[][] weaponMirrorBanks = AssetCache.spriteBanks;
                        Image[] weaponMirror = new Image[merger.frameCount()];
                        mirroredFrames = weaponMirror;
                        weaponMirrorBanks[9] = weaponMirror;
                    }
                    spriteBank = 3;
                    if (element != 0) {
                        switch (element) {
                            case 1:
                                merger.remapPalette(255, 16744255);
                                break;
                            case 2:
                                merger.remapPalette(255, 6258623);
                                break;
                            case 3:
                                merger.remapPalette(255, 8388479);
                                break;
                        }
                    }
                    break;
                case 6:
                    merger = new PngMerger(new StringBuffer().append(atlasDirs[classIndex]).append(shieldFiles[fileIndex]).toString());
                    Image[][] shieldBanks = AssetCache.spriteBanks;
                    Image[] shieldFrames = new Image[merger.frameCount()];
                    frames = shieldFrames;
                    shieldBanks[5] = shieldFrames;
                    Image[][] shieldMirrorBanks = AssetCache.spriteBanks;
                    Image[] shieldMirror = new Image[merger.frameCount()];
                    mirroredFrames = shieldMirror;
                    shieldMirrorBanks[11] = shieldMirror;
                    spriteBank = 5;
                    break;
            }
            BaseCanvas.yieldTick();
            merger.preloadAll = true;
            byte[] script = AssetCache.readResource(new StringBuffer().append(scriptDirs[classIndex]).append(scriptSuffixes[bankKind]).toString());
            BaseCanvas.yieldTick();
            int pos = 0;
            while (pos < script.length) {
                int p0 = pos;
                int p1 = pos + 1;
                byte action = script[p0];
                int p2 = p1 + 1;
                byte row = script[p1];
                int p3 = p2 + 1;
                byte col = script[p2];
                pos = p3 + 1;
                byte frameCount = script[p3];
                if (bankKind != 2) {
                    byte[] entry = new byte[1 + (frameCount * 4)];
                    int w = 0 + 1;
                    entry[0] = frameCount;
                    for (int i = 0; i < frameCount; i++) {
                        int wDx = w;
                        int wDx1 = w + 1;
                        int posDx = pos;
                        int posDx1 = pos + 1;
                        entry[wDx] = script[posDx];
                        int wDy = wDx1 + 1;
                        int posDy = posDx1 + 1;
                        entry[wDx1] = script[posDx1];
                        int posFlag = posDy + 1;
                        boolean mirrored = script[posDy] != 0;
                        if (mirrored) {
                            mirrorDest = wDy + 1;
                            entry[wDy] = (byte) (spriteBank + 6);
                        } else {
                            mirrorDest = wDy + 1;
                            entry[wDy] = spriteBank;
                        }
                        int wImg = mirrorDest;
                        w = mirrorDest + 1;
                        pos = posFlag + 1;
                        byte imageIndex = script[posFlag];
                        entry[wImg] = imageIndex;
                        if (imageIndex != -1) {
                            if (!mirrored && frames[imageIndex] == null) {
                                frames[imageIndex] = merger.image((int) imageIndex);
                            } else if (mirrored && mirroredFrames[imageIndex] == null) {
                                mirroredFrames[imageIndex] = merger.imageMirrored((int) imageIndex);
                            }
                        }
                    }
                    if (weaponPreview) {
                        AssetCache.weaponPreviewFrames[(row * 4) + col] = entry;
                    } else {
                        AssetCache.heroFrames[(row * 36) + (col * 9) + action] = entry;
                    }
                } else {
                    int scan = pos;
                    for (int i = 0; i < frameCount; i++) {
                        int scanCount = scan;
                        scan++;
                        for (int sub = 0; sub < script[scanCount]; sub++) {
                            scan += 4;
                        }
                    }
                    byte[] entry = new byte[1 + (scan - pos)];
                    int w = 0 + 1;
                    entry[0] = frameCount;
                    for (int i = 0; i < frameCount; i++) {
                        int wSub = w;
                        w++;
                        int posSub = pos;
                        pos++;
                        byte subCount = script[posSub];
                        entry[wSub] = subCount;
                        for (int sub = 0; sub < subCount; sub++) {
                            int wDx = w;
                            int wDx1 = w + 1;
                            int posDx = pos;
                            int posDx1 = pos + 1;
                            entry[wDx] = script[posDx];
                            int wDy = wDx1 + 1;
                            int posDy = posDx1 + 1;
                            entry[wDx1] = script[posDx1];
                            int posFlag = posDy + 1;
                            boolean mirrored = script[posDy] != 0;
                            if (mirrored) {
                                normalDest = wDy + 1;
                                entry[wDy] = (byte) (spriteBank + 6);
                            } else {
                                normalDest = wDy + 1;
                                entry[wDy] = spriteBank;
                            }
                            int wImg = normalDest;
                            w = normalDest + 1;
                            pos = posFlag + 1;
                            byte imageIndex = script[posFlag];
                            entry[wImg] = imageIndex;
                            if (!mirrored && frames[imageIndex] == null) {
                                frames[imageIndex] = merger.image((int) imageIndex);
                            } else if (mirrored && mirroredFrames[imageIndex] == null) {
                                mirroredFrames[imageIndex] = merger.imageMirrored((int) imageIndex);
                            }
                        }
                    }
                    AssetCache.heroFrames[(row * 36) + (col * 9) + action] = entry;
                }
                BaseCanvas.yieldTick();
            }
            merger.unloadAllMpd();
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: a */
    /** Clears sprite bank {@code bank} (and its mirror, and derived frame tables). */
    public static final void unloadSpriteBank(int bank) {
        AssetCache.spriteBanks[bank] = null;
        AssetCache.spriteBanks[bank + 6] = null;
        if (bank == 3) {
            AssetCache.weaponPreviewFrames = null;
        }
        for (int row = 0; row < 11; row++) {
            for (int col = 0; col < 4; col++) {
                switch (bank) {
                    case 0:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 2] = null;
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 3] = null;
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 4] = null;
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 5] = null;
                        break;
                    case 1:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 0] = null;
                        break;
                    case 2:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 1] = null;
                        break;
                    case 3:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 6] = null;
                        break;
                    case 4:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 7] = null;
                        break;
                    case 5:
                        AssetCache.heroFrames[(row * 36) + (col * 9) + 8] = null;
                        break;
                }
            }
        }
    }

    /* renamed from: a */
    /** Loads the extra mage-shield aura frame sets ({@code ea2}/{@code ea3}/{@code ea4}). */
    public static final void loadMageShieldFrames(PngMerger merger, byte bank, byte mirrorBank) {
        merger.preloadAll = true;
        AssetCache.mageAuraScripts = new Object[3];
        Object[] extra0 = AssetCache.mageAuraScripts;
        byte[] ea2 = AssetCache.readResource(new StringBuffer().append(scriptDirs[2]).append("ea2").toString());
        extra0[0] = ea2;
        AssetCache.assembleSprites(true, ea2, 0, bank, mirrorBank, merger);
        Object[] extra1 = AssetCache.mageAuraScripts;
        byte[] ea3 = AssetCache.readResource(new StringBuffer().append(scriptDirs[2]).append("ea3").toString());
        extra1[1] = ea3;
        AssetCache.assembleSprites(true, ea3, 0, bank, mirrorBank, merger);
        Object[] extra2 = AssetCache.mageAuraScripts;
        byte[] ea4 = AssetCache.readResource(new StringBuffer().append(scriptDirs[2]).append("ea4").toString());
        extra2[2] = ea4;
        AssetCache.assembleSprites(true, ea4, 0, bank, mirrorBank, merger);
    }

    /* renamed from: a */
    /** Draws the phase-appropriate loading overlay. */
    public static final void drawLoadingOverlay(Graphics graphics) {
        switch (phase) {
            case 1:
            case 2:
                BaseCanvas.drawLoadingScreen(graphics);
                break;
            case 3:
                GameScreen.drawLoadBox(graphics, (BaseCanvas.width - 145) >> 1, BaseCanvas.halfH - 15, 145, 30);
                break;
        }
    }
}
