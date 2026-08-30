//! Transliterated from `java/src/main/java/defpackage/ShopItemList.java`
//! (original `v.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The scrollable stock list for one shop [`category`](ShopItemListState::category)
//! tab (`ShopItemList extends Menu`), pushed by [`ShopMenu`](crate::shop_menu)
//! whenever its category cursor moves. It shows the category's purchasable
//! [`items`](ShopItemListState::items); OK opens a buy
//! [`BuySellDialog`](crate::buy_sell_dialog), and the `#` key switches to the sell
//! tab ([`SellList`](crate::sell_list)) over the hero's bag.
//!
//! ## ANTI-BOG boundary
//!
//! The constructor (`super(parent, (byte) stock.size())` + the reference copy + the
//! category store) and `handleKey` are ported **fully** — the OK buy dialog, the `#`
//! sell-tab / empty-bag popup, and the parent-repaint callback are all real. `paint`
//! is **PARTIAL**: the `drawSoftKeys` (shop labels unported) and the whole per-row
//! item/equipped-value marker block are DEFERRED — they cross into the unported
//! `Hero.getWeapon/getArmor/getAccessory1..3` (feeding `equippedValue`, used only by
//! the coin/box art), `Menu.drawItemIcon`/`drawItemInfo` (DEFERRED in `menu`), and the
//! `AssetCache.shopCoinIcon`/`shopSelectBox`/`shopSellIcon` banks + `ShopMenu.panelX/
//! panelY` offsets not modelled in the partial `AssetCache`. The paginated list frame
//! (`drawListPage`) is drawn.
//!
//! `ShopItemList`'s fields (`items`, `category`) are per-INSTANCE (the `Menu` base
//! likewise), so it contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `v.<init>:(Lcb;Ljava/util/Vector;B)V => [i2b,iinc]` (constructor — `(byte)
//! stock.size()` + the copy loop), `v.a:(II)Z => []` (handleKey — pure branches),
//! `v.a:(…Graphics;II)V => [iinc,iinc,iadd,…,irem,imul,…]` (paint — the ported
//! `drawListPage` prefix; the remaining `% 5`/`* 23` live in the DEFERRED per-row draws).

use crate::buy_sell_dialog;
use crate::game::Game;
use crate::item_bag::{self, ItemRef};
use crate::menu::{self, MenuChild, MenuNode};
use crate::sell_list;
use crate::shop_menu;

/// Java `v` / `ShopItemList` instance state (the `Menu` base + the tab's own
/// per-instance fields).
#[derive(Debug, Default, Clone)]
pub struct ShopItemListState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private Item[] items;` (obf `a`) — the purchasable items in this category tab
    /// (shared references copied from the shop stock vector).
    pub items: Vec<ItemRef>,
    /// `public byte category;` (obf `c`) — shop category id (0 misc, 1 weapon, 2
    /// armor, 3-5 accessory slots).
    pub category: i8,
}

/// `public ShopItemList(Menu parent, Vector stock, byte category)`
/// (`v.<init>:(Lcb;Ljava/util/Vector;B)V => [i2b,iinc]`).
pub fn construct(g: &mut Game, stock: Vec<ItemRef>, category: i8) {
    // super(parent, (byte) stock.size());   (parent is the shop menu → present)
    g.shop_item_list.base = menu::construct(true, (stock.len() as i32) as i8);
    // this.items = new Item[stock.size()];
    // for (int i = 0; i < items.length; i++) this.items[i] = (Item) stock.elementAt(i);
    //   (the per-element copy shares the Item references — the passed `stock` already
    //    holds cloned Rc handles to the shop-stock Items.)
    g.shop_item_list.items = stock;
    // this.category = category;
    g.shop_item_list.category = category;
}

/// `public final boolean handleKey(int action, int keyCode)` (`v.a:(II)Z => []`):
/// child forward, vertical no-wrap nav (marking the parent for repaint), FIRE opens a
/// buy [`BuySellDialog`](crate::buy_sell_dialog), `#` opens the sell
/// [`SellList`](crate::sell_list) (or an empty-bag popup). Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::ShopItemList, action, key_code) {
        return true;
    }
    // if (moveCursorVerticalNoWrap(action, keyCode)) { ((Menu) this).parent.needsRepaint = true; return true; }
    if menu::move_cursor_vertical_no_wrap(&mut g.shop_item_list.base, action, key_code) {
        if let Some(parent) = menu::parent_of(g, MenuNode::ShopItemList) {
            menu::set_needs_repaint(g, parent, true);
        }
        return true;
    }
    // if (keyCode == 53 || action == 8) { ((Menu) this).child = new BuySellDialog(this, items[cursorIndex], true); return true; }
    if key_code == 53 || action == 8 {
        let cursor = g.shop_item_list.base.cursor_index;
        let item = g.shop_item_list.items[cursor as usize].clone();
        buy_sell_dialog::construct(g, item, true);
        g.shop_item_list.base.child = MenuChild::BuySell;
        return true;
    }
    // if (keyCode != 35) return false;
    if key_code != 35 {
        return false;
    }
    // byte[] occupiedSlots = GameState.hero().bag.occupiedSlots();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let occupied_slots = {
        let bag = &g.entity_arena[hero_id].as_hero().expect("Hero node").bag;
        item_bag::occupied_slots(bag)
    };
    // if (occupiedSlots.length > 0) { ((Menu) this).child = new SellList(this, occupiedSlots); return true; }
    if !occupied_slots.is_empty() {
        sell_list::construct(g, occupied_slots);
        g.shop_item_list.base.child = MenuChild::SellList;
        return true;
    }
    // showPopup((byte) 1, (byte) 0, new Object[]{ShopMenu.text.get(16), ShopMenu.text.get(17)});
    let line16 = shop_menu::text_get(g, 16);
    let line17 = shop_menu::text_get(g, 17);
    menu::show_popup(g, MenuNode::ShopItemList, 1, 0, vec![line16, line17]);
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`v.a:(…Graphics;II)V`): draws the paginated list frame. The soft keys and the
/// per-row item/equipped-value/detail/sell-icon block are DEFERRED (see the module
/// header).
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    // FontManager.drawSoftKeys(graphics, FontManager.labelSelect, FontManager.labelBack);
    // (DEFERRED: the shop soft-key labels are unported.)
    // int x = originX + 2; int y = originY + 15;
    let x = origin_x.wrapping_add(2);
    let y = origin_y.wrapping_add(15);
    let base = g.shop_item_list.base.clone();
    {
        let Game { screen, .. } = &mut *g;
        let target = screen.as_mut().expect("framebuffer");
        let mut graphics = j2me_me::Graphics::new(target);
        // drawListPage(graphics, x, y, true);
        menu::draw_list_page(&mut graphics, &base, x, y, true);
    }
    // short equippedValue = -1; Hero hero = GameState.hero();
    // switch (this.category) { case 1..5: equippedValue = hero.getWeapon/getArmor/getAccessoryN().value; }
    // for (index = pageFirstIndex(); index <= pageLastIndex(); index++) {
    //     Item item = this.items[index];
    //     if (item != null) {
    //         Menu.drawItemIcon(graphics, x + 13, y + 18 + (23 * (index % 5)), item, false);
    //         if (this.category != 0) {
    //             short listedValue = ((Equipment) items[cursorIndex]).value;
    //             if (equippedValue > listedValue) graphics.drawImage(AssetCache.shopCoinIcon, ...);
    //             else if (equippedValue < listedValue) graphics.drawImage(AssetCache.shopSelectBox, ...);
    //         }
    //     }
    // }
    // Item selectedItem = this.items[cursorIndex];
    // if (selectedItem != null) Menu.drawItemInfo(graphics, x + 33, y + 14, selectedItem);
    // graphics.drawImage(AssetCache.shopSellIcon, (ShopMenu.panelX + 155) - 38, (ShopMenu.panelY + 170) - 22, 20);
    // (DEFERRED: `Hero.getWeapon/getArmor/getAccessory1..3` feed `equippedValue`, used
    //  only by the coin/box art; `Menu.drawItemIcon`/`drawItemInfo` are DEFERRED in
    //  `menu`; the `AssetCache.shopCoinIcon`/`shopSelectBox`/`shopSellIcon` banks +
    //  `ShopMenu.panelX/panelY` offsets are not modelled in the partial AssetCache.)
}
