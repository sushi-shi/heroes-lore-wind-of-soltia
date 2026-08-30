//! Transliterated from `java/src/main/java/defpackage/BuySellDialog.java`
//! (original `ab.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The confirm-and-quantity dialog for a single shop transaction (`BuySellDialog
//! extends Menu`), pushed by [`ShopItemList`](crate::shop_item_list) (buy) or
//! [`SellList`](crate::sell_list) (sell). [`buying`](BuySellDialogState::buying)
//! selects the mode; the left/right cursor adjusts
//! [`quantity`](BuySellDialogState::quantity) (1..99 buying a stackable, else up to
//! the owned count) via the overriding [`move_cursor`], and OK opens a yes/no popup.
//! Buying checks gold and bag space and, for a class-mismatched item, warns first;
//! selling refunds one fifth of the item's price per unit.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. The dialog's own logic (quantity adjust, the gold/bag
//! transaction over [`item_bag`](crate::item_bag), the class-mismatch warning, the
//! popup dispatch through the [`menu`](crate::menu) child stack and
//! [`ShopMenu.text`](crate::shop_menu::text_get)) is real. Only the render's
//! genuinely-unported hops are DEFERRED: `FontManager.clearScreen`, the
//! `FontManager.drawSoftKeys` custom shop labels (`labelBuy`/`labelSell`/`labelBack`,
//! not in the partial `FontManager`), `Menu.drawGold`/`BaseCanvas.drawNumberAt`, and
//! the `AssetCache.slotFrame`/`cursorArrow`/`itemIcons` art. The panel frames and the
//! `ShopMenu.text` captions (both ported) are drawn.
//!
//! `BuySellDialog`'s fields (`item`, `quantity`, `buying`) are all per-INSTANCE (the
//! `Menu` base fields likewise), so it contributes no
//! `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ab.<init>:(Lcb;Lad;Z)V =>
//! []` (constructor), `ab.a:(II)Z => []` (handleKey — pure branches),
//! `ab.a:(BB)V => [imul,isub,imul,idiv,iadd]` (onPopupResult — the sell refund
//! `(price*qty)/5` + `gold +=` and the buy `price*qty` + `gold -=`),
//! `ab.a:(B)V => [iadd,i2b,isub,i2b]` (moveCursor — `quantity +/- 1`),
//! `ab.a:(…Graphics;II)V => [iinc,iinc,iadd, …]` (paint — the ported panel geometry
//! prefix; the remaining adds live in the DEFERRED gold/number/icon draws).

use crate::font_manager;
use crate::game::Game;
use crate::item;
use crate::item_bag::{self, ItemRef};
use crate::menu::{self, MenuNode};
use crate::shop_menu;
use j2me_jvm::java_div;
use std::cell::RefCell;
use std::rc::Rc;

/// Java `ab` / `BuySellDialog` instance state (the `Menu` base + the dialog's own
/// per-instance fields).
#[derive(Debug, Default, Clone)]
pub struct BuySellDialogState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private Item item;` (obf `a`) — the item being bought or sold (a shared
    /// reference into the shop stock or the hero's bag). `None` == Java null.
    pub item: Option<ItemRef>,
    /// `private byte quantity;` (obf `c`) — selected transaction quantity.
    pub quantity: i8,
    /// `private boolean buying;` (obf `c`) — `true` = buying (shop stock), `false` =
    /// selling (hero's bag).
    pub buying: bool,
}

/// `public BuySellDialog(Menu parent, Item item, boolean buying)`
/// (`ab.<init>:(Lcb;Lad;Z)V => []`).
pub fn construct(g: &mut Game, item: ItemRef, buying: bool) {
    // super(parent, (byte) 0);   (parent is the pushing shop list → present)
    g.buy_sell_dialog.base = menu::construct(true, 0);
    // this.item = item;
    g.buy_sell_dialog.item = Some(item);
    // this.quantity = (byte) 1;
    g.buy_sell_dialog.quantity = 1;
    // this.buying = buying;
    g.buy_sell_dialog.buying = buying;
}

/// `public final boolean handleKey(int action, int keyCode)` (`ab.a:(II)Z => []`):
/// child forward + horizontal quantity nav; Back (`-8`) closes the parent; FIRE
/// (`53`/action 8) opens the yes/no confirm popup — selling asks once, buying warns
/// first on a class-mismatched item. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::BuySell, action, key_code)
        || menu::move_cursor_horizontal_node(g, MenuNode::BuySell, action, key_code)
    {
        return true;
    }
    // if (keyCode != 53 && action != 8) {
    if key_code != 53 && action != 8 {
        // if (keyCode != -8) return true;
        if key_code != -8 {
            return true;
        }
        // ((Menu) this).parent.close();
        let parent = menu::parent_of(g, MenuNode::BuySell)
            .expect("NullPointerException: BuySellDialog.parent");
        menu::close(g, parent);
        // return true;
        return true;
    }
    // if (!this.buying) { showPopup((byte) 2, (byte) 2, {ShopMenu.text.get(23)}); return true; }
    if !g.buy_sell_dialog.buying {
        let line = shop_menu::text_get(g, 23);
        menu::show_popup(g, MenuNode::BuySell, 2, 2, vec![line]);
        return true;
    }
    // Object[] lines = {ShopMenu.text.get(7)};
    let mut lines: Vec<Vec<u16>> = vec![shop_menu::text_get(g, 7)];
    // if ((item.type == 0 && classId != 6) || (item.type == 2 && classId != 7)
    //     || (item.type == 1 && classId != 8) || (item.type == 3 && classId != 8)) {
    //     lines = {ShopMenu.text.get(26), ShopMenu.text.get(7)};
    // }
    let item_type = g
        .buy_sell_dialog
        .item
        .as_ref()
        .expect("NullPointerException: BuySellDialog.item")
        .borrow()
        .r#type;
    // GameState.classId (`n` static field).
    let class_id = g.game_state.class_id;
    if (item_type == 0 && class_id != 6)
        || (item_type == 2 && class_id != 7)
        || (item_type == 1 && class_id != 8)
        || (item_type == 3 && class_id != 8)
    {
        lines = vec![shop_menu::text_get(g, 26), shop_menu::text_get(g, 7)];
    }
    // showPopup((byte) 2, (byte) 2, lines);
    menu::show_popup(g, MenuNode::BuySell, 2, 2, lines);
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)`
/// (`ab.a:(BB)V => [imul,isub,imul,idiv,iadd]`): runs the base dismiss (`super`),
/// then — on a confirmed transaction (tag 2, result 0) — performs the sell refund or
/// the buy (gold/bag checks). A cancel of a warning popup (tag 1) forwards a cancel
/// to the parent.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // super.onPopupResult(tag, result);   (Menu base dismiss — BuySellDialog's super is Menu)
    menu::on_popup_result_base(g, MenuNode::BuySell, tag, result);
    // Hero hero = GameState.hero();   (the reference; a null deref is deferred to `.bag`)
    let hero = g.game_state.hero;
    // if (tag != 2 || result != 0) {
    if tag != 2 || result != 0 {
        // if (tag == 1) { ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1); return; }
        if tag == 1 {
            let parent = menu::parent_of(g, MenuNode::BuySell)
                .expect("NullPointerException: BuySellDialog.parent");
            menu::on_popup_result(g, parent, -1, -1);
        }
        // return;
        return;
    }
    let hero_id = hero.expect("NullPointerException: GameState.hero()");
    let buying = g.buy_sell_dialog.buying;
    let quantity = g.buy_sell_dialog.quantity;
    let item = g
        .buy_sell_dialog
        .item
        .clone()
        .expect("NullPointerException: BuySellDialog.item");
    // if (!this.buying) {
    if !buying {
        // hero.bag.decrementItem(this.item, this.quantity);
        item_bag::decrement_item(
            &mut g.entity_arena[hero_id]
                .as_hero_mut()
                .expect("Hero node")
                .bag,
            &item,
            quantity,
        );
        // hero.bag.gold += (this.item.price * this.quantity) / 5;
        let item_price = item.borrow().price;
        let refund =
            java_div(item_price.wrapping_mul(quantity as i32), 5).expect("(price * quantity) / 5");
        {
            let bag = &mut g.entity_arena[hero_id]
                .as_hero_mut()
                .expect("Hero node")
                .bag;
            bag.gold = bag.gold.wrapping_add(refund);
        }
        // showMessage(new Object[]{this.item.name, ShopMenu.text.get(24)});
        let name = item.borrow().name.clone();
        let line24 = shop_menu::text_get(g, 24);
        menu::show_message(g, MenuNode::BuySell, vec![name, line24]);
        // return;
        return;
    }
    // Item bought = Item.create(this.item.type, this.item.subId, true, false);
    let (item_type, item_sub_id) = {
        let b = item.borrow();
        (b.r#type, b.sub_id)
    };
    let mut bought = item::create(g, item_type, item_sub_id, true, false);
    // if (bought instanceof Equipment) ((Equipment) bought).identified = true;
    if item::is_equipment(&bought) {
        bought.identified = true;
    }
    // int totalCost = bought.price * this.quantity;
    let total_cost = bought.price.wrapping_mul(quantity as i32);
    // if (hero.bag.gold < totalCost) {
    let gold = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .bag
        .gold;
    if gold < total_cost {
        // showMessage(new Object[]{ShopMenu.text.get(8)});
        let line8 = shop_menu::text_get(g, 8);
        menu::show_message(g, MenuNode::BuySell, vec![line8]);
    } else {
        // if (!hero.bag.add(bought, (int) this.quantity)) {
        let bought_ref: ItemRef = Rc::new(RefCell::new(bought));
        let added = {
            let bag = &mut g.entity_arena[hero_id]
                .as_hero_mut()
                .expect("Hero node")
                .bag;
            item_bag::add(bag, bought_ref, quantity as i32)
        };
        if !added {
            // showMessage(new Object[]{ShopMenu.text.get(9), ShopMenu.text.get(10)});
            let line9 = shop_menu::text_get(g, 9);
            let line10 = shop_menu::text_get(g, 10);
            menu::show_message(g, MenuNode::BuySell, vec![line9, line10]);
            // return;
            return;
        }
        // hero.bag.gold -= totalCost;
        {
            let bag = &mut g.entity_arena[hero_id]
                .as_hero_mut()
                .expect("Hero node")
                .bag;
            bag.gold = bag.gold.wrapping_sub(total_cost);
        }
        // showMessage(new Object[]{ShopMenu.text.get(11), ShopMenu.text.get(12)});
        let line11 = shop_menu::text_get(g, 11);
        let line12 = shop_menu::text_get(g, 12);
        menu::show_message(g, MenuNode::BuySell, vec![line11, line12]);
    }
}

/// `public final void moveCursor(byte direction)`
/// (`ab.a:(B)V => [iadd,i2b,isub,i2b]`): adjusts [`quantity`](BuySellDialogState::quantity)
/// (only when the item is a stackable buy or a multi-unit sell); wraps 1↔99 when buying.
pub fn move_cursor(g: &mut Game, direction: i8) {
    let buying = g.buy_sell_dialog.buying;
    let (item_type, item_quantity) = {
        let b = g
            .buy_sell_dialog
            .item
            .as_ref()
            .expect("NullPointerException: BuySellDialog.item")
            .borrow();
        (b.r#type, b.quantity)
    };
    // if (!(this.buying && Item.STACKABLE[this.item.type]) && (this.buying || this.item.quantity <= 1)) return;
    if !(buying && item::STACKABLE[item_type as usize]) && (buying || item_quantity <= 1) {
        return;
    }
    // if (direction == 4) {
    if direction == 4 {
        // if (this.quantity < (this.buying ? (byte) 99 : this.item.quantity)) {
        let limit: i8 = if buying { 99 } else { item_quantity };
        if g.buy_sell_dialog.quantity < limit {
            // this.quantity = (byte) (this.quantity + 1); return;
            g.buy_sell_dialog.quantity = (g.buy_sell_dialog.quantity as i32).wrapping_add(1) as i8;
            return;
        }
    }
    // if (direction == 4 && this.buying && this.quantity == 99) { this.quantity = (byte) 1; return; }
    if direction == 4 && buying && g.buy_sell_dialog.quantity == 99 {
        g.buy_sell_dialog.quantity = 1;
        return;
    }
    // if (direction == 3 && this.quantity > 1) this.quantity = (byte) (this.quantity - 1);
    if direction == 3 && g.buy_sell_dialog.quantity > 1 {
        g.buy_sell_dialog.quantity = (g.buy_sell_dialog.quantity as i32).wrapping_sub(1) as i8;
    }
    // else if (direction == 3 && this.buying && this.quantity == 1) this.quantity = (byte) 99;
    else if direction == 3 && buying && g.buy_sell_dialog.quantity == 1 {
        g.buy_sell_dialog.quantity = 99;
    }
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ab.a:(…Graphics;II)V`): draws the two beveled panels + the gold-line and
/// quantity captions (`ShopMenu.text`, both ported). The clear/soft-key/gold/number/
/// icon draws are DEFERRED (see the module header).
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    // FontManager.clearScreen(graphics);
    // FontManager.drawSoftKeys(graphics, this.buying ? labelBuy : labelSell, labelBack);
    // (DEFERRED: FontManager.clearScreen + the shop soft-key labels are unported.)
    let buying = g.buy_sell_dialog.buying;
    let item_quantity = g
        .buy_sell_dialog
        .item
        .as_ref()
        .expect("NullPointerException: BuySellDialog.item")
        .borrow()
        .quantity;
    let item_type = g
        .buy_sell_dialog
        .item
        .as_ref()
        .expect("NullPointerException: BuySellDialog.item")
        .borrow()
        .r#type;
    // Pre-resolve the drawn ShopMenu.text captions (ShopMenu.text is ported).
    // The gold-line label (text.get(13)) and the quantity/stack caption below.
    let gold_label = shop_menu::text_get(g, 13);
    // The `!(buying && STACKABLE[type]) && (buying || item.quantity <= 1)` branch hides
    // the quantity row and captions text.get(15); else text.get(14) (buy) / 25 (sell).
    let quantity_row_hidden =
        !(buying && item::STACKABLE[item_type as usize]) && (buying || item_quantity <= 1);
    let caption = if quantity_row_hidden {
        shop_menu::text_get(g, 15)
    } else if buying {
        shop_menu::text_get(g, 14)
    } else {
        shop_menu::text_get(g, 25)
    };
    // int x = originX + 3; int y = originY + 20; int labelX = x + 15;
    let x = origin_x.wrapping_add(3);
    let y = origin_y.wrapping_add(20);
    let label_x = x.wrapping_add(15);

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // Menu.drawPanelFrame(graphics, x, y, 149, 29);
    menu::draw_panel_frame(&mut graphics, x, y, 149, 29);
    // Menu.fillPanelInterior(graphics, x, y, 149, 29);
    menu::fill_panel_interior(&mut graphics, x, y, 149, 29);
    // Menu.drawPanelFrame(graphics, x, y + 31, 149, 67);
    menu::draw_panel_frame(&mut graphics, x, y.wrapping_add(31), 149, 67);
    // Menu.fillPanelInterior(graphics, x, y + 31, 149, 67);
    menu::fill_panel_interior(&mut graphics, x, y.wrapping_add(31), 149, 67);
    // graphics.setColor(14663551);
    graphics.set_color(14663551);
    // FontManager.drawChars(graphics, labelX + 8, y + 7, ShopMenu.text.get(13), 1);
    font_manager::draw_chars(
        font_manager,
        &mut graphics,
        label_x.wrapping_add(8),
        y.wrapping_add(7),
        &gold_label,
        1,
    );
    // Menu.drawGold(graphics, labelX + 102, y + 11, GameState.hero().bag.gold);
    // (DEFERRED: Menu.drawGold is unported.)
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, labelX + 6, y + 38, <caption>, 1);
    //   (text.get(15) when the quantity row is hidden, else text.get(14)/25 buy/sell.)
    font_manager::draw_chars(
        font_manager,
        &mut graphics,
        label_x.wrapping_add(6),
        y.wrapping_add(38),
        &caption,
        1,
    );
    // if (!quantity_row_hidden) {
    //     graphics.drawImage(AssetCache.slotFrame, labelX + 32, y + 65, 20);
    //     BaseCanvas.drawNumberAt(graphics, this.quantity, labelX + 68, y + 65, 8);
    //     graphics.drawImage(AssetCache.cursorArrow, labelX + 77, y + 65, 20);
    // }
    // graphics.drawImage(AssetCache.itemIcons[this.item.type], labelX + 45, y + 57, 20);
    // if (this.buying) Menu.drawGold(graphics, labelX + 77, y + 85, this.quantity * this.item.price);
    // else Menu.drawGold(graphics, labelX + 77, y + 85, (this.quantity * this.item.price) / 5);
    // (DEFERRED: AssetCache.slotFrame/cursorArrow/itemIcons art + BaseCanvas.drawNumberAt
    //  + Menu.drawGold are all unported; only drawn when the quantity row is shown.)
}
