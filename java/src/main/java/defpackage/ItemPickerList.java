package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: m */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:m.class */
/**
 * A scrollable item-slot picker pushed by the equip/craft/blacksmith menus. It
 * lists the items named by {@link #slots} (a slot-code array) under a
 * {@link #title} header, and on OK reports the chosen slot back to its parent via
 * {@link Menu#onPopupResult}, passing {@link #resultTag} as the callback tag so
 * the parent knows which pick this was. Each slot code is resolved to an
 * {@link Item} the same way everywhere: {@code >= 100} is an equipped slot
 * ({@code code - 100}), {@code < 0} is a quick-slot ({@code -code - 1}), and the
 * rest are bag indices. {@link SellList} subclasses this for the shop's sell tab.
 */
public class ItemPickerList extends Menu {
    /* renamed from: h */
    /** Slot codes to list (equipped {@code >=100}, quick-slot {@code <0}, else bag index). */
    public byte[] slots;
    /* renamed from: c */
    /** Callback tag echoed to the parent's {@link Menu#onPopupResult} on OK/cancel. */
    public byte resultTag;
    /* renamed from: a */
    /** Header caption drawn above the list. */
    private char[] title;

    public ItemPickerList(Menu parent, byte[] slots, byte resultTag, char[] title) {
        super(parent, (byte) slots.length);
        this.slots = slots;
        this.resultTag = resultTag;
        this.title = title;
    }

    @Override // defpackage.cb
    public boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode == 53 || action == 8) {
            ((Menu) this).parent.onPopupResult(this.resultTag, this.slots[((Menu) this).cursorIndex]);
            return true;
        }
        if (keyCode != -8) {
            return true;
        }
        ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        return true;
    }

    @Override // defpackage.cb
    public void paint(Graphics graphics, int originX, int originY) {
        Hero hero = GameState.hero();
        int x = originX + 2;
        int y = (originY - 3) + 14;
        Menu.drawInsetPanel(graphics, x, y - 14, 151, 170);
        boolean multiPage = pageCount() > 1;
        Menu.fillOutlinedRect(graphics, x + 3, (y - 13) + (multiPage ? 0 : 3), 145, 14, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, x + 6, (y - 10) + (multiPage ? 0 : 3), this.title, 1);
        drawListPage(graphics, x, y, multiPage);
        for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
            Item item = this.slots[slot] >= 100 ? hero.getEquip(this.slots[slot] - 100) : this.slots[slot] < 0 ? hero.quickItems.get((-this.slots[slot]) - 1) : hero.bag.get((int) this.slots[slot]);
            if (item != null) {
                Menu.drawItemIcon(graphics, x + 13, y + 18 + (23 * (slot % 5)), item, true);
            }
        }
        Item selectedItem = this.slots[((Menu) this).cursorIndex] >= 100 ? hero.getEquip(this.slots[((Menu) this).cursorIndex] - 100) : this.slots[((Menu) this).cursorIndex] < 0 ? hero.quickItems.get((-this.slots[((Menu) this).cursorIndex]) - 1) : hero.bag.get((int) this.slots[((Menu) this).cursorIndex]);
        if (selectedItem != null) {
            Menu.drawItemInfo(graphics, x + 33, y + 14, selectedItem);
        }
    }
}
