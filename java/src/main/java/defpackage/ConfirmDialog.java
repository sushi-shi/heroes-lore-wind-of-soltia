package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: am */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ConfirmDialog.class */
/**
 * Two-line confirmation dialog used by {@link SkillTab} for the learn-skill and
 * accept-quest prompts. It draws a highlighted title ({@link #line1}) above a
 * body line ({@link #line2}) in a centered panel whose height follows
 * {@link #lineCount} (the wrapped line total of both texts). Pressing OK reports
 * {@code result = 1} and Back reports {@code result = 0} to the parent through
 * {@code onPopupResult}, tagged with {@link #resultTag} so the parent knows
 * which prompt answered.
 */
public final class ConfirmDialog extends Menu {
    /* renamed from: a */
    /** Highlighted title/prompt line. */
    private char[] line1;

    /* renamed from: b */
    /** Body/detail line drawn under the title. */
    private char[] line2;

    /* renamed from: d */
    /** Total wrapped line count of both texts; drives the panel height. */
    private byte lineCount;

    /* renamed from: c */
    /** Caller-supplied tag echoed back through {@code onPopupResult}. */
    public byte resultTag;

    public ConfirmDialog(Menu parent, char[] line1, char[] line2, byte tag) {
        super(parent, (byte) 0);
        this.line1 = line1;
        this.line2 = line2;
        this.resultTag = tag;
        this.lineCount = (byte) 0;
        this.lineCount = (byte) (this.lineCount + FontManager.lineCount(line1, 135));
        this.lineCount = (byte) (this.lineCount + FontManager.lineCount(line2, 135));
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (keyCode == 53 || action == 8) {
            ((Menu) this).parent.onPopupResult(this.resultTag, (byte) 1);
            return true;
        }
        if (keyCode != -8) {
            return true;
        }
        ((Menu) this).parent.onPopupResult(this.resultTag, (byte) 0);
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int dialogHeight = (this.lineCount * 15) + 10;
        int boxX = BaseCanvas.halfW - 72;
        int boxY = BaseCanvas.halfH - (dialogHeight / 2);
        Menu.drawPanelFrame(graphics, boxX, boxY, 145, dialogHeight);
        Menu.fillPanelInterior(graphics, boxX, boxY, 145, dialogHeight);
        int textY = boxY + 5;
        graphics.setColor(14663551);
        FontManager.drawWrappedText(graphics, boxX + 5, textY, 135, 1, this.line1);
        int line2Y = textY + (15 * FontManager.lineCount(this.line1, 135));
        graphics.setColor(16777215);
        FontManager.drawWrappedText(graphics, boxX + 5, line2Y, 135, 1, this.line2);
    }
}
