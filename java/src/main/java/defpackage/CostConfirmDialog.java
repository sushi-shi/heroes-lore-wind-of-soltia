package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bo */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:CostConfirmDialog.class */
/**
 * Cost-confirmation dialog shown before a paid action — e.g. the item-combine
 * fee opened from {@link CombineMenu}. Under a {@link #title} it lists the
 * affected item names ({@link #itemLines}), the player's current gold, and the
 * {@link #cost} to be charged (labelled {@link #costLabel}). Pressing OK reports
 * back to the parent through {@code onPopupResult} tagged {@link #resultTag};
 * Back closes the dialog.
 */
public final class CostConfirmDialog extends Menu {
    /* renamed from: a */
    /** Dialog title/header line. */
    private char[] title;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Item-name lines listed in the dialog body (may contain nulls). */
    private Object[] itemLines;

    /* renamed from: b */
    /** Label for the cost row (e.g. "Fee"). */
    private char[] costLabel;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Gold amount that will be charged on confirm. */
    private int cost;

    /* renamed from: c */
    /** Caller-supplied tag echoed back through {@code onPopupResult}. */
    private byte resultTag;

    public CostConfirmDialog(Menu parent, char[] title, Object[] itemLines, char[] costLabel, int cost, byte tag) {
        super(parent, (byte) 0);
        this.title = title;
        this.resultTag = tag;
        this.itemLines = itemLines;
        this.costLabel = costLabel;
        this.cost = cost;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (keyCode == -8) {
            ((Menu) this).parent.close();
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return true;
        }
        ((Menu) this).parent.onPopupResult(this.resultTag, (byte) 0);
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int boxX = BaseCanvas.halfW - 67;
        int boxY = BaseCanvas.halfH - 60;
        Hero hero = GameState.hero();
        Menu.drawInsetPanel(graphics, boxX, boxY, 135, 120);
        Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 3, 129, 17, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, boxX + 6, boxY + 4, this.title, 1);
        Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 25, 129, 60, 10452863);
        graphics.setColor(16777215);
        for (int line = 0; line < this.itemLines.length; line++) {
            if (this.itemLines[line] != null) {
                FontManager.drawChars(graphics, boxX + 6, boxY + 27 + (line * 18), (char[]) this.itemLines[line], 1);
            }
        }
        Menu.drawGold(graphics, (boxX + 135) - 5, boxY + 90, hero.bag.gold);
        Menu.fillOutlinedRect(graphics, boxX + 3, boxY + 98, 129, 15, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, boxX + 6, boxY + 99, this.costLabel, 1);
        Menu.drawGold(graphics, (boxX + 135) - 5, boxY + 105, this.cost);
    }
}
