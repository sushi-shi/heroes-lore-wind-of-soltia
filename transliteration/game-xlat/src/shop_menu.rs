//! Transliterated from `java/src/main/java/defpackage/ShopMenu.java`
//! (original `bp.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The merchant shop screen (six category tabs), `ShopMenu extends Menu`. Reached
//! from the world via event op 11/0 (`GameState.requestState`, which does
//! `setScreen(6)` + [`load_strings`]). The purchasable stock is
//! [`Item::build_shop_stock`](crate::item::build_shop_stock), grouped into six
//! category vectors ([`shop_stock`](ShopMenuState::shop_stock)); the selected category
//! is shown by the child [`ShopItemList`](crate::shop_item_list).
//!
//! ## Statics + singleton
//!
//! `ShopMenu` is a lazily-created singleton (like `MainMenu`): [`instance`] creates it
//! on first use and centres the panel. Its `static` fields — `panelX`/`panelY`
//! (the centred origin), `text` (the `/sgui/shop` label table) and `singleton`
//! (the presence flag) — live on [`ShopMenuState`] (`Game.shop_menu`), the sole owner
//! per `java/reconstruction/ownership.tsv`. The `shopStock` instance field is
//! per-INSTANCE and carried here too (the singleton is unique).
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `instance`/`<init>`/`load_strings`/`close_shop`/`handleKey`/
//! the overriding `moveCursor`/`draw` are real (the child-rebuild threads the shared
//! stock references, and `close_shop` returns to world screen 2 via the ported
//! `GameState.setScreen`). Only the render's genuinely-unported hops are DEFERRED:
//! `FontManager.clearScreen`, the `drawSoftKeys` `labelBack` (unported),
//! `BaseCanvas.drawLabelBox`, and the `AssetCache.shopCategoryIcons`/`slotFrame`/
//! `cursorArrow` art; `GameScreen.markRedraw` in `close_shop` is likewise DEFERRED.
//! The panel fill + tab-cursor bevel are drawn.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bp.<init>:()V => []`,
//! `bp.a:()Lbp; => [isub,isub]` (instance — `halfW - 77` / `halfH - 85`),
//! `bp.a:(B)V => []` (moveCursor — the child rebuild, no `bp` arithmetic),
//! `bp.a:(II)Z => []` (handleKey), `bp.a:(…Graphics;)V => []` (draw),
//! `bp.d:()V => []` (loadStrings), `bp.e:()V => []` (closeShop),
//! `bp.a:(…Graphics;II)V => [iadd,iadd,iadd,imul,…]` (paint — the tab-cursor bevel
//! geometry; the remaining adds live in the DEFERRED category-icon draws).

use crate::game::Game;
use crate::game_state;
use crate::item;
use crate::item_bag::ItemRef;
use crate::menu::{self, MenuChild, MenuNode};
use crate::shop_item_list;
use crate::text_table::{self, TextTableState};
use std::cell::RefCell;
use std::rc::Rc;

/// Java `bp` / `ShopMenu` state — the `Menu` base + the `shopStock` instance field +
/// the class's four `static` fields (`panelX`, `panelY`, `text`, `singleton`).
#[derive(Debug, Default, Clone)]
pub struct ShopMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private Vector[] shopStock;` (obf `a`) — purchasable stock grouped into six
    /// category vectors (shared `Item` references). Empty == Java null.
    pub shop_stock: Vec<Vec<ItemRef>>,
    /// `public static int panelX;` (obf `a`) — centred panel origin X.
    pub panel_x: i32,
    /// `public static int panelY;` (obf `b`) — centred panel origin Y.
    pub panel_y: i32,
    /// `public static TextTable text;` (obf `a`, `Lz;`) — the `/sgui/shop` label
    /// table (`None` == Java null, until [`load_strings`]).
    pub text: Option<TextTableState>,
    /// `private static ShopMenu singleton;` (obf `a`, `Lbp;`) — presence flag.
    pub singleton: bool,
}

/// `ShopMenu.text.get(index)` — resolves a `/sgui/shop` label through the loaded
/// [`StringTable`](crate::string_table). A null `text` (before [`load_strings`])
/// panics, matching the Java NPE.
pub(crate) fn text_get(g: &Game, index: i32) -> Vec<u16> {
    // ShopMenu.text.get(index)
    text_table::get(
        g,
        g.shop_menu
            .text
            .as_ref()
            .expect("NullPointerException: ShopMenu.text (loadStrings not called)"),
        index,
    )
}

/// `public static final ShopMenu instance()` (`bp.a:()Lbp; => [isub,isub]`): returns
/// (creating on first use) the shop singleton, centring the panel at
/// `halfW - 77` / `halfH - 85`.
pub fn instance(g: &mut Game) {
    // if (singleton == null) {
    if !g.shop_menu.singleton {
        // singleton = new ShopMenu();
        construct(g);
        g.shop_menu.singleton = true;
        // panelX = BaseCanvas.halfW - 77;
        g.shop_menu.panel_x = g.base_canvas.half_w.wrapping_sub(77);
        // panelY = BaseCanvas.halfH - 85;
        g.shop_menu.panel_y = g.base_canvas.half_h.wrapping_sub(85);
    }
    // return singleton;   (the shop node — identity, not returned in the flat model)
}

/// `private ShopMenu()` (`bp.<init>:()V => []`): builds the six-tab shop over the
/// decoded stock and pushes the first category's [`ShopItemList`](crate::shop_item_list).
pub fn construct(g: &mut Game) {
    // super(null, (byte) 6);   (null parent → the shop is a root)
    g.shop_menu.base = menu::construct(false, 6);
    // this.shopStock = Item.buildShopStock();
    //   (wrap each decoded Item in a shared reference — the stock is shared with the
    //    child ShopItemList and any sell dialog, matching Java's object identity.)
    let stock = item::build_shop_stock(g);
    g.shop_menu.shop_stock = stock
        .into_iter()
        .map(|category| {
            category
                .into_iter()
                .map(|it| Rc::new(RefCell::new(it)) as ItemRef)
                .collect()
        })
        .collect();
    // ((Menu) this).child = new ShopItemList(this, this.shopStock[cursorIndex], cursorIndex);
    let cursor = g.shop_menu.base.cursor_index;
    let stock_vec: Vec<ItemRef> = g.shop_menu.shop_stock[cursor as usize].clone();
    shop_item_list::construct(g, stock_vec, cursor);
    g.shop_menu.base.child = MenuChild::ShopItemList;
}

/// `public final void loadStrings()` (`bp.d:()V => []`): loads the `/sgui/shop`
/// label table into [`text`](ShopMenuState::text). The Java `catch (IOException)`
/// only logs (leaving `text` null); `/sgui/shop.tdf` ships in the JAR, so the read
/// succeeds (`TextTable::construct` would panic loud on a truly-absent resource,
/// matching an uncaught read failure).
pub fn load_strings(g: &mut Game) {
    // try { text = new TextTable("/sgui/shop"); } catch (IOException e) { e.printStackTrace(); }
    let table = text_table::construct(g, "/sgui/shop");
    g.shop_menu.text = Some(table);
}

/// `private void closeShop()` (`bp.e:()V => []`): tears the shop down and returns to
/// the world screen.
fn close_shop(g: &mut Game) {
    // singleton = null;
    g.shop_menu.singleton = false;
    // text = null;
    g.shop_menu.text = None;
    // this.shopStock = null;
    g.shop_menu.shop_stock = Vec::new();
    // ((Menu) this).child = null;
    g.shop_menu.base.child = MenuChild::None;
    // GameState.setScreen(2);
    game_state::set_screen(g, 2);
    // GameLoop.gameScreen.markRedraw();
    // (DEFERRED: GameScreen.markRedraw is unported — game_screen not this lane.)
    // System.gc();   — no-op.
}

/// `public final boolean handleKey(int action, int keyCode)` (`bp.a:(II)Z => []`):
/// child forward + horizontal category nav (the overriding [`move_cursor`]); Back
/// (`-8`) closes the shop. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::ShopMenu, action, key_code)
        || menu::move_cursor_horizontal_node(g, MenuNode::ShopMenu, action, key_code)
    {
        return true;
    }
    // if (keyCode != -8) return false;
    if key_code != -8 {
        return false;
    }
    // closeShop(); return true;
    close_shop(g);
    true
}

/// `public final void moveCursor(byte direction)` (`bp.a:(B)V => []`): steps the
/// category cursor (base `moveCursor`) then rebuilds the child
/// [`ShopItemList`](crate::shop_item_list) over the newly-selected category's stock.
pub fn move_cursor(g: &mut Game, direction: i8) {
    // super.moveCursor(direction);
    menu::move_cursor(&mut g.shop_menu.base, direction);
    // ((Menu) this).child = new ShopItemList(this, this.shopStock[cursorIndex], cursorIndex);
    let cursor = g.shop_menu.base.cursor_index;
    let stock_vec: Vec<ItemRef> = g.shop_menu.shop_stock[cursor as usize].clone();
    shop_item_list::construct(g, stock_vec, cursor);
    g.shop_menu.base.child = MenuChild::ShopItemList;
}

/// `public final void draw(Graphics graphics)` (`bp.a:(…Graphics;)V => []`): draws the
/// whole shop screen tree at the centred panel origin.
pub fn draw(g: &mut Game) {
    // render(graphics, panelX, panelY);
    let (panel_x, panel_y) = (g.shop_menu.panel_x, g.shop_menu.panel_y);
    menu::render_at(g, MenuNode::ShopMenu, panel_x, panel_y);
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bp.a:(…Graphics;II)V`): draws the panel fill + the selected-tab cursor bevel. The
/// clear/soft-keys, the category-icon row, the label box and the frame/arrow art are
/// DEFERRED (see the module header).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // FontManager.clearScreen(graphics);
    // FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    // (DEFERRED: FontManager.clearScreen + labelBack are unported.)
    // int cursor = ((Menu) this).cursorIndex;
    let cursor = g.shop_menu.base.cursor_index as i32;
    let Game { screen, .. } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(x, y, 155, 170);
    graphics.fill_rect(x, y, 155, 170);
    // Menu.drawInsetPanel(graphics, x + 2, y + 15, 151, 155);
    menu::draw_inset_panel(
        &mut graphics,
        x.wrapping_add(2),
        y.wrapping_add(15),
        151,
        155,
    );
    // graphics.setColor(16768959);
    graphics.set_color(16768959);
    // graphics.fillRect(x + 11 + (cursorIndex * 16) + 1, y, 14, 1);
    graphics.fill_rect(
        x.wrapping_add(11)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(1),
        y,
        14,
        1,
    );
    // graphics.fillRect(x + 11 + (cursorIndex * 16), y + 1, 1, 16);
    graphics.fill_rect(
        x.wrapping_add(11).wrapping_add(cursor.wrapping_mul(16)),
        y.wrapping_add(1),
        1,
        16,
    );
    // graphics.setColor(12558207);
    graphics.set_color(12558207);
    // graphics.fillRect(x + 11 + (cursorIndex * 16) + 15, y + 1, 1, 15);
    graphics.fill_rect(
        x.wrapping_add(11)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(15),
        y.wrapping_add(1),
        1,
        15,
    );
    // graphics.setColor(14663551);
    graphics.set_color(14663551);
    // graphics.fillRect(x + 11 + (cursorIndex * 16) + 1, y + 1, 14, 16);
    graphics.fill_rect(
        x.wrapping_add(11)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(1),
        y.wrapping_add(1),
        14,
        16,
    );
    // for (int category = 0; category < 6; category++)
    //     graphics.drawImage(AssetCache.shopCategoryIcons[category], x + 13 + (category * 16), y + 1, 20);
    // BaseCanvas.drawLabelBox(graphics, text.get(cursorIndex + 1), x + 3, y + 15);
    // graphics.drawImage(AssetCache.slotFrame, x + 4, y + 4, 20);
    // graphics.drawImage(AssetCache.cursorArrow, x + 109, y + 4, 20);
    // (DEFERRED: AssetCache.shopCategoryIcons/slotFrame/cursorArrow art + the
    //  BaseCanvas.drawLabelBox tab caption are unported.)
}
