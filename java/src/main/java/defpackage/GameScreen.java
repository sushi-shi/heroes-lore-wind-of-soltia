package defpackage;

import java.io.IOException;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;
import javax.microedition.lcdui.Image;

/* renamed from: as */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:as.class */
/**
 * The in-game {@link BaseCanvas}: routes each frame by {@link GameState#screen}
 * (world, menus, event scenes, game-over, credits, ending), owns the HUD
 * (health/mana/exp bars, target-monster panel, floating messages), the key
 * handling for movement/actions, and the staff-roll / ending sequences.
 */
public final class GameScreen extends BaseCanvas {
    /** Playfield width in pixels. */
    public static int width;
    /** Playfield height in pixels (screen height minus the HUD strip). */
    public static int worldHeight;
    /** World-view center X. */
    public static int centerX;
    /** World-view center Y. */
    public static int centerY;
    /** Number of HUD frame filler tiles. */
    private static int hudSlots;
    /** HP/MP bar width in pixels. */
    private static int barWidth;
    /** Exp bar width in pixels. */
    private static int expWidth;

    /* renamed from: c */
    /** Whether the world layer is drawn (set once activated). */
    private boolean worldVisible;

    /* renamed from: d */
    /** Force a full HUD redraw next frame. */
    private boolean redrawAll;
    /** HP bar needs redraw. */
    private boolean hpDirty;
    /** MP bar needs redraw. */
    private boolean mpDirty;
    /** Exp bar needs redraw. */
    private boolean expDirty;
    /** Level-up icon blink counter. */
    private int lowHpBlink;
    /** Remaining frames the floating message is shown. */
    private int messageTtl;
    /** Set when a new message replaced one still showing (skip a frame). */
    private boolean messageReplaced;
    /** Remaining frames the target-monster panel is shown. */
    private int targetTtl;

    /* renamed from: a */
    /** Monster currently shown in the target panel. */
    private Enemy targetMonster;

    /* renamed from: e */
    /** Cinematic fade/timer shared by game-over, credits, and ending. */
    public static int fxTimer;

    /* renamed from: a */
    /** Text table for the current ending/credits sequence. */
    private TextTable endingText;
    /** First visible credit line index. */
    private int creditTop;
    /** Current (bottom) credit line index. */
    private int creditBottom;

    /* renamed from: a */
    /** Ending/credits face or background images. */
    private Image[] endingImages;

    /* renamed from: a */
    /** Live staff-roll {@link ScrollCaption} instances. */
    private Vector creditCaptions;

    /* renamed from: i */
    /** Set when the player advances the credits. */
    private boolean advanceCredits;

    /* renamed from: a */
    /** Current floating message text. */
    public char[] message = "".toCharArray();

    public GameScreen() {
        System.out.println("MyGameCanvas");
        width = defpackage.BaseCanvas.width;
        worldHeight = defpackage.BaseCanvas.height - 21;
        centerX = (width / 2) - 8;
        centerY = worldHeight / 2;
        hudSlots = (defpackage.BaseCanvas.width - 74) / 6;
        barWidth = defpackage.BaseCanvas.width - 67;
        expWidth = defpackage.BaseCanvas.width - 6;
        this.redrawAll = true;
        this.lowHpBlink = 0;
        this.messageTtl = 0;
        this.targetTtl = 0;
    }

    public final void paint(Graphics graphics) {
        synchronized (GameLoop.lock) {
            defpackage.GameState.processStateRequest();
            switch (defpackage.GameState.screen) {
                case 1:
                    AssetLoader.drawLoadingOverlay(graphics);
                    break;
                case 2:
                    if (GameLoop.instance.cameraFollow) {
                        defpackage.GameState.centerCamera();
                        defpackage.GameState.update();
                    } else {
                        defpackage.GameState.update();
                        defpackage.GameState.centerCamera();
                    }
                    if (defpackage.GameState.screen == 2) {
                        if (!defpackage.GameState.map.lockedCamera && GameLoop.instance.cameraFollow) {
                            defpackage.GameState.scrollCamera(true, true);
                        }
                        defpackage.GameState.map.paint(graphics);
                        drawHud(graphics);
                        if (Debug.fullVersion && !GameLoop.instance.soundEnabled && defpackage.GameState.hero().level >= 8) {
                            defpackage.GameState.requestState((byte) 13, (byte) 1);
                            return;
                        }
                    }
                    break;
                case 4:
                    defpackage.GameState.stepEvents();
                    if (defpackage.GameState.screen == 4) {
                        EventScript.applyCamera();
                        EventScript.paint(graphics);
                    }
                    break;
                case 5:
                    CharacterMenu.instance().invalidateUp();
                    CharacterMenu.instance().invalidateDown();
                    CharacterMenu.instance().draw(graphics);
                    break;
                case 6:
                    drawWorldBehindMenu(graphics, ShopMenu.instance());
                    ShopMenu.instance().draw(graphics);
                    break;
                case 7:
                    drawWorldBehindMenu(graphics, RefineMenu.instance());
                    RefineMenu.instance().draw(graphics);
                    break;
                case 8:
                    drawWorldBehindMenu(graphics, BlacksmithMenu.instance());
                    BlacksmithMenu.instance().draw(graphics);
                    break;
                case 9:
                    MainMenu.instance().invalidateDown();
                    MainMenu.instance().draw(graphics);
                    break;
                case 10:
                    drawGameOver(graphics);
                    if (fxTimer > 0) {
                        fxTimer--;
                    }
                    if (fxTimer == 0) {
                        defpackage.GameState.setScreen(1);
                        AssetLoader.loadMainMenu();
                        AudioManager.unloadClip((byte) 12);
                    }
                    break;
                case 11:
                    drawWorldBehindMenu(graphics, (Menu) null);
                    defpackage.GameState.map.paintMinimap(graphics);
                    break;
                case 12:
                    drawCredits(graphics);
                    break;
                case 13:
                    drawEndingScroll(graphics);
                    break;
                case 14:
                    drawClearMenu(graphics);
                    break;
                case 15:
                    char[] pausedText = FontManager.pausedLabel;
                    FontManager.clearScreen(graphics);
                    graphics.setColor(16777215);
                    FontManager.drawCharsCentered(graphics, defpackage.BaseCanvas.halfW, defpackage.BaseCanvas.halfH - 15, pausedText, 1);
                    FontManager.drawSoftKeys(graphics, FontManager.labelOk, (char[]) null);
                    break;
            }
            graphics.setColor(16777215);
            GameLoop.instance.throttle();
        }
    }

    private final void drawWorldBehindMenu(Graphics graphics, Menu menu) {
        if (this.worldVisible) {
            defpackage.GameState.map.paint(graphics);
            drawHud(graphics);
            if (menu != null) {
                menu.invalidateDown();
            }
        }
    }

    public final void hideNotify() {
        AudioManager.pause();
        synchronized (GameLoop.lock) {
            if (defpackage.GameState.screen == 2) {
                keyReleased(-8);
                defpackage.GameState.requestState((byte) 13);
            } else if (defpackage.GameState.screen == 1) {
                defpackage.GameState.setScreen(15);
            }
        }
    }

    public final void showNotify() {
        AudioManager.resume();
        synchronized (GameLoop.lock) {
            if (defpackage.GameState.screen == 2) {
                defpackage.GameState.requestState((byte) 13);
            }
        }
    }

    public final void keyPressed(int keyCode) {
        synchronized (GameLoop.lock) {
            if (keyCode == -6) {
                keyCode = 53;
            }
            if (keyCode == -7) {
                keyCode = -8;
            }
            if (GameLoop.instance == null || GameLoop.instance.stopped) {
                return;
            }
            ((BaseCanvas) this).keyDown = true;
            int gameAction = getGameAction(keyCode);
            switch (defpackage.GameState.screen) {
                case 2:
                    handlePlayKey(gameAction, keyCode);
                    break;
                case 4:
                    EventScript.handleKey(gameAction, keyCode);
                    break;
                case 5:
                    CharacterMenu.instance().handleKey(gameAction, keyCode);
                    break;
                case 6:
                    ShopMenu.instance().handleKey(gameAction, keyCode);
                    break;
                case 7:
                    RefineMenu.instance().handleKey(gameAction, keyCode);
                    break;
                case 8:
                    BlacksmithMenu.instance().handleKey(gameAction, keyCode);
                    break;
                case 9:
                    MainMenu.instance().handleKey(gameAction, keyCode);
                    break;
                case 11:
                    defpackage.GameState.requestState((byte) 2, (byte) 2, (byte) 1);
                    GameLoop.gameScreen.markRedraw();
                    break;
                case 12:
                    this.advanceCredits = true;
                    break;
                case 14:
                    if (gameAction == 8 || keyCode == 53) {
                        defpackage.GameState.requestState((byte) 21, (byte) 2);
                    }
                    break;
                case 15:
                    if (keyCode == 53) {
                        defpackage.GameState.setScreen(1);
                    }
                    break;
            }
        }
    }

    public final void keyReleased(int keyCode) {
        synchronized (GameLoop.lock) {
            if (keyCode == -6) {
                keyCode = 53;
            }
            if (keyCode == -7) {
                keyCode = -8;
            }
            if (defpackage.GameState.screen != 2) {
                return;
            }
            if (((BaseCanvas) this).keyDown) {
                super.pendingKey = keyCode;
                return;
            }
            switch (defpackage.GameState.heroState()) {
                case 2:
                    defpackage.GameState.stopHero();
                    break;
            }
        }
    }

    private final void handlePlayKey(int gameAction, int keyCode) {
        switch (keyCode) {
            case -8:
                if (((Battler) defpackage.GameState.hero()).state == 1) {
                    defpackage.GameState.requestState((byte) 13);
                }
                break;
            case 35:
                defpackage.GameState.hero().bag.cycleQuickType();
                markRedraw();
                break;
            case 48:
                if (((Battler) defpackage.GameState.hero()).state == 1 && defpackage.GameState.map.tilesetId <= 14) {
                    defpackage.GameState.requestState((byte) 2, (byte) 11, (byte) 3);
                    break;
                }
                break;
            case 49:
                defpackage.GameState.hero().castGuardianSkill(true);
                break;
            case 50:
                defpackage.GameState.walkHero((byte) 1);
                break;
            case 51:
                defpackage.GameState.hero().castGuardianSkill(false);
                break;
            case 52:
                defpackage.GameState.walkHero((byte) 3);
                break;
            case 53:
                if (!defpackage.GameState.tryPickup() && !EventScript.checkActionTrigger()) {
                    defpackage.GameState.requestHeroAttack(false);
                }
                break;
            case 54:
                defpackage.GameState.walkHero((byte) 4);
                break;
            case 55:
                defpackage.GameState.requestHeroAttack(true);
                break;
            case 56:
                defpackage.GameState.walkHero((byte) 2);
                break;
            case 57:
                defpackage.GameState.hero().useQuickItem();
                break;
            default:
                switch (gameAction) {
                    case 1:
                        defpackage.GameState.walkHero((byte) 1);
                        break;
                    case 2:
                        defpackage.GameState.walkHero((byte) 3);
                        break;
                    case 5:
                        defpackage.GameState.walkHero((byte) 4);
                        break;
                    case 6:
                        defpackage.GameState.walkHero((byte) 2);
                        break;
                    case 8:
                        if (defpackage.GameState.tryPickup()) {
                            markRedraw();
                            break;
                        } else if (!EventScript.checkActionTrigger()) {
                            defpackage.GameState.requestHeroAttack(false);
                            break;
                        }
                        break;
                }
                break;
        }
    }

    /** Clips {@code graphics} to a rect, clamped to the playfield height. */
    public static final void clipToWorld(Graphics graphics, int x, int y, int w, int h) {
        if (y + h > worldHeight) {
            h = worldHeight - y;
        }
        graphics.setClip(x, y, w, h);
    }

    /** Activates the world view and forces a redraw. */
    public final void activate() {
        this.worldVisible = true;
        markRedraw();
    }

    /** Requests a full HUD redraw next frame. */
    public final void markRedraw() {
        this.redrawAll = true;
    }

    /** Marks the HP bar dirty. */
    public final void markHpDirty() {
        this.hpDirty = true;
    }

    /** Marks the MP bar dirty. */
    public final void markMpDirty() {
        this.mpDirty = true;
    }

    /** Marks the exp bar dirty. */
    public final void markExpDirty() {
        this.expDirty = true;
    }

    /* renamed from: f */
    /** Clears the transient HUD state (message and target panel). */
    public final void resetHudState() {
        this.messageTtl = 0;
        this.messageReplaced = false;
        this.targetTtl = 0;
        this.targetMonster = null;
    }

    /* JADX WARN: Multi-variable type inference failed */
    /** Loads the class-specific ending text and face images. */
    public final void loadEnding() {
        AssetLoader.unloadInGame();
        AssetLoader.unloadHeroSprites();
        AssetLoader.unloadGuardianSprites();
        AudioManager.stopBgm();
        fxTimer = -16;
        this.creditTop = 0;
        this.creditBottom = -1;
        try {
            this.endingText = new TextTable(new StringBuffer().append("/sgui/ed").append((int) defpackage.GameState.classId).toString());
            AudioManager.loadClip((byte) 23);
            AudioManager.playBgm(23);
            switch (defpackage.GameState.classId) {
                case 6:
                    try {
                        PngMerger pngMerger = new PngMerger("/m/face");
                        pngMerger.preloadAll = true;
                        this.endingImages = new Image[2];
                        this.endingImages[0] = pngMerger.imageGray(0);
                        this.endingImages[1] = pngMerger.imageGray(8);
                        return;
                    } catch (IOException e) {
                        e.printStackTrace();
                        return;
                    }
                case 8:
                    try {
                        PngMerger pngMerger2 = new PngMerger("/m/face");
                        pngMerger2.preloadAll = true;
                        this.endingImages = new Image[1];
                        this.endingImages[0] = pngMerger2.image(17);
                        return;
                    } catch (IOException e2) {
                        e2.printStackTrace();
                        return;
                    }
                default:
                    return;
            }
        } catch (IOException e3) {
            e3.printStackTrace();
        }
    }

    /* JADX WARN: Multi-variable type inference failed */
    /** Loads the staff-roll text and background image. */
    public final void loadStaffRoll() {
        fxTimer = 0;
        this.creditTop = 0;
        this.creditBottom = -1;
        this.creditCaptions = new Vector();
        try {
            this.endingText = new TextTable("/sgui/edsr");
            PngMerger pngMerger = new PngMerger("/img/end");
            pngMerger.preloadAll = true;
            this.endingImages = new Image[1];
            this.endingImages[0] = pngMerger.image(defpackage.GameState.classId - 6);
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /** Shows a floating {@code text} message for {@code ttl} frames. */
    public final void showMessage(char[] text, int ttl) {
        if (this.messageTtl > 0) {
            this.messageReplaced = true;
        }
        this.messageTtl = ttl;
        this.message = text;
    }

    /** Sets the target-monster panel to {@code enemy} ({@code keepCurrent} keeps a set target). */
    public final void setTarget(Enemy enemy, boolean keepCurrent) {
        this.targetTtl = 24;
        if ((!keepCurrent || this.targetMonster == null) && this.targetMonster != enemy) {
            this.targetMonster = enemy;
        }
    }

    /** Draws the full HUD: frame, item slot, guardian skills, bars, target, message. */
    public final void drawHud(Graphics graphics) {
        Hero hero = defpackage.GameState.hero();
        Guardian guardian = hero.getActiveGuardian();
        int hudY = (defpackage.BaseCanvas.height - 31) - 5;
        if (hero.statPoints > 0) {
            this.lowHpBlink++;
            if (this.lowHpBlink < 5) {
                graphics.drawImage(AssetCache.statPointAlert, 5, hudY + 9, 36);
            }
            if (this.lowHpBlink >= 8) {
                this.lowHpBlink = 0;
            }
        }
        if (this.redrawAll) {
            graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        } else {
            graphics.setClip(0, hudY, defpackage.BaseCanvas.width, 15);
        }
        drawHudFrame(graphics, 0, hudY);
        Item activeItem = hero.bag.currentQuickItem();
        graphics.drawImage(AssetCache.itemIcons[hero.bag.currentQuickType()], defpackage.BaseCanvas.width - 10, hudY + 19, 3);
        if (activeItem != null) {
            defpackage.BaseCanvas.drawNumberAt(graphics, activeItem.quantity, defpackage.BaseCanvas.width - 4, hudY + 22, 24);
        } else {
            defpackage.BaseCanvas.drawNumberAt(graphics, 0, defpackage.BaseCanvas.width - 4, hudY + 22, 24);
        }
        if (guardian.skillSlotA != -1) {
            graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
            graphics.setColor(0);
            graphics.drawRect(7, hudY + 15, 14, 14);
            graphics.drawImage(AssetCache.guardianSkillIcons[(guardian.type * 4) + guardian.skillSlotA], 7, hudY + 15, 20);
            if (guardian.skillCharges[guardian.skillSlotA] != 0) {
                graphics.setColor(12525375);
                graphics.drawRect(7, hudY + 15, 14, 14);
            }
            graphics.setClip(8, hudY + 16, 13, (13 * guardian.skillCharges[guardian.skillSlotA]) / defpackage.Guardian.skillCostTable[(guardian.type * 3) + guardian.skillSlotA]);
            graphics.drawImage(AssetCache.skillChargeFill, 7, hudY + 15, 20);
        }
        if (guardian.skillSlotB != -1) {
            graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
            graphics.setColor(0);
            graphics.drawRect(29, hudY + 15, 14, 14);
            graphics.drawImage(AssetCache.guardianSkillIcons[(guardian.type * 4) + guardian.skillSlotB], 29, hudY + 15, 20);
            if (guardian.skillCharges[guardian.skillSlotB] != 0) {
                graphics.setColor(12525375);
                graphics.drawRect(29, hudY + 15, 14, 14);
            }
            graphics.setClip(30, hudY + 16, 13, (13 * guardian.skillCharges[guardian.skillSlotB]) / defpackage.Guardian.skillCostTable[(guardian.type * 3) + guardian.skillSlotB]);
            graphics.drawImage(AssetCache.skillChargeFill, 29, hudY + 15, 20);
        }
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        if (this.redrawAll || this.hpDirty) {
            int hpFill = (hero.hp * barWidth) / hero.maxHp;
            graphics.setClip(47, hudY + 18, barWidth, 7);
            drawHudFrame(graphics, 0, hudY);
            if (hpFill > 0) {
                graphics.setColor(16711680);
                graphics.fillRect(47, hudY + 20, hpFill, 4);
                graphics.setColor(16752447);
                graphics.fillRect(47, hudY + 21, hpFill, 2);
            }
            defpackage.BaseCanvas.drawNumberAt(graphics, hero.hp, (46 + barWidth) - 2, hudY + 18, 8);
            this.hpDirty = false;
            graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        }
        if (this.redrawAll || this.mpDirty) {
            int mpFill = (hero.mp * barWidth) / hero.maxMp;
            graphics.setColor(4194239);
            graphics.fillRect(47, hudY + 27, mpFill, 2);
            graphics.setColor(0);
            graphics.fillRect(47 + mpFill, hudY + 27, barWidth - mpFill, 2);
            this.mpDirty = false;
        }
        if (this.redrawAll || this.expDirty) {
            int expFill = (hero.exp * expWidth) / hero.expToNext;
            graphics.setColor(10461055);
            graphics.fillRect(0, hudY + 31, defpackage.BaseCanvas.width, 5);
            graphics.setColor(4144959);
            graphics.fillRect(2, hudY + 32, defpackage.BaseCanvas.width - 4, 3);
            graphics.setColor(12566399);
            graphics.drawLine(3, hudY + 33, (3 + expFill) - 1, hudY + 33);
            this.expDirty = false;
        }
        if (this.targetTtl <= 0 || this.targetMonster == null || this.targetMonster.state == 6) {
            this.targetMonster = null;
        } else {
            int panelX = defpackage.BaseCanvas.width - 105;
            int panelY = 2;
            if (guardian != null && guardian.castState == 2) {
                panelY = 2 + 20;
            }
            Menu.drawSelectableBox(graphics, panelX, panelY, 105, 27, false);
            graphics.translate(panelX + 2, panelY + 2);
            int nameColor = 16777215;
            if (this.targetMonster.stats.elemColor == 1) {
                nameColor = 16744239;
            } else if (this.targetMonster.stats.elemColor == 2) {
                nameColor = 16776991;
            }
            Menu.drawTextField(graphics, 0, 0, 101, 23, this.targetMonster.stats.name, 0, 1, 6233919, nameColor);
            defpackage.BaseCanvas.drawNumberAt(graphics, this.targetMonster.stats.level, defpackage.BaseCanvas.drawLabelBox(graphics, FontManager.levelAbbrev, 1, 16), 16, 4);
            graphics.translate(-(panelX + 2), -(panelY + 2));
            graphics.setColor(16727855);
            if (this.targetMonster.hp > 0) {
                graphics.fillRect(panelX + 24 + 5, panelY + 22, (((77 * (this.targetMonster.hp - 1)) / this.targetMonster.stats.maxHp) + 1) - 5, 2);
            }
            this.targetTtl--;
        }
        if (guardian != null && guardian.castState == 2) {
            guardian.drawSkillBanner(graphics);
        }
        graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        if (this.messageReplaced) {
            this.messageReplaced = false;
            return;
        }
        if (this.messageTtl > 0) {
            int msgX = defpackage.BaseCanvas.halfW - 50;
            int msgY = defpackage.BaseCanvas.height - 46;
            Menu.drawSelectableBox(graphics, msgX, msgY, 100, 23, false);
            graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
            Menu.drawTextField(graphics, msgX + 2, msgY + 2, 96, 19, this.message, 0, 1, 6233919, 16777215);
            this.messageTtl--;
        }
    }

    /* renamed from: a */
    /** Draws the static HUD frame chrome at ({@code x},{@code y}). */
    private static final void drawHudFrame(Graphics graphics, int x, int y) {
        graphics.drawImage(AssetCache.hudFrame[1], x, y + 12, 20);
        graphics.drawImage(AssetCache.hudFrame[1], x + 22, y + 12, 20);
        graphics.drawImage(AssetCache.hudFrame[2], x + 23, y + 23, 20);
        graphics.drawImage(AssetCache.hudFrame[3], x + 44, y + 12, 20);
        for (int slot = 0; slot < hudSlots; slot++) {
            graphics.drawImage(AssetCache.hudFrame[4], x + 49 + (slot * 6), y + 14, 20);
        }
        graphics.drawImage(AssetCache.hudFrame[0], x, y + 9, 20);
        graphics.drawImage(AssetCache.hudFrame[6], defpackage.BaseCanvas.width - 26, y, 20);
        graphics.drawImage(AssetCache.hudFrame[5], defpackage.BaseCanvas.width - 30, y + 11, 20);
    }

    /** Draws a single animation frame {@code frameIndex} from {@code frames} at ({@code x},{@code y}). */
    public static final void drawFrame(Graphics graphics, byte[] frames, byte frameIndex, int x, int y) {
        if (frames == null || frameIndex >= frames[0]) {
            return;
        }
        int base = 1 + (frameIndex * 4);
        Image[] images = AssetCache.spriteBanks[frames[base + 2]];
        byte imgIdx = frames[base + 3];
        if (imgIdx == -1 || images[imgIdx] == null) {
            return;
        }
        graphics.drawImage(images[imgIdx], x + frames[base], y + frames[base + 1], 20);
    }

    /** Draws all parts of animation group {@code groupIndex} from {@code frames} at ({@code x},{@code y}). */
    public static final void drawFrameGroup(Graphics graphics, byte[] frames, byte groupIndex, int x, int y) {
        if (frames == null || groupIndex >= frames[0]) {
            return;
        }
        int cursor = 1;
        for (int group = 0; group < groupIndex; group++) {
            cursor = cursor + 1 + (frames[cursor] * 4);
        }
        int countPos = cursor;
        int part = cursor + 1;
        byte partCount = frames[countPos];
        for (int p = 0; p < partCount; p++) {
            Image[] images = AssetCache.spriteBanks[frames[part + 2]];
            byte imgIdx = frames[part + 3];
            if (imgIdx != -1 && images[imgIdx] != null) {
                graphics.drawImage(images[imgIdx], x + frames[part], y + frames[part + 1], 20);
            }
            part += 4;
        }
    }

    /** Draws a titled box with a progress bar at ({@code x},{@code y}) sized {@code w}x{@code h}. */
    public static final void drawLoadBox(Graphics graphics, int x, int y, int w, int h) {
        graphics.setColor(0);
        Menu.drawPanelFrame(graphics, x, y, w, h);
        Menu.fillPanelInterior(graphics, x, y, w, h);
        int innerX = x + 4;
        int innerY = y + 6;
        int innerW = w - 8;
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, innerX + 5, innerY, AssetCache.commonText.get(31), 1);
        graphics.setColor(16723759);
        graphics.drawLine(innerX, innerY + 17 + 0, innerX + ((innerW * defpackage.BaseCanvas.loadProgress) / defpackage.BaseCanvas.loadTotal), innerY + 17 + 0);
        graphics.drawLine(innerX, innerY + 17 + 1, innerX + ((innerW * defpackage.BaseCanvas.loadProgress) / defpackage.BaseCanvas.loadTotal), innerY + 17 + 1);
    }

    /** Draws the game-over screen (dead hero + fading caption). */
    public static final void drawGameOver(Graphics graphics) {
        graphics.setColor(0);
        graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        defpackage.GameState.hero().drawSummonPose(graphics, defpackage.BaseCanvas.halfW, defpackage.BaseCanvas.halfH + 20);
        char[] text = AssetCache.commonText.get(32);
        int textWidth = FontManager.stringWidth(text);
        System.out.println(FontManager.charsToString(text));
        graphics.setColor(8355711);
        FontManager.drawWrappedBlock(graphics, (defpackage.BaseCanvas.halfW - (textWidth / 2)) + 1, (defpackage.BaseCanvas.halfH - 20) + 1, 200, 1, text, 0, 0, (17 - fxTimer) * 2);
        graphics.setColor(16777215);
        FontManager.drawWrappedBlock(graphics, defpackage.BaseCanvas.halfW - (textWidth / 2), defpackage.BaseCanvas.halfH - 20, 200, 1, text, 0, 0, (17 - fxTimer) * 2);
    }

    /** Draws the scrolling ending credits (class 6/8 face frames). */
    private final void drawCredits(Graphics graphics) {
        if (fxTimer < 0) {
            int fade = (255 * (-fxTimer)) / 16;
            graphics.setColor(fade, fade, fade);
            graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
            fxTimer++;
            return;
        }
        graphics.setColor(0);
        graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        if (this.advanceCredits || this.creditBottom == -1) {
            this.advanceCredits = false;
            this.creditBottom++;
            this.creditTop = this.creditBottom;
            while (this.creditBottom < this.endingText.count) {
                char[] line = this.endingText.get(this.creditBottom);
                if (line[0] == '_') {
                    fxTimer = Integer.parseInt(new String(line, 1, line.length - 1));
                    break;
                }
                this.creditBottom++;
            }
        }
        if (this.creditBottom >= this.endingText.count) {
            this.endingText = null;
            this.endingImages = null;
            loadStaffRoll();
            defpackage.GameState.requestState((byte) 2, (byte) 13, (byte) 1);
            return;
        }
        int lineY = defpackage.BaseCanvas.halfH - ((((this.creditBottom - this.creditTop) + 1) * 15) / 2);
        switch (defpackage.GameState.classId) {
            case 6:
                if (this.creditTop == 2 || this.creditTop == 6 || this.creditTop == 9 || this.creditTop == 13) {
                    graphics.setColor(12566463);
                    graphics.fillRect(0, 15, defpackage.BaseCanvas.width, 40);
                    graphics.setClip(0, 15, defpackage.BaseCanvas.width, 40);
                    graphics.drawImage(this.endingImages[0], defpackage.BaseCanvas.width / 4, 5, 17);
                    if (this.creditTop == 9) {
                        graphics.drawImage(this.endingImages[1], ((defpackage.BaseCanvas.width / 4) * 3) + (fxTimer >= 27 ? (fxTimer - 27) * 10 : 0), 5, 17);
                    }
                    graphics.setClip(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
                    lineY += 30;
                }
                break;
            case 8:
                graphics.drawImage(this.endingImages[0], defpackage.BaseCanvas.width, defpackage.BaseCanvas.height, 40);
                break;
        }
        graphics.setColor(16777215);
        for (int line = this.creditTop; line < this.creditBottom; line++) {
            char[] lineText = this.endingText.get(line);
            FontManager.drawChars(graphics, defpackage.BaseCanvas.halfW - (FontManager.stringWidth(lineText) / 2), lineY, lineText, 1);
            lineY += 15;
        }
        if (fxTimer > 0) {
            fxTimer--;
        }
    }

    /** Draws the staff-roll: background plus upward-scrolling caption images. */
    private final void drawEndingScroll(Graphics graphics) {
        graphics.setColor(0);
        graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        graphics.drawImage(this.endingImages[0], 0, defpackage.BaseCanvas.height / 2, 6);
        if (fxTimer == 0 && this.creditTop < this.endingText.count) {
            char[] line = this.endingText.get(this.creditTop);
            if (line[0] == '-') {
                fxTimer = 4;
            } else if (line[0] == '=') {
                fxTimer = 10;
            } else {
                Image captionImage = Image.createImage(FontManager.stringWidth(line), FontManager.lineHeight());
                Graphics captionG = captionImage.getGraphics();
                captionG.setColor(0);
                captionG.fillRect(0, 0, captionImage.getWidth(), captionImage.getHeight());
                captionG.setColor(16777215);
                FontManager.drawChars(captionG, 0, 0, line, 1);
                this.creditCaptions.addElement(new ScrollCaption(captionImage, defpackage.BaseCanvas.height));
                fxTimer = 5;
            }
            this.creditTop++;
        }
        if (this.creditTop >= this.endingText.count && this.creditCaptions.size() == 0) {
            this.creditCaptions = null;
            this.endingText = null;
            this.endingImages = null;
            AudioManager.stopBgm();
            AudioManager.unloadClip((byte) 23);
            defpackage.GameState.requestState((byte) 21, (byte) 2);
            return;
        }
        if (fxTimer > 0) {
            fxTimer--;
        }
        for (int idx = this.creditCaptions.size() - 1; idx >= 0; idx--) {
            ScrollCaption caption = (ScrollCaption) this.creditCaptions.elementAt(idx);
            graphics.drawImage(caption.image, defpackage.BaseCanvas.halfW, caption.y, 17);
            caption.y -= 2;
            if (caption.y < -8) {
                this.creditCaptions.removeElementAt(idx);
            }
        }
    }

    /** Draws the four-line "stage cleared" menu box. */
    private final void drawClearMenu(Graphics graphics) {
        char[] line1 = AssetCache.commonText.get(33);
        char[] line2 = AssetCache.commonText.get(34);
        char[] line3 = AssetCache.commonText.get(35);
        char[] line4 = AssetCache.commonText.get(36);
        graphics.setColor(0);
        graphics.fillRect(0, 0, defpackage.BaseCanvas.width, defpackage.BaseCanvas.height);
        int boxX = defpackage.BaseCanvas.halfW - 55;
        int boxY = defpackage.BaseCanvas.halfH - 36;
        Menu.drawPanelFrame(graphics, boxX, boxY, 110, 72);
        Menu.fillPanelInterior(graphics, boxX, boxY, 110, 72);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, boxX + 5, boxY + 5, line1, 1);
        FontManager.drawChars(graphics, boxX + 5, boxY + 21, line2, 1);
        FontManager.drawChars(graphics, boxX + 5, boxY + 37, line3, 1);
        FontManager.drawChars(graphics, boxX + 5, boxY + 53, line4, 1);
    }
}
