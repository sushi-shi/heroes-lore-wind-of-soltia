//! Transliterated from `java/src/main/java/defpackage/ItemPickerList.java`
//! (original `m.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The generic scrollable item-slot picker (`ItemPickerList extends Menu`) pushed by
//! the equip/craft/blacksmith menus. It lists the items named by
//! [`slots`](ItemPickerListState::slots) (a slot-code array) under a
//! [`title`](ItemPickerListState::title) header and, on OK, reports the chosen slot
//! back to its parent via [`Menu::onPopupResult`](crate::menu::on_popup_result),
//! passing [`result_tag`](ItemPickerListState::result_tag) so the parent knows which
//! pick this was. Each slot code resolves to an [`Item`](crate::item) the same way
//! everywhere: `>= 100` is an equipped slot (`code - 100`), `< 0` is a quick-slot
//! (`-code - 1`), and the rest are bag indices. [`SellList`](crate::sell_list)
//! subclasses this for the shop's sell tab.
//!
//! ## ANTI-BOG boundary
//!
//! The constructor (`super(parent, slots.length)` + the three field stores) and
//! `handleKey` are ported **fully** — the OK/Back `parent.onPopupResult` callbacks are
//! made real via the flat model's [`parent_of`](crate::menu::parent_of) scan. `paint`
//! is **PARTIAL** (the inset panel + the title-field + the paginated list scaffold, all
//! pure-graphics Menu draw-kit); the per-slot item icons
//! ([`Menu::draw_item_icon`](crate::menu) — `AssetCache.itemIcons` art +
//! `BaseCanvas.drawNumberAt`) and the selected-item detail block (`Menu.drawItemInfo`
//! — `FontManager.drawWrappedText` wrapped-text + `AssetCache.commonText`/`heroText`)
//! are DEFERRED, along with the `hero.getEquip` slot resolution (`Hero.getEquip`
//! unported). `ItemPickerList` is only pushed by the (unported) equip/craft menus, so
//! it is reachable in this increment only by an explicit drive.
//!
//! `ItemPickerList`'s fields (`slots`, `resultTag`, `title`) are all per-INSTANCE (the
//! `Menu` base fields likewise), so it contributes no
//! `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `m.<init>:(Lcb;[BB[C)V => [i2b]` (constructor — `(byte) slots.length`),
//! `m.a:(II)Z => []` (handleKey — pure branches over the slot read),
//! `m.a:(…Graphics;II)V => [iadd,isub,iadd,isub,iadd,isub,iadd,isub,iadd, ...]` (paint —
//! the ported prefix is the panel/title/list geometry; the remaining adds/muls live in
//! the DEFERRED per-slot icon + detail draws).

use crate::font_manager;
use crate::game::Game;
use crate::menu::{self, MenuNode};

/// Java `m` / `ItemPickerList` instance state — the `Menu` (`cb`) base fields plus the
/// picker's own instance fields.
#[derive(Debug, Default, Clone)]
pub struct ItemPickerListState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `public byte[] slots;` (obf `h`) — slot codes to list (equipped `>=100`,
    /// quick-slot `<0`, else bag index).
    pub slots: Vec<i8>,
    /// `public byte resultTag;` (obf `c`) — tag echoed to the parent's
    /// `onPopupResult` on OK/cancel.
    pub result_tag: i8,
    /// `private char[] title;` (obf `a`) — header caption drawn above the list.
    pub title: Vec<u16>,
}

/// The `ItemPickerList(Menu, byte[], byte, char[])` constructor body over an owned
/// [`ItemPickerListState`] — shared by the direct constructor and by
/// [`SellList`](crate::sell_list)'s `super(...)` call (which owns its inherited
/// picker fields).
pub fn construct_into(
    s: &mut ItemPickerListState,
    slots: Vec<i8>,
    result_tag: i8,
    title: Vec<u16>,
) {
    // super(parent, (byte) slots.length);   (parent is the pushing menu → present)
    s.base = menu::construct(true, (slots.len() as i32) as i8);
    // this.slots = slots;
    s.slots = slots;
    // this.resultTag = resultTag;
    s.result_tag = result_tag;
    // this.title = title;
    s.title = title;
}

/// `public ItemPickerList(Menu parent, byte[] slots, byte resultTag, char[] title)`
/// (`m.<init>:(Lcb;[BB[C)V => [i2b]`).
pub fn construct(g: &mut Game, slots: Vec<i8>, result_tag: i8, title: Vec<u16>) {
    construct_into(&mut g.item_picker_list, slots, result_tag, title);
}

/// `public boolean handleKey(int action, int keyCode)` (`m.a:(II)Z => []`): forwards
/// to the child, then vertical no-wrap navigation; FIRE (`keyCode 53` / `action 8`)
/// reports the chosen slot to the parent, Back (`keyCode -8`) reports the cancel
/// sentinel. Always returns true.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::ItemPicker, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.item_picker_list.base, action, key_code)
    {
        return true;
    }
    // if (keyCode == 53 || action == 8) {
    if key_code == 53 || action == 8 {
        // ((Menu) this).parent.onPopupResult(this.resultTag, this.slots[cursorIndex]);
        let parent = menu::parent_of(g, MenuNode::ItemPicker)
            .expect("NullPointerException: ItemPickerList.parent");
        let cursor = g.item_picker_list.base.cursor_index;
        let slot = g.item_picker_list.slots[cursor as usize];
        let tag = g.item_picker_list.result_tag;
        menu::on_popup_result(g, parent, tag, slot);
        // return true;
        return true;
    }
    // if (keyCode != -8) return true;
    if key_code != -8 {
        return true;
    }
    // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
    let parent = menu::parent_of(g, MenuNode::ItemPicker)
        .expect("NullPointerException: ItemPickerList.parent");
    menu::on_popup_result(g, parent, -1, -1);
    // return true;
    true
}

/// `public void paint(Graphics graphics, int originX, int originY)`
/// (`m.a:(…Graphics;II)V`): dispatch entry for the `ItemPickerList` node — paints over
/// this instance's own [`ItemPickerListState`].
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    let base = g.item_picker_list.base.clone();
    let title = g.item_picker_list.title.clone();
    paint_fields(g, &base, &title, origin_x, origin_y);
}

/// The `ItemPickerList.paint` body over a snapshot of the picker's `base`/`title` —
/// shared by the direct [`paint`] and by [`SellList`](crate::sell_list)'s `super.paint`.
/// **PARTIAL**: the inset panel + title field + paginated-list scaffold are drawn; the
/// per-slot item icons and the selected-item detail block are DEFERRED (see the module
/// header).
pub fn paint_fields(
    g: &mut Game,
    base: &menu::MenuBase,
    title: &[u16],
    origin_x: i32,
    origin_y: i32,
) {
    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // Hero hero = GameState.hero();   — read only to resolve the DEFERRED per-slot items.
    // int x = originX + 2;
    let x = origin_x.wrapping_add(2);
    // int y = (originY - 3) + 14;
    let y = origin_y.wrapping_sub(3).wrapping_add(14);
    // Menu.drawInsetPanel(graphics, x, y - 14, 151, 170);
    menu::draw_inset_panel(&mut graphics, x, y.wrapping_sub(14), 151, 170);
    // boolean multiPage = pageCount() > 1;
    let multi_page = menu::page_count(base) > 1;
    // Menu.fillOutlinedRect(graphics, x + 3, (y - 13) + (multiPage ? 0 : 3), 145, 14, 10452863);
    menu::fill_outlined_rect(
        &mut graphics,
        x.wrapping_add(3),
        y.wrapping_sub(13)
            .wrapping_add(if multi_page { 0 } else { 3 }),
        145,
        14,
        10452863,
    );
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // FontManager.drawChars(graphics, x + 6, (y - 10) + (multiPage ? 0 : 3), this.title, 1);
    font_manager::draw_chars(
        font_manager,
        &mut graphics,
        x.wrapping_add(6),
        y.wrapping_sub(10)
            .wrapping_add(if multi_page { 0 } else { 3 }),
        title,
        1,
    );
    // drawListPage(graphics, x, y, multiPage);
    menu::draw_list_page(&mut graphics, base, x, y, multi_page);
    // for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
    //     Item item = slots[slot] >= 100 ? hero.getEquip(slots[slot] - 100)
    //               : slots[slot] < 0 ? hero.quickItems.get((-slots[slot]) - 1)
    //               : hero.bag.get((int) slots[slot]);
    //     if (item != null) Menu.drawItemIcon(graphics, x + 13, y + 18 + (23 * (slot % 5)), item, true);
    // }
    // Item selectedItem = ... (same resolution over cursorIndex) ...;
    // if (selectedItem != null) Menu.drawItemInfo(graphics, x + 33, y + 14, selectedItem);
    // (DEFERRED: the per-slot resolution uses the unported `Hero.getEquip`; the draws
    //  cross into unported art/text — Menu.drawItemIcon (AssetCache.itemIcons +
    //  BaseCanvas.drawNumberAt) and Menu.drawItemInfo (FontManager.drawWrappedText +
    //  AssetCache.commonText/heroText).)
}
