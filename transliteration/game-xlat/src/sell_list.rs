//! Transliterated from `java/src/main/java/defpackage/SellList.java`
//! (original `bb.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The shop's sell tab (`SellList extends ItemPickerList`): an
//! [`ItemPickerList`](crate::item_picker_list) over the hero's occupied bag slots.
//! Selecting a non-quest item opens a sell `BuySellDialog`; quest items are refused
//! with a popup. After a sale it rebuilds itself over the remaining bag contents (or
//! closes the shop-list parent when the bag empties). It draws the shop "buy" icon so
//! the player can toggle back to buying, and reads its label strings from
//! `ShopMenu.text`.
//!
//! ## Inheritance model
//!
//! `SellList` adds **no** fields to `ItemPickerList`; the flat model carries the
//! inherited superclass state as [`SellListState::picker`]
//! (an [`ItemPickerListState`](crate::item_picker_list::ItemPickerListState)). The
//! `Menu` base, `slots`, `resultTag` and `title` therefore live at
//! `g.sell_list.picker.*`, and `super.paint` dispatches straight to
//! [`item_picker_list::paint_fields`](crate::item_picker_list::paint_fields).
//!
//! ## ANTI-BOG boundary
//!
//! The constructor is ported **fully bar the title source**: `super(parent, slots,
//! (byte) 0, ShopMenu.text.get(18))` — the `itemCount = slots.length` row-count is real,
//! and only the title `ShopMenu.text.get(18)` is a DEFERRED placeholder (`ShopMenu`
//! unported). `handleKey` is ported bar its two unported hops: the quest-item refusal
//! `showPopup` machinery is real (its `ShopMenu.text` line content DEFERRED), and the
//! non-quest branch `child = new BuySellDialog(...)` is DEFERRED (`BuySellDialog`
//! unported). `onPopupResult` runs the real base dismiss (`super`); its
//! `previousChild instanceof BuySellDialog` rebuild block is DEFERRED (`BuySellDialog`
//! can never be the flat model's child). `paint` calls the real
//! `super.paint` (the partial `ItemPickerList` scaffold); `FontManager.clearScreen`,
//! the `drawSoftKeys(labelSelect, labelBack)` and the shop-buy icon are DEFERRED.
//!
//! `SellList` adds no fields (all inherited, per-INSTANCE), so it contributes no
//! `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bb.<init>:(Lcb;[B)V => []`
//! (constructor — a pure `super(...)` forward), `bb.a:(II)Z => []` (handleKey — pure
//! branches over the bag read), `bb.a:(BB)V => []` (onPopupResult — the
//! `instanceof`/rebuild block, DEFERRED), `bb.a:(…Graphics;II)V => [iadd,isub,iadd,isub]`
//! (paint — the DEFERRED shop-buy icon offsets).

use crate::game::Game;
use crate::item;
use crate::item_bag;
use crate::item_picker_list::{self, ItemPickerListState};
use crate::menu::{self, MenuNode};

/// Java `bb` / `SellList` instance state — no new fields over `ItemPickerList`; the
/// inherited superclass state is carried by [`picker`](Self::picker).
#[derive(Debug, Default, Clone)]
pub struct SellListState {
    /// The `ItemPickerList` (`m`) superclass part (the `Menu` base + `slots`/
    /// `resultTag`/`title`).
    pub picker: ItemPickerListState,
}

/// `public SellList(Menu parent, byte[] slots)` (`bb.<init>:(Lcb;[B)V => []`):
/// `super(parent, slots, (byte) 0, ShopMenu.text.get(18))`.
pub fn construct(g: &mut Game, slots: Vec<i8>) {
    // super(parent, slots, (byte) 0, ShopMenu.text.get(18));
    //   (ShopMenu.text is unported → the title is a DEFERRED placeholder (empty).)
    let title: Vec<u16> = Vec::new(); // ShopMenu.text.get(18) — DEFERRED (ShopMenu unported)
    item_picker_list::construct_into(&mut g.sell_list.picker, slots, 0, title);
}

/// `public final boolean handleKey(int action, int keyCode)` (`bb.a:(II)Z => []`):
/// child forward + vertical no-wrap navigation; Back/`#` reports the cancel sentinel;
/// FIRE resolves the bag item — quest items open a refusal popup, others (would) open
/// the sell `BuySellDialog` (DEFERRED). Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::SellList, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.sell_list.picker.base, action, key_code)
    {
        return true;
    }
    // if (keyCode != 53 && action != 8) {
    if key_code != 53 && action != 8 {
        // if (keyCode != -8 && keyCode != 35) return true;
        if key_code != -8 && key_code != 35 {
            return true;
        }
        // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        let parent =
            menu::parent_of(g, MenuNode::SellList).expect("NullPointerException: SellList.parent");
        menu::on_popup_result(g, parent, -1, -1);
        // return true;
        return true;
    }
    // Item item = GameState.hero().bag.get((int) this.slots[cursorIndex]);
    let cursor = g.sell_list.picker.base.cursor_index;
    let slot = g.sell_list.picker.slots[cursor as usize];
    let hero_id = g.game_state.hero.expect("GameState.hero null");
    let item = {
        let hero = g.entity_arena[hero_id].as_hero().expect("Hero node");
        item_bag::get(&hero.bag, slot as i32)
    }
    .expect("NullPointerException: SellList.handleKey item");
    // if (item.isQuestItem()) {
    if item::is_quest_item(&item.borrow()) {
        // showPopup((byte) 1, (byte) 0, new Object[]{ShopMenu.text.get(19), ShopMenu.text.get(20)});
        //   (ShopMenu.text unported → the two option lines carry DEFERRED placeholders.)
        let lines: Vec<Vec<u16>> = vec![Vec::new(), Vec::new()];
        menu::show_popup(g, MenuNode::SellList, 1, 0, lines);
        // return true;
        return true;
    }
    // ((Menu) this).child = new BuySellDialog(this, item, false);
    // (DEFERRED: BuySellDialog is unported — the sell dialog child is not created.)
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`bb.a:(BB)V => []`):
/// captures the previous child, runs the base dismiss (`super`), then — when a sale
/// dialog just closed — rebuilds the list or closes the shop parent. The rebuild block
/// is DEFERRED (`BuySellDialog` unported, so `child` is never a `BuySellDialog` here).
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu previousChild = ((Menu) this).child;
    let _previous_child = g.sell_list.picker.base.child;
    // super.onPopupResult(tag, result);
    //   (ItemPickerList does not override onPopupResult → the Menu base dismiss.)
    menu::on_popup_result_base(g, MenuNode::SellList, tag, result);
    // if (previousChild instanceof BuySellDialog) {
    //     ((Menu) this).parent.close();
    //     byte[] occupiedSlots = GameState.hero().bag.occupiedSlots();
    //     if (occupiedSlots.length > 0) ((Menu) this).parent.child = new SellList(parent, occupiedSlots);
    //     else ((Menu) this).parent.showPopup((byte) 1, (byte) 0, {ShopMenu.text.get(21), ShopMenu.text.get(22)});
    // }
    // (DEFERRED: BuySellDialog is unported, so `previousChild` is never a BuySellDialog
    //  in the flat model — this whole rebuild block (parent.close + occupiedSlots +
    //  new SellList / showPopup(ShopMenu.text 21/22)) is never entered.)
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`bb.a:(…Graphics;II)V`): clears the screen + soft keys (DEFERRED), calls the real
/// `super.paint` (the partial `ItemPickerList` scaffold), then draws the shop-buy icon
/// (DEFERRED).
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    // FontManager.clearScreen(graphics);
    // FontManager.drawSoftKeys(graphics, FontManager.labelSelect, FontManager.labelBack);
    // (DEFERRED: FontManager.clearScreen + labelSelect/labelBack are unported.)
    // super.paint(graphics, originX, originY);
    let base = g.sell_list.picker.base.clone();
    let title = g.sell_list.picker.title.clone();
    item_picker_list::paint_fields(g, &base, &title, origin_x, origin_y);
    // graphics.drawImage(AssetCache.shopBuyIcon, (ShopMenu.panelX + 155) - 38, (ShopMenu.panelY + 170) - 22, 20);
    // (DEFERRED: AssetCache.shopBuyIcon art + ShopMenu.panelX/panelY are unported.)
}
