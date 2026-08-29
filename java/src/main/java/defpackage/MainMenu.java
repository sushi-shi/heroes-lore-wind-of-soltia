package defpackage;

import javax.microedition.lcdui.Graphics;
import rpg.GameMIDlet;

/* renamed from: bf */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bf.class */
/**
 * The front/start menu: New Game / Continue / Options / Help / About / Exit
 * (built by {@link GameState} with the save blob). Continue is skipped when
 * there is no save ({@link #hasSave}). The logo has a three-frame intro
 * animation driven by {@link #logoFrame}. This screen also owns the demo
 * time-limit splash: when {@link #demoExpiry} is armed it shows the buy/exit
 * text and quits on timeout, and the static flags {@link #pendingBuyPrompt} /
 * {@link #pendingExitPrompt} let other screens request that splash on return.
 *
 * <p>The two static helpers {@link #drawTitlePlate}/{@link #drawMenuPanel} draw
 * the shared decorative menu frame from the UI atlas {@link AssetCache#f207l},
 * reused by most front-menu screens.
 */
public final class MainMenu extends Menu {
    /* renamed from: a */
    /** Centered panel origin X. */
    public static int panelX;
    /** Centered panel origin Y. */
    public static int panelY;

    /* renamed from: e */
    /** Whether a save exists (governs New Game vs Continue). */
    private boolean hasSave;

    /* renamed from: h */
    /** Save blob handed to the {@link ContinueMenu} slot picker. */
    private byte[] saveBlob;

    /* renamed from: c */
    /** Logo intro animation frame (0 &rarr; 1 &rarr; 2, then steady). */
    private byte logoFrame;

    /* renamed from: d */
    /** Which confirm a pending popup belongs to (0 new-game, 2 exit, 3/4 buy). */
    private byte pendingAction;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Demo trial deadline in ms (&le;0 = not a demo splash). */
    private long demoExpiry;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Menu index of the About item (shifts by one in the demo build). */
    private static int aboutIndex = 5;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Menu index of the Exit item. */
    private static int exitIndex = 5;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Singleton instance (created by {@link #create}). */
    private static MainMenu singleton;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Set by other screens to show the demo buy prompt on the next open. */
    public static boolean pendingBuyPrompt;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Set by other screens to show the demo exit prompt on the next open. */
    public static boolean pendingExitPrompt;

    /* renamed from: a */
    /** Returns the current main-menu instance (may be {@code null}). */
    public static final MainMenu instance() {
        return singleton;
    }

    private MainMenu(boolean hasSave, byte[] saveBlob) {
        super(null, (byte) 6);
        if (AppConfig.menuBuyEnabled) {
            ((Menu) this).itemCount = (byte) (((Menu) this).itemCount + 1);
        }
        this.hasSave = hasSave;
        this.saveBlob = saveBlob;
        this.logoFrame = (byte) 0;
        if (pendingExitPrompt || pendingBuyPrompt) {
            AssetCache.loadLogo();
            this.demoExpiry = System.currentTimeMillis() + 5000;
            if (pendingExitPrompt) {
                this.pendingAction = (byte) 2;
                pendingExitPrompt = false;
            } else if (pendingBuyPrompt) {
                this.pendingAction = (byte) 3;
                pendingBuyPrompt = false;
            }
        }
    }

    /* renamed from: a */
    /** Builds the singleton at the centered origin; {@code hasSave} enables Continue. */
    public static final void create(boolean hasSave, byte[] saveBlob) {
        panelX = BaseCanvas.halfW - 77;
        panelY = BaseCanvas.halfH - 85;
        singleton = new MainMenu(hasSave, saveBlob);
        if (AppConfig.menuBuyEnabled) {
            aboutIndex = 6;
            exitIndex = 5;
        }
    }

    /* renamed from: d */
    /** Releases the singleton. */
    public static final void dispose() {
        singleton = null;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (this.demoExpiry > 0) {
            if (AppConfig.fullVersion || !AppConfig.exitBuyEnabled) {
                return true;
            }
            if (keyCode == 53) {
                FontManager.requestBuyAndExit(AppConfig.buyUrl);
                return true;
            }
            if (keyCode != -8) {
                return true;
            }
            GameMIDlet.instance.exit();
            return true;
        }
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (moveCursorVertical(action, keyCode, false)) {
            if (!this.hasSave && ((Menu) this).cursorIndex == 1) {
                if (action == 6 || keyCode == 56) {
                    ((Menu) this).cursorIndex = (byte) (((Menu) this).cursorIndex + 1);
                } else {
                    ((Menu) this).cursorIndex = (byte) (((Menu) this).cursorIndex - 1);
                }
            }
            this.logoFrame = (byte) 0;
            return true;
        }
        if (keyCode == -8) {
            showPopup((byte) 2, (byte) 2, new Object[]{FontManager.confirmPrompt});
            this.pendingAction = (byte) 2;
        }
        if (action != 8 && keyCode != 53) {
            return false;
        }
        switch (((Menu) this).cursorIndex) {
            case 0:
                if (!this.hasSave) {
                    ((Menu) this).child = new ClassSelectMenu(this);
                    return false;
                }
                this.pendingAction = (byte) 0;
                showPopup((byte) 12, (byte) 2, new Object[]{FontManager.getString(3929).toCharArray()}, FontManager.labelOk, FontManager.labelBack);
                return false;
            case 1:
                ((Menu) this).child = new ContinueMenu(this, this.saveBlob);
                return false;
            case 2:
                ((Menu) this).child = new OptionsMenu(this, false);
                return false;
            case 3:
                ((Menu) this).child = new HelpMenu(this, false);
                return false;
            case 4:
                ((Menu) this).child = new AboutScreen(this, false);
                return false;
            default:
                if (((Menu) this).cursorIndex == aboutIndex) {
                    Object[] exitLines = {FontManager.confirmPrompt};
                    this.pendingAction = (byte) 2;
                    showPopup((byte) 2, (byte) 2, exitLines);
                    return false;
                }
                if (((Menu) this).cursorIndex != exitIndex) {
                    return false;
                }
                Object[] buyLines = {FontManager.getString(3918).toCharArray()};
                this.pendingAction = (byte) 3;
                showPopup((byte) 12, (byte) 2, buyLines);
                return false;
        }
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if (tag == 2 || tag == 12) {
            if (result != 0) {
                switch (this.pendingAction) {
                    case 4:
                        AssetCache.loadLogo();
                        this.demoExpiry = System.currentTimeMillis() + 5000;
                        break;
                }
            }
            switch (this.pendingAction) {
                case 0:
                    ((Menu) this).child = new ClassSelectMenu(this);
                    break;
                case 2:
                    if (!AppConfig.fullVersion) {
                        AssetCache.loadLogo();
                        this.demoExpiry = System.currentTimeMillis() + 5000;
                    } else {
                        Object[] buyLines = {FontManager.getString(3919).toCharArray()};
                        this.pendingAction = (byte) 4;
                        showPopup((byte) 12, (byte) 2, buyLines, FontManager.labelBuy, FontManager.labelExit);
                    }
                    break;
                case 3:
                case 4:
                    FontManager.requestBuyAndExit(AppConfig.buyUrl);
                    break;
            }
        }
    }

    /* renamed from: a */
    /** Draws the menu (or the demo splash when {@link #demoExpiry} is armed). */
    public final void draw(Graphics graphics) {
        if (this.demoExpiry <= 0) {
            render(graphics, panelX, panelY);
            if (this.logoFrame >= 2 || ((Menu) this).child != null) {
                return;
            }
            ((Menu) this).needsRepaint = true;
            this.logoFrame = (byte) (this.logoFrame + 1);
            return;
        }
        graphics.setColor(16777215);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        graphics.drawImage(AssetCache.logoFrames[4], BaseCanvas.halfW, GameScreen.centerY, 3);
        graphics.setColor(0);
        FontManager.drawCharsCentered(graphics, BaseCanvas.width >> 1, BaseCanvas.height - 23, FontManager.websiteText, 1);
        FontManager.drawCharsCentered(graphics, BaseCanvas.width >> 1, 10, StringTable.instance.get(3941).toCharArray(), 1);
        if (!AppConfig.fullVersion && AppConfig.exitBuyEnabled) {
            FontManager.drawSoftKeys(graphics, AppConfig.resolveBuyLabel().toCharArray(), FontManager.labelExit);
        }
        if (System.currentTimeMillis() > this.demoExpiry) {
            GameMIDlet.instance.exit();
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int menuY = y + 13;
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        drawMenuPanel(graphics, x, menuY - 12, 4);
        char logoSprite = 18;
        if (this.logoFrame == 0) {
            logoSprite = 14;
        } else if (this.logoFrame == 1) {
            logoSprite = 16;
        }
        graphics.drawImage(AssetCache.menuFrames[logoSprite], (x + (155 - AssetCache.menuFrames[logoSprite].getWidth())) >> 1, menuY + 12 + (((Menu) this).cursorIndex * 16), 20);
        for (int item = 0; item < ((Menu) this).itemCount; item++) {
            int itemY = menuY + 14 + (item * 16);
            byte labelId = (byte) (item * 2);
            if (((Menu) this).cursorIndex != item || this.logoFrame < 2) {
                labelId = (byte) (labelId + 1);
            }
            FontManager.drawMenuItem(graphics, labelId, (x + 155) >> 1, itemY);
        }
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelExit);
    }

    /* renamed from: c */
    /** Draws the two-row decorative title plate from the UI atlas at ({@code x},{@code y}). */
    public static final void drawTitlePlate(Graphics graphics, int x, int y) {
        graphics.drawImage(AssetCache.menuFrames[0], x, y, 20);
        int topX = x + 12;
        graphics.drawImage(AssetCache.menuFrames[1], topX, y, 20);
        for (int col = 0; col < 3; col++) {
            topX += 32;
            graphics.drawImage(AssetCache.menuFrames[1], topX, y, 20);
        }
        graphics.drawImage(AssetCache.menuFrames[2], topX + 32, y, 20);
        graphics.drawImage(AssetCache.menuFrames[11], x, y + 12, 20);
        int bottomX = x + 12;
        graphics.drawImage(AssetCache.menuFrames[12], bottomX, y + 12, 20);
        for (int col2 = 0; col2 < 3; col2++) {
            bottomX += 32;
            graphics.drawImage(AssetCache.menuFrames[12], bottomX, y + 12, 20);
        }
        graphics.drawImage(AssetCache.menuFrames[13], bottomX + 32, y + 12, 20);
    }

    /* renamed from: b */
    /** Draws a bordered menu panel from the UI atlas with {@code rows}+1 content rows. */
    public static final void drawMenuPanel(Graphics graphics, int x, int y, int rows) {
        int contentRows = rows + 1;
        graphics.drawImage(AssetCache.menuFrames[3], x, y, 20);
        int topX = x + 12;
        graphics.drawImage(AssetCache.menuFrames[4], topX, y, 20);
        for (int col = 0; col < 3; col++) {
            topX += 32;
            graphics.drawImage(AssetCache.menuFrames[4], topX, y, 20);
        }
        graphics.drawImage(AssetCache.menuFrames[5], topX + 32, y, 20);
        graphics.drawImage(AssetCache.menuFrames[6], x, y + 12, 20);
        int midX = x + 12;
        graphics.drawImage(AssetCache.menuFrames[7], midX, y + 12, 20);
        for (int col2 = 0; col2 < 3; col2++) {
            midX += 32;
            graphics.drawImage(AssetCache.menuFrames[7], midX, y + 12, 20);
        }
        graphics.drawImage(AssetCache.menuFrames[8], midX + 32, y + 12, 20);
        for (int row = 0; row < contentRows; row++) {
            graphics.drawImage(AssetCache.menuFrames[9], x, y + 36 + (24 * row), 20);
            graphics.drawImage(AssetCache.menuFrames[10], x + 12 + 128, y + 36 + (24 * row), 20);
        }
        graphics.setColor(16763769);
        graphics.fillRect(x + 12, y + 36, 128, 24 * contentRows);
        graphics.drawImage(AssetCache.menuFrames[11], x, y + 36 + (24 * contentRows), 20);
        int bottomX = x + 12;
        graphics.drawImage(AssetCache.menuFrames[12], bottomX, y + 36 + (24 * contentRows), 20);
        for (int col3 = 0; col3 < 3; col3++) {
            bottomX += 32;
            graphics.drawImage(AssetCache.menuFrames[12], bottomX, y + 36 + (24 * contentRows), 20);
        }
        graphics.drawImage(AssetCache.menuFrames[13], bottomX + 32, y + 36 + (24 * contentRows), 20);
    }
}
