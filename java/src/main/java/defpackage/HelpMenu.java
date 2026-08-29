package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bt */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:HelpMenu.class */
/**
 * Help topic list, reached from the in-game {@link SystemTab}
 * ({@code inGame == true}, drawn as an inset panel) and from the title
 * {@link MainMenu} ({@code inGame == false}, full screen). Each entry in
 * {@link #topicLabels} (from {@link AssetCache#helpText}, plus an optional
 * buy/full-version link row) opens a {@link HelpPage} with the matching body
 * text. While this menu is open it un-hides the small fonts' control glyphs, and
 * re-hides them on Back.
 */
public final class HelpMenu extends Menu {
    /* renamed from: c */
    /** True when opened in-game (inset panel); false when opened from the title menu. */
    private boolean inGame;

    /* renamed from: a */
    /** Per-row topic titles shown in the list (last row may be a buy/full-version link). */
    private char[][] topicLabels;

    public HelpMenu(Menu parent, boolean inGame) {
        super(parent, (byte) 4);
        if (AppConfig.showFullVersionLink() || AppConfig.showDemoBuyLink()) {
            ((Menu) this).itemCount = (byte) (((Menu) this).itemCount + 1);
        }
        this.inGame = inGame;
        ((BitmapFont) FontManager.smallBlack).hideControls = false;
        ((BitmapFont) FontManager.smallWhite).hideControls = false;
        ((BitmapFont) FontManager.smallOrange).hideControls = false;
        this.topicLabels = new char[((Menu) this).itemCount][];
        byte topic = 0;
        while (true) {
            byte topicIndex = topic;
            if (topicIndex >= ((Menu) this).itemCount) {
                return;
            }
            if (topicIndex == 4) {
                this.topicLabels[topicIndex] = AppConfig.resolveBuyLabel().toCharArray();
            } else {
                this.topicLabels[topicIndex] = AssetCache.helpText.get(topicIndex);
            }
            topic = (byte) (topicIndex + 1);
        }
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
            ((BitmapFont) FontManager.smallBlack).hideControls = true;
            ((BitmapFont) FontManager.smallWhite).hideControls = true;
            ((BitmapFont) FontManager.smallOrange).hideControls = true;
            return true;
        }
        byte topic = (byte) (((Menu) this).cursorIndex + 6);
        if (Debug.fullVersion && ((Menu) this).cursorIndex == 5) {
            topic = (byte) (topic + 1);
        }
        char[] pageText = AssetCache.helpText.get(topic);
        if (((Menu) this).cursorIndex == ((Menu) this).itemCount - 1) {
            if (AppConfig.showFullVersionLink()) {
                pageText = StringTable.instance.get(3930).toCharArray();
            } else if (AppConfig.showDemoBuyLink()) {
                pageText = FontManager.replaceAll(StringTable.instance.get(3934), "XXX", new String(AppConfig.resolveBuyLabel())).toCharArray();
            }
        }
        ((Menu) this).child = new HelpPage(this, pageText, this.inGame, this.topicLabels[((Menu) this).cursorIndex]);
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int listTop;
        int dimColor = 0;
        if (this.inGame) {
            int panelX = x + 6;
            int panelY = y + 25;
            Menu.drawPanelFrame(graphics, panelX, panelY, 143, 139);
            Menu.fillPanelInterior(graphics, panelX, panelY, 143, 139);
            dimColor = 10452799;
            listTop = panelY + 8;
        } else {
            graphics.setColor(4136767);
            graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
            MainMenu.drawTitlePlate(graphics, x, y);
            MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
            FontManager.drawMenuItem(graphics, 7, BaseCanvas.width >> 1, y + 5);
            listTop = y + 41;
        }
        int firstItemY = listTop + 10;
        FontManager.setBigFont(true);
        byte item = 0;
        while (true) {
            byte itemIndex = item;
            if (itemIndex >= ((Menu) this).itemCount) {
                break;
            }
            if (((Menu) this).cursorIndex == itemIndex) {
                graphics.setColor(16777215);
            } else {
                graphics.setColor(dimColor);
            }
            FontManager.drawCharsCentered(graphics, BaseCanvas.width >> 1, firstItemY + (itemIndex * 15), this.topicLabels[itemIndex], 1);
            item = (byte) (itemIndex + 1);
        }
        FontManager.setBigFont(false);
        if (this.inGame) {
            return;
        }
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }
}
