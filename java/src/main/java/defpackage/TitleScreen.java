package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Graphics;
import rpg.GameMIDlet;

/* renamed from: bg */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bg.class */
/**
 * The title/intro {@link BaseCanvas}: multi-phase async asset loading, the
 * falling-logo-letter title animation, the sliding logo, and the entry points
 * into story mode. The {@code state} field drives both {@link #paint} and the
 * background {@link #run} worker.
 */
public final class TitleScreen extends BaseCanvas implements Runnable {
    /** Screen state machine (0=loading, 1=title, 2=menu, 10=logo, ...). */
    private byte state;
    /** Sub-phase of the state-0 async loader. */
    private byte loadPhase;

    /* renamed from: a */
    /** Animation tick counter, reused across states. */
    private int animTick;

    /* renamed from: a */
    /** X of the first falling title glyph. */
    private short glyph1X;

    /* renamed from: b */
    /** Y of the first falling title glyph. */
    private short glyph1Y;
    /** Animation frame of the first falling glyph (0..7 ping-pong). */
    private byte glyph1Frame;

    /* renamed from: c */
    /** X of the second falling title glyph. */
    private short glyph2X;
    /** Y of the second falling title glyph. */
    private short glyph2Y;

    /* renamed from: d */
    /** Animation frame of the second falling glyph. */
    private byte glyph2Frame;

    /* renamed from: c */
    /** When set, story entry skips the intro straight to the class menu. */
    private boolean skipStoryIntro = false;

    /* renamed from: a */
    /** Latest instance (used as the worker {@link Runnable}). */
    public static TitleScreen instance;

    public TitleScreen() {
        new Object();
        this.loadPhase = (byte) 0;
    }

    public final void keyPressed(int keyCode) {
        getGameAction(keyCode);
        if (GameLoop.instance == null || GameLoop.instance.stopped) {
            return;
        }
        switch (this.state) {
            case 1:
                AudioManager.stopBgm();
                AssetCache.unloadLogo();
                AssetCache.unloadTitleScreen();
                enterStoryMode(false, (byte) 1);
                break;
            case 2:
                this.state = (byte) 3;
                this.animTick = 0;
                GameLoop.instance.setLoadingFps();
                instance = this;
                new Thread(instance).start();
                break;
            case 6:
                GameMIDlet.instance.destroyApp(true);
                break;
            case 10:
                startTitle();
                break;
        }
    }

    @Override // java.lang.Runnable
    public final void run() {
        switch (this.state) {
            case 0:
                switch (this.loadPhase) {
                    case 1:
                        AssetCache.loadGlobalUi();
                        BaseCanvas.yieldTick();
                        try {
                            Item.typeNames = new TextTable("/itm/itmtp");
                            BaseCanvas.yieldTick();
                            Armor.attributeNames = new TextTable("/itm/itmatt");
                            BaseCanvas.yieldTick();
                            break;
                        } catch (IOException unused) {
                        }
                        AssetCache.loadLogo();
                        AssetCache.loadTitleScreen();
                        try {
                            if (RmsFile.exists("/c")) {
                                GameLoop.instance.loadOptions();
                            } else {
                                if (!RmsFile.exists("/c")) {
                                    if (RmsFile.exists(GameState.saveSlots[0])) {
                                        RmsFile.delete(GameState.saveSlots[0]);
                                    }
                                    if (RmsFile.exists(GameState.saveSlots[1])) {
                                        RmsFile.delete(GameState.saveSlots[1]);
                                    }
                                    if (RmsFile.exists(GameState.saveSlots[2])) {
                                        RmsFile.delete(GameState.saveSlots[2]);
                                    }
                                    if (RmsFile.exists("/o")) {
                                        RmsFile.delete("/o");
                                    }
                                }
                                GameLoop.instance.saveOptions();
                            }
                        } catch (Exception unused2) {
                        }
                        startLogo();
                        BaseCanvas.yieldTick();
                        break;
                    case 2:
                        AssetLoader.loadStringTables();
                        AssetCache.loadMainMenuAssets();
                        GameLoop.instance.showGameScreen();
                        this.state = (byte) -1;
                        this.loadPhase = (byte) 0;
                        break;
                }
                break;
        }
    }

    public final void paint(Graphics graphics) {
        setViewHeight(graphics.getClipHeight());
        switch (this.state) {
            case 1:
                graphics.setColor(16777215);
                graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
                int logoTopY = BaseCanvas.halfH - 68;
                int logoLeftX = BaseCanvas.halfW - 60;
                graphics.drawImage(AssetCache.titleBgFrames[2], logoLeftX + 0, logoTopY + 25, 20);
                graphics.drawImage(AssetCache.titleBgFrames[3], logoLeftX + 52, logoTopY + 25, 20);
                graphics.drawImage(AssetCache.titleBgFrames[4], logoLeftX + 93, logoTopY + 2, 20);
                graphics.setColor(4136767);
                if (FontManager.versionText != null) {
                    FontManager.drawChars(graphics, (BaseCanvas.width - 2) - FontManager.stringWidth(FontManager.versionText), BaseCanvas.height - 31, FontManager.versionText, 0);
                }
                graphics.drawImage(AssetCache.titleMenuFrames[this.glyph1Frame < 4 ? this.glyph1Frame : 8 - this.glyph1Frame], this.glyph1X, this.glyph1Y, 33);
                graphics.drawImage(AssetCache.titleMenuFrames[(this.glyph2Frame < 4 ? this.glyph2Frame : 8 - this.glyph2Frame) + 5], this.glyph2X, this.glyph2Y, 33);
                this.glyph1X = (short) (this.glyph1X + (10 * (this.glyph1Frame < 4 ? 1 : -1)));
                this.glyph1Y = (short) (this.glyph1Y + defpackage.ByteUtil.randRange(-1, 4));
                this.glyph2X = (short) (this.glyph2X + (10 * (this.glyph2Frame < 4 ? -1 : 1)));
                this.glyph2Y = (short) (this.glyph2Y + defpackage.ByteUtil.randRange(-1, 4));
                this.glyph1Frame = (byte) (this.glyph1Frame + 1);
                this.glyph2Frame = (byte) (this.glyph2Frame + 1);
                if (this.glyph1Frame > 7) {
                    this.glyph1Frame = (byte) 0;
                }
                if (this.glyph2Frame > 7) {
                    this.glyph2Frame = (byte) 0;
                }
                if (this.glyph1Y > BaseCanvas.height + 10) {
                    this.glyph1X = (short) defpackage.ByteUtil.randRange(10, (BaseCanvas.width / 2) - 10);
                    this.glyph1Y = (short) ((-10) * defpackage.ByteUtil.randRange(0, 4));
                    this.glyph1Frame = (byte) defpackage.ByteUtil.randRange(0, 7);
                }
                if (this.glyph2Y > BaseCanvas.height + 10) {
                    this.glyph2X = (short) defpackage.ByteUtil.randRange((BaseCanvas.width / 2) + 10, BaseCanvas.width - 10);
                    this.glyph2Y = (short) ((-10) * defpackage.ByteUtil.randRange(3, 7));
                    this.glyph2Frame = (byte) defpackage.ByteUtil.randRange(0, 7);
                }
                if (this.animTick % 4 < 2) {
                    graphics.setColor(0);
                    FontManager.drawCharsCentered(graphics, BaseCanvas.halfW, BaseCanvas.height - 45, FontManager.titleFooter, 1);
                }
                this.animTick++;
                break;
            case 10:
                graphics.setColor(16777215);
                graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
                if (this.glyph1Frame > 40) {
                    this.glyph1X = (short) (this.glyph1X * 2);
                }
                graphics.drawImage(AssetCache.logoFrames[4], BaseCanvas.halfW, this.animTick - this.glyph1X, 3);
                if (this.glyph1Frame != 0) {
                    switch (this.glyph1Frame) {
                        case 1:
                        case 3:
                            this.animTick = BaseCanvas.halfH - 1;
                            break;
                        case 2:
                        case 4:
                            this.animTick = BaseCanvas.halfH;
                            break;
                    }
                    this.glyph1Frame = (byte) (this.glyph1Frame + 1);
                } else if (this.animTick < BaseCanvas.halfH - 1) {
                    this.animTick += (BaseCanvas.halfH - this.animTick) / 2;
                } else {
                    this.glyph1Frame = (byte) 1;
                }
                if (this.glyph1X > BaseCanvas.height) {
                    startTitle();
                }
                break;
        }
        GameLoop.instance.throttle();
    }

    /** First-run initialization: config, language choice, and the async loader. */
    public final void boot() {
        BaseCanvas.beginLoading("- INITIALIZE", 30);
        FontManager.initFonts();
        AppConfig.init(GameMIDlet.instance);
        int langChoice = AppConfig.initAvailableLocales();
        if (langChoice < 0) {
            BaseCanvas.loadPhase = 3;
            this.loadPhase = (byte) 1;
            this.state = (byte) 0;
            loadLanguage(1);
            new Thread(this).start();
            return;
        }
        System.out.println(new StringBuffer().append("langChoice ").append(langChoice).toString());
        BaseCanvas.loadPhase = 3;
        this.loadPhase = (byte) 1;
        this.state = (byte) 0;
        loadLanguage(langChoice);
        new Thread(this).start();
    }

    private void loadLanguage(int langChoice) {
        try {
            StringTable.instance.load("/lang/language", "", langChoice);
            FontManager.loadLabels(StringTable.instance);
            AppConfig.apply();
            AssetCache.commonText = new TextTable("/sgui/com");
            FontManager.loadingTitle = AssetCache.commonText.get(37);
            FontManager.loadingSubtitle = AssetCache.commonText.get(38);
            BaseCanvas.yieldTick();
            this.loadPhase = (byte) 1;
        } catch (IOException unused) {
        }
    }

    public final void startLogo() {
        GameLoop.instance.setFps(20);
        this.animTick = -20;
        this.glyph1Frame = (byte) 0;
        this.glyph1X = (short) 1;
        this.state = (byte) 10;
    }

    public final void hideNotify() {
        AudioManager.pause();
    }

    public final void showNotify() {
        AudioManager.resume();
    }

    private final void startTitle() {
        GameLoop.instance.setFps(15);
        this.animTick = 0;
        this.state = (byte) 1;
        this.glyph1X = (short) defpackage.ByteUtil.randRange(0, (BaseCanvas.width / 2) - 10);
        this.glyph1Y = (short) (10 * defpackage.ByteUtil.randRange(0, 4));
        this.glyph1Frame = (byte) defpackage.ByteUtil.randRange(0, 7);
        this.glyph2X = (short) defpackage.ByteUtil.randRange(BaseCanvas.width / 2, BaseCanvas.width - 10);
        this.glyph2Y = (short) (10 * defpackage.ByteUtil.randRange(3, 7));
        this.glyph2Frame = (byte) defpackage.ByteUtil.randRange(0, 7);
        AudioManager.playBgm(22);
    }

    /**
     * Enters story mode. {@code mode} selects which progress-flag bit
     * ({@code mode==1} -> bit 8, else bit 2) is consulted when {@code resume}
     * is not already forced; an unstarted story shows the loading intro.
     */
    public final void enterStoryMode(boolean resume, byte mode) {
        AudioManager.stopSfx();
        if (!resume) {
            if ((GameLoop.instance.progressFlags & (mode == 1 ? (byte) 8 : (byte) 2)) != 0) {
                resume = true;
            }
        }
        if (!resume || this.skipStoryIntro) {
            this.state = (byte) 2;
            return;
        }
        this.state = (byte) 0;
        this.loadPhase = (byte) 2;
        BaseCanvas.beginLoading("- STORY MODE", 52);
        GameLoop.instance.setLoadingFps();
        new Thread(this).start();
    }

    static {
        "*:MAP UPDATE".toCharArray();
    }
}
