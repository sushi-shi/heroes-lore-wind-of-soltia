//! STATE oracle for the shop-screen trio ported this lane: `ShopMenu` (`bp`),
//! `ShopItemList` (`v`) and `BuySellDialog` (`ab`).
//!
//! * `ShopMenu.instance()` builds the singleton over the decoded `/itm/forshop`
//!   stock (`Item.buildShopStock`), centres the panel at `halfW-77`/`halfH-85`, and
//!   pushes the first category's `ShopItemList`; the category cursor (RIGHT) rebuilds
//!   the child over the next category's stock. `loadStrings` loads the `/sgui/shop`
//!   label table.
//! * `ShopItemList` over a synthetic stock vector has one row per stock item and
//!   shares the item references; FIRE opens a buy `BuySellDialog` over the selected
//!   row (`buying == true`, `quantity == 1`).
//!
//! These are STATE assertions, not pixel diffs: the shop art is partial (DEFERRED —
//! it crosses into unported `FontManager`/`AssetCache`/`Menu` item widgets).

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::item;
use heroes_lore_wind_of_soltia_game_xlat::item_bag::ItemRef;
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{shop_item_list, shop_menu, Game};
use std::cell::RefCell;
use std::rc::Rc;

/// RIGHT — `keyCode 54` (KEY_NUM6).
const KEY_RIGHT: i32 = 54;
/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;

/// A base (type-7) stock item with a chosen `subId` — a shared `Item` reference, as
/// the shop stock holds.
fn shop_item(sub_id: i8) -> ItemRef {
    Rc::new(RefCell::new(item::new_item(7, sub_id)))
}

/// A `Game` with the baseline JAR's resources loaded (so `Item.buildShopStock` can
/// decode `/itm/forshop`).
fn game_with_resources() -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    g
}

/// `ShopMenu.instance()` creates the singleton, centres the panel, decodes the
/// six-category stock, and pushes category 0's `ShopItemList`; RIGHT rebuilds it over
/// category 1.
#[test]
fn shop_menu_instance_builds_stock_and_child() {
    let mut g = game_with_resources();
    // A concrete centred origin so panelX/panelY are checkable (halfW/halfH otherwise 0).
    g.base_canvas.half_w = 88;
    g.base_canvas.half_h = 104;

    shop_menu::instance(&mut g);

    assert!(g.shop_menu.singleton, "instance() created the singleton");
    assert_eq!(
        g.shop_menu.base.item_count, 6,
        "super(null, (byte) 6) → six category tabs"
    );
    assert!(
        !g.shop_menu.base.parent,
        "the shop is a root (super(null, …) → no parent)"
    );
    assert_eq!(
        g.shop_menu.base.child,
        MenuChild::ShopItemList,
        "the constructor pushed a ShopItemList child"
    );
    assert_eq!(g.shop_menu.panel_x, 88 - 77, "panelX = halfW - 77");
    assert_eq!(g.shop_menu.panel_y, 104 - 85, "panelY = halfH - 85");
    assert_eq!(
        g.shop_menu.shop_stock.len(),
        6,
        "buildShopStock returns six category vectors"
    );

    // The pushed ShopItemList shows category 0, one row per stock item.
    assert_eq!(g.shop_item_list.category, 0, "first tab is category 0");
    assert_eq!(
        g.shop_item_list.base.item_count as usize,
        g.shop_item_list.items.len(),
        "itemCount == items.length"
    );
    assert_eq!(
        g.shop_item_list.items.len(),
        g.shop_menu.shop_stock[0].len(),
        "the child lists exactly category 0's stock"
    );

    // RIGHT moves the category cursor 0 → 1 (the overriding moveCursor rebuilds the child).
    assert!(
        shop_menu::handle_key(&mut g, 0, KEY_RIGHT),
        "RIGHT is consumed"
    );
    assert_eq!(g.shop_menu.base.cursor_index, 1, "category cursor 0 → 1");
    assert_eq!(
        g.shop_item_list.category, 1,
        "child rebuilt over category 1"
    );
    assert_eq!(
        g.shop_item_list.items.len(),
        g.shop_menu.shop_stock[1].len(),
        "the rebuilt child lists category 1's stock"
    );

    // loadStrings loads the /sgui/shop label table.
    shop_menu::load_strings(&mut g);
    let text = g.shop_menu.text.as_ref().expect("loadStrings set text");
    assert!(text.count > 0, "the /sgui/shop table has entries");
}

/// A `ShopItemList` built over a synthetic stock vector has one row per item and
/// shares the item references; FIRE opens a buy `BuySellDialog` over the selected row.
#[test]
fn shop_item_list_rows_and_buy_dialog_open() {
    let mut g = Game::new();
    let stock: Vec<ItemRef> = vec![shop_item(0), shop_item(1), shop_item(2)];
    // new ShopItemList(parent, stock, (byte) 1) → super(parent, 3), category 1.
    shop_item_list::construct(&mut g, stock.clone(), 1);

    assert_eq!(
        g.shop_item_list.base.item_count, 3,
        "super(parent, stock.size()) → itemCount 3"
    );
    assert_eq!(g.shop_item_list.category, 1, "category stored");
    assert_eq!(g.shop_item_list.items.len(), 3, "three rows");
    assert!(
        Rc::ptr_eq(&g.shop_item_list.items[0], &stock[0]),
        "items share the stock's Item references"
    );
    assert_eq!(
        g.shop_item_list.base.cursor_index, 0,
        "cursor starts at row 0"
    );

    // FIRE: child = new BuySellDialog(this, items[cursorIndex], true).
    assert!(
        shop_item_list::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE is consumed"
    );
    assert_eq!(
        g.shop_item_list.base.child,
        MenuChild::BuySell,
        "FIRE pushed a BuySellDialog"
    );
    assert!(g.buy_sell_dialog.buying, "opened in buy mode");
    assert_eq!(g.buy_sell_dialog.quantity, 1, "quantity starts at 1");
    assert!(
        Rc::ptr_eq(
            g.buy_sell_dialog.item.as_ref().expect("dialog item"),
            &stock[0]
        ),
        "the dialog holds the selected row's Item reference"
    );
    assert!(
        g.buy_sell_dialog.base.parent,
        "super(parent, 0) → parent present"
    );
    assert_eq!(
        g.buy_sell_dialog.base.item_count, 0,
        "super(parent, (byte) 0) → itemCount 0"
    );

    // NEGATIVE CONTROL: a two-item stock yields exactly two rows (so the three-row
    // assertion above cannot read as a fixed constant).
    let mut g2 = Game::new();
    shop_item_list::construct(&mut g2, vec![shop_item(3), shop_item(4)], 2);
    assert_eq!(
        g2.shop_item_list.base.item_count, 2,
        "a two-item stock → a two-row ShopItemList"
    );
    assert_eq!(g2.shop_item_list.category, 2, "category 2 stored");
}
