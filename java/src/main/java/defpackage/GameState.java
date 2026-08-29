package defpackage;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;

/* renamed from: n */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:n.class */
/**
 * Global session state and the deferred state-request queue that mediates
 * transitions between menus, the world, and save/load. Owns the hero, the
 * active map, the camera target/position, the switch/flag progress bitsets, and
 * the RMS save format. Static-only; never instantiated.
 */
public final class GameState implements Directions {

    /* renamed from: a */
    /** Active map. */
    public static GameMap map;

    /* renamed from: a */
    /** Camera target X (world pixels). */
    public static int camTargetX;
    /** Camera target Y (world pixels). */
    public static int camTargetY;
    /** Current camera X (eases toward {@link #camTargetX}). */
    public static int camX;
    /** Current camera Y (eases toward {@link #camTargetY}). */
    public static int camY;

    /* renamed from: a */
    /** The player character. */
    private static Hero hero;

    /* renamed from: a */
    /** Selected class id (6..8). */
    public static byte classId;

    /* renamed from: c */
    /** Pending state-request argument 0. */
    public static byte arg0;

    /* renamed from: d */
    /** Pending state-request argument 1. */
    public static byte arg1;

    /* renamed from: e */
    /** Pending state-request argument 2. */
    public static byte arg2;

    /** Story map id for the current class start. */
    public static byte storyMapId;
    /** Number of times the story has been cleared (New Game+ counter). */
    public static byte clearCount;
    /** Per-clear scale used by {@link #progressBonus}. */
    private static byte[] clearBonusTable = {60, 30, 10};
    /** SaveCipher key for the RMS blobs. */
    private static byte[] saveKey = {5, 11, 8, 81, 3, 20};
    /** Per-class {storyMapId, arg0, arg1} start triples for classes 6..8. */
    private static final byte[] classStartTable = {0, 22, 4, 60, 5, 36, 77, 10, 18};
    /** RMS record names for the three save slots (classes 6,7,8). */
    public static final String[] saveSlots = {"/k", "/s", "/w"};
    /** Requested game screen id. */
    public static int screen = 0;

    /* renamed from: b */
    /** Pending state-request opcode (see {@link #processStateRequest}). */
    public static byte nextState = 0;

    /* renamed from: h */
    /** Deferred hero state applied once the hero is grid-aligned. */
    private static byte pendingHeroState = 0;

    /* renamed from: i */
    /** Deferred hero facing applied with {@link #pendingHeroState}. */
    private static byte pendingHeroFacing = 0;

    /** Story switch bitset (128 bits). */
    private static byte[] switches = new byte[128];
    /** Story flag bitset (128 bits). */
    private static byte[] flags = new byte[128];

    /* renamed from: a */
    /** Per-class initial flag layout applied on new game. */
    public static final boolean[][] classStartFlags = {new boolean[]{true, true, true, true, true, true, false, false, false, false, false, false, false, false, false}, new boolean[]{true, false, true, false, false, false, false, true, true, true, true, false, false, true, true}, new boolean[]{true, true, true, true, false, false, false, false, false, false, false, false, false, false, false}};

    private GameState() {
    }

    /** Starts a fresh map for the current class (loads quick items, resets progress). */
    public static final void startNewMap() {
        clearCount = (byte) 0;
        hero.initClass(classId);
        try {
            byte[] header = new byte[2];
            RmsFile rms = new RmsFile("/o", 1);
            rms.read(header, 0, header.length);
            byte[] quickItemBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
            rms.read(quickItemBytes, 0, quickItemBytes.length);
            hero.quickItems.deserialize(SaveCipher.decrypt(quickItemBytes, saveKey));
            rms.close();
        } catch (Exception unused) {
        }
        clearSwitches();
        clearFlags();
        setSwitch(0);
        storyMapId = classStartTable[(classId - 6) * 3];
        arg0 = classStartTable[((classId - 6) * 3) + 1];
        arg1 = classStartTable[((classId - 6) * 3) + 2];
    }

    /** Continues a saved game, falling back to a new map if load fails. */
    public static final void continueGame() {
        clearSwitches();
        clearFlags();
        Throwable th = null;
        setSwitch(0);
        try {
            loadGame();
        } catch (Exception e2) {
            th.printStackTrace();
            startNewMap();
        }
    }

    /** Finalizes a newly created character (applies class bonus, saves options). */
    public static final void startNewCharacter() {
        Hero player = hero;
        player.classId = (byte) (player.classId + progressBonus(classId));
        if (hero.classId > 100) {
            hero.classId = (byte) 100;
        }
        clearCount = (byte) 1;
        GameLoop.instance.hasCreatedCharacter = true;
        try {
            GameLoop.instance.saveOptions();
        } catch (Exception unused) {
        }
        clearSwitches();
        clearFlags();
        setSwitch(0);
        hero.bag.removeQuestItems();
        hero.hp = hero.maxHp;
        hero.mp = hero.maxMp;
        storyMapId = classStartTable[(classId - 6) * 3];
        arg0 = classStartTable[((classId - 6) * 3) + 1];
        arg1 = classStartTable[((classId - 6) * 3) + 2];
    }

    /** Queues a state request with three arguments. */
    public static final synchronized void requestState(byte state, byte a0, byte a1, byte a2) {
        arg0 = a0;
        arg1 = a1;
        arg2 = a2;
        nextState = state;
    }

    /** Queues a state request with two arguments. */
    public static final synchronized void requestState(byte state, byte a0, byte a1) {
        arg0 = a0;
        arg1 = a1;
        arg2 = (byte) 0;
        nextState = state;
    }

    /** Queues a state request with one argument. */
    public static final synchronized void requestState(byte state, byte a0) {
        arg0 = a0;
        arg1 = (byte) 0;
        arg2 = (byte) 0;
        nextState = state;
    }

    /** Queues a state request with no arguments. */
    public static final synchronized void requestState(byte state) {
        arg0 = (byte) 0;
        arg1 = (byte) 0;
        arg2 = (byte) 0;
        nextState = state;
    }

    /** Clears any queued state request. */
    public static final void clearRequest() {
        nextState = (byte) 0;
        arg0 = (byte) 0;
        arg1 = (byte) 0;
        arg2 = (byte) 0;
    }

    /** Dispatches the queued state request to the appropriate transition. */
    public static final void processStateRequest() {
        if (nextState == 0) {
        }
        byte state = nextState;
        nextState = (byte) 0;
        switch (state) {
            case 1:
                setScreen(1);
                GameLoop.instance.setLoadingFps();
                AssetLoader.loadMap();
                break;
            case 2:
                setScreen((int) arg0);
                if (arg1 == 0) {
                    GameLoop.instance.setFps((int) arg2);
                } else if (arg1 == 1) {
                    GameLoop.instance.applyDifficultyFps();
                } else if (arg1 == 2) {
                    GameLoop.instance.setLoadingFps();
                } else if (arg1 == 3) {
                    GameLoop.instance.setFastFps();
                }
                break;
            case 11:
                switch (arg0) {
                    case 0:
                        setScreen(6);
                        ShopMenu.instance().loadStrings();
                        break;
                    case 1:
                        setScreen(7);
                        RefineMenu.instance().open();
                        break;
                    case 2:
                        setScreen(8);
                        BlacksmithMenu.instance().open();
                        break;
                }
                break;
            case 12:
                setScreen(2);
                switch (arg0) {
                    case 1:
                        RefineMenu.instance().closeRefine();
                        break;
                    case 2:
                        BlacksmithMenu.instance().closeBlacksmith();
                        break;
                }
                break;
            case 13:
                setScreen(5);
                CharacterMenu.instance().open();
                if ((Debug.fullVersion && arg0 == 1) || (AppConfig.fullVersion && hero.level >= 8)) {
                    CharacterMenu.instance().openSystemQuit();
                    break;
                }
                break;
            case 14:
                if (arg0 != 1) {
                    CharacterMenu.instance().closeMenu(true);
                } else {
                    CharacterMenu.instance().closeMenu(false);
                    setScreen(1);
                    AssetLoader.loadMainMenu();
                }
                break;
            case 15:
                warpMap();
                break;
            case 16:
                setScreen(10);
                AudioManager.loadClip((byte) 12);
                AudioManager.playSfx((byte) 12, false);
                GameScreen.fxTimer = 16;
                break;
            case 21:
                if (arg0 == 1) {
                    continueGame();
                } else if (arg0 == 0) {
                    startNewMap();
                } else if (arg0 == 2) {
                    startNewCharacter();
                    CharacterMenu.instance().closeMenu(false);
                    setScreen(1);
                    AssetLoader.loadMainMenu();
                    AudioManager.stopBgm();
                }
                setScreen(1);
                GameLoop.instance.setLoadingFps();
                AssetLoader.loadResources();
                break;
        }
    }

    /** Requests a map warp to {@code mapId} with sub-arguments {@code a0}/{@code a1}. */
    public static final void requestMapWarp(byte mapId, byte a0, byte a1, byte a2) {
        System.gc();
        requestState((byte) 1, a0, a1, a2);
        AudioManager.stopBgm1();
        AudioManager.stopBgm();
        storyMapId = mapId;
    }

    /** Places the hero on the new map, centers the camera, and starts play. */
    public static final void warpMap() {
        map.addEntity(hero);
        hero.addGuardianToMap();
        hero.init();
        hero.setFacing((byte) (arg0 + 1));
        centerCamera();
        camX = camTargetX;
        camY = camTargetY;
        clearRequest();
        storyMapId = (byte) -1;
        hero.setState((byte) 1);
        hero.resetCombo();
        GameLoop.gameScreen.markRedraw();
        requestState((byte) 2, (byte) 2, (byte) 1);
    }

    /** Recomputes the camera target so the hero is centered on screen. */
    public static final void centerCamera() {
        camTargetX = GameScreen.centerX - ((Entity) hero).pixelX;
        camTargetY = GameScreen.centerY - ((Entity) hero).pixelY;
    }

    /** Sets the requested game screen id. */
    public static final void setScreen(int screenId) {
        screen = screenId;
    }

    /** Sets the active map. */
    public static final void setMap(GameMap newMap) {
        map = newMap;
    }

    /**
     * Scrolls the camera toward its target. When {@code followHero} is false the
     * camera simply eases on both axes; when true it follows the moving hero,
     * optionally leading by the facing direction ({@code lead}) and easing only
     * the axis the facing does not lock.
     */
    public static final void scrollCamera(boolean lead, boolean followHero) {
        if (!followHero) {
            camX += (((camTargetX - camX) + 1) / 2) - 1;
            camY += (((camTargetY - camY) + 1) / 2) - 1;
            return;
        }
        byte facing = heroFacing();
        if (lead) {
            camTargetY -= 15 * Directions.dirDy[facing];
            camTargetX -= 15 * Directions.dirDx[facing];
        }
        if (!Directions.facingIsHorizontal[facing] && camY != camTargetY) {
            camY += (((camTargetY - camY) + 1) / 2) - 1;
        }
        if (!Directions.facingIsHorizontal[facing] || camX == camTargetX) {
            return;
        }
        camX += (((camTargetX - camX) + 1) / 2) - 1;
    }

    /** Starts the hero walking in {@code direction}, or queues it if already moving. */
    public static final void walkHero(byte direction) {
        if (heroState() == 1) {
            pendingHeroState = (byte) 0;
            pendingHeroFacing = (byte) 0;
            setHeroState((byte) 2);
            setHeroFacing(direction);
            return;
        }
        if (heroState() == 2) {
            pendingHeroState = (byte) 2;
            pendingHeroFacing = direction;
        }
    }

    /** Queues the hero to stop (return to idle) keeping the current facing. */
    public static final void stopHero() {
        pendingHeroState = (byte) 1;
        pendingHeroFacing = heroFacing();
    }

    /** Requests a hero attack; queues the attack state when grid-aligned. */
    public static final void requestHeroAttack(boolean strong) {
        if (hero.queueComboStep(strong)) {
            if (heroState() == 2) {
                pendingHeroState = (byte) 3;
                pendingHeroFacing = heroFacing();
            } else if (heroState() == 1) {
                hero.setState((byte) 3);
                hero.turnTowardEnemy();
            }
        }
    }

    /** One simulation step: apply pending hero action, refresh, update the world. */
    public static final void update() {
        applyPendingHeroAction();
        updateHero();
        map.updateWorld();
    }

    /** Advances the active event script one step. */
    public static final void stepEvents() {
        EventScript.step();
    }

    private static final void applyPendingHeroAction() {
        if (isHeroOnGrid() && pendingHeroState != 0) {
            setHeroState(pendingHeroState);
            setHeroFacing(pendingHeroFacing);
            pendingHeroState = (byte) 0;
            pendingHeroFacing = (byte) 0;
        }
    }

    /** Discards the queued hero state. */
    public static final void clearPendingHeroAction() {
        pendingHeroState = (byte) 0;
    }

    /* renamed from: a */
    /** Picks up whatever is on the hero's tile (money/items); returns true if taken. */
    public static final boolean tryPickup() {
        byte[] pickup = map.takePickup(((Entity) hero).tileX, ((Entity) hero).tileY);
        if (pickup == null) {
            if (!map.hasPickup(((Entity) hero).tileX, ((Entity) hero).tileY)) {
                return false;
            }
            GameLoop.gameScreen.showMessage(FontManager.noSpaceLabel, 16);
            return false;
        }
        if (pickup[2] == -1) {
            int amount = (pickup[3] * 100) + pickup[4];
            int scaled = amount;
            if (amount > 0) {
                scaled *= 9;
            }
            hero.addMoney(scaled);
            GameLoop.gameScreen.showMessage(new StringBuffer().append(FontManager.goldGainPrefix).append(scaled).append(FontManager.goldAbbrev).toString().toCharArray(), 16);
            return true;
        }
        if (pickup[2] == 22) {
            hero.quickItems.add(Item.create(pickup[2], pickup[3], true, true), (int) pickup[4]);
            GameLoop.gameScreen.showMessage(defpackage.ByteUtil.concat(FontManager.goldGainPrefix.toCharArray(), Item.typeNames.get(pickup[2])), 16);
            return true;
        }
        Item item = Item.create(pickup[2], pickup[3], true, true);
        if ((item instanceof Equipment) && !((Equipment) item).needsIdentify) {
            ((Equipment) item).identified = true;
        }
        hero.bag.add(item, (int) pickup[4]);
        GameLoop.gameScreen.showMessage(defpackage.ByteUtil.concat(FontManager.goldGainPrefix.toCharArray(), Item.typeNames.get(pickup[2])), 16);
        return true;
    }

    /** Clears all story switches. */
    public static final void clearSwitches() {
        for (int bit = 0; bit < 128; bit++) {
            switches[bit] = 0;
        }
    }

    /* renamed from: a */
    /** Returns whether switch {@code bit} is set. */
    public static final boolean isSwitch(int bit) {
        return ((switches[bit / 8] >> (bit % 8)) & 1) == 1;
    }

    /** Sets switch {@code bit}. */
    public static final void setSwitch(int bit) {
        switches[bit / 8] = (byte) (switches[bit / 8] | (1 << (bit % 8)));
    }

    /** Clears switch {@code bit}. */
    public static final void clearSwitch(int bit) {
        switches[bit / 8] = (byte) (switches[bit / 8] & ((1 << (bit % 8)) ^ (-1)));
    }

    /** Toggles switch {@code bit}. */
    public static final void toggleSwitch(int bit) {
        if (isSwitch(bit)) {
            clearSwitch(bit);
        } else {
            setSwitch(bit);
        }
    }

    /** Clears all story flags. */
    public static final void clearFlags() {
        for (int bit = 0; bit < 128; bit++) {
            flags[bit] = 0;
        }
    }

    /* renamed from: b */
    /** Returns whether flag {@code bit} is set. */
    public static final boolean isFlag(int bit) {
        return ((flags[bit / 8] >> (bit % 8)) & 1) == 1;
    }

    /** Sets flag {@code bit} (special-cases the combo-depth unlock). */
    public static final void setFlag(int bit) {
        flags[bit / 8] = (byte) (flags[bit / 8] | (1 << (bit % 8)));
        if (bit == 29 && classId == 6) {
            hero().setComboDepth((byte) 2);
        }
    }

    /** Clears flag {@code bit}. */
    public static final void clearFlag(int bit) {
        flags[bit / 8] = (byte) (flags[bit / 8] & ((1 << (bit % 8)) ^ (-1)));
    }

    /** Toggles flag {@code bit}. */
    public static final void toggleFlag(int bit) {
        if (isFlag(bit)) {
            clearFlag(bit);
        } else {
            setFlag(bit);
        }
    }

    /* renamed from: a */
    /** Returns the player character. */
    public static final Hero hero() {
        return hero;
    }

    /* renamed from: a */
    /** Returns the hero's current Battler state. */
    public static final byte heroState() {
        return ((Battler) hero).state;
    }

    /* renamed from: b */
    /** Returns the hero's current facing. */
    public static final byte heroFacing() {
        return ((Battler) hero).facing;
    }

    /** Sets the hero's Battler state. */
    public static final void setHeroState(byte state) {
        hero.setState(state);
    }

    /** Sets the hero's facing. */
    public static final void setHeroFacing(byte facing) {
        hero.setFacing(facing);
    }

    /** Refreshes the hero and its active guardian in the world. */
    public static final void updateHero() {
        hero.update();
        map.unlinkEntity(hero);
        Guardian guardian = hero.getActiveGuardian();
        if (guardian != null) {
            guardian.update();
            map.unlinkEntity(guardian);
        }
    }

    /** Teleports the hero to tile ({@code tileX},{@code tileY}). */
    public static final void setHeroTile(int tileX, int tileY) {
        hero.setPixelPos((short) (tileX * 16), (short) (tileY * 16));
        hero.setOccupancy();
    }

    /* renamed from: b */
    /** Returns true when the hero is aligned to the tile grid. */
    public static final boolean isHeroOnGrid() {
        return (((Entity) hero).offGridX || ((Entity) hero).offGridY) ? false : true;
    }

    /* renamed from: a */
    /** Serializes switches, flags, and clear count into a byte array. */
    private static final byte[] packProgress() {
        ByteArrayOutputStream byteStream = null;
        byte[] result = null;
        DataOutputStream dataOut = null;
        try {
            try {
                byteStream = new ByteArrayOutputStream();
                DataOutputStream dataOut2 = new DataOutputStream(byteStream);
                dataOut = dataOut2;
                dataOut2.write(switches);
                dataOut.write(flags);
                dataOut.writeByte(clearCount);
                result = byteStream.toByteArray();
                try {
                    dataOut.close();
                    byteStream.close();
                } catch (IOException unused) {
                }
                return result;
            } catch (IOException e2) {
                e2.printStackTrace();
                if (dataOut != null) {
                    try {
                        dataOut.close();
                    } catch (IOException unused2) {
                        return null;
                    }
                }
                if (byteStream != null) {
                    try {
                        byteStream.close();
                    } catch (IOException unusedC) {
                    }
                }
                return null;
            }
        } catch (Throwable th) {
            if (dataOut != null) {
                try {
                    dataOut.close();
                } catch (IOException unused3) {
                    throw th;
                }
            }
            if (byteStream != null) {
                try {
                    byteStream.close();
                } catch (IOException unusedC) {
                }
            }
            throw th;
        }
    }

    /** Restores switches, flags, and clear count from a {@link #packProgress()} blob. */
    private static final void unpackProgress(byte[] data) {
        ByteArrayInputStream byteStream = null;
        DataInputStream dataIn = null;
        try {
            try {
                byteStream = new ByteArrayInputStream(data);
                DataInputStream dataIn2 = new DataInputStream(byteStream);
                dataIn = dataIn2;
                dataIn2.read(switches);
                dataIn.read(flags);
                clearCount = dataIn.readByte();
                try {
                    dataIn.close();
                    byteStream.close();
                } catch (IOException unused) {
                }
            } catch (IOException e2) {
                e2.printStackTrace();
                if (dataIn != null) {
                    try {
                        dataIn.close();
                    } catch (IOException unused2) {
                        return;
                    }
                }
                if (byteStream != null) {
                    try {
                        byteStream.close();
                    } catch (IOException unusedC) {
                    }
                }
            }
        } catch (Throwable th) {
            if (dataIn != null) {
                try {
                    dataIn.close();
                } catch (IOException unused3) {
                    throw th;
                }
            }
            if (byteStream != null) {
                try {
                    byteStream.close();
                } catch (IOException unusedC) {
                }
            }
            throw th;
        }
    }

    /** Writes the encrypted hero/bag/progress/position save blob to RMS. */
    public static final void saveGame() throws Exception {
        Hero player = hero;
        byte[] heroBytes = player.save();
        byte[] bagBytes = player.bag.serialize();
        byte[] progressBytes = packProgress();
        byte[] posBytes = {map.mapType, ((Entity) player).tileX, ((Entity) player).tileY};
        byte[] encHero = SaveCipher.encrypt(heroBytes, saveKey);
        byte[] encBag = SaveCipher.encrypt(bagBytes, saveKey);
        byte[] encProgress = SaveCipher.encrypt(progressBytes, saveKey);
        byte[] encPos = SaveCipher.encrypt(posBytes, saveKey);
        byte[] blob = new byte[encHero.length + encBag.length + encProgress.length + encPos.length + 8];
        blob[0] = (byte) ((encHero.length & 65280) >> 8);
        blob[1] = (byte) (encHero.length & 255);
        System.arraycopy(encHero, 0, blob, 2, encHero.length);
        int off1 = 2 + encHero.length;
        int off1b = off1 + 1;
        blob[off1] = (byte) ((encBag.length & 65280) >> 8);
        int off2 = off1b + 1;
        blob[off1b] = (byte) (encBag.length & 255);
        System.arraycopy(encBag, 0, blob, off2, encBag.length);
        int off3 = off2 + encBag.length;
        int off3b = off3 + 1;
        blob[off3] = (byte) ((encProgress.length & 65280) >> 8);
        int off4 = off3b + 1;
        blob[off3b] = (byte) (encProgress.length & 255);
        System.arraycopy(encProgress, 0, blob, off4, encProgress.length);
        int off5 = off4 + encProgress.length;
        int off5b = off5 + 1;
        blob[off5] = (byte) ((encPos.length & 65280) >> 8);
        blob[off5b] = (byte) (encPos.length & 255);
        System.arraycopy(encPos, 0, blob, off5b + 1, encPos.length);
        RmsFile rms = new RmsFile(saveSlots[classId - 6], 0);
        rms.write(blob, 0, blob.length);
        rms.close();
        byte[] encQuick = SaveCipher.encrypt(player.quickItems.serialize(), saveKey);
        RmsFile quickRms = new RmsFile("/o", 0);
        byte[] quickHeader = {(byte) ((encQuick.length & 65280) >> 8), (byte) (encQuick.length & 255)};
        quickRms.write(quickHeader, 0, quickHeader.length);
        quickRms.write(encQuick, 0, encQuick.length);
        quickRms.close();
    }

    /** Reads and decrypts the save blob, restoring hero/bag/progress/position. */
    private static final void loadGame() throws Exception {
        byte[] header = new byte[2];
        RmsFile rms = new RmsFile(saveSlots[classId - 6], 1);
        rms.read(header, 0, header.length);
        byte[] heroBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
        rms.read(heroBytes, 0, heroBytes.length);
        hero.load(SaveCipher.decrypt(heroBytes, saveKey));
        rms.read(header, 0, header.length);
        byte[] bagBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
        rms.read(bagBytes, 0, bagBytes.length);
        hero.bag.deserialize(SaveCipher.decrypt(bagBytes, saveKey));
        rms.read(header, 0, header.length);
        byte[] progressBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
        rms.read(progressBytes, 0, progressBytes.length);
        unpackProgress(SaveCipher.decrypt(progressBytes, saveKey));
        rms.read(header, 0, header.length);
        byte[] posBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
        rms.read(posBytes, 0, posBytes.length);
        byte[] pos = SaveCipher.decrypt(posBytes, saveKey);
        storyMapId = pos[0];
        arg0 = pos[1];
        arg1 = pos[2];
        rms.close();
        RmsFile quickRms = new RmsFile("/o", 1);
        quickRms.read(header, 0, header.length);
        byte[] quickBytes = new byte[((header[0] & 255) << 8) | (header[1] & 255)];
        quickRms.read(quickBytes, 0, quickBytes.length);
        hero.quickItems.deserialize(SaveCipher.decrypt(quickBytes, saveKey));
        quickRms.close();
    }

    /* renamed from: a */
    /** Returns the raw save record for slot {@code classId}, or null if absent. */
    private static final byte[] readSaveSlot(byte slotClassId) {
        System.out.println("getSavedDataFor");
        byte[] data = null;
        try {
            if (RmsFile.exists(saveSlots[slotClassId - 6])) {
                RmsFile rms = new RmsFile(saveSlots[slotClassId - 6], 1);
                data = new byte[rms.size()];
                System.out.println(new StringBuffer().append("::::").append(rms.size()).toString());
                rms.read(data, 0, data.length);
                rms.close();
            }
        } catch (Exception unused) {
        }
        return data;
    }

    /* renamed from: a */
    /** Computes the New Game+ stat bonus for class {@code classId} from progress. */
    public static final byte progressBonus(byte forClassId) {
        if (clearCount >= 3) {
            return (byte) 0;
        }
        byte count = 0;
        for (int flagIndex = 0; flagIndex < 20; flagIndex++) {
            if (isFlag(1 + (flagIndex * 3) + 1)) {
                count = (byte) (count + 1);
            }
        }
        for (int switchIndex = 100; switchIndex <= 105; switchIndex++) {
            if (isSwitch(switchIndex)) {
                count = (byte) (count + 1);
            }
        }
        switch (forClassId) {
            case 6:
                return (byte) ((count * clearBonusTable[clearCount]) / 19);
            case 7:
                return (byte) ((count * clearBonusTable[clearCount]) / 21);
            case 8:
                return (byte) ((count * clearBonusTable[clearCount]) / 16);
            default:
                return (byte) 0;
        }
    }

    /** Builds the continue/load menu from the saved slots and requests the menu state. */
    public static final void buildLoadMenu() {
        int slotCount = 0;
        Object[] slotData = new Object[3];
        byte slot = 6;
        while (true) {
            byte slotId = slot;
            if (slotId > 8) {
                break;
            }
            slotData[slotId - 6] = readSaveSlot(slotId);
            if (slotData[slotId - 6] != null) {
                slotCount++;
            }
            slot = (byte) (slotId + 1);
        }
        byte[] menuData = new byte[slotCount * 4];
        int writePos = 0;
        byte scan = 6;
        while (true) {
            byte slotId = scan;
            if (slotId > 8) {
                break;
            }
            if (slotData[slotId - 6] != null) {
                byte[] record = (byte[]) slotData[slotId - 6];
                try {
                    int heroLen = defpackage.ByteUtil.readU16(record, 0);
                    byte[] heroBytes = new byte[heroLen];
                    System.arraycopy(record, 2, heroBytes, 0, heroLen);
                    int afterHero = 2 + heroLen;
                    byte[] decHero = SaveCipher.decrypt(heroBytes, saveKey);
                    int afterBag = afterHero + 2 + defpackage.ByteUtil.readU16(record, afterHero);
                    int progressLen = defpackage.ByteUtil.readU16(record, afterBag);
                    int progressStart = afterBag + 2;
                    byte[] progressBytes = new byte[progressLen];
                    System.arraycopy(record, progressStart, progressBytes, 0, progressLen);
                    unpackProgress(SaveCipher.decrypt(progressBytes, saveKey));
                    int p0 = writePos;
                    int p1 = writePos + 1;
                    menuData[p0] = slotId;
                    int p2 = p1 + 1;
                    menuData[p1] = decHero[1];
                    int p3 = p2 + 1;
                    menuData[p2] = (byte) (decHero[0] + progressBonus(slotId));
                    writePos = p3 + 1;
                    menuData[p3] = clearCount;
                } catch (Exception e2) {
                    e2.printStackTrace();
                }
            }
            scan = (byte) (slotId + 1);
        }
        MainMenu.create(slotCount > 0, menuData);
        ((Menu) MainMenu.instance()).cursorIndex = slotCount > 0 ? (byte) 1 : (byte) 0;
        requestState((byte) 2, (byte) 9, (byte) 3);
    }

    /** Creates a brand-new hero of class {@code classId} and requests class setup. */
    public static final void newGame(boolean resume, byte newClassId, boolean[] traits) {
        MainMenu.dispose();
        AssetCache.unloadMainMenuAssets();
        classId = newClassId;
        hero = new Hero((short) 0, (short) 0, (byte) 8, (byte) 8, newClassId);
        if (!resume) {
            if (traits[0]) {
                hero.setState((byte) 0);
            }
            if (traits[1]) {
                hero.setState((byte) 1);
            }
            if (traits[2]) {
                hero.setState((byte) 2);
            }
        }
        setScreen(0);
        requestState((byte) 21, resume ? (byte) 1 : (byte) 0);
    }
}
