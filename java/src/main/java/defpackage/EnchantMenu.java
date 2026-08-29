package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ap */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ap.class */
/**
 * The refinery's armor-enchant screen, pushed by {@link RefineMenu}. The player
 * picks an identified, un-enchanted {@link #armor} piece and an enchant-scroll
 * {@link #material} (item type 17); confirming spends 500 gold, consumes the
 * scroll and stamps the scroll's element ({@link Item#subId}) onto the armor's
 * {@link Armor#attribute}. Reads its label strings from {@link RefineMenu#text}.
 */
public final class EnchantMenu extends Menu {
    /* renamed from: a */
    /** The armor piece being enchanted (must be identified and un-enchanted). */
    private Armor armor;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** The enchant scroll (item type 17) whose element is applied. */
    private Item material;

    public EnchantMenu(Menu parent) {
        super(parent, (byte) 3);
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        Hero hero = GameState.hero();
        if (((Menu) this).cursorIndex == 0) {
            byte[] armorSlots = hero.serializeItems(true, (byte) 1);
            Debug.assertTrue(armorSlots.length > 0);
            ((Menu) this).child = new ItemPickerList(this, armorSlots, ((Menu) this).cursorIndex, RefineMenu.text.get(3));
            return true;
        }
        if (((Menu) this).cursorIndex == 1) {
            byte[] scrollSlots = hero.bag.slotsOfType((byte) 17);
            if (scrollSlots.length > 0) {
                ((Menu) this).child = new ItemPickerList(this, scrollSlots, ((Menu) this).cursorIndex, RefineMenu.text.get(4));
                return true;
            }
            showMessage(new Object[]{RefineMenu.text.get(5)});
            return true;
        }
        if (((Menu) this).cursorIndex != 2) {
            return true;
        }
        if (this.armor == null) {
            showMessage(new Object[]{RefineMenu.text.get(6)});
            return true;
        }
        if (this.material == null) {
            showMessage(new Object[]{RefineMenu.text.get(7)});
            return true;
        }
        if (hero.bag.gold < 500) {
            showMessage(new Object[]{RefineMenu.text.get(8)});
            return true;
        }
        showPopup((byte) 2, (byte) 2, new Object[]{RefineMenu.text.get(9)});
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
            Hero hero = GameState.hero();
            this.armor.attribute = this.material.subId;
            hero.bag.gold -= 500;
            hero.bag.decrementItem(this.material, (byte) 1);
            ((Menu) this).child = new ItemPickerList(this, new byte[]{hero.slotOf((Item) this.armor)}, (byte) 10, RefineMenu.text.get(10));
            this.armor = null;
            this.material = null;
            return;
        }
        if (previousChild instanceof ItemPickerList) {
            if (tag == 0 || tag == 1) {
                Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
                if (tag != 0) {
                    Debug.assertTrue(picked.type == 17);
                    this.material = picked;
                    return;
                }
                Debug.assertTrue(picked instanceof Armor);
                Armor pickedArmor = (Armor) picked;
                if (!((Equipment) pickedArmor).identified) {
                    showMessage(new Object[]{RefineMenu.text.get(11), RefineMenu.text.get(13)});
                } else if (pickedArmor.attribute != -1) {
                    showMessage(new Object[]{RefineMenu.text.get(12), RefineMenu.text.get(13)});
                } else {
                    this.armor = (Armor) picked;
                }
            }
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        graphics.setColor(4136767);
        graphics.fillRect(originX, originY, 155, 170);
        Menu.drawInsetPanel(graphics, originX + 2, originY + 4, 151, 162);
        BaseCanvas.drawLabelBox(graphics, RefineMenu.text.get(14), originX + 3, originY - 2);
        Menu.drawQuickSlotRow(graphics, originX + 4, originY + 9, this.armor, (byte) 1, RefineMenu.text.get(15), ((Menu) this).cursorIndex == 0);
        Menu.drawQuickSlotRow(graphics, originX + 4, originY + 9 + 36, this.material, (byte) 2, RefineMenu.text.get(16), ((Menu) this).cursorIndex == 1);
        Menu.fillOutlinedRect(graphics, originX + 4, originY + 9 + 72, 147, 31, 12558207);
        if (this.armor != null && this.material != null) {
            graphics.setColor(16777215);
            int textX = originX + 6;
            int valueX = textX + 2 + FontManager.drawChars(graphics, textX, originY + 9 + 72 + 4, Armor.attributeNames.get(this.material.subId), 1);
            FontManager.drawChars(graphics, valueX + 2 + FontManager.drawChars(graphics, valueX, originY + 9 + 72 + 4, RefineMenu.text.get(17), 1), originY + 9 + 72 + 4, RefineMenu.text.get(18), 1);
            Menu.drawGold(graphics, (originX + 155) - 10, originY + 9 + 72 + 5, 500);
            Menu.drawGold(graphics, (originX + 155) - 10, originY + 9 + 72 + 20, GameState.hero().bag.gold);
        }
        int buttonWidth = FontManager.percentOf(155, 80);
        Menu.drawButton(graphics, originX + ((155 - buttonWidth) >> 1), originY + 138, buttonWidth, RefineMenu.text.get(19), ((Menu) this).cursorIndex == 2);
    }
}
