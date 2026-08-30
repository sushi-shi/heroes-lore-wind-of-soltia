//! Transliterated from `java/src/main/java/defpackage/EnchantMenu.java`
//! (original `ap.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The refinery's armor-enchant screen (`EnchantMenu extends Menu`), pushed by
//! [`RefineMenu`](crate::refine_menu). The player picks an identified, un-enchanted
//! [`armor`](EnchantMenuState::armor) piece and an enchant-scroll
//! [`material`](EnchantMenuState::material) (item type 17); confirming spends 500
//! gold, consumes the scroll and stamps the scroll's element (`Item.subId`) onto the
//! armor's `Armor.attribute`. Reads its label strings from
//! [`RefineMenu.text`](crate::refine_menu::text_get).
//!
//! ## ANTI-BOG boundary
//!
//! The constructor, `onPopupResult` and the confirm/scroll-pick paths of `handleKey`
//! are ported **fully** — the enchant transaction (armor `attribute` stamp, `gold -=
//! 500`, scroll `decrementItem`) over [`item_bag`](crate::item_bag), the picker-pick
//! resolution (`bag.get` + the inlined `Hero.getEquip` equipment-array read) and the
//! identify/enchanted validation are real; the popup/message dispatch threads the flat
//! [`menu`](crate::menu) child stack. Only the genuinely-unported hops are DEFERRED:
//! the `cursorIndex == 0` **armor picker** needs `Hero.serializeItems` (unported this
//! lane; `hero.rs` untouched), and the post-enchant **results picker** needs
//! `Hero.slotOf` (likewise) — both DEFERRED with named comments (the mutations they
//! follow are complete). `paint` is **PARTIAL**: the panel fill + inset frame + the
//! outlined attribute row are drawn; the label box, the two quick-slot rows, the
//! attribute/cost detail block (`Armor.attributeNames` unported) and the confirm button
//! (`Menu.drawLabelBox`/`drawQuickSlotRow`/`drawGold`/`drawButton` — the item/gold draw
//! kit is unported) are DEFERRED.
//!
//! `EnchantMenu`'s fields (`armor`, `material`) are per-INSTANCE (the `Menu` base
//! fields likewise), so it contributes no `java/reconstruction/ownership.tsv` static
//! rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ap.<init>:(Lcb;)V => []`
//! (constructor), `ap.a:(II)Z => []` (handleKey — pure branches),
//! `ap.a:(BB)V => [isub,isub]` (onPopupResult — the `result - 100` equip-slot read +
//! the `gold -= 500` charge), `ap.a:(…Graphics;II)V => [iadd,…]` (paint — the ported
//! panel/frame geometry; the remaining adds live in the DEFERRED row/detail draws).

use crate::debug;
use crate::game::Game;
use crate::item;
use crate::item_bag::{self, ItemRef};
use crate::menu::{self, MenuChild, MenuNode};
use crate::refine_menu;

/// Java `ap` / `EnchantMenu` instance state — the `Menu` (`cb`) base fields plus the
/// enchant screen's own per-instance fields.
#[derive(Debug, Default, Clone)]
pub struct EnchantMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private Armor armor;` (obf `a`, `Lt;`) — the armor piece being enchanted
    /// (must be identified and un-enchanted). `None` == Java null.
    pub armor: Option<ItemRef>,
    /// `private Item material;` (obf `a`, `Lad;`) — the enchant scroll (item type 17)
    /// whose element is applied. `None` == Java null.
    pub material: Option<ItemRef>,
}

/// `public EnchantMenu(Menu parent)` (`ap.<init>:(Lcb;)V => []`): `super(parent, (byte) 3)`.
pub fn construct(g: &mut Game) {
    // super(parent, (byte) 3);   (parent is the pushing RefineMenu → present)
    g.enchant_menu.base = menu::construct(true, 3);
    // (armor / material default null)
    g.enchant_menu.armor = None;
    g.enchant_menu.material = None;
}

/// `public final boolean handleKey(int action, int keyCode)` (`ap.a:(II)Z => []`):
/// child forward + vertical no-wrap nav; FIRE (`53`/action 8) drives the selected row —
/// the armor picker (DEFERRED: `Hero.serializeItems`), the scroll picker, or the
/// confirm popup (with the missing-armor / missing-scroll / low-gold guards).
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Enchant, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.enchant_menu.base, action, key_code)
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
    let cursor = g.enchant_menu.base.cursor_index;
    // if (((Menu) this).cursorIndex == 0) {
    if cursor == 0 {
        // byte[] armorSlots = hero.serializeItems(true, (byte) 1);
        // Debug.assertTrue(armorSlots.length > 0);
        // ((Menu) this).child = new ItemPickerList(this, armorSlots, cursorIndex, RefineMenu.text.get(3));
        // return true;
        // (DEFERRED: Hero.serializeItems is unported this lane (hero.rs untouched), so the
        //  identified-armor picker cannot be built here. The pick RESOLUTION it feeds —
        //  onPopupResult tag 0 — is fully ported and is driven directly by the craft oracle.)
        return true;
    }
    // if (((Menu) this).cursorIndex == 1) {
    if cursor == 1 {
        // byte[] scrollSlots = hero.bag.slotsOfType((byte) 17);
        let scroll_slots =
            item_bag::slots_of_type(&g.entity_arena[hero].as_hero().expect("Hero node").bag, 17);
        // if (scrollSlots.length > 0) { child = new ItemPickerList(this, scrollSlots, cursorIndex, text.get(4)); return true; }
        if !scroll_slots.is_empty() {
            let title = refine_menu::text_get(g, 4);
            crate::item_picker_list::construct(g, scroll_slots, cursor, title);
            g.enchant_menu.base.child = MenuChild::ItemPicker;
            return true;
        }
        // showMessage(new Object[]{RefineMenu.text.get(5)});
        let line = refine_menu::text_get(g, 5);
        menu::show_message(g, MenuNode::Enchant, vec![line]);
        // return true;
        return true;
    }
    // if (((Menu) this).cursorIndex != 2) return true;
    if cursor != 2 {
        return true;
    }
    // if (this.armor == null) { showMessage({text.get(6)}); return true; }
    if g.enchant_menu.armor.is_none() {
        let line = refine_menu::text_get(g, 6);
        menu::show_message(g, MenuNode::Enchant, vec![line]);
        return true;
    }
    // if (this.material == null) { showMessage({text.get(7)}); return true; }
    if g.enchant_menu.material.is_none() {
        let line = refine_menu::text_get(g, 7);
        menu::show_message(g, MenuNode::Enchant, vec![line]);
        return true;
    }
    // if (hero.bag.gold < 500) { showMessage({text.get(8)}); return true; }
    let gold = g.entity_arena[hero].as_hero().expect("Hero node").bag.gold;
    if gold < 500 {
        let line = refine_menu::text_get(g, 8);
        menu::show_message(g, MenuNode::Enchant, vec![line]);
        return true;
    }
    // showPopup((byte) 2, (byte) 2, new Object[]{RefineMenu.text.get(9)});
    let line = refine_menu::text_get(g, 9);
    menu::show_popup(g, MenuNode::Enchant, 2, 2, vec![line]);
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`ap.a:(BB)V => [isub,isub]`):
/// runs the base dismiss (`super`), then — on a confirmed enchant (previous child a
/// popup, tag 2, result 0) — stamps the scroll's element onto the armor, charges 500
/// gold and consumes the scroll; or, on a picker pick (previous child an
/// `ItemPickerList`, tag 0/1) resolves and validates the chosen armor/scroll.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu previousChild = ((Menu) this).child;
    let previous_child = g.enchant_menu.base.child;
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Enchant, tag, result);
    // if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
    if previous_child == MenuChild::Popup && tag == 2 && result == 0 {
        // Hero hero = GameState.hero();
        let hero = g
            .game_state
            .hero
            .expect("NullPointerException: GameState.hero()");
        let armor = g
            .enchant_menu
            .armor
            .clone()
            .expect("NullPointerException: EnchantMenu.armor");
        let material = g
            .enchant_menu
            .material
            .clone()
            .expect("NullPointerException: EnchantMenu.material");
        // this.armor.attribute = this.material.subId;
        let sub_id = material.borrow().sub_id;
        armor.borrow_mut().attribute = sub_id;
        // hero.bag.gold -= 500;
        {
            let bag = &mut g.entity_arena[hero].as_hero_mut().expect("Hero node").bag;
            bag.gold = bag.gold.wrapping_sub(500);
        }
        // hero.bag.decrementItem(this.material, (byte) 1);
        item_bag::decrement_item(
            &mut g.entity_arena[hero].as_hero_mut().expect("Hero node").bag,
            &material,
            1,
        );
        // ((Menu) this).child = new ItemPickerList(this, new byte[]{hero.slotOf((Item) this.armor)}, (byte) 10, RefineMenu.text.get(10));
        // (DEFERRED: Hero.slotOf is unported this lane (hero.rs untouched) — its bag-slot
        //  primary path is ItemBag.slotOf (ported), but the equipment-array fallback is
        //  not, so the just-enchanted armor's results picker cannot be built. The enchant
        //  state mutations above are complete.)
        // this.armor = null; this.material = null;
        g.enchant_menu.armor = None;
        g.enchant_menu.material = None;
        return;
    }
    // if (previousChild instanceof ItemPickerList) {
    if previous_child == MenuChild::ItemPicker {
        // if (tag == 0 || tag == 1) {
        if tag == 0 || tag == 1 {
            let hero = g
                .game_state
                .hero
                .expect("NullPointerException: GameState.hero()");
            // Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
            let picked: Option<ItemRef> = if result >= 100 {
                // Hero.getEquip(slot) == hero.equipment[slot] (inlined: a one-line accessor
                // over the modelled equipment array — precedent in character_menu/asset_loader).
                g.entity_arena[hero].as_hero().expect("Hero node").equipment
                    [(result as i32).wrapping_sub(100) as usize]
                    .clone()
            } else {
                item_bag::get(
                    &g.entity_arena[hero].as_hero().expect("Hero node").bag,
                    result as i32,
                )
            };
            // if (tag != 0) { Debug.assertTrue(picked.type == 17); this.material = picked; return; }
            if tag != 0 {
                let p = picked
                    .clone()
                    .expect("NullPointerException: EnchantMenu picked material");
                debug::assert_true(p.borrow().r#type == 17);
                g.enchant_menu.material = picked;
                return;
            }
            // Debug.assertTrue(picked instanceof Armor);
            let p = picked
                .clone()
                .expect("NullPointerException: EnchantMenu picked armor");
            debug::assert_true(item::is_armor(&p.borrow()));
            // Armor pickedArmor = (Armor) picked;
            let (identified, attribute) = {
                let b = p.borrow();
                (b.identified, b.attribute)
            };
            // if (!((Equipment) pickedArmor).identified) showMessage({text.get(11), text.get(13)});
            if !identified {
                let l11 = refine_menu::text_get(g, 11);
                let l13 = refine_menu::text_get(g, 13);
                menu::show_message(g, MenuNode::Enchant, vec![l11, l13]);
            // } else if (pickedArmor.attribute != -1) showMessage({text.get(12), text.get(13)});
            } else if attribute != -1 {
                let l12 = refine_menu::text_get(g, 12);
                let l13 = refine_menu::text_get(g, 13);
                menu::show_message(g, MenuNode::Enchant, vec![l12, l13]);
            } else {
                // this.armor = (Armor) picked;
                g.enchant_menu.armor = picked;
            }
        }
    }
}

/// `public final void paint(Graphics graphics, int originX, int originY)`
/// (`ap.a:(…Graphics;II)V`): draws the panel fill + inset frame + the outlined
/// attribute row. The label box, the two quick-slot rows, the attribute/cost detail
/// block and the confirm button are DEFERRED (see the module header).
pub fn paint(g: &mut Game, origin_x: i32, origin_y: i32) {
    let Game { screen, .. } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(originX, originY, 155, 170);
    graphics.fill_rect(origin_x, origin_y, 155, 170);
    // Menu.drawInsetPanel(graphics, originX + 2, originY + 4, 151, 162);
    menu::draw_inset_panel(
        &mut graphics,
        origin_x.wrapping_add(2),
        origin_y.wrapping_add(4),
        151,
        162,
    );
    // BaseCanvas.drawLabelBox(graphics, RefineMenu.text.get(14), originX + 3, originY - 2);
    // Menu.drawQuickSlotRow(graphics, originX + 4, originY + 9, this.armor, (byte) 1, text.get(15), cursorIndex == 0);
    // Menu.drawQuickSlotRow(graphics, originX + 4, originY + 9 + 36, this.material, (byte) 2, text.get(16), cursorIndex == 1);
    // (DEFERRED: BaseCanvas.drawLabelBox + Menu.drawQuickSlotRow — the item/gold draw kit
    //  is unported.)
    // Menu.fillOutlinedRect(graphics, originX + 4, originY + 9 + 72, 147, 31, 12558207);
    menu::fill_outlined_rect(
        &mut graphics,
        origin_x.wrapping_add(4),
        origin_y.wrapping_add(9).wrapping_add(72),
        147,
        31,
        12558207,
    );
    // if (this.armor != null && this.material != null) { ... attributeNames + drawGold ... }
    // int buttonWidth = FontManager.percentOf(155, 80);
    // Menu.drawButton(graphics, originX + ((155 - buttonWidth) >> 1), originY + 138, buttonWidth, text.get(19), cursorIndex == 2);
    // (DEFERRED: the armor/material detail block chains through the unported
    //  Armor.attributeNames width + Menu.drawGold, and the confirm button needs the
    //  unported Menu.drawButton.)
}
