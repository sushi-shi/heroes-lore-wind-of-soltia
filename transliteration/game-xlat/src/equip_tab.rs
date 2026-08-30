//! Transliterated from `java/src/main/java/defpackage/EquipTab.java`
//! (original `bz.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Equipment tab (tab 2) of [`CharacterMenu`](crate::character_menu): the five
//! equipped slots (weapon / armour / two accessories / shield, mapped per class). OK
//! on a slot resolves the matching item category, and if the bag holds any candidates
//! opens an [`ItemPickerList`](crate::item_picker_list) to swap gear; the chosen item
//! is equipped on the popup callback (unidentified gear is rejected).
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `<init>`/`handleKey` are fully real — the per-slot/per-class
//! category `switch`, the empty-category `showMessage(StringTable.instance.get(3937))`,
//! the `bag.slotsOfType` candidate scan, and the `new ItemPickerList(…, text.get(16))`
//! push all land over modelled state. In `onPopupResult` the previous-child
//! `instanceof ItemPickerList` gate, the `((Equipment) bag.get(result)).identified`
//! check and the reject `showMessage` are real; the equip commit (`hero.equipItem`) is
//! DEFERRED (`Hero.equipItem` unported this lane, `hero.rs` untouched). In `paint` the
//! paginated `drawListPage` and the empty-selection `FontManager.drawChars`
//! (`text.get(21)`/`text.get(49)`) are drawn; the per-slot equip icons
//! (`Menu.drawItemIcon` / `AssetCache.equipSlotIcons`), the selected-item
//! `Menu.drawItemInfo`, and the `BaseCanvas.drawLabelBox(text.get(20))` header are
//! DEFERRED (unported art). `Hero.getEquip(slot)` is inlined as `hero.equipment[slot]`
//! (a one-line accessor over the modelled equipment array — the `enchant_menu`/
//! `character_menu` precedent).
//!
//! `EquipTab` has no fields of its own and no `static`s → no
//! `java/reconstruction/ownership.tsv` rows (its `Menu` base fields are per-INSTANCE on
//! [`EquipTabState`]).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bz.<init>:(Lcb;)V => []`,
//! `bz.a:(II)Z => []` (handleKey — pure branches/switch), `bz.a:(BB)V => []`
//! (onPopupResult — pure branches), `bz.a:(…Graphics;II)V => [iinc,iinc,iadd,iadd,iadd,
//! irem,imul,iadd,iadd,iadd,irem,imul,iadd,iinc,iadd,iadd,iadd,iadd,iadd,iadd]` (paint —
//! the ported `panelX=x+2`/`panelY=y+15` (iadd,iadd) + the empty-selection `drawChars`
//! geometry (iadd); the two `irem,imul` (`23*(slot%5)` in the if/else icon branches) +
//! the `slot++` iinc + icon geometry iadds live in the DEFERRED per-slot icon loop).

use crate::character_menu;
use crate::font_manager;
use crate::game::Game;
use crate::item_bag;
use crate::menu::{self, MenuChild, MenuNode};
use crate::string_table;

/// Java `bz` / `EquipTab` state — just the `Menu` (`cb`) base (`EquipTab` adds no
/// fields of its own).
#[derive(Debug, Default, Clone)]
pub struct EquipTabState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
}

/// `public EquipTab(Menu parentMenu)` (`bz.<init>:(Lcb;)V => []`): the five-slot equip tab.
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 5);   (parent is the CharacterMenu → non-null → present)
    g.equip_tab.base = menu::construct(true, 5);
}

/// `public final boolean handleKey(int action, int keyCode)` (`bz.a:(II)Z => []`):
/// child forward + non-wrapping vertical nav; FIRE resolves the slot's item category
/// and opens an `ItemPickerList` over the bag candidates (or a "no candidates"
/// message). Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Equip, action, key_code) {
        return true;
    }
    // if (moveCursorVerticalNoWrap(action, keyCode)) { ((Menu) this).parent.needsRepaint = true; return true; }
    if menu::move_cursor_vertical_no_wrap(&mut g.equip_tab.base, action, key_code) {
        if let Some(parent) = menu::parent_of(g, MenuNode::Equip) {
            menu::set_needs_repaint(g, parent, true);
        }
        return true;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // byte category = -1;
    let mut category: i8 = -1;
    let class_id = g.game_state.class_id;
    // switch (((Menu) this).cursorIndex) { ... }
    match g.equip_tab.base.cursor_index as i32 {
        // case 0: switch (classId) { case 6: category=0; case 7: category=2; case 8: category=1; }
        0 => match class_id as i32 {
            6 => category = 0,
            7 => category = 2,
            8 => category = 1,
            _ => {}
        },
        // case 1: if (classId == 8) category = 3;
        1 => {
            if class_id == 8 {
                category = 3;
            }
        }
        // case 2: category = 5;
        2 => category = 5,
        // case 3: category = 6;
        3 => category = 6,
        // case 4: category = 4;
        4 => category = 4,
        _ => {}
    }
    // if (category == -1) { showMessage({StringTable.instance.get(3937).toCharArray()}); return true; }
    if category == -1 {
        let msg = string_table::get(&g.string_table, 3937);
        menu::show_message(g, MenuNode::Equip, vec![msg]);
        return true;
    }
    // byte[] candidates = GameState.hero().bag.slotsOfType(category);
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let candidates = item_bag::slots_of_type(
        &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
        category,
    );
    // if (candidates.length > 0) { child = new ItemPickerList(this, candidates, cursorIndex, text.get(16)); return true; }
    if !candidates.is_empty() {
        let cursor = g.equip_tab.base.cursor_index;
        let title = character_menu::text_get(g, 16);
        crate::item_picker_list::construct(g, candidates, cursor, title);
        g.equip_tab.base.child = MenuChild::ItemPicker;
        return true;
    }
    // showMessage({StringTable.instance.get(3937).toCharArray()}); return true;
    let msg = string_table::get(&g.string_table, 3937);
    menu::show_message(g, MenuNode::Equip, vec![msg]);
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`bz.a:(BB)V => []`):
/// snapshots the previous child, runs the base dismiss (`super`), then — if a picker
/// answered (`previousChild instanceof ItemPickerList`, `tag != -1`) — equips the
/// chosen identified item or rejects it. The equip commit (`hero.equipItem`) is
/// DEFERRED (`Hero.equipItem` unported this lane).
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu previousChild = ((Menu) this).child;
    let previous_child = g.equip_tab.base.child;
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Equip, tag, result);
    // if (!(previousChild instanceof ItemPickerList) || tag == -1) return;
    if previous_child != MenuChild::ItemPicker || tag == -1 {
        return;
    }
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    // if (((Equipment) hero.bag.get((int) result)).identified) hero.equipItem(result, tag);
    let identified = item_bag::get(
        &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
        result as i32,
    )
    .expect("NullPointerException: EquipTab picked item")
    .borrow()
    .identified;
    if identified {
        // (DEFERRED: Hero.equipItem(result, tag) — Hero.equipItem unported this lane.)
    } else {
        // else showMessage({text.get(18), text.get(19)});
        let l18 = character_menu::text_get(g, 18);
        let l19 = character_menu::text_get(g, 19);
        menu::show_message(g, MenuNode::Equip, vec![l18, l19]);
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bz.a:(…Graphics;II)V`): the paginated equip list. The `drawListPage` grid and the
/// empty-selection label (`text.get(21)`/`text.get(49)`) are drawn; the per-slot equip
/// icons (`Menu.drawItemIcon` / `AssetCache.equipSlotIcons`), the selected-item info
/// panel (`Menu.drawItemInfo`), and the `BaseCanvas.drawLabelBox` header are DEFERRED.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int panelX = x + 2; int panelY = y + 15;
    let panel_x = x.wrapping_add(2);
    let panel_y = y.wrapping_add(15);
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let cursor = g.equip_tab.base.cursor_index as i32;
    let class_id = g.game_state.class_id;
    // Item selectedItem = hero.getEquip((int) cursorIndex);   (getEquip == hero.equipment[slot])
    let selected_is_some = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .equipment[cursor as usize]
        .is_some();
    // The empty-selection label + its x-origin (only read on the null-selection path).
    // if (cursorIndex != 1 || classId == 8) drawChars(text.get(21), panelX+33, …);
    // else drawChars(text.get(49), panelX+30, …);
    let (empty_text, empty_x) = if !selected_is_some {
        if cursor != 1 || class_id == 8 {
            (character_menu::text_get(g, 21), panel_x.wrapping_add(33))
        } else {
            (character_menu::text_get(g, 49), panel_x.wrapping_add(30))
        }
    } else {
        (Vec::new(), 0)
    };
    let base = g.equip_tab.base.clone();

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(20), panelX + 5, panelY);
    // (DEFERRED: BaseCanvas.drawLabelBox is unported.)
    // drawListPage(graphics, panelX, panelY, false);
    menu::draw_list_page(&mut graphics, &base, panel_x, panel_y, false);
    // for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
    //   Item item = hero.getEquip(slot);
    //   if (item != null) Menu.drawItemIcon(graphics, panelX+13, panelY+18+(23*(slot%5)), item, false);
    //   else graphics.drawImage(AssetCache.equipSlotIcons[slot], panelX+13, panelY+19+(23*(slot%5)), 3);
    // }
    // (DEFERRED: Menu.drawItemIcon + AssetCache.equipSlotIcons art unported — the per-slot
    //  icon loop (the two shape irem/imul `23*(slot%5)` + the `slot++` iinc) is skipped.)
    // if (selectedItem != null) { Menu.drawItemInfo(graphics, panelX+33, panelY+14, selectedItem); return; }
    // (DEFERRED: the selectedItem != null branch → Menu.drawItemInfo (unported art).)
    if !selected_is_some {
        // graphics.setColor(16777215);
        graphics.set_color(16777215);
        // FontManager.drawChars(graphics, empty_x, panelY + 14, empty_text, 1);
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            empty_x,
            panel_y.wrapping_add(14),
            &empty_text,
            1,
        );
    }
}
