package defpackage;

import java.io.IOException;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: ae */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:GameMap.class */
/**
 * One loaded level: tile and collision grids, the tile-occupancy array, the NPC
 * / object / enemy entity list, dropped pickups, the delayed-enemy spawn queue,
 * the trigger/event/dialogue tables, and the boss-encounter setup for the four
 * special map types. Parses the packed {@code .map}/{@code .evt} assets and
 * paints the world, the zone banner, and the minimap.
 */
public final class GameMap implements Directions {
    /* renamed from: a */
    /** Minimap floor/wall colors, two per tileset id. */
    private static final int[] minimapColors = {16768831, 4136767, 16768831, 4136767, 16768959, 8339263, 12582719, 2047807, 0, 0, 12582719, 2047807, 4177919, 2047807, 12582719, 2047807, 12582719, 2047807, 12582719, 2047807, 0, 0, 16768959, 8339263, 14680063, 2047871, 14680063, 2047871, 16768959, 8339263};
    /* renamed from: c */
    /** Minimap tile size in pixels (2, or 3 on larger screens). */
    private static byte minimapScale = 2;
    /* renamed from: h */
    /** Music track id per tileset id (-1 = keep current). */
    private static final byte[] musicByTileset = {28, 0, 27, 29, -1, 1, 26, 31, 31, 2, -1, 25, 24, 24, 30, 3, -1, -1, -1, -1};
    /* renamed from: d */
    /** Tileset id of the previously loaded map (skips reloading tiles). */
    private static byte lastTilesetId = -1;

    /* renamed from: a */
    /** Map type/id (drives boss setup, music, and camera). */
    public byte mapType;
    public byte tilesetId;

    /* renamed from: a */
    /** True for the four boss map types (11, 13, 15, 82). */
    public boolean bossMap;

    /* renamed from: b */
    /** Fixed boss-arena camera (map types 13 and 15). */
    public boolean lockedCamera;
    /* renamed from: e */
    /** Countdown for the zone-name banner overlay. */
    private byte zoneBannerTimer;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public int widthTiles;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    public int heightTiles;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    public int widthPx;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    public int heightPx;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    public byte[][] tileGrid;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    public byte[][] collisionGrid;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public Entity[][] occupancy;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public Npc[] npcs;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public MapObject[] objects;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    private EntityList entities;

    /* renamed from: a */
    /** Dropped pickups on the ground (each a packed byte[]: x,y,type,arg,arg,ttl). */
    private Vector pickups;

    /* renamed from: b */
    /** Delayed enemy spawns pending a walkable tile (each an int[]: delay,type,x,y). */
    private Vector spawnQueue;
    /** Countdown driving {@link #spawnQueue} / fade processing. */
    private int spawnTick;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    public Object[] triggers;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    public Object[] eventScripts;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    public Object[] dialogueStrings;

    /* renamed from: d */
    /** Toggles the minimap player-dot blink each frame. */
    private boolean minimapBlink;
    /** Scratch buffer holding the raw {@code .map}/{@code .evt} bytes while parsing. */
    private byte[] mapData;

    /* renamed from: a */
    /** Zone name shown in the banner and minimap header. */
    public char[] zoneName;

    /* renamed from: e */
    /** One-frame camera nudge X (applied then cleared in {@link #paint}). */
    public int cameraShiftX = 0;
    /** One-frame camera nudge Y. */
    public int cameraShiftY = 0;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    public boolean combatEnabled = false;

    public static final boolean isBossMap(int mapType) {
        return mapType == 11 || mapType == 13 || mapType == 15 || mapType == 82;
    }

    public GameMap(byte b) {
        this.mapType = b;
        this.bossMap = isBossMap(b);
        this.lockedCamera = b == 13 || b == 15;
        this.entities = new EntityList();
        this.spawnQueue = new Vector();
        this.spawnTick = 16;
        this.pickups = new Vector();
        if (BaseCanvas.width < 240 || BaseCanvas.height < 240) {
            return;
        }
        minimapScale = (byte) 3;
    }

    public final void load() {
        System.out.println(new StringBuffer().append("map : ").append((int) GameState.storyMapId).toString());
        AssetCache.unloadMapObjects();
        AssetCache.unloadMapNpcImages();
        AssetCache.resetEnemyBossBanks();
        AssetCache.unloadBossSprite();
        this.triggers = null;
        this.eventScripts = null;
        this.dialogueStrings = null;
        clearFaces();
        byte b = 0;
        while (true) {
            byte b2 = b;
            if (b2 > 4) {
                break;
            }
            AudioManager.unloadClip(b2);
            b = (byte) (b2 + 1);
        }
        byte b3 = 24;
        while (true) {
            byte b4 = b3;
            if (b4 > 31) {
                break;
            }
            AudioManager.unloadClip(b4);
            b3 = (byte) (b4 + 1);
        }
        BaseCanvas.yieldTick();
        this.mapData = AssetCache.readResource(new StringBuffer().append("/m/").append(GameState.storyMapId < 10 ? "0" : "").append((int) GameState.storyMapId).append(".map").toString());
        BaseCanvas.yieldTick();
        this.tilesetId = this.mapData[0];
        this.widthTiles = this.mapData[1];
        this.heightTiles = this.mapData[2];
        if (this.tilesetId != 1 && this.tilesetId != 5 && this.tilesetId != 9 && this.tilesetId != 15) {
            this.combatEnabled = true;
        }
        this.occupancy = new Entity[this.heightTiles][this.widthTiles];
        BaseCanvas.yieldTick();
        this.widthPx = this.widthTiles * 16;
        this.heightPx = this.heightTiles * 16;
        BaseCanvas.yieldTick();
        parseTiles(this.mapData, 3);
        this.mapData = null;
        if (lastTilesetId != this.tilesetId) {
            AssetCache.unloadMapTiles();
        }
        System.gc();
        this.mapData = AssetCache.readResource(new StringBuffer().append("/m/").append((int) GameState.classId).append("/").append(GameState.storyMapId < 10 ? "0" : "").append((int) GameState.storyMapId).append(".evt").toString());
        BaseCanvas.yieldTick();
        BaseCanvas.yieldTick();
        parseCollision(this.mapData, 0);
        int i = 0 + (this.widthTiles * this.heightTiles);
        BaseCanvas.yieldTick();
        int iM5a = i + parseObjects(this.mapData, i);
        BaseCanvas.yieldTick();
        int iM6b = iM5a + parseNpcs(this.mapData, iM5a);
        BaseCanvas.yieldTick();
        int iC = iM6b + parseEnemies(this.mapData, iM6b);
        BaseCanvas.yieldTick();
        int iD = iC + parseFaces(this.mapData, iC);
        BaseCanvas.yieldTick();
        applyInitialPatches(this.mapData, iD + parseTriggers(this.mapData, iD));
        this.mapData = null;
        switch (this.mapType) {
            case 11:
                loadRockyBossData();
                spawnRockyBoss();
                break;
            case 13:
                loadNordBossData();
                spawnNordBoss(true);
                break;
            case 15:
                loadGebBossData();
                spawnGebBoss();
                break;
            case 82:
                loadGebCoreData();
                spawnGebCore();
                break;
        }
        if (AssetCache.mapTiles == null) {
            BaseCanvas.yieldTick();
            try {
                AssetCache.mapTiles = new PngMerger(new StringBuffer().append("/m/t/t").append(this.tilesetId < 10 ? "0" : "").append((int) this.tilesetId).toString()).allImages();
                BaseCanvas.yieldTick();
            } catch (IOException unused) {
            }
        }
        lastTilesetId = this.tilesetId;
        if (GameState.classId == 8 && this.mapType == 65) {
            this.zoneName = AssetCache.mapNameText.get(85);
        } else {
            this.zoneName = AssetCache.mapNameText.get(this.mapType);
        }
        if (this.zoneName == null || this.zoneName.length <= 0) {
            this.zoneBannerTimer = (byte) 0;
        } else {
            this.zoneBannerTimer = (byte) 10;
        }
        if (this.mapType == 79 || this.mapType == 80 || this.mapType == 81) {
            AudioManager.loadClip((byte) 4);
            AudioManager.loadClip((byte) 8);
            AudioManager.playBgm(4);
        } else {
            if (this.tilesetId == 1 || this.tilesetId == 5 || this.tilesetId == 9 || this.tilesetId == 15) {
                AudioManager.loadClip((byte) 8);
            }
            if (musicByTileset[this.tilesetId] != -1) {
                AudioManager.loadClip(musicByTileset[this.tilesetId]);
                AudioManager.playBgm((int) musicByTileset[this.tilesetId]);
            }
        }
        GameState.hero().clearFloaters();
    }

    private final void parseTiles(byte[] bArr, int i) {
        this.tileGrid = new byte[this.heightTiles][this.widthTiles];
        BaseCanvas.yieldTick();
        for (int i2 = 0; i2 < this.heightTiles; i2++) {
            System.arraycopy(bArr, i, this.tileGrid[i2], 0, this.widthTiles);
            i += this.widthTiles;
        }
        BaseCanvas.yieldTick();
    }

    private final void parseCollision(byte[] bArr, int i) {
        this.collisionGrid = new byte[this.heightTiles][this.widthTiles];
        BaseCanvas.yieldTick();
        for (int i2 = 0; i2 < this.heightTiles; i2++) {
            System.arraycopy(bArr, i, this.collisionGrid[i2], 0, this.widthTiles);
            i += this.widthTiles;
        }
        BaseCanvas.yieldTick();
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r0v15 */
    /* JADX WARN: Type inference failed for: r0v3, types: [int] */
    /* JADX WARN: Type inference failed for: r0v4, types: [java.lang.Throwable] */
    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    private final int parseObjects(byte[] bArr, int i) {
        Image[] imageArr = null;
        int i2 = i + 1;
        int r0 = bArr[i] & 255;
        if (r0 > 0) {
            try {
                PngMerger brVar = new PngMerger(new StringBuffer().append("/m/t/o").append(this.tilesetId < 10 ? "0" : "").append((int) this.tilesetId).toString());
                brVar.preloadAll = true;
                BaseCanvas.yieldTick();
                AssetCache.mapObjects = new Image[brVar.frameCount()];
                imageArr = AssetCache.mapObjects;
                BaseCanvas.yieldTick();
                for (int i3 = 0; i3 < r0; i3++) {
                    int i4 = i2;
                    i2++;
                    int i5 = bArr[i4] & 255;
                    imageArr[i5] = brVar.image(i5);
                    BaseCanvas.yieldTick();
                }
            } catch (IOException e) {
                e.printStackTrace();
            }
        }
        BaseCanvas.yieldTick();
        int i6 = i2;
        int i7 = i2 + 1;
        int i8 = bArr[i6] & 255;
        this.objects = new MapObject[i8];
        for (int i9 = 0; i9 < i8; i9++) {
            int i10 = i7;
            int i11 = i7 + 1;
            short s = (short) ((bArr[i10] & 255) * 16);
            int i12 = i11 + 1;
            short s2 = (short) ((bArr[i11] & 255) * 16);
            int i13 = i12 + 1;
            byte b = bArr[i12];
            int i14 = i13 + 1;
            byte b2 = bArr[i13];
            i7 = i14 + 1;
            MapObject ajVar = new MapObject(s, s2, b, b2, imageArr[bArr[i14] & 255]);
            this.entities.addBack(ajVar);
            this.entities.reorderByDepth(ajVar);
            this.objects[i9] = ajVar;
        }
        BaseCanvas.yieldTick();
        return 1 + r0 + 1 + (i8 * 5);
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r0v12 */
    /* JADX WARN: Type inference failed for: r0v13, types: [java.lang.Throwable] */
    /* JADX WARN: Type inference failed for: r0v61 */
    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    private final int parseNpcs(byte[] bArr, int i) {
        int i2 = i + 1;
        int i3 = bArr[i] & 255;
        int i4 = 0 + 1;
        for (byte b = 0; b < 5; b = (byte) (b + 1)) {
            AssetCache.unloadNpcSprite(b);
        }
        AssetCache.enemySpriteIds = new byte[5];
        for (byte b3 = 0; b3 < 5; b3 = (byte) (b3 + 1)) {
            AssetCache.enemySpriteIds[b3] = -1;
        }
        try {
            PngMerger brVar = new PngMerger("/npc/all");
            brVar.preloadAll = true;
            BaseCanvas.yieldTick();
            AssetCache.mapNpcImages = new Image[brVar.frameCount()];
            Image[] imageArr = AssetCache.mapNpcImages;
            byte b5 = 0;
            for (byte b6 = 0; b6 < i3; b6 = (byte) (b6 + 1)) {
                int i5 = i2;
                i2++;
                byte b7 = bArr[i5];
                i4++;
                if (b7 >= 18) {
                    imageArr[b7 - 18] = brVar.image(b7 - 18);
                    BaseCanvas.yieldTick();
                } else if (b7 == 3) {
                    AssetCache.enemySpriteIds[b5] = b7;
                    AssetCache.loadEnemySprite((short) 17, b5, true);
                    b5 = (byte) (b5 + 1);
                } else if (b7 == 6) {
                    AssetCache.enemySpriteIds[b5] = b7;
                    AssetCache.loadEnemySprite((short) 20, b5, true);
                    b5 = (byte) (b5 + 1);
                } else {
                    AssetCache.enemySpriteIds[b5] = b7;
                    BaseCanvas.yieldTick();
                    AssetCache.loadNpcSprite(b7, b5);
                    b5 = (byte) (b5 + 1);
                    new StringBuffer().append("Npc Loaded - ").append((int) b7).toString();
                }
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
        BaseCanvas.yieldTick();
        int i6 = i2;
        int i7 = i2 + 1;
        int i8 = bArr[i6] & 255;
        int i9 = i4 + 1;
        this.npcs = new Npc[i8];
        for (int i10 = 0; i10 < i8; i10++) {
            int i11 = i7;
            int i12 = i7 + 1;
            byte b8 = bArr[i11];
            int i13 = i12 + 1;
            byte b9 = bArr[i12];
            i7 = i13 + 1;
            byte b10 = bArr[i13];
            byte b11 = -1;
            if (b10 >= 18) {
                b11 = -1;
            } else {
                byte b12 = 0;
                while (true) {
                    byte b13 = b12;
                    if (b13 >= AssetCache.enemySpriteIds.length) {
                        break;
                    }
                    if (AssetCache.enemySpriteIds[b13] == b10) {
                        b11 = b13;
                        break;
                    }
                    b12 = (byte) (b13 + 1);
                }
                Debug.assertTrue(b11 != -1);
            }
            Npc acVar = new Npc((short) (b8 * 16), (short) (b9 * 16), b10, b11);
            this.entities.addBack(acVar);
            this.entities.reorderByDepth(acVar);
            acVar.setOccupancy();
            i9 += 3;
            this.npcs[i10] = acVar;
        }
        BaseCanvas.yieldTick();
        return i9;
    }

    private final int parseEnemies(byte[] bArr, int i) {
        byte b = 0;
        while (true) {
            byte b2 = b;
            if (b2 >= 5) {
                break;
            }
            AssetCache.unloadEnemySprite(b2);
            b = (byte) (b2 + 1);
        }
        AssetCache.bossSpriteIds = new byte[5];
        byte b3 = 0;
        while (true) {
            byte b4 = b3;
            if (b4 >= 5) {
                break;
            }
            AssetCache.bossSpriteIds[b4] = -1;
            b3 = (byte) (b4 + 1);
        }
        int i2 = i + 1;
        int i3 = bArr[i] & 255;
        Debug.assertTrue(i3 <= 5);
        int i4 = 0 + 1;
        BaseCanvas.yieldTick();
        byte[] bArrA = null;
        if (i3 != 0) {
            EnemyType.alloc(5);
            bArrA = AssetCache.readResource(new StringBuffer().append("/enm/data").append((int) (GameState.clearCount >= 2 ? (byte) 2 : GameState.clearCount)).toString());
            BaseCanvas.yieldTick();
        }
        byte b5 = 0;
        while (true) {
            byte b6 = b5;
            if (b6 >= i3) {
                break;
            }
            int i5 = i2;
            i2++;
            byte b7 = (byte) (bArr[i5] & 255);
            i4++;
            AssetCache.bossSpriteIds[b6] = b7;
            EnemyType.parse(bArrA, b7, b6);
            BaseCanvas.yieldTick();
            AssetCache.loadEnemySprite(b7, b6, false);
            EnemyType.bindSprites(b6);
            BaseCanvas.yieldTick();
            new StringBuffer().append("Enemy Loaded - ").append((int) b7).toString();
            b5 = (byte) (b6 + 1);
        }
        BaseCanvas.yieldTick();
        int i6 = i2;
        int i7 = i2 + 1;
        int i8 = bArr[i6] & 255;
        int i9 = i4 + 1;
        for (int i10 = 0; i10 < i8; i10++) {
            int i11 = i7;
            int i12 = i7 + 1;
            int i13 = bArr[i11] & 255;
            int i14 = i12 + 1;
            int i15 = bArr[i12] & 255;
            i7 = i14 + 1;
            i9 += 3;
            queueEnemySpawn(bArr[i14], 0, i13, i15);
            BaseCanvas.yieldTick();
        }
        return i9;
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r0v1, types: [int] */
    /* JADX WARN: Type inference failed for: r0v12 */
    /* JADX WARN: Type inference failed for: r0v13, types: [javax.microedition.lcdui.Image[]] */
    /* JADX WARN: Type inference failed for: r0v14 */
    /* JADX WARN: Type inference failed for: r0v2, types: [java.lang.Throwable] */
    private final int parseFaces(byte[] bArr, int i) {
        int i2 = i + 1;
        int r0 = bArr[i];
        try {
            PngMerger brVar = new PngMerger("/m/face");
            brVar.preloadAll = true;
            AssetCache.dialoguePortraits = new Image[brVar.frameCount()];
            for (int i3 = 0; i3 < r0; i3++) {
                int i4 = i2;
                i2++;
                int r1 = bArr[i4];
                AssetCache.dialoguePortraits[r1] = brVar.image(r1);
            }
        } catch (IOException e) {
            e.printStackTrace();
        }
        return r0 + 1;
    }

    private final void clearFaces() {
        AssetCache.dialoguePortraits = null;
    }

    private final int parseTriggers(byte[] bArr, int i) {
        int i2 = i + 1;
        int i3 = bArr[i] & 255;
        int i4 = 0 + 1;
        this.triggers = new Object[i3];
        BaseCanvas.yieldTick();
        for (int i5 = 0; i5 < i3; i5++) {
            int i6 = i2;
            i2++;
            int i7 = bArr[i6] & 255;
            i4++;
            if (i7 > 0) {
                byte[][] bArr2 = new byte[i7][7];
                for (int i8 = 0; i8 < i7; i8++) {
                    System.arraycopy(bArr, i2, bArr2[i8], 0, 7);
                    i2 += 7;
                    i4 += 7;
                }
                this.triggers[i5] = bArr2;
            }
        }
        BaseCanvas.yieldTick();
        int i9 = i2;
        int i10 = i2 + 1;
        int i11 = bArr[i9] & 255;
        int i12 = i4 + 1;
        this.eventScripts = new Object[i11];
        for (int i13 = 0; i13 < i11; i13++) {
            int i14 = i10;
            i10++;
            int i15 = bArr[i14] & 255;
            i12++;
            if (i15 > 0) {
                byte[][] bArr3 = new byte[i15][3];
                for (int i16 = 0; i16 < i15; i16++) {
                    System.arraycopy(bArr, i10, bArr3[i16], 0, 3);
                    i10 += 3;
                    i12 += 3;
                }
                this.eventScripts[i13] = bArr3;
            }
        }
        BaseCanvas.yieldTick();
        int i17 = i10;
        int i18 = i10 + 1;
        int i19 = bArr[i17] & 255;
        int i20 = i12 + 1;
        this.dialogueStrings = new Object[i19];
        for (int i21 = 0; i21 < i19; i21++) {
            int i22 = i18;
            int i23 = i18 + 1;
            int i24 = bArr[i22] & 255;
            this.dialogueStrings[i21] = FontManager.getStringChars(new String(bArr, i23, i24));
            i18 = i23 + i24;
            i20 = i20 + 1 + i24;
        }
        BaseCanvas.yieldTick();
        return i20;
    }

    /* JADX INFO: renamed from: c, reason: collision with other method in class */
    private final void applyInitialPatches(byte[] bArr, int i) {
        byte b = -1;
        int i2 = i + 1;
        byte b2 = bArr[i];
        byte b3 = 0;
        while (true) {
            byte b4 = b3;
            if (b4 >= b2) {
                break;
            }
            int i3 = i2;
            int i4 = i2 + 1;
            int i5 = i4 + 1;
            if (GameState.isSwitch(0 | ((bArr[i3] & 3) << 8) | bArr[i4]) && b == -1) {
                b = bArr[i5];
            }
            i2 = i5 + 1;
            b3 = (byte) (b4 + 1);
        }
        int i6 = i2;
        int i7 = i2 + 1;
        byte b5 = bArr[i6];
        byte b6 = 0;
        while (true) {
            byte b7 = b6;
            if (b7 >= b5) {
                return;
            }
            int i8 = i7;
            i7++;
            byte b8 = bArr[i8];
            if (b == b7) {
                byte b9 = 0;
                while (true) {
                    byte b10 = b9;
                    if (b10 < b8) {
                        int i9 = i7;
                        int i10 = i7 + 1;
                        byte b11 = bArr[i9];
                        int i11 = i10 + 1;
                        byte b12 = bArr[i10];
                        int i12 = i11 + 1;
                        byte b13 = bArr[i11];
                        i7 = i12 + 1;
                        applyPatch(b11, b12, b13, bArr[i12]);
                        b9 = (byte) (b10 + 1);
                    }
                }
            } else {
                i7 += b8 * 4;
            }
            b6 = (byte) (b7 + 1);
        }
    }

    private final void applyPatch(byte b, byte b2, byte b3, byte b4) {
        switch (b) {
            case 100:
                this.tileGrid[b3][b2] = b4;
                break;
            case 101:
                this.collisionGrid[b3][b2] = b4;
                break;
            case 102:
                this.objects[b4 & 255].setPixelPos((short) ((b2 & 255) << 4), (short) ((b3 & 255) << 4));
                break;
            case 103:
                removeEntity(this.objects[b2 & 255]);
                this.objects[b2 & 255] = null;
                break;
            case 104:
                this.npcs[b4 & 255].setPixelPos((short) ((b2 & 255) << 4), (short) ((b3 & 255) << 4));
                break;
            case 105:
                Npc acVar = this.npcs[b2 & 255];
                acVar.visible = false;
                acVar.clearOccupancy();
                break;
            case 106:
                Debug.assertTrue(false);
                break;
            case 107:
                Debug.assertTrue(false);
                break;
            case 109:
                this.objects[b2 & 255].image = AssetCache.mapObjects[b3 & 255];
                break;
            case 110:
                this.npcs[b2 & 255].kind = b3;
                break;
            case 111:
                Hero aoVarM100a = GameState.hero();
                aoVarM100a.clearFloaters();
                if (b2 != 0) {
                    aoVarM100a.addFloater(new Floater((byte) 10, (short) -1, (short) (b2 - 1)));
                }
                break;
            case 112:
                Npc acVar2 = this.npcs[b2];
                acVar2.clearFloaters();
                if (b3 != 0) {
                    acVar2.addFloater(new Floater((byte) 10, (short) -1, (short) (b3 - 1)));
                }
                break;
        }
    }

    public final void paint(Graphics graphics) {
        int i = GameLoop.instance.cameraFollow ? GameState.camX : GameState.camTargetX;
        int i2 = GameLoop.instance.cameraFollow ? GameState.camY : GameState.camTargetY;
        int i3 = GameScreen.width;
        int i4 = GameScreen.worldHeight;
        if (this.lockedCamera) {
            i = GameState.camTargetX;
            i2 = GameState.camTargetY + 30;
        }
        if (i > 0) {
            i = 0;
        }
        if (i < i3 - this.widthPx) {
            i = i3 - this.widthPx;
        }
        if (i2 > 0) {
            i2 = 0;
        }
        if (i2 < i4 - this.heightPx) {
            i2 = i4 - this.heightPx;
        }
        if (i > 0) {
            i = (i3 - this.widthPx) / 2;
            graphics.setColor(0);
            graphics.fillRect(0, 0, i3, i4);
        }
        if (i2 > 0) {
            i2 = (i4 - this.heightPx) / 2;
            graphics.setColor(0);
            graphics.fillRect(0, 0, i3, i4);
        }
        graphics.setClip(0, 0, i3, i4);
        if (this.cameraShiftX != 0) {
            i += this.cameraShiftX;
            this.cameraShiftX = 0;
        }
        if (this.cameraShiftY != 0) {
            i2 += this.cameraShiftY;
            this.cameraShiftY = 0;
        }
        drawTiles(graphics, i, i2, i3, i4);
        drawPickups(graphics, i, i2);
        drawEntities(graphics, i, i2);
        if (this.zoneBannerTimer > 0) {
            int i5 = i4 / 4;
            if (this.zoneBannerTimer > 8) {
                graphics.setClip(0, i5 + (4 * (this.zoneBannerTimer - 8)), i3, 8 * ((10 - this.zoneBannerTimer) + 1));
            } else if (this.zoneBannerTimer < 3) {
                graphics.setClip(0, i5 + (4 * (3 - this.zoneBannerTimer)), i3, 8 * this.zoneBannerTimer);
            } else {
                graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
            }
            graphics.setColor(0);
            graphics.fillRect(0, i5, i3, 22);
            graphics.setColor(14663551);
            graphics.drawLine(0, i5, i3, i5);
            graphics.drawLine(0, i5 + 21, i3, i5 + 21);
            graphics.setColor(16777215);
            FontManager.drawCharsCentered(graphics, i3 / 2, (i5 + 12) - 4, this.zoneName, 1);
            this.zoneBannerTimer = (byte) (this.zoneBannerTimer - 1);
        }
        graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
    }

    private final void drawTiles(Graphics graphics, int i, int i2, int i3, int i4) {
        int i5 = (-i) / 16;
        int i6 = (-i2) / 16;
        int i7 = ((i3 - i) - 1) / 16;
        int i8 = ((i4 - i2) - 1) / 16;
        if (i5 < 0) {
            i5 = 0;
        }
        if (i6 < 0) {
            i6 = 0;
        }
        if (i7 >= this.widthTiles) {
            i7 = this.widthTiles - 1;
        }
        if (i8 >= this.heightTiles) {
            i8 = this.heightTiles - 1;
        }
        Image[] imageArr = AssetCache.mapTiles;
        for (int i9 = i6; i9 <= i8; i9++) {
            int i10 = i2 + (i9 * 16);
            int i11 = i + (i5 * 16);
            for (int i12 = i5; i12 <= i7; i12++) {
                Image image = imageArr[this.tileGrid[i9][i12]];
                if (image == null) {
                    if (this.collisionGrid[i9][i12] < 0) {
                        graphics.setColor(0);
                    } else if (this.collisionGrid[i9][i12] >= 0) {
                        graphics.setColor(16777215);
                    }
                    graphics.fillRect(i11, i10, 16, 16);
                } else {
                    graphics.drawImage(image, i11, i10, 20);
                }
                i11 += 16;
            }
        }
    }

    /* renamed from: a */
    /** Draws the blinking pickup markers at camera offset ({@code offsetX},{@code offsetY}). */
    private final void drawPickups(Graphics graphics, int offsetX, int offsetY) {
        for (int idx = 0; idx < this.pickups.size(); idx++) {
            byte[] pickup = (byte[]) this.pickups.elementAt(idx);
            if (pickup[5] > 16 || pickup[5] % 3 != 0) {
                graphics.drawImage(pickup[2] == -1 ? AssetCache.dropGoldMarker : AssetCache.dropItemMarker, offsetX + (pickup[0] << 4) + 8, offsetY + (pickup[1] << 4) + 8, 33);
            }
        }
    }

    private final void drawEntities(Graphics graphics, int i, int i2) {
        Entity ckVar = this.entities.head;
        while (true) {
            Entity ckVar2 = ckVar;
            if (ckVar2 == null) {
                return;
            }
            ckVar2.paint(graphics, i, i2);
            ckVar = ckVar2.next;
        }
    }

    public final void paintMinimap(Graphics graphics) {
        int i = this.widthTiles * minimapScale;
        int i2 = this.heightTiles * minimapScale;
        int i3 = BaseCanvas.halfW - (i / 2);
        int i4 = BaseCanvas.halfH - (i2 / 2);
        graphics.setColor(0);
        graphics.drawRect(i3 - 1, i4 - 1, i + 1, i2 + 1);
        byte b = 0;
        while (true) {
            byte b2 = b;
            if (b2 >= this.heightTiles) {
                break;
            }
            byte b3 = 0;
            while (true) {
                byte b4 = b3;
                if (b4 >= this.widthTiles) {
                    break;
                }
                if (this.collisionGrid[b2][b4] < 0) {
                    graphics.setColor(minimapColors[(this.tilesetId * 2) + 1]);
                } else if (this.collisionGrid[b2][b4] >= 0) {
                    graphics.setColor(minimapColors[this.tilesetId * 2]);
                }
                if (this.collisionGrid[b2][b4] != 0 && this.collisionGrid[b2][b4] != -128) {
                    if (EventScript.hasTrigger(this.collisionGrid[b2][b4] < 0 ? (byte) (-this.collisionGrid[b2][b4]) : this.collisionGrid[b2][b4])) {
                        if (this.tilesetId == 6) {
                            graphics.setColor(16727999);
                        } else {
                            graphics.setColor(4161535);
                        }
                    }
                }
                graphics.fillRect(i3, i4, minimapScale, minimapScale);
                i3 += minimapScale;
                b3 = (byte) (b4 + 1);
            }
            i4 += minimapScale;
            i3 = BaseCanvas.halfW - (i / 2);
            b = (byte) (b2 + 1);
        }
        if (this.minimapBlink) {
            Hero aoVarM100a = GameState.hero();
            graphics.setColor(16727871);
            graphics.fillRect((BaseCanvas.halfW - (i / 2)) + (((Entity) aoVarM100a).tileX * minimapScale), (BaseCanvas.halfH - (i2 / 2)) + (((Entity) aoVarM100a).tileY * minimapScale), minimapScale, minimapScale);
        }
        this.minimapBlink = !this.minimapBlink;
        graphics.setColor(0);
        graphics.fillRect(0, 0, BaseCanvas.width, 20);
        graphics.setColor(16777215);
        FontManager.drawCharsCentered(graphics, BaseCanvas.halfW, 8, this.zoneName, 1);
    }

    public final void updateWorld() {
        processSpawnQueue(true, (byte) 3);
        expirePickups();
        updateCombatants();
    }

    public final void updateNpcs() {
        updateNpcEntities();
    }

    public final void fadeStep() {
        this.spawnTick = 0;
        processSpawnQueue(false, (byte) 3);
    }

    private final void processSpawnQueue(boolean z, byte b) {
        this.spawnTick--;
        if (this.spawnTick < 0) {
            this.spawnTick = 16;
            for (int size = this.spawnQueue.size() - 1; size >= 0; size--) {
                int[] iArr = (int[]) this.spawnQueue.elementAt(size);
                iArr[0] = iArr[0] - 16;
                if (iArr[0] < 0) {
                    byte b2 = (byte) iArr[1];
                    byte b3 = (byte) iArr[2];
                    byte b4 = (byte) iArr[3];
                    byte b5 = -1;
                    byte b6 = 0;
                    while (true) {
                        byte b7 = b6;
                        if (b7 >= AssetCache.bossSpriteIds.length) {
                            break;
                        }
                        if (AssetCache.bossSpriteIds[b7] == b2) {
                            b5 = b7;
                            break;
                        }
                        b6 = (byte) (b7 + 1);
                    }
                    if (EnemyType.types[b5].size == 2 && b3 >= this.widthTiles - 1) {
                        System.out.println("INVALID location for enemy - delayed creation.");
                        this.spawnQueue.removeElementAt(size);
                        queueEnemySpawn(iArr[1], 0, b3 - 1, b4);
                    } else if (spawnEnemyAt(b3, b4, b2, b5, z, b, (byte) 5)) {
                        this.spawnQueue.removeElementAt(size);
                    } else {
                        iArr[0] = 0;
                    }
                }
            }
        }
    }

    public final boolean spawnEnemyAt(byte b, byte b2, byte b3, byte b4, boolean z, byte b5, byte b6) {
        byte b7 = 1;
        if (EnemyType.types[b4].size == 2) {
            b7 = 2;
        }
        if (z) {
            b = (byte) (b + defpackage.ByteUtil.randRange(-b5, b5));
            b2 = (byte) (b2 + defpackage.ByteUtil.randRange(-b5, b5));
        }
        while (!isWalkableSpan((int) b, (int) b2, b7) && b6 > 0) {
            b6 = (byte) (b6 - 1);
            b = (byte) (b + defpackage.ByteUtil.randRange(-b5, b5));
            b2 = (byte) (b2 + defpackage.ByteUtil.randRange(-b5, b5));
        }
        if (b6 <= 0) {
            return false;
        }
        Enemy alVar = new Enemy((short) (b << 4), (short) (b2 << 4), b3, b4);
        this.entities.addBack(alVar);
        this.entities.reorderByDepth(alVar);
        alVar.setState((byte) 1);
        alVar.setFacing((byte) 2);
        return true;
    }

    private final void updateCombatants() {
        Entity ckVar = this.entities.head;
        while (ckVar != null) {
            if (ckVar instanceof MapObject) {
                ckVar = ckVar.next;
            } else if ((ckVar instanceof Enemy) && !ckVar.removed) {
                Enemy alVar = (Enemy) ckVar;
                alVar.update();
                ckVar = ckVar.next;
                this.entities.reorderByDepth(alVar);
                if (alVar.state == 6) {
                    removeEntity(alVar);
                }
            } else if ((ckVar instanceof Effect) && !ckVar.removed) {
                Effect yVar = (Effect) ckVar;
                yVar.onFrame();
                ckVar = ckVar.next;
                this.entities.reorderByDepth(yVar);
                if (yVar.isFinished()) {
                    removeEntity(yVar);
                }
            } else if (ckVar.removed) {
                ckVar.removed = false;
                ckVar = ckVar.next;
            } else {
                ckVar = ckVar.next;
            }
        }
    }

    private final void updateNpcEntities() {
        Entity ckVar = this.entities.head;
        while (ckVar != null) {
            if ((ckVar instanceof Npc) && !ckVar.removed) {
                Npc acVar = (Npc) ckVar;
                acVar.update();
                ckVar = ckVar.next;
                this.entities.reorderByDepth(acVar);
            } else if (ckVar.removed) {
                ckVar.removed = false;
                ckVar = ckVar.next;
            } else {
                ckVar = ckVar.next;
            }
        }
    }

    private final void expirePickups() {
        for (int size = this.pickups.size() - 1; size >= 0; size--) {
            byte[] bArr = (byte[]) this.pickups.elementAt(size);
            bArr[5] = (byte) (bArr[5] - 1);
            if (bArr[5] < 0) {
                this.pickups.removeElementAt(size);
            }
        }
    }

    public final boolean isWalkable(int i, int i2) {
        return i >= 0 && i2 >= 0 && i < this.widthTiles && i2 < this.heightTiles && this.collisionGrid[i2][i] >= 0 && this.occupancy[i2][i] == null;
    }

    public final boolean isWalkableSpan(int i, int i2, byte b) {
        for (int i3 = 0; i3 < b; i3++) {
            if (!isWalkable(i + i3, i2)) {
                return false;
            }
        }
        return true;
    }

    public final boolean canOccupy(Battler oVar, int i, int i2) {
        for (int i3 = 0; i3 < ((Entity) oVar).layer; i3++) {
            if (i + i3 < 0 || i2 < 0 || i + i3 >= this.widthTiles || i2 >= this.heightTiles) {
                return false;
            }
            if (!isWalkable(i + i3, i2) && this.occupancy[i2][i + i3] != oVar) {
                return false;
            }
        }
        return true;
    }

    public final boolean canStep(Battler oVar, byte b) {
        return canOccupy(oVar, ((Entity) oVar).tileX + Directions.dirDx[b], ((Entity) oVar).tileY + Directions.dirDy[b]);
    }

    public final void removeEntity(Entity ckVar) {
        if (ckVar instanceof Battler) {
            ((Battler) ckVar).clearOccupancy();
        }
        this.entities.remove(ckVar);
    }

    public final void addEntity(Entity ckVar) {
        this.entities.addBack(ckVar);
        this.entities.reorderByDepth(ckVar);
    }

    public final void unlinkEntity(Entity ckVar) {
        this.entities.reorderByDepth(ckVar);
    }

    public final void queueEnemySpawn(int i, int i2, int i3, int i4) {
        this.spawnQueue.addElement(new int[]{i2, i, i3, i4});
    }

    public final void dropPickup(byte b, byte b2, byte b3, byte b4) {
        if (b3 == 22) {
            return;
        }
        this.pickups.addElement(new byte[]{b, b2, b3, b4, 1, 120});
    }

    public final void dropPickup(byte b, byte b2, short s) {
        this.pickups.addElement(new byte[]{b, b2, -1, (byte) (s / 100), (byte) (s % 100), 120});
    }

    public final byte[] takePickup(byte b, byte b2) {
        byte[] bArr = null;
        int i = 0;
        while (i < this.pickups.size()) {
            byte[] bArr2 = (byte[]) this.pickups.elementAt(i);
            if (bArr2[0] == b && bArr2[1] == b2) {
                if (bArr2[2] == -1) {
                    bArr = bArr2;
                    break;
                }
                if (bArr2[2] == 22) {
                    if (GameState.hero().quickItems.canAdd(bArr2[2], bArr2[3], (int) bArr2[4])) {
                        bArr = bArr2;
                        break;
                    }
                } else if (GameState.hero().bag.canAdd(bArr2[2], bArr2[3], (int) bArr2[4])) {
                    bArr = bArr2;
                    break;
                }
            }
            i++;
        }
        if (bArr == null) {
            return null;
        }
        this.pickups.removeElementAt(i);
        return bArr;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public final boolean hasPickup(byte b, byte b2) {
        for (int i = 0; i < this.pickups.size(); i++) {
            byte[] bArr = (byte[]) this.pickups.elementAt(i);
            if (bArr[0] == b && bArr[1] == b2) {
                return true;
            }
        }
        return false;
    }

    public static final void loadRockyBossData() {
        EnemyType.alloc(5);
        byte[] bArrA = AssetCache.readResource(new StringBuffer().append("/enm/data").append((int) (GameState.clearCount >= 2 ? (byte) 2 : GameState.clearCount)).toString());
        AssetCache.bossSpriteIds[0] = 32;
        EnemyType.parse(bArrA, (byte) 32, (byte) 0);
        AssetCache.loadBossSprite((byte) 1);
        EnemyType.bindSpritesBoss((byte) 0);
    }

    public final void spawnRockyBoss() {
        RockyBoss ccVar = new RockyBoss((byte) 10, (byte) 9, (byte) 32, (byte) 0);
        addEntity(ccVar);
        ccVar.setState((byte) 1);
        ccVar.setFacing((byte) 2);
    }

    public static final void loadNordBossData() {
        EnemyType.alloc(5);
        byte[] bArrA = AssetCache.readResource(new StringBuffer().append("/enm/data").append((int) (GameState.clearCount >= 2 ? (byte) 2 : GameState.clearCount)).toString());
        AssetCache.bossSpriteIds[0] = 35;
        AssetCache.bossSpriteIds[1] = 36;
        AssetCache.bossSpriteIds[2] = 37;
        AssetCache.bossSpriteIds[3] = 38;
        AssetCache.bossSpriteIds[4] = 4;
        EnemyType.parse(bArrA, (byte) 35, (byte) 0);
        EnemyType.parse(bArrA, (byte) 36, (byte) 1);
        EnemyType.parse(bArrA, (byte) 37, (byte) 2);
        EnemyType.parse(bArrA, (byte) 38, (byte) 3);
        EnemyType.parse(bArrA, (byte) 4, (byte) 4);
        AssetCache.loadBossSprite((byte) 2);
        AssetCache.loadEnemySprite((short) 4, (byte) 4, false);
        EnemyType.bindSpritesBoss((byte) 0);
        EnemyType.bindSpritesBoss((byte) 1);
        EnemyType.bindSpritesBoss((byte) 2);
        EnemyType.bindSpritesBoss((byte) 3);
        EnemyType.bindSprites((byte) 4);
    }

    public final void spawnNordBoss(boolean z) {
        if (z) {
            NordBody1 arVar = new NordBody1((byte) 9, (byte) 5, (byte) 35, (byte) 0);
            addEntity(arVar);
            arVar.setState((byte) 1);
            arVar.setFacing((byte) 2);
            return;
        }
        NordBody2 agVar = new NordBody2((byte) 9, (byte) 5, (byte) 36, (byte) 1);
        addEntity(agVar);
        agVar.setState((byte) 2);
        agVar.setFacing((byte) 2);
        NordTentacle bdVar = new NordTentacle((byte) 6, (byte) 5, (byte) 37, (byte) 2);
        addEntity(bdVar);
        bdVar.setState((byte) 2);
        bdVar.setFacing((byte) 2);
        NordHealer cdVar = new NordHealer((byte) 13, (byte) 5, (byte) 38, (byte) 3);
        addEntity(cdVar);
        cdVar.setState((byte) 2);
        cdVar.setFacing((byte) 2);
        cdVar.setParts(agVar, bdVar);
        agVar.setParts(cdVar, bdVar);
    }

    public static final void loadGebBossData() {
        EnemyType.alloc(5);
        byte[] bArrA = AssetCache.readResource(new StringBuffer().append("/enm/data").append((int) (GameState.clearCount >= 2 ? (byte) 2 : GameState.clearCount)).toString());
        AssetCache.bossSpriteIds[0] = 39;
        AssetCache.bossSpriteIds[1] = 40;
        AssetCache.bossSpriteIds[2] = 41;
        EnemyType.parse(bArrA, (byte) 39, (byte) 0);
        EnemyType.parse(bArrA, (byte) 40, (byte) 1);
        EnemyType.parse(bArrA, (byte) 41, (byte) 2);
        AssetCache.loadBossSprite((byte) 3);
        EnemyType.bindSpritesBoss((byte) 0);
        EnemyType.bindSpritesBoss((byte) 1);
        EnemyType.bindSpritesBoss((byte) 2);
    }

    public final void spawnGebBoss() {
        GebHandLeft baVar = new GebHandLeft(this, (byte) 0, (byte) 7, (byte) 40, (byte) 1);
        addEntity(baVar);
        baVar.setState((byte) 1);
        baVar.setFacing((byte) 1);
        GebHandRight akVar = new GebHandRight(this, (byte) 13, (byte) 7, (byte) 41, (byte) 2);
        addEntity(akVar);
        akVar.setState((byte) 1);
        akVar.setFacing((byte) 1);
        GebHead cgVar = new GebHead((byte) 7, (byte) 7, (byte) 39, (byte) 0, baVar, akVar);
        addEntity(cgVar);
        cgVar.setState((byte) 2);
        cgVar.setFacing((byte) 1);
    }

    public static final void loadGebCoreData() {
        byte[] bArrA = AssetCache.readResource(new StringBuffer().append("/enm/data").append((int) (GameState.clearCount >= 2 ? (byte) 2 : GameState.clearCount)).toString());
        AssetCache.bossSpriteIds[1] = 42;
        EnemyType.parse(bArrA, (byte) 42, (byte) 1);
        AssetCache.loadBossSprite((byte) 4);
        EnemyType.bindSpritesBoss((byte) 1);
    }

    public final void spawnGebCore() {
        GebCore bvVar = new GebCore(this, (byte) 7, (byte) 10, (byte) 42, (byte) 1);
        addEntity(bvVar);
        bvVar.setState((byte) 1);
        bvVar.setFacing((byte) 2);
    }

    static {
        String[] strArr = {"SET_TILE", "SET_COLI", "OBJ_XY  ", "OBJ_DEL ", "NPC_XY  ", "NPC_DEL ", "ENM_XY  ", "ENM_DEL ", "END     ", "OBJ_NUM ", "NPC_NUM ", "EMO_HERO", "EMO_NPC "};
    }
}
