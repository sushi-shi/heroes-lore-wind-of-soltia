package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bm */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bm.class */
/**
 * Guardian tab (tab 3) of {@link CharacterMenu}: the five guardian companion
 * slots. The cursor starts on the active guardian; OK on a filled slot opens a
 * popup to either make it the active guardian (with a summon check) or open its
 * {@link GuardianSkillPanel}. The right side shows the selected guardian's
 * name/level/exp.
 */
public final class GuardianTab extends Menu {
    public GuardianTab(Menu parentMenu) {
        super(parentMenu, (byte) 5);
        Hero hero = GameState.hero();
        for (byte slot = 0; slot < 5; slot = (byte) (slot + 1)) {
            if (hero.guardians[slot] == hero.getActiveGuardian()) {
                ((Menu) this).cursorIndex = slot;
                return;
            }
        }
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        Hero hero = GameState.hero();
        if ((keyCode != 53 && action != 8) || hero.guardians[((Menu) this).cursorIndex] == null) {
            return false;
        }
        showPopup((byte) 3, (byte) 2, new Object[]{CharacterMenu.text.get(22), CharacterMenu.text.get(23)});
        return false;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        Hero hero = GameState.hero();
        if ((previousChild instanceof PopupMenu) && tag == 3) {
            switch (result) {
                case 0:
                    if (!hero.setActiveGuardian(hero.guardians[((Menu) this).cursorIndex])) {
                        showMessage(new Object[]{CharacterMenu.text.get(27), CharacterMenu.text.get(28), CharacterMenu.text.get(29)});
                    } else {
                        showMessage(new Object[]{new StringBuffer().append(StringTable.instance.get(3933)).append(" ").append(new String(AssetCache.guardianText.get(hero.guardians[((Menu) this).cursorIndex].type))).toString().toCharArray()});
                    }
                    break;
                case 1:
                    ((Menu) this).child = new GuardianSkillPanel(this, hero.guardians[((Menu) this).cursorIndex]);
                    break;
            }
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int panelY = y + 15;
        Hero hero = GameState.hero();
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(30), panelX + 5, panelY);
        drawListPage(graphics, panelX, panelY, false);
        for (int slot = 0; slot < 5; slot++) {
            if (hero.guardians[slot] != null) {
                graphics.drawImage(AssetCache.guardianIcons[hero.guardians[slot].type], panelX + 15, panelY + 19 + ((slot % 5) * 23), 3);
            }
        }
        if (hero.guardians[((Menu) this).cursorIndex] == null) {
            graphics.setColor(14663551);
            FontManager.drawChars(graphics, panelX + 34, panelY + 18, CharacterMenu.text.get(31), 1);
            return;
        }
        Guardian guardian = hero.guardians[((Menu) this).cursorIndex];
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(32), panelX + 89, panelY + 22);
        if (guardian == hero.getActiveGuardian()) {
            graphics.drawImage(AssetCache.portraitFrame, panelX + 100, panelY + 26, 36);
        }
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, panelX + 34, panelY + 18, AssetCache.guardianText.get(hero.guardians[((Menu) this).cursorIndex].type), 1);
        graphics.setColor(14663551);
        FontManager.drawChars(graphics, panelX + 34, panelY + 35, AssetCache.guardianText.get(hero.guardians[((Menu) this).cursorIndex].type + 6), 1);
        graphics.drawImage(AssetCache.statLabel3, panelX + 34, panelY + 53, 20);
        BaseCanvas.drawNumberAt(graphics, guardian.level, panelX + 50, panelY + 53, 4);
        graphics.drawImage(AssetCache.statLabel1, panelX + 34, panelY + 67, 20);
        BaseCanvas.drawNumberAt(graphics, guardian.exp, panelX + 102, panelY + 67, 8);
        graphics.setColor(4136767);
        graphics.fillRect(panelX + 34, panelY + 76, 72, 3);
        graphics.setColor(16777215);
        graphics.fillRect(panelX + 35, panelY + 77, (guardian.exp * 70) / guardian.expToNext, 1);
        graphics.drawImage(AssetCache.statLabel4, panelX + 38, panelY + 81, 20);
        BaseCanvas.drawNumberAt(graphics, guardian.expToNext, panelX + 102, panelY + 81, 8);
    }
}
