package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bb */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bb.class */
/**
 * The shop's sell tab: an {@link ItemPickerList} over the hero's occupied bag
 * slots. Selecting a non-quest item opens a sell {@link BuySellDialog}; quest
 * items are refused with a popup. After a sale it rebuilds itself over the
 * remaining bag contents (or closes the shop-list parent when the bag empties).
 * It draws the shop "buy" icon so the player can toggle back to buying, and reads
 * its label strings from {@link ShopMenu#text}.
 */
public final class SellList extends ItemPickerList {
    public SellList(Menu parent, byte[] slots) {
        super(parent, slots, (byte) 0, ShopMenu.text.get(18));
    }

    @Override // defpackage.m, defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8 && keyCode != 35) {
                return true;
            }
            ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
            return true;
        }
        Item item = GameState.hero().bag.get((int) this.slots[((Menu) this).cursorIndex]);
        if (item.isQuestItem()) {
            showPopup((byte) 1, (byte) 0, new Object[]{ShopMenu.text.get(19), ShopMenu.text.get(20)});
            return true;
        }
        ((Menu) this).child = new BuySellDialog(this, item, false);
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if (previousChild instanceof BuySellDialog) {
            ((Menu) this).parent.close();
            byte[] occupiedSlots = GameState.hero().bag.occupiedSlots();
            if (occupiedSlots.length > 0) {
                ((Menu) this).parent.child = new SellList(((Menu) this).parent, occupiedSlots);
            } else {
                ((Menu) this).parent.showPopup((byte) 1, (byte) 0, new Object[]{ShopMenu.text.get(21), ShopMenu.text.get(22)});
            }
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        FontManager.clearScreen(graphics);
        FontManager.drawSoftKeys(graphics, FontManager.labelSelect, FontManager.labelBack);
        super.paint(graphics, originX, originY);
        graphics.drawImage(AssetCache.shopBuyIcon, (ShopMenu.panelX + 155) - 38, (ShopMenu.panelY + 170) - 22, 20);
    }
}
