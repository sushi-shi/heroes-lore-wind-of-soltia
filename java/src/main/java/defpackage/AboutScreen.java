package defpackage;

import javax.microedition.lcdui.Graphics;
import rpg.GameMIDlet;

/* renamed from: bl */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:AboutScreen.class */
/**
 * About screen, reached from the title {@link MainMenu}. The constructor builds
 * an info blob from the MIDlet properties (name, vendor), the copyright, website
 * and version strings and two localized labels, wraps it into lines (start
 * offsets in {@link #lineOffsets}) and lets the player scroll through
 * {@link #body}. It un-hides the small fonts' control glyphs while open and
 * re-hides them on Back.
 */
public final class AboutScreen extends Menu {
    /* renamed from: a */
    /** Assembled, wrapped about-text (name, credits, website, version). */
    private char[] body;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Start offset into {@link #body} of each wrapped line; one entry per scroll position. */
    private short[] lineOffsets;

    public AboutScreen(Menu parent, boolean inGame) {
        super(parent, (byte) 1);
        GameMIDlet gameMIDlet = GameMIDlet.instance;
        String developerLabel = FontManager.getString(3927);
        String versionLabel = FontManager.getString(3928);
        String appName = gameMIDlet.getAppProperty("MIDlet-Name").toUpperCase();
        this.body = new StringBuffer().append(appName).append("\n\n").append(FontManager.getString(3905)).append("\n\n").append(developerLabel).append('\n').append(gameMIDlet.getAppProperty("MIDlet-Vendor")).append('\n').append(FontManager.charsToString(FontManager.websiteText)).append("\n\n").append(versionLabel).append("\nv.").append(FontManager.charsToString(FontManager.versionText)).toString().toCharArray();
        short[] offsets = new short[20];
        int charPos = 0;
        int lineCount = 0;
        while (charPos < this.body.length) {
            int line = lineCount;
            lineCount++;
            offsets[line] = (short) charPos;
            charPos += FontManager.charsInLine(this.body, charPos, 130, 11);
        }
        this.lineOffsets = new short[lineCount];
        System.arraycopy(offsets, 0, this.lineOffsets, 0, this.lineOffsets.length);
        ((Menu) this).itemCount = (byte) this.lineOffsets.length;
        ((BitmapFont) FontManager.smallBlack).hideControls = false;
        ((BitmapFont) FontManager.smallWhite).hideControls = false;
        ((BitmapFont) FontManager.smallOrange).hideControls = false;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode) || keyCode != -8) {
            return true;
        }
        ((Menu) this).parent.close();
        ((BitmapFont) FontManager.smallBlack).hideControls = true;
        ((BitmapFont) FontManager.smallWhite).hideControls = true;
        ((BitmapFont) FontManager.smallOrange).hideControls = true;
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        MainMenu.drawTitlePlate(graphics, x, y);
        FontManager.drawMenuItem(graphics, 9, (x + 155) >> 1, y + 5);
        MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        int contentX = x + 12;
        int contentY = y + 42;
        if (((Menu) this).itemCount > 1) {
            if (((Menu) this).cursorIndex > 0) {
                graphics.drawImage(AssetCache.scrollUpArrow, contentX + 62, contentY - 6, 20);
            }
            if (((Menu) this).cursorIndex < ((Menu) this).itemCount - 1) {
                graphics.drawImage(AssetCache.scrollDownArrow, contentX + 62, contentY + 114, 20);
            }
        }
        BaseCanvas.drawFraction(graphics, (contentX + 155) - 25, contentY - 8, ((Menu) this).cursorIndex + 1, ((Menu) this).itemCount);
        short lineStart = this.lineOffsets[((Menu) this).cursorIndex];
        short lineEnd = ((Menu) this).cursorIndex == ((Menu) this).itemCount - 1 ? (short) this.body.length : this.lineOffsets[((Menu) this).cursorIndex + 1];
        if (this.body[0] == '!' && lineStart == 0) {
            lineStart = 1;
        }
        graphics.setColor(0);
        FontManager.drawWrappedBlockCentered(graphics, (contentX + 155) >> 1, contentY + 3, 130, 1, this.body, lineStart, 0, lineEnd - lineStart);
        FontManager.drawSoftKeys(graphics, (char[]) null, FontManager.labelBack);
    }
}
