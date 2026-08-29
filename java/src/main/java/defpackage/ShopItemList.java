package defpackage;

import java.util.Vector;
import javax.microedition.lcdui.Graphics;

/* renamed from: v */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:v.class */
/**
 * The scrollable stock list for one shop {@link #category} tab, pushed by
 * {@link ShopMenu} whenever its category cursor moves. It shows the category's
 * purchasable {@link #items}; OK opens a buy {@link BuySellDialog}, and the
 * {@code #} key switches to the sell tab ({@link SellList}) over the hero's bag.
 * For equipment categories it flags each listed piece with a coin/box marker
 * showing whether its value beats the gear the hero currently has equipped.
 */
public final class ShopItemList extends Menu {
    /* renamed from: a */
    /** The purchasable items in this category tab. */
    private Item[] items;
    /* renamed from: c */
    /** Shop category id (0 misc, 1 weapon, 2 armor, 3-5 accessory slots). */
    public byte category;

    public ShopItemList(Menu parent, Vector stock, byte category) {
        super(parent, (byte) stock.size());
        this.items = new Item[stock.size()];
        for (int i = 0; i < this.items.length; i++) {
            this.items[i] = (Item) stock.elementAt(i);
        }
        this.category = category;
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
        if (keyCode == 53 || action == 8) {
            ((Menu) this).child = new BuySellDialog(this, this.items[((Menu) this).cursorIndex], true);
            return true;
        }
        if (keyCode != 35) {
            return false;
        }
        byte[] occupiedSlots = GameState.hero().bag.occupiedSlots();
        if (occupiedSlots.length > 0) {
            ((Menu) this).child = new SellList(this, occupiedSlots);
            return true;
        }
        showPopup((byte) 1, (byte) 0, new Object[]{ShopMenu.text.get(16), ShopMenu.text.get(17)});
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        FontManager.drawSoftKeys(graphics, FontManager.labelSelect, FontManager.labelBack);
        int x = originX + 2;
        int y = originY + 15;
        drawListPage(graphics, x, y, true);
        short equippedValue = -1;
        Hero hero = GameState.hero();
        switch (this.category) {
            case 1:
                if (hero.getWeapon() != null) {
                    equippedValue = ((Equipment) hero.getWeapon()).value;
                }
                break;
            case 2:
                if (hero.getArmor() != null) {
                    equippedValue = ((Equipment) hero.getArmor()).value;
                }
                break;
            case 3:
                if (hero.getAccessory1() != null) {
                    equippedValue = hero.getAccessory1().value;
                }
                break;
            case 4:
                if (hero.getAccessory2() != null) {
                    equippedValue = hero.getAccessory2().value;
                }
                break;
            case 5:
                if (hero.getAccessory3() != null) {
                    equippedValue = hero.getAccessory3().value;
                }
                break;
        }
        for (int index = pageFirstIndex(); index <= pageLastIndex(); index++) {
            Item item = this.items[index];
            if (item != null) {
                Menu.drawItemIcon(graphics, x + 13, y + 18 + (23 * (index % 5)), item, false);
                if (this.category != 0) {
                    short listedValue = ((Equipment) this.items[((Menu) this).cursorIndex]).value;
                    if (equippedValue > listedValue) {
                        graphics.drawImage(AssetCache.shopCoinIcon, x + 20, y + 18 + ((((Menu) this).cursorIndex % 5) * 23), 33);
                    } else if (equippedValue < listedValue) {
                        graphics.drawImage(AssetCache.shopSelectBox, x + 20, y + 18 + ((((Menu) this).cursorIndex % 5) * 23), 33);
                    }
                }
            }
        }
        Item selectedItem = this.items[((Menu) this).cursorIndex];
        if (selectedItem != null) {
            Menu.drawItemInfo(graphics, x + 33, y + 14, selectedItem);
        }
        graphics.drawImage(AssetCache.shopSellIcon, (ShopMenu.panelX + 155) - 38, (ShopMenu.panelY + 170) - 22, 20);
    }
}
