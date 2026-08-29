package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bz */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bz.class */
/**
 * Equipment tab (tab 2) of {@link CharacterMenu}: the five equipped slots
 * (weapon / armour / two accessories / shield, mapped per class). OK on a slot
 * resolves the matching item category, and if the bag holds any candidates opens
 * an {@link ItemPickerList} to swap gear; the chosen item is equipped on the
 * popup callback (unidentified gear is rejected).
 */
public final class EquipTab extends Menu {
    public EquipTab(Menu parentMenu) {
        super(parentMenu, (byte) 5);
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (moveCursorVerticalNoWrap(action, keyCode)) {
            ((Menu) this).parent.needsRepaint = true;
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        byte category = -1;
        switch (((Menu) this).cursorIndex) {
            case 0:
                switch (GameState.classId) {
                    case 6:
                        category = 0;
                        break;
                    case 7:
                        category = 2;
                        break;
                    case 8:
                        category = 1;
                        break;
                }
                break;
            case 1:
                if (GameState.classId == 8) {
                    category = 3;
                }
                break;
            case 2:
                category = 5;
                break;
            case 3:
                category = 6;
                break;
            case 4:
                category = 4;
                break;
        }
        if (category == -1) {
            showMessage(new Object[]{StringTable.instance.get(3937).toCharArray()});
            return true;
        }
        byte[] candidates = GameState.hero().bag.slotsOfType(category);
        if (candidates.length > 0) {
            ((Menu) this).child = new ItemPickerList(this, candidates, ((Menu) this).cursorIndex, CharacterMenu.text.get(16));
            return true;
        }
        showMessage(new Object[]{StringTable.instance.get(3937).toCharArray()});
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if (!(previousChild instanceof ItemPickerList) || tag == -1) {
            return;
        }
        Hero hero = GameState.hero();
        if (((Equipment) hero.bag.get((int) result)).identified) {
            hero.equipItem(result, tag);
        } else {
            showMessage(new Object[]{CharacterMenu.text.get(18), CharacterMenu.text.get(19)});
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int panelY = y + 15;
        Hero hero = GameState.hero();
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(20), panelX + 5, panelY);
        drawListPage(graphics, panelX, panelY, false);
        for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
            Item item = hero.getEquip(slot);
            if (item != null) {
                Menu.drawItemIcon(graphics, panelX + 13, panelY + 18 + (23 * (slot % 5)), item, false);
            } else {
                graphics.drawImage(AssetCache.equipSlotIcons[slot], panelX + 13, panelY + 19 + (23 * (slot % 5)), 3);
            }
        }
        Item selectedItem = hero.getEquip((int) ((Menu) this).cursorIndex);
        if (selectedItem != null) {
            Menu.drawItemInfo(graphics, panelX + 33, panelY + 14, selectedItem);
            return;
        }
        graphics.setColor(16777215);
        if (((Menu) this).cursorIndex != 1 || GameState.classId == 8) {
            FontManager.drawChars(graphics, panelX + 33, panelY + 14, CharacterMenu.text.get(21), 1);
        } else {
            FontManager.drawChars(graphics, panelX + 30, panelY + 14, CharacterMenu.text.get(49), 1);
        }
    }
}
