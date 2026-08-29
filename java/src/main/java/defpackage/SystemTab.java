package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: d */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:d.class */
/**
 * System tab (tab 5) of {@link CharacterMenu}: Save / Help / Options / Quit.
 * Save runs a tiny two-frame state machine in {@link #paint} via
 * {@link #saveState} (draws a "saving" box, then performs {@link GameState#saveGame()}
 * and reports the result); Help and Options push their menus; Save (in the demo
 * build) and Quit route through {@link #promptExit}, which shows the buy-full
 * prompt in the demo or the quit-confirm otherwise, handled in
 * {@link #onPopupResult}.
 */
public final class SystemTab extends Menu {
    /* renamed from: c */
    /** Save state machine: 0 = idle, 2 = save requested (show box), 1 = perform save. */
    private byte saveState;

    public SystemTab(Menu parentMenu) {
        super(parentMenu, (byte) 4);
        this.saveState = (byte) 0;
    }

    @Override // defpackage.cb
    public final synchronized boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || this.saveState != 0 || moveCursorVertical(action, keyCode, false)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        switch (((Menu) this).cursorIndex) {
            case 0:
                if (AppConfig.fullVersion) {
                    promptExit();
                    return true;
                }
                if (GameState.map.bossMap) {
                    showMessage(new Object[]{CharacterMenu.text.get(51), CharacterMenu.text.get(52)});
                    return true;
                }
                this.saveState = (byte) 2;
                invalidateUp();
                return true;
            case 1:
                ((Menu) this).child = new HelpMenu(this, true);
                return true;
            case 2:
                ((Menu) this).child = new OptionsMenu(this, true);
                return true;
            case 3:
                promptExit();
                return true;
            default:
                return true;
        }
    }

    /* renamed from: d */
    /** Shows the buy-full prompt (demo build) or the quit-to-menu confirm. */
    public final void promptExit() {
        if (AppConfig.fullVersion) {
            showPopup((byte) 12, (byte) 2, new Object[]{FontManager.getString(3919).toCharArray()}, FontManager.labelBuy, FontManager.labelExit);
        } else {
            showPopup((byte) 2, (byte) 2, new Object[]{FontManager.confirmPrompt});
        }
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if (tag == 12 || tag == 2) {
            if (!AppConfig.fullVersion) {
                if (result == 0) {
                    GameState.requestState((byte) 14, (byte) 1);
                    AudioManager.stopBgm();
                    return;
                }
                return;
            }
            if (result == 0) {
                FontManager.requestBuyAndExit(AppConfig.buyUrl);
                return;
            }
            GameState.requestState((byte) 14, (byte) 1);
            AudioManager.stopBgm();
            MainMenu.pendingExitPrompt = true;
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelY = y + 15;
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(41), x + 5, panelY);
        int buttonX = (BaseCanvas.width - 108) >> 1;
        Menu.drawButton(graphics, buttonX, panelY + 15, 108, CharacterMenu.text.get(42), ((Menu) this).cursorIndex == 0);
        Menu.drawButton(graphics, buttonX, panelY + 37, 108, CharacterMenu.text.get(43), ((Menu) this).cursorIndex == 1);
        Menu.drawButton(graphics, buttonX, panelY + 59, 108, CharacterMenu.text.get(44), ((Menu) this).cursorIndex == 2);
        Menu.drawButton(graphics, buttonX, panelY + 81, 108, CharacterMenu.text.get(45), ((Menu) this).cursorIndex == 3);
        if (this.saveState != 2) {
            if (this.saveState == 1) {
                this.saveState = (byte) 0;
                try {
                    GameState.saveGame();
                    showMessage(new Object[]{CharacterMenu.text.get(46)});
                    return;
                } catch (Exception unused) {
                    showMessage(new Object[]{CharacterMenu.text.get(47), CharacterMenu.text.get(48)});
                    return;
                }
            }
            return;
        }
        this.saveState = (byte) 1;
        int boxX = BaseCanvas.halfW - 55;
        int boxY = BaseCanvas.halfH - 11;
        Menu.drawPanelFrame(graphics, boxX, boxY, 110, 22);
        Menu.fillPanelInterior(graphics, boxX, boxY, 110, 22);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, boxX + 5, boxY + 5, CharacterMenu.text.get(53), 1);
        invalidateUp();
    }
}
