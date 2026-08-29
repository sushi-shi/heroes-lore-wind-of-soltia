package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: k */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:k.class */
/**
 * Item-combine (crafting) sub-screen of {@link RefineMenu}, opened from its
 * "combine" entry. The player fills up to three {@link #craftSlots} by picking
 * quick-use items through an {@link ItemPickerList}; pressing the fourth
 * (button) row with at least two slots filled and 500 gold in hand pops a
 * {@link CostConfirmDialog}, and confirming there routes back through
 * {@link #onPopupResult} to consume the ingredients and {@link Item#craft} them
 * into a new item (added to the bag, or refunded on failure).
 */
public final class CombineMenu extends Menu {
    /* renamed from: a */
    /** The up-to-three ingredient items staged for crafting (slots 0-2). */
    private Item[] craftSlots;

    public CombineMenu(Menu parentMenu) {
        super(parentMenu, (byte) 4);
        this.craftSlots = new Item[3];
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if ((((Menu) this).child != null && ((Menu) this).child.handleKey(action, keyCode)) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        Hero hero = GameState.hero();
        if (((Menu) this).cursorIndex < 3) {
            byte[] usableSlots = hero.bag.quickUsableSlots();
            if (usableSlots.length < 1) {
                showMessage(new Object[]{RefineMenu.text.get(20)});
                return true;
            }
            ((Menu) this).child = new ItemPickerList(this, usableSlots, ((Menu) this).cursorIndex, RefineMenu.text.get(21));
            return true;
        }
        byte filled = 0;
        Object[] names = new Object[3];
        if (this.craftSlots[0] != null) {
            filled = 1;
            names[0] = this.craftSlots[0].name;
        }
        if (this.craftSlots[1] != null) {
            byte slotIndex = filled;
            filled = (byte) (filled + 1);
            names[slotIndex] = this.craftSlots[1].name;
        }
        if (this.craftSlots[2] != null) {
            byte slotIndex = filled;
            filled = (byte) (filled + 1);
            names[slotIndex] = this.craftSlots[2].name;
        }
        if (filled < 2) {
            showMessage(new Object[]{RefineMenu.text.get(22), RefineMenu.text.get(23)});
            return true;
        }
        if (500 > hero.bag.gold) {
            showMessage(new Object[]{RefineMenu.text.get(24)});
            return true;
        }
        ((Menu) this).child = new CostConfirmDialog(this, RefineMenu.text.get(25), names, RefineMenu.text.get(26), 500, (byte) 20);
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu child = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if (!(child instanceof PopupMenu) || tag != 2 || result != 0) {
            if (!(child instanceof ItemPickerList) || (tag != 0 && tag != 1 && tag != 2)) {
                if ((child instanceof CostConfirmDialog) && tag == 20) {
                    showPopup((byte) 2, (byte) 2, new Object[]{RefineMenu.text.get(32)});
                    return;
                }
                return;
            }
            Hero hero = GameState.hero();
            Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
            Debug.assertTrue(Item.QUICK_USABLE[picked.type]);
            int duplicateCount = 0;
            for (int slot = 0; slot < 3; slot++) {
                if (tag != slot && this.craftSlots[slot] != null && this.craftSlots[slot].type == picked.type && this.craftSlots[slot].subId == picked.subId) {
                    duplicateCount++;
                }
            }
            if (hero.bag.totalQuantity(picked.type, picked.subId) <= duplicateCount) {
                showMessage(new Object[]{RefineMenu.text.get(31)});
                return;
            } else {
                this.craftSlots[tag] = picked;
                return;
            }
        }
        Hero hero2 = GameState.hero();
        Item crafted = Item.craft(this.craftSlots[0], this.craftSlots[1], this.craftSlots[2]);
        if (crafted == null) {
            if (this.craftSlots[0] != null) {
                hero2.bag.decrementItem(this.craftSlots[0], (byte) 1);
            }
            if (this.craftSlots[1] != null) {
                hero2.bag.decrementItem(this.craftSlots[1], (byte) 1);
            }
            if (this.craftSlots[2] != null) {
                hero2.bag.decrementItem(this.craftSlots[2], (byte) 1);
            }
            this.craftSlots[0] = null;
            this.craftSlots[1] = null;
            this.craftSlots[2] = null;
            showMessage(new Object[]{RefineMenu.text.get(30)});
            return;
        }
        if (this.craftSlots[0] != null) {
            hero2.bag.decrementItem(this.craftSlots[0], (byte) 1);
        }
        if (this.craftSlots[1] != null) {
            hero2.bag.decrementItem(this.craftSlots[1], (byte) 1);
        }
        if (this.craftSlots[2] != null) {
            hero2.bag.decrementItem(this.craftSlots[2], (byte) 1);
        }
        if (hero2.bag.add(crafted, 1)) {
            ((Menu) this).child = new ItemPickerList(this, new byte[]{hero2.bag.findSlot(crafted.type, crafted.subId)}, (byte) 10, RefineMenu.text.get(27));
            this.craftSlots[0] = null;
            this.craftSlots[1] = null;
            this.craftSlots[2] = null;
            return;
        }
        if (this.craftSlots[0] != null) {
            hero2.bag.add(this.craftSlots[0], 1);
        }
        if (this.craftSlots[1] != null) {
            hero2.bag.add(this.craftSlots[1], 1);
        }
        if (this.craftSlots[2] != null) {
            hero2.bag.add(this.craftSlots[2], 1);
        }
        showMessage(new Object[]{RefineMenu.text.get(28), RefineMenu.text.get(29)});
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        graphics.setColor(4136767);
        graphics.fillRect(x, y, 155, 170);
        Menu.drawInsetPanel(graphics, x + 2, y + 4, 151, 162);
        BaseCanvas.drawLabelBox(graphics, RefineMenu.text.get(14), x + 3, y - 2);
        Menu.drawQuickSlotRow(graphics, x + 4, y + 9, this.craftSlots[0], (byte) 1, RefineMenu.text.get(33), ((Menu) this).cursorIndex == 0);
        Menu.drawQuickSlotRow(graphics, x + 4, y + 9 + 36, this.craftSlots[1], (byte) 2, RefineMenu.text.get(33), ((Menu) this).cursorIndex == 1);
        Menu.drawQuickSlotRow(graphics, x + 4, y + 9 + 72, this.craftSlots[2], (byte) 3, RefineMenu.text.get(33), ((Menu) this).cursorIndex == 2);
        int buttonWidth = FontManager.percentOf(155, 80);
        Menu.drawButton(graphics, x + ((155 - buttonWidth) >> 1), y + 138, buttonWidth, RefineMenu.text.get(25), ((Menu) this).cursorIndex == 3);
    }
}
