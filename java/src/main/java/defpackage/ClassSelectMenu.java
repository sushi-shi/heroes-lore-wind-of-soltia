package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: c */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:c.class */
/**
 * Starting-class selection screen, reached from {@link MainMenu} (New Game, and
 * the "change class" entry). It shows three class portraits from
 * {@link AssetCache#classFaces}, highlighting the one under the cursor; the two
 * side classes stay locked until a character has been created
 * ({@link GameLoop#hasCreatedCharacter}), otherwise picking them just shows a
 * hint message. Confirming a class opens {@link ClassConfirmMenu} for
 * {@code classId = 6 + (2 - cursorIndex)}. {@link #onPopupResult} handles the
 * tag-2/12 "return to title" confirm that tears down the main menu.
 */
public final class ClassSelectMenu extends Menu {
    public ClassSelectMenu(MainMenu parent) {
        super(parent, (byte) 3);
        ((Menu) this).cursorIndex = (byte) 2;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.close();
            return true;
        }
        if (((Menu) this).cursorIndex != 0 && ((Menu) this).cursorIndex != 1) {
            ((Menu) this).child = new ClassConfirmMenu(this, (byte) (6 + (2 - ((Menu) this).cursorIndex)));
            return true;
        }
        if (GameLoop.instance.hasCreatedCharacter) {
            ((Menu) this).child = new ClassConfirmMenu(this, (byte) (6 + (2 - ((Menu) this).cursorIndex)));
            return true;
        }
        showMessage(new Object[]{AssetCache.commonText.get(6), AssetCache.commonText.get(7)});
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if ((tag == 2 || tag == 12) && result == 0) {
            MainMenu.dispose();
            AssetCache.unloadMainMenuAssets();
            GameLoop.instance.returnToTitle();
            GameState.screen = 0;
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        MainMenu.drawTitlePlate(graphics, x, y);
        FontManager.drawMenuItem(graphics, 1, (x + 155) >> 1, y + 5);
        MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
        int baseX = x + 15;
        int baseY = y + 10;
        if (((Menu) this).cursorIndex != 0) {
            graphics.drawImage(AssetCache.classFaces[5], baseX + 6, baseY + 38, 20);
        }
        if (((Menu) this).cursorIndex != 1) {
            graphics.drawImage(AssetCache.classFaces[4], baseX + 34, baseY + 38, 20);
        }
        if (((Menu) this).cursorIndex != 2) {
            graphics.drawImage(AssetCache.classFaces[3], baseX + 59, baseY + 38, 20);
        }
        if (((Menu) this).cursorIndex == 0) {
            graphics.drawImage(AssetCache.classFaces[2], baseX + 6, baseY + 38, 20);
        }
        if (((Menu) this).cursorIndex == 1) {
            graphics.drawImage(AssetCache.classFaces[1], baseX + 34, baseY + 38, 20);
        }
        if (((Menu) this).cursorIndex == 2) {
            graphics.drawImage(AssetCache.classFaces[0], baseX + 59, baseY + 38, 20);
        }
        graphics.setColor(0);
        FontManager.drawChars(graphics, baseX + 11, baseY + 104, AssetCache.commonText.get(12), 1);
        FontManager.drawChars(graphics, baseX + 11, baseY + 119, AssetCache.commonText.get(13), 1);
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }
}
