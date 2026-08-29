package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ab */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ab.class */
/**
 * The confirm-and-quantity dialog for a single shop transaction, pushed by
 * {@link ShopItemList} (buy) or {@link SellList} (sell). {@link #buying} selects
 * the mode; the left/right cursor adjusts {@link #quantity} (1..99 when buying a
 * stackable, else up to the owned count), and OK opens a yes/no popup. Buying
 * checks gold and bag space and, for a class-mismatched item, warns first;
 * selling refunds one fifth of the item's price per unit. The panel shows the
 * item icon, the hero's gold and the running total.
 */
public final class BuySellDialog extends Menu {
    /* renamed from: a */
    /** The item being bought or sold. */
    private Item item;
    /* renamed from: c */
    /** Selected transaction quantity. */
    private byte quantity;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** {@code true} = buying (shop stock), {@code false} = selling (hero's bag). */
    private boolean buying;

    public BuySellDialog(Menu parent, Item item, boolean buying) {
        super(parent, (byte) 0);
        this.item = item;
        this.quantity = (byte) 1;
        this.buying = buying;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.close();
            return true;
        }
        if (!this.buying) {
            showPopup((byte) 2, (byte) 2, new Object[]{ShopMenu.text.get(23)});
            return true;
        }
        Object[] lines = {ShopMenu.text.get(7)};
        if ((this.item.type == 0 && GameState.classId != 6) || ((this.item.type == 2 && GameState.classId != 7) || ((this.item.type == 1 && GameState.classId != 8) || (this.item.type == 3 && GameState.classId != 8)))) {
            lines = new Object[]{ShopMenu.text.get(26), ShopMenu.text.get(7)};
        }
        showPopup((byte) 2, (byte) 2, lines);
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        Hero hero = GameState.hero();
        if (tag != 2 || result != 0) {
            if (tag == 1) {
                ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
                return;
            }
            return;
        }
        if (!this.buying) {
            hero.bag.decrementItem(this.item, this.quantity);
            hero.bag.gold += (this.item.price * this.quantity) / 5;
            showMessage(new Object[]{this.item.name, ShopMenu.text.get(24)});
            return;
        }
        Item bought = Item.create(this.item.type, this.item.subId, true, false);
        if (bought instanceof Equipment) {
            ((Equipment) bought).identified = true;
        }
        int totalCost = bought.price * this.quantity;
        if (hero.bag.gold < totalCost) {
            showMessage(new Object[]{ShopMenu.text.get(8)});
        } else {
            if (!hero.bag.add(bought, (int) this.quantity)) {
                showMessage(new Object[]{ShopMenu.text.get(9), ShopMenu.text.get(10)});
                return;
            }
            hero.bag.gold -= totalCost;
            showMessage(new Object[]{ShopMenu.text.get(11), ShopMenu.text.get(12)});
        }
    }

    @Override // defpackage.cb
    public final void moveCursor(byte direction) {
        if (!(this.buying && Item.STACKABLE[this.item.type]) && (this.buying || this.item.quantity <= 1)) {
            return;
        }
        if (direction == 4) {
            if (this.quantity < (this.buying ? (byte) 99 : this.item.quantity)) {
                this.quantity = (byte) (this.quantity + 1);
                return;
            }
        }
        if (direction == 4 && this.buying && this.quantity == 99) {
            this.quantity = (byte) 1;
            return;
        }
        if (direction == 3 && this.quantity > 1) {
            this.quantity = (byte) (this.quantity - 1);
        } else if (direction == 3 && this.buying && this.quantity == 1) {
            this.quantity = (byte) 99;
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        FontManager.clearScreen(graphics);
        if (this.buying) {
            FontManager.drawSoftKeys(graphics, FontManager.labelBuy, FontManager.labelBack);
        } else {
            FontManager.drawSoftKeys(graphics, FontManager.labelSell, FontManager.labelBack);
        }
        int x = originX + 3;
        int y = originY + 20;
        Menu.drawPanelFrame(graphics, x, y, 149, 29);
        Menu.fillPanelInterior(graphics, x, y, 149, 29);
        Menu.drawPanelFrame(graphics, x, y + 31, 149, 67);
        Menu.fillPanelInterior(graphics, x, y + 31, 149, 67);
        int labelX = x + 15;
        graphics.setColor(14663551);
        FontManager.drawChars(graphics, labelX + 8, y + 7, ShopMenu.text.get(13), 1);
        Menu.drawGold(graphics, labelX + 102, y + 11, GameState.hero().bag.gold);
        graphics.setColor(16777215);
        if (!(this.buying && Item.STACKABLE[this.item.type]) && (this.buying || this.item.quantity <= 1)) {
            FontManager.drawChars(graphics, labelX + 6, y + 38, ShopMenu.text.get(15), 1);
        } else {
            if (this.buying) {
                FontManager.drawChars(graphics, labelX + 6, y + 38, ShopMenu.text.get(14), 1);
            } else {
                FontManager.drawChars(graphics, labelX + 6, y + 38, ShopMenu.text.get(25), 1);
            }
            graphics.drawImage(AssetCache.slotFrame, labelX + 32, y + 65, 20);
            BaseCanvas.drawNumberAt(graphics, this.quantity, labelX + 68, y + 65, 8);
            graphics.drawImage(AssetCache.cursorArrow, labelX + 77, y + 65, 20);
        }
        graphics.drawImage(AssetCache.itemIcons[this.item.type], labelX + 45, y + 57, 20);
        if (this.buying) {
            Menu.drawGold(graphics, labelX + 77, y + 85, this.quantity * this.item.price);
        } else {
            Menu.drawGold(graphics, labelX + 77, y + 85, (this.quantity * this.item.price) / 5);
        }
    }
}
