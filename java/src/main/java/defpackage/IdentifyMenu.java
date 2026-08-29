package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ch */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ch.class */
/**
 * The blacksmith's identify screen, pushed by {@link BlacksmithMenu}. The player
 * picks an unidentified {@link #equipment} piece; confirming spends 100 gold and
 * sets {@link Equipment#identified}, revealing the piece's real stats. Reads its
 * label strings from {@link BlacksmithMenu#text}.
 */
public final class IdentifyMenu extends Menu {
    /* renamed from: a */
    /** The equipment piece selected for identification. */
    private Equipment equipment;

    public IdentifyMenu(Menu parent) {
        super(parent, (byte) 2);
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
            byte[] slots = hero.serializeItems(false, (byte) -1);
            if (slots.length == 0) {
                showMessage(new Object[]{StringTable.instance.get(3935).toCharArray()});
                return true;
            }
            ((Menu) this).child = new ItemPickerList(this, slots, ((Menu) this).cursorIndex, BlacksmithMenu.text.get(3));
            return true;
        }
        if (((Menu) this).cursorIndex != 1) {
            return true;
        }
        if (this.equipment == null) {
            showMessage(new Object[]{BlacksmithMenu.text.get(3)});
            return true;
        }
        if (hero.bag.gold < 100) {
            showMessage(new Object[]{BlacksmithMenu.text.get(8)});
            return true;
        }
        showPopup((byte) 2, (byte) 2, new Object[]{BlacksmithMenu.text.get(19)});
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
            Hero hero = GameState.hero();
            this.equipment.identified = true;
            hero.bag.gold -= 100;
            ((Menu) this).child = new ItemPickerList(this, new byte[]{hero.slotOf((Item) this.equipment)}, (byte) 10, BlacksmithMenu.text.get(20));
            this.equipment = null;
            return;
        }
        if ((previousChild instanceof ItemPickerList) && tag == 0) {
            Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
            Debug.assertTrue(picked instanceof Equipment);
            Equipment equip = (Equipment) picked;
            if (equip.identified) {
                showMessage(new Object[]{BlacksmithMenu.text.get(21)});
            } else {
                this.equipment = equip;
            }
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        Hero hero = GameState.hero();
        graphics.setColor(4136767);
        graphics.fillRect(originX, originY, 155, 170);
        Menu.drawInsetPanel(graphics, originX + 2, originY + 4, 151, 162);
        BaseCanvas.drawLabelBox(graphics, BlacksmithMenu.text.get(13), originX + 3, originY - 2);
        Menu.fillOutlinedRect(graphics, originX + 3, originY + 7, 149, 17, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, originX + 6, originY + 10, BlacksmithMenu.text.get(23), 1);
        Menu.drawQuickSlotRow(graphics, originX + 4, originY + 30, this.equipment, (byte) 1, BlacksmithMenu.text.get(15), ((Menu) this).cursorIndex == 0);
        Menu.drawGold(graphics, (originX + 155) - 10, originY + 75, hero.bag.gold);
        Menu.fillOutlinedRect(graphics, originX + 4, originY + 83, 147, 20, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, originX + 8, originY + 88, BlacksmithMenu.text.get(24), 1);
        Menu.drawGold(graphics, (originX + 155) - 8, originY + 89, 100);
        int buttonWidth = FontManager.percentOf(155, 80);
        Menu.drawButton(graphics, originX + ((155 - buttonWidth) >> 1), originY + 138, buttonWidth, BlacksmithMenu.text.get(25), ((Menu) this).cursorIndex == 1);
    }
}
