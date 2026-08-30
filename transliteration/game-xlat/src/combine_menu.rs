//! Transliterated from `java/src/main/java/defpackage/CombineMenu.java`
//! (original `k.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Item-combine (crafting) sub-screen of [`RefineMenu`](crate::refine_menu),
//! `CombineMenu extends Menu`. The player fills up to three
//! [`craft_slots`](CombineMenuState::craft_slots) by picking quick-use items through
//! an [`ItemPickerList`](crate::item_picker_list); pressing the fourth (button) row
//! with at least two slots filled and 500 gold in hand pops a
//! [`CostConfirmDialog`](crate::cost_confirm_dialog), and confirming there routes back
//! through [`on_popup_result`] to consume the ingredients and
//! [`Item::craft`](crate::item::craft) them into a new item (added to the bag, or
//! refunded on failure). Reads its label strings from
//! [`RefineMenu.text`](crate::refine_menu::text_get).
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. The fill/craft transaction is real end-to-end: the
//! quick-usable pick resolution (`bag.get` + the inlined `Hero.getEquip` equipment read),
//! the duplicate-count guard over [`item_bag`](crate::item_bag), the
//! [`Item::craft`](crate::item::craft) recipe match, and the ingredient
//! `decrementItem`/`add` bag mutations. The popup/dialog dispatch threads the flat
//! [`menu`](crate::menu) child stack. Only the render's genuinely-unported hops are
//! DEFERRED (`Menu.drawLabelBox`/`drawQuickSlotRow`/`drawButton` — the item/gold draw
//! kit is unported); the panel fill + inset frame are drawn.
//!
//! `CombineMenu`'s `craftSlots` field is per-INSTANCE (the `Menu` base fields
//! likewise), so it contributes no `java/reconstruction/ownership.tsv` static rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `k.<init>:(Lcb;)V => []`
//! (constructor), `k.a:(II)Z => [iadd,i2b,iadd,i2b]` (handleKey — the two
//! `filled = (byte)(filled + 1)` promotions), `k.a:(BB)V => [isub,iinc,iinc]`
//! (onPopupResult — the `result - 100` equip-slot read + the `duplicateCount++` and
//! the loop `slot++`), `k.a:(…Graphics;II)V => [iadd,…,isub,ishr,iadd,iadd]` (paint —
//! the panel geometry + the `(155 - buttonWidth) >> 1` centring; the button/row draws
//! are DEFERRED).

use crate::cost_confirm_dialog;
use crate::debug;
use crate::game::Game;
use crate::item;
use crate::item_bag::{self, ItemRef};
use crate::menu::{self, MenuChild, MenuNode};
use crate::refine_menu;
use std::cell::RefCell;
use std::rc::Rc;

/// Java `k` / `CombineMenu` instance state — the `Menu` (`cb`) base fields plus the
/// crafting screen's own per-instance field.
#[derive(Debug, Default, Clone)]
pub struct CombineMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private Item[] craftSlots;` (obf `a`, `[Lad;`) — the up-to-three ingredient
    /// items staged for crafting (slots 0-2; each `None` == Java null).
    pub craft_slots: Vec<Option<ItemRef>>,
}

/// `public CombineMenu(Menu parentMenu)` (`k.<init>:(Lcb;)V => []`): `super(parentMenu,
/// (byte) 4); this.craftSlots = new Item[3];`.
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 4);   (parent is the pushing RefineMenu → present)
    g.combine_menu.base = menu::construct(true, 4);
    // this.craftSlots = new Item[3];
    g.combine_menu.craft_slots = vec![None, None, None];
}

/// `public final boolean handleKey(int action, int keyCode)`
/// (`k.a:(II)Z => [iadd,i2b,iadd,i2b]`): child forward (twice, as the Java does — a
/// verbatim `passKeyToChild` then `child.handleKey`) + vertical no-wrap nav; FIRE
/// (`53`/action 8) either opens the quick-usable picker for a slot row or — on the
/// button row — validates two-plus filled slots + 500 gold and opens the
/// [`CostConfirmDialog`](crate::cost_confirm_dialog).
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Combine, action, key_code) {
        return true;
    }
    // if ((((Menu) this).child != null && ((Menu) this).child.handleKey(action, keyCode))
    //     || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::child_handle_key(g, MenuNode::Combine, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.combine_menu.base, action, key_code)
    {
        return true;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // Hero hero = GameState.hero();
    let hero = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let cursor = g.combine_menu.base.cursor_index;
    // if (((Menu) this).cursorIndex < 3) {
    if (cursor as i32) < 3 {
        // byte[] usableSlots = hero.bag.quickUsableSlots();
        let usable_slots =
            item_bag::quick_usable_slots(&g.entity_arena[hero].as_hero().expect("Hero node").bag);
        // if (usableSlots.length < 1) { showMessage({text.get(20)}); return true; }
        if usable_slots.is_empty() {
            let line = refine_menu::text_get(g, 20);
            menu::show_message(g, MenuNode::Combine, vec![line]);
            return true;
        }
        // ((Menu) this).child = new ItemPickerList(this, usableSlots, cursorIndex, text.get(21));
        let title = refine_menu::text_get(g, 21);
        crate::item_picker_list::construct(g, usable_slots, cursor, title);
        g.combine_menu.base.child = MenuChild::ItemPicker;
        // return true;
        return true;
    }
    // byte filled = 0; Object[] names = new Object[3];
    let mut filled: i8 = 0;
    let mut names: Vec<Option<Vec<u16>>> = vec![None, None, None];
    // if (this.craftSlots[0] != null) { filled = 1; names[0] = this.craftSlots[0].name; }
    if let Some(slot0) = g.combine_menu.craft_slots[0].clone() {
        filled = 1;
        names[0] = Some(slot0.borrow().name.clone());
    }
    // if (this.craftSlots[1] != null) { byte slotIndex = filled; filled = (byte)(filled+1); names[slotIndex] = craftSlots[1].name; }
    if let Some(slot1) = g.combine_menu.craft_slots[1].clone() {
        let slot_index = filled;
        filled = (filled as i32).wrapping_add(1) as i8;
        names[slot_index as usize] = Some(slot1.borrow().name.clone());
    }
    // if (this.craftSlots[2] != null) { byte slotIndex = filled; filled = (byte)(filled+1); names[slotIndex] = craftSlots[2].name; }
    if let Some(slot2) = g.combine_menu.craft_slots[2].clone() {
        let slot_index = filled;
        filled = (filled as i32).wrapping_add(1) as i8;
        names[slot_index as usize] = Some(slot2.borrow().name.clone());
    }
    // if (filled < 2) { showMessage({text.get(22), text.get(23)}); return true; }
    if filled < 2 {
        let l22 = refine_menu::text_get(g, 22);
        let l23 = refine_menu::text_get(g, 23);
        menu::show_message(g, MenuNode::Combine, vec![l22, l23]);
        return true;
    }
    // if (500 > hero.bag.gold) { showMessage({text.get(24)}); return true; }
    let gold = g.entity_arena[hero].as_hero().expect("Hero node").bag.gold;
    if 500 > gold {
        let line = refine_menu::text_get(g, 24);
        menu::show_message(g, MenuNode::Combine, vec![line]);
        return true;
    }
    // ((Menu) this).child = new CostConfirmDialog(this, text.get(25), names, text.get(26), 500, (byte) 20);
    let title = refine_menu::text_get(g, 25);
    let cost_label = refine_menu::text_get(g, 26);
    cost_confirm_dialog::construct(g, title, names, cost_label, 500, 20);
    g.combine_menu.base.child = MenuChild::CostConfirm;
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`k.a:(BB)V =>
/// [isub,iinc,iinc]`): runs the base dismiss (`super`), then dispatches on the
/// previous child. A picker pick (tag 0/1/2) stages an ingredient (guarding the
/// available quantity); a `CostConfirmDialog` OK (tag 20) opens the final yes/no
/// popup; a popup yes (tag 2, result 0) crafts — consuming the ingredients and adding
/// the result (or refunding on a bag-full failure / clearing on a no-recipe failure).
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu child = ((Menu) this).child;
    let child = g.combine_menu.base.child;
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Combine, tag, result);
    // if (!(child instanceof PopupMenu) || tag != 2 || result != 0) {
    if child != MenuChild::Popup || tag != 2 || result != 0 {
        // if (!(child instanceof ItemPickerList) || (tag != 0 && tag != 1 && tag != 2)) {
        if child != MenuChild::ItemPicker || (tag != 0 && tag != 1 && tag != 2) {
            // if ((child instanceof CostConfirmDialog) && tag == 20) {
            if child == MenuChild::CostConfirm && tag == 20 {
                // showPopup((byte) 2, (byte) 2, new Object[]{RefineMenu.text.get(32)});
                let line = refine_menu::text_get(g, 32);
                menu::show_popup(g, MenuNode::Combine, 2, 2, vec![line]);
                // return;
                return;
            }
            // return;
            return;
        }
        // Hero hero = GameState.hero();
        let hero = g
            .game_state
            .hero
            .expect("NullPointerException: GameState.hero()");
        // Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
        let picked: ItemRef = if result >= 100 {
            // Hero.getEquip(slot) == hero.equipment[slot] (inlined — see enchant_menu).
            g.entity_arena[hero].as_hero().expect("Hero node").equipment
                [(result as i32).wrapping_sub(100) as usize]
                .clone()
                .expect("NullPointerException: CombineMenu picked equip")
        } else {
            item_bag::get(
                &g.entity_arena[hero].as_hero().expect("Hero node").bag,
                result as i32,
            )
            .expect("NullPointerException: CombineMenu picked bag item")
        };
        // Debug.assertTrue(Item.QUICK_USABLE[picked.type]);
        let (picked_type, picked_sub_id) = {
            let b = picked.borrow();
            (b.r#type, b.sub_id)
        };
        debug::assert_true(item::QUICK_USABLE[picked_type as usize]);
        // int duplicateCount = 0;
        let mut duplicate_count: i32 = 0;
        // for (int slot = 0; slot < 3; slot++) {
        let mut slot: i32 = 0;
        while slot < 3 {
            // if (tag != slot && craftSlots[slot] != null && craftSlots[slot].type == picked.type && craftSlots[slot].subId == picked.subId) duplicateCount++;
            if (tag as i32) != slot {
                if let Some(cs) = &g.combine_menu.craft_slots[slot as usize] {
                    let b = cs.borrow();
                    if b.r#type == picked_type && b.sub_id == picked_sub_id {
                        duplicate_count = duplicate_count.wrapping_add(1);
                    }
                }
            }
            slot = slot.wrapping_add(1);
        }
        // if (hero.bag.totalQuantity(picked.type, picked.subId) <= duplicateCount) {
        let total = item_bag::total_quantity(
            &g.entity_arena[hero].as_hero().expect("Hero node").bag,
            picked_type,
            picked_sub_id,
        );
        if total <= duplicate_count {
            // showMessage(new Object[]{RefineMenu.text.get(31)}); return;
            let line = refine_menu::text_get(g, 31);
            menu::show_message(g, MenuNode::Combine, vec![line]);
            return;
        } else {
            // this.craftSlots[tag] = picked; return;
            g.combine_menu.craft_slots[tag as usize] = Some(picked);
            return;
        }
    }
    // Hero hero2 = GameState.hero();
    let hero2 = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    // Item crafted = Item.craft(this.craftSlots[0], this.craftSlots[1], this.craftSlots[2]);
    let ingredient_a = g.combine_menu.craft_slots[0]
        .as_ref()
        .map(|rc| rc.borrow().clone());
    let ingredient_b = g.combine_menu.craft_slots[1]
        .as_ref()
        .map(|rc| rc.borrow().clone());
    let ingredient_c = g.combine_menu.craft_slots[2]
        .as_ref()
        .map(|rc| rc.borrow().clone());
    let crafted = item::craft(g, ingredient_a, ingredient_b, ingredient_c);
    // if (crafted == null) {
    if crafted.is_none() {
        // if (craftSlots[0] != null) hero2.bag.decrementItem(craftSlots[0], (byte) 1); (×3)
        decrement_filled_slots(g, hero2);
        // this.craftSlots[0..2] = null;
        g.combine_menu.craft_slots[0] = None;
        g.combine_menu.craft_slots[1] = None;
        g.combine_menu.craft_slots[2] = None;
        // showMessage(new Object[]{RefineMenu.text.get(30)}); return;
        let line = refine_menu::text_get(g, 30);
        menu::show_message(g, MenuNode::Combine, vec![line]);
        return;
    }
    let crafted = crafted.expect("Item.craft succeeded");
    // if (craftSlots[0] != null) hero2.bag.decrementItem(craftSlots[0], (byte) 1); (×3)
    decrement_filled_slots(g, hero2);
    // if (hero2.bag.add(crafted, 1)) {
    let crafted_ref: ItemRef = Rc::new(RefCell::new(crafted));
    let (crafted_type, crafted_sub_id) = {
        let b = crafted_ref.borrow();
        (b.r#type, b.sub_id)
    };
    let added = {
        let bag = &mut g.entity_arena[hero2].as_hero_mut().expect("Hero node").bag;
        item_bag::add(bag, crafted_ref, 1)
    };
    if added {
        // ((Menu) this).child = new ItemPickerList(this, new byte[]{hero2.bag.findSlot(crafted.type, crafted.subId)}, (byte) 10, text.get(27));
        let slot = item_bag::find_slot(
            &g.entity_arena[hero2].as_hero().expect("Hero node").bag,
            crafted_type,
            crafted_sub_id,
        );
        let title = refine_menu::text_get(g, 27);
        crate::item_picker_list::construct(g, vec![slot], 10, title);
        g.combine_menu.base.child = MenuChild::ItemPicker;
        // this.craftSlots[0..2] = null;
        g.combine_menu.craft_slots[0] = None;
        g.combine_menu.craft_slots[1] = None;
        g.combine_menu.craft_slots[2] = None;
        // return;
        return;
    }
    // if (craftSlots[0] != null) hero2.bag.add(craftSlots[0], 1); (×3) — refund on bag-full
    for k in 0..3usize {
        if let Some(cs) = g.combine_menu.craft_slots[k].clone() {
            let bag = &mut g.entity_arena[hero2].as_hero_mut().expect("Hero node").bag;
            item_bag::add(bag, cs, 1);
        }
    }
    // showMessage(new Object[]{RefineMenu.text.get(28), RefineMenu.text.get(29)});
    let l28 = refine_menu::text_get(g, 28);
    let l29 = refine_menu::text_get(g, 29);
    menu::show_message(g, MenuNode::Combine, vec![l28, l29]);
}

/// The verbatim `if (craftSlots[k] != null) hero.bag.decrementItem(craftSlots[k], (byte) 1);`
/// triple that both craft outcomes run before diverging (consume the staged
/// ingredients). Kept as a helper only to avoid textual duplication — the three
/// guarded `decrementItem(craftSlots[0..2], 1)` calls are transliterated verbatim.
fn decrement_filled_slots(g: &mut Game, hero: crate::entity::EntityId) {
    for k in 0..3usize {
        if let Some(cs) = g.combine_menu.craft_slots[k].clone() {
            item_bag::decrement_item(
                &mut g.entity_arena[hero].as_hero_mut().expect("Hero node").bag,
                &cs,
                1,
            );
        }
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`k.a:(…Graphics;II)V`): draws the panel fill + inset frame. The label box, the
/// three quick-slot rows and the confirm button are DEFERRED (see the module header).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    let Game { screen, .. } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(x, y, 155, 170);
    graphics.fill_rect(x, y, 155, 170);
    // Menu.drawInsetPanel(graphics, x + 2, y + 4, 151, 162);
    menu::draw_inset_panel(
        &mut graphics,
        x.wrapping_add(2),
        y.wrapping_add(4),
        151,
        162,
    );
    // BaseCanvas.drawLabelBox(graphics, RefineMenu.text.get(14), x + 3, y - 2);
    // Menu.drawQuickSlotRow(graphics, x + 4, y + 9,      craftSlots[0], (byte) 1, text.get(33), cursorIndex == 0);
    // Menu.drawQuickSlotRow(graphics, x + 4, y + 9 + 36, craftSlots[1], (byte) 2, text.get(33), cursorIndex == 1);
    // Menu.drawQuickSlotRow(graphics, x + 4, y + 9 + 72, craftSlots[2], (byte) 3, text.get(33), cursorIndex == 2);
    // int buttonWidth = FontManager.percentOf(155, 80);
    // Menu.drawButton(graphics, x + ((155 - buttonWidth) >> 1), y + 138, buttonWidth, text.get(25), cursorIndex == 3);
    // (DEFERRED: BaseCanvas.drawLabelBox + Menu.drawQuickSlotRow + Menu.drawButton — the
    //  item/gold draw kit is unported.)
}
