package defpackage;

import java.io.IOException;
import java.util.Vector;
import javax.microedition.lcdui.Graphics;

/* renamed from: bp */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bp.class */
/**
 * The merchant shop screen (six category tabs). Reached from the world via event
 * op 11/0 ({@link GameState#requestState}, which does {@code setScreen(6)} and
 * {@link #loadStrings()}). The purchasable stock is the shop item table decoded
 * by {@link Item#buildShopStock()} (from {@link AssetCache#loadShopItemData()}), grouped into six
 * category vectors ({@link #shopStock}); the selected category is shown by the
 * child {@link ShopItemList}, whose OK opens a buy {@link BuySellDialog} and
 * whose {@code #} key opens a {@link SellList} over the hero's bag.
 *
 * <p><b>renamed from InventoryMenu:</b> earlier slices labelled {@code bp} the
 * player's inventory, but the bytecode reads the shop stock (not {@code Hero.bag})
 * and its children buy/sell &mdash; the real carried bag is {@link ItemsTab}.
 */
public final class ShopMenu extends Menu {
    /* renamed from: a */
    /** Purchasable stock grouped into six category vectors (from {@link Item#buildShopStock()}). */
    private Vector[] shopStock;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Centered panel origin X. */
    public static int panelX;
    /** Centered panel origin Y. */
    public static int panelY;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** {@code /sgui/shop} category/label strings. */
    public static TextTable text;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Lazily-created singleton instance. */
    private static ShopMenu singleton;

    /* renamed from: a */
    /** Returns (creating on first use) the shop singleton. */
    public static final ShopMenu instance() {
        if (singleton == null) {
            singleton = new ShopMenu();
            panelX = BaseCanvas.halfW - 77;
            panelY = BaseCanvas.halfH - 85;
        }
        return singleton;
    }

    private ShopMenu() {
        super(null, (byte) 6);
        this.shopStock = Item.buildShopStock();
        ((Menu) this).child = new ShopItemList(this, this.shopStock[((Menu) this).cursorIndex], ((Menu) this).cursorIndex);
    }

    /* renamed from: d */
    /** Loads the {@code /sgui/shop} label table. */
    public final void loadStrings() {
        try {
            text = new TextTable("/sgui/shop");
        } catch (IOException e) {
            e.printStackTrace();
        }
    }

    /* renamed from: e */
    /** Tears the shop down and returns to the world screen. */
    private void closeShop() {
        singleton = null;
        text = null;
        this.shopStock = null;
        ((Menu) this).child = null;
        GameState.setScreen(2);
        GameLoop.gameScreen.markRedraw();
        System.gc();
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) {
            return true;
        }
        if (keyCode != -8) {
            return false;
        }
        closeShop();
        return true;
    }

    @Override // defpackage.cb
    public final void moveCursor(byte direction) {
        super.moveCursor(direction);
        ((Menu) this).child = new ShopItemList(this, this.shopStock[((Menu) this).cursorIndex], ((Menu) this).cursorIndex);
    }

    /* renamed from: a */
    /** Draws the whole shop screen tree at the centered panel origin. */
    public final void draw(Graphics graphics) {
        render(graphics, panelX, panelY);
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        FontManager.clearScreen(graphics);
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
        graphics.setColor(4136767);
        graphics.fillRect(x, y, 155, 170);
        Menu.drawInsetPanel(graphics, x + 2, y + 15, 151, 155);
        graphics.setColor(16768959);
        graphics.fillRect(x + 11 + (((Menu) this).cursorIndex * 16) + 1, y, 14, 1);
        graphics.fillRect(x + 11 + (((Menu) this).cursorIndex * 16), y + 1, 1, 16);
        graphics.setColor(12558207);
        graphics.fillRect(x + 11 + (((Menu) this).cursorIndex * 16) + 15, y + 1, 1, 15);
        graphics.setColor(14663551);
        graphics.fillRect(x + 11 + (((Menu) this).cursorIndex * 16) + 1, y + 1, 14, 16);
        for (int category = 0; category < 6; category++) {
            graphics.drawImage(AssetCache.shopCategoryIcons[category], x + 13 + (category * 16), y + 1, 20);
        }
        BaseCanvas.drawLabelBox(graphics, text.get(((Menu) this).cursorIndex + 1), x + 3, y + 15);
        graphics.drawImage(AssetCache.slotFrame, x + 4, y + 4, 20);
        graphics.drawImage(AssetCache.cursorArrow, x + 109, y + 4, 20);
    }
}
