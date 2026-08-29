package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ay */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ay.class */
/**
 * Items tab (tab 1) of {@link CharacterMenu}: the hero's carried bag
 * ({@link Hero#bag}, 30 slots). OK on a slot offers the appropriate action via a
 * popup &mdash; equip for gear the class can wear, use for consumables &mdash; or
 * a "cannot" message; drop is offered otherwise. The popup result then equips,
 * uses, or drops the selected item.
 *
 * <p>Unlike {@link ShopMenu} (which reads the shop stock and buys/sells), this
 * tab reads and mutates {@link Hero#bag} directly, so it is the real carried
 * inventory.
 */
public final class ItemsTab extends Menu {
    public ItemsTab(Menu parentMenu) {
        super(parentMenu, (byte) 30);
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
        Item item = GameState.hero().bag.get((int) ((Menu) this).cursorIndex);
        if (item == null) {
            return true;
        }
        if (!(item instanceof Equipment)) {
            if (item.isUsable()) {
                showPopup((byte) 5, (byte) 2, new Object[]{CharacterMenu.text.get(13), CharacterMenu.text.get(10)});
                return true;
            }
            if (item.isQuestItem()) {
                showMessage(new Object[]{CharacterMenu.text.get(14)});
                return true;
            }
            showPopup((byte) 6, (byte) 2, new Object[]{CharacterMenu.text.get(12)});
            return true;
        }
        Equipment equipment = (Equipment) item;
        if (!equipment.identified) {
            showPopup((byte) 6, (byte) 2, new Object[]{CharacterMenu.text.get(12)});
            return true;
        }
        if (!(equipment instanceof Weapon)) {
            if (equipment.type != 3 || GameState.classId == 8) {
                showPopup((byte) 4, (byte) 2, new Object[]{CharacterMenu.text.get(11), CharacterMenu.text.get(10)});
                return true;
            }
            showPopup((byte) 6, (byte) 2, new Object[]{CharacterMenu.text.get(12)});
            return true;
        }
        if ((GameState.classId == 6 && equipment.type == 0) || ((GameState.classId == 7 && equipment.type == 2) || (GameState.classId == 8 && equipment.type == 1))) {
            showPopup((byte) 4, (byte) 2, new Object[]{CharacterMenu.text.get(11), CharacterMenu.text.get(10)});
            return true;
        }
        Object[] messageLines = new Object[2];
        if (equipment.type == 0) {
            messageLines[0] = CharacterMenu.text.get(8);
        } else if (equipment.type == 2) {
            messageLines[0] = CharacterMenu.text.get(9);
        } else if (equipment.type == 1) {
            messageLines[0] = CharacterMenu.text.get(50);
        }
        messageLines[1] = CharacterMenu.text.get(12);
        showPopup((byte) 6, (byte) 2, messageLines);
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        Hero hero = GameState.hero();
        ItemBag bag = hero.bag;
        if (tag == 4 && result == 0) {
            switch (((Equipment) bag.get((int) ((Menu) this).cursorIndex)).type) {
                case 0:
                case 1:
                case 2:
                    hero.equipItem(((Menu) this).cursorIndex, (byte) 0);
                    break;
                case 3:
                    hero.equipItem(((Menu) this).cursorIndex, (byte) 1);
                    break;
                case 4:
                    hero.equipItem(((Menu) this).cursorIndex, (byte) 4);
                    break;
                case 5:
                    hero.equipItem(((Menu) this).cursorIndex, (byte) 2);
                    break;
                case 6:
                    hero.equipItem(((Menu) this).cursorIndex, (byte) 3);
                    break;
            }
            return;
        }
        if (tag == 5 && result == 0) {
            hero.useItem(bag.get((int) ((Menu) this).cursorIndex));
            return;
        }
        if ((tag == 4 && result == 1) || (tag == 5 && result == 1)) {
            showPopup((byte) 6, (byte) 2, new Object[]{CharacterMenu.text.get(12)});
        } else if (tag == 6 && result == 0) {
            bag.removeFromSlot(((Menu) this).cursorIndex, (byte) 1);
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int panelY = y + 15;
        ItemBag bag = GameState.hero().bag;
        BaseCanvas.drawLabelBox(graphics, AssetCache.commonText.get(2), panelX + 5, panelY);
        drawListPage(graphics, panelX, panelY, true);
        for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
            Item item = bag.get(slot);
            if (item != null) {
                Menu.drawItemIcon(graphics, panelX + 13, panelY + 18 + (23 * (slot % 5)), item, true);
            }
        }
        Item selectedItem = bag.get((int) ((Menu) this).cursorIndex);
        if (selectedItem != null) {
            Menu.drawItemInfo(graphics, panelX + 33, panelY + 14, selectedItem);
        } else {
            graphics.setColor(16777215);
            FontManager.drawChars(graphics, panelX + 33, panelY + 14, CharacterMenu.text.get(15), 1);
        }
    }
}
