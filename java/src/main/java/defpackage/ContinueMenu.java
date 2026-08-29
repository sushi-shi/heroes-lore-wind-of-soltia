package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: a */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:a.class */
/**
 * Load-game slot picker — the "Continue" screen reached from {@link MainMenu}
 * when saved games exist. It lists each saved character (class name looked up in
 * {@link AssetCache#heroText}) with its level and progress percentage, and
 * starts the highlighted save through {@link GameState#newGame}. The
 * {@link #slotData} blob packs four bytes per slot
 * ({@code [classId, level, progress%, ...]}); {@link #cursorAnimFrame} plays a
 * short three-frame highlight intro on the selected row.
 */
public final class ContinueMenu extends Menu {
    /* renamed from: h */
    /** Saved-slot summary blob, four bytes per slot: class id, level, progress %, spare. */
    private byte[] slotData;

    /* renamed from: c */
    /** Selection-highlight intro frame counter (0-2), advanced each paint. */
    private byte cursorAnimFrame;

    public ContinueMenu(MainMenu parent, byte[] slotData) {
        super(parent, (byte) (slotData.length / 4));
        this.slotData = slotData;
        this.cursorAnimFrame = (byte) 0;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (((Menu) this).itemCount > 1 && moveCursorVerticalNoWrap(action, keyCode)) {
            this.cursorAnimFrame = (byte) 0;
            return true;
        }
        if (keyCode == 53 || action == 8) {
            System.out.println(new StringBuffer().append("continue game with ").append((int) this.slotData[((Menu) this).cursorIndex * 4]).toString());
            GameState.newGame(true, this.slotData[((Menu) this).cursorIndex * 4], (boolean[]) null);
            return true;
        }
        if (keyCode != -8) {
            return true;
        }
        ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        MainMenu.drawTitlePlate(graphics, x, y);
        FontManager.drawMenuItem(graphics, 3, BaseCanvas.width >> 1, y + 5);
        MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        int baseY = y + 5;
        int baseX = x + 10;
        for (int row = 0; row < 5; row++) {
            graphics.drawImage(AssetCache.menuFrames[19], baseX + 13, baseY + 49 + (row * 16), 20);
        }
        switch (this.cursorAnimFrame) {
            case 0:
                graphics.drawImage(AssetCache.menuFrames[14], baseX + 5, baseY + 31 + (((Menu) this).cursorIndex * 16), 20);
                break;
            case 1:
                graphics.drawImage(AssetCache.menuFrames[16], baseX + 5, baseY + 31 + (((Menu) this).cursorIndex * 16), 20);
                break;
            default:
                graphics.drawImage(AssetCache.menuFrames[18], baseX + 5, baseY + 31 + (((Menu) this).cursorIndex * 16), 20);
                break;
        }
        if (this.cursorAnimFrame < 2) {
            ((Menu) this).needsRepaint = true;
            this.cursorAnimFrame = (byte) (this.cursorAnimFrame + 1);
        }
        byte item = 0;
        while (true) {
            byte itemIndex = item;
            if (itemIndex >= ((Menu) this).itemCount) {
                graphics.drawImage(AssetCache.statLabel3, baseX + 15, baseY + 104, 20);
                BaseCanvas.drawNumberAt(graphics, this.slotData[(((Menu) this).cursorIndex * 4) + 1], baseX + 30, baseY + 104, 4);
                graphics.setColor(8347487);
                FontManager.drawChars(graphics, baseX + 15, baseY + 117, new StringBuffer().append(FontManager.progressLabel).append((int) this.slotData[(((Menu) this).cursorIndex * 4) + 2]).append("%").toString().toCharArray(), 1);
                if (BaseCanvas.width > 128) {
                    graphics.drawImage(AssetCache.classFaces[this.slotData[((Menu) this).cursorIndex * 4] - 6], baseX + 61 + 22, baseY + 74 + 15, 20);
                } else {
                    graphics.drawImage(AssetCache.classFaces[this.slotData[((Menu) this).cursorIndex * 4] - 6], baseX + 61, baseY + 74, 20);
                }
                FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
                return;
            }
            if (((Menu) this).cursorIndex == itemIndex) {
                graphics.setColor(16777215);
            } else {
                graphics.setColor(10452863);
            }
            FontManager.drawChars(graphics, baseX + 21, baseY + 36 + (itemIndex * 16), AssetCache.heroText.get(this.slotData[itemIndex * 4] - 6), 1);
            item = (byte) (itemIndex + 1);
        }
    }
}
