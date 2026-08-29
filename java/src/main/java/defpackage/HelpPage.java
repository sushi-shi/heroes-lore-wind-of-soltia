package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bx */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:HelpPage.class */
/**
 * A single scrollable help article, opened by {@link HelpMenu} for the selected
 * topic. The constructor breaks {@link #body} into up to twenty wrapped lines,
 * recording each line's start offset in {@link #lineOffsets}; the cursor then
 * scrolls through those lines one screenful at a time. It draws either as an
 * in-game inset ({@link #inGame == true}) or full screen with a {@link #title}
 * header and up/down scroll arrows; Back returns to the topic list.
 */
public final class HelpPage extends Menu {
    /* renamed from: a */
    /** Full body text of the help article. */
    private char[] body;

    /* renamed from: b */
    /** Title/header shown above the body (full-screen mode). */
    private char[] title;

    /* renamed from: c */
    /** True when opened in-game (inset panel); false when opened from the title menu. */
    private boolean inGame;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Start offset into {@link #body} of each wrapped line; one entry per scroll position. */
    private short[] lineOffsets;

    public HelpPage(HelpMenu parent, char[] body, boolean inGame, char[] title) {
        super(parent, (byte) 1);
        this.body = body;
        this.title = title;
        this.inGame = inGame;
        short[] offsets = new short[20];
        int charPos = 0;
        int lineCount = 0;
        while (charPos < body.length) {
            int line = lineCount;
            lineCount++;
            offsets[line] = (short) charPos;
            charPos += FontManager.charsInLine(body, charPos, 130, 11);
        }
        this.lineOffsets = new short[lineCount];
        System.arraycopy(offsets, 0, this.lineOffsets, 0, this.lineOffsets.length);
        ((Menu) this).itemCount = (byte) this.lineOffsets.length;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode) || keyCode != -8) {
            return true;
        }
        ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int contentX;
        int contentY;
        if (this.inGame) {
            int panelX = x + 2;
            int panelY = y + 15;
            Menu.fillPanelInterior(graphics, panelX + 4, panelY + 10, 143, 139);
            contentX = panelX + 8;
            contentY = panelY + 25;
            graphics.setColor(16777215);
            FontManager.drawSoftKeys(graphics, FontManager.blankLabel, FontManager.labelBack);
        } else {
            graphics.setColor(4136767);
            graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
            MainMenu.drawTitlePlate(graphics, x, y);
            FontManager.setBigFont(true);
            graphics.setColor(0);
            FontManager.drawCharsCentered(graphics, (x + 155) >> 1, y + 5 + 4, this.title, 1);
            FontManager.setBigFont(false);
            MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
            contentX = x + 10;
            contentY = y + 43;
            graphics.setColor(6242111);
            FontManager.drawSoftKeys(graphics, (char[]) null, FontManager.labelBack);
        }
        BaseCanvas.drawFraction(graphics, (contentX + 155) - 25, contentY - 8, ((Menu) this).cursorIndex + 1, ((Menu) this).itemCount);
        if (((Menu) this).itemCount > 1) {
            if (((Menu) this).cursorIndex > 0) {
                graphics.drawImage(AssetCache.scrollUpArrow, contentX + 62, contentY - 6, 20);
            }
            if (((Menu) this).cursorIndex < ((Menu) this).itemCount - 1) {
                graphics.drawImage(AssetCache.scrollDownArrow, contentX + 62, contentY + 114, 20);
            }
        }
        short lineStart = this.lineOffsets[((Menu) this).cursorIndex];
        short lineEnd = ((Menu) this).cursorIndex == ((Menu) this).itemCount - 1 ? (short) this.body.length : this.lineOffsets[((Menu) this).cursorIndex + 1];
        if (this.body[0] == '!' && lineStart == 0) {
            lineStart = 1;
        }
        if (this.inGame) {
            graphics.setColor(16777215);
        } else {
            graphics.setColor(0);
        }
        FontManager.drawWrappedBlock(graphics, contentX, contentY + 3, 130, 1, this.body, lineStart, 0, lineEnd - lineStart);
    }
}
