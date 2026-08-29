package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: by */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ClassConfirmMenu.class */
/**
 * Confirmation screen for the class chosen in {@link ClassSelectMenu}. It shows
 * the class name and description ({@link AssetCache#heroText}) plus its portrait
 * ({@link AssetCache#classFaces}), with a Yes/No selector. Choosing "Yes"
 * (cursor 0) opens {@link StartTraitMenu} to begin a new game as
 * {@link #classId}; "No"/Back returns to the class list.
 */
public final class ClassConfirmMenu extends Menu {
    /* renamed from: c */
    /** Selected starting class id ({@code 6 + (2 - cursorIndex)} from the class list). */
    private byte classId;

    public ClassConfirmMenu(ClassSelectMenu parent, byte classId) {
        super(parent, (byte) 2);
        ((Menu) this).cursorIndex = (byte) 1;
        this.classId = classId;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) {
            return true;
        }
        switch (action) {
            case 8:
                if (((Menu) this).cursorIndex != 0) {
                    ((Menu) this).parent.close();
                } else {
                    ((Menu) this).child = new StartTraitMenu(this, this.classId);
                }
                break;
            default:
                switch (keyCode) {
                    case -8:
                        ((Menu) this).parent.close();
                        break;
                    case 53:
                        if (((Menu) this).cursorIndex != 0) {
                            ((Menu) this).parent.close();
                        } else {
                            ((Menu) this).child = new StartTraitMenu(this, this.classId);
                        }
                        break;
                }
                break;
        }
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        MainMenu.drawTitlePlate(graphics, x, y);
        FontManager.drawMenuItem(graphics, 1, x + 77, y + 5);
        MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        int baseY = y + 5;
        int baseX = x + 10;
        graphics.setColor(0);
        FontManager.drawWrappedText(graphics, baseX + 11, baseY + 34, 133, 1, AssetCache.heroText.get((15 + this.classId) - 6));
        graphics.drawImage(AssetCache.menuFrames[19], baseX + 7, baseY + 80, 20);
        FontManager.drawChars(graphics, baseX + 11, baseY + 84, AssetCache.heroText.get(this.classId - 6), 1);
        graphics.drawImage(AssetCache.classFaces[this.classId - 6], baseX + 125, baseY + 137, 40);
        graphics.drawImage(AssetCache.menuFrames[17], baseX + 5 + (((Menu) this).cursorIndex == 0 ? 0 : 28), baseY + 118, 20);
        if (((Menu) this).cursorIndex == 0) {
            graphics.setColor(16777215);
        } else {
            graphics.setColor(0);
        }
        FontManager.drawChars(graphics, baseX + 9, baseY + 121, AssetCache.commonText.get(14), 1);
        if (((Menu) this).cursorIndex == 1) {
            graphics.setColor(16777215);
        } else {
            graphics.setColor(0);
        }
        FontManager.drawChars(graphics, baseX + 37, baseY + 121, AssetCache.commonText.get(15), 1);
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }
}
