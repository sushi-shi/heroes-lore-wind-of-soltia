//! Transliterated from `java/src/main/java/defpackage/ItemsTab.java`
//! (original `ay.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Items tab (tab 1) of [`CharacterMenu`](crate::character_menu): the hero's carried
//! bag ([`Hero.bag`](crate::hero), 30 slots). OK on a slot offers the appropriate
//! action via a popup — equip for gear the class can wear, use for consumables — or a
//! "cannot" message; drop is offered otherwise. The popup result then equips, uses, or
//! drops the selected item.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `<init>`/`handleKey` are fully real — the FIRE dispatch
//! reads the bag item's runtime class (`instanceof Equipment`/`Weapon`),
//! `isUsable`/`isQuestItem`/`identified`/`type` and `GameState.classId` (all modelled)
//! and pushes the equip/use/cannot/drop popups through the ported `showPopup`/
//! `showMessage`. In `onPopupResult` the drop (`bag.removeFromSlot`) and the
//! re-prompt (`showPopup(6, …)`) paths are real; the equip (`hero.equipItem`) and use
//! (`hero.useItem`) commits are DEFERRED — those two `Hero` mutators are not ported
//! this lane (`hero.rs` untouched). In `paint` the paginated `drawListPage` and the
//! empty-selection `FontManager.drawChars(text.get(15))` are drawn; the per-slot
//! `Menu.drawItemIcon` loop, the selected-item `Menu.drawItemInfo`, and the
//! `BaseCanvas.drawLabelBox(AssetCache.commonText.get(2))` header are DEFERRED (those
//! item-icon / label widgets + the `AssetCache.commonText` bank are unported).
//!
//! `ItemsTab` has no fields of its own (it extends `Menu`) and no `static`s, so it
//! contributes no `java/reconstruction/ownership.tsv` rows (its `Menu` base fields are
//! per-INSTANCE on [`ItemsTabState`]).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ay.<init>:(Lcb;)V => []`,
//! `ay.a:(II)Z => []` (handleKey — pure branches), `ay.a:(BB)V => []` (onPopupResult —
//! pure branches; the `hero.equipItem`/`useItem` targets carry no arithmetic on this
//! side), `ay.a:(…Graphics;II)V => [iinc,iinc,iadd,iadd,iadd,irem,imul,iadd,iinc,iadd,
//! iadd,iadd,iadd]` (paint — the ported prefix is `panelX=x+2`/`panelY=y+15` (iadd,
//! iadd) + the empty-selection `drawChars` at `panelX+33`/`panelY+14` (iadd,iadd); the
//! `irem,imul` (`23*(slot%5)`) + the `slot++` iinc + the icon geometry iadds live in
//! the DEFERRED per-slot icon loop).

use crate::character_menu;
use crate::font_manager;
use crate::game::Game;
use crate::item::{self, ItemClass};
use crate::item_bag;
use crate::menu::{self, MenuNode};

/// Java `ay` / `ItemsTab` state — just the `Menu` (`cb`) base (`ItemsTab` adds no
/// fields of its own).
#[derive(Debug, Default, Clone)]
pub struct ItemsTabState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
}

/// `public ItemsTab(Menu parentMenu)` (`ay.<init>:(Lcb;)V => []`): the 30-slot bag tab.
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 30);   (parent is the CharacterMenu → non-null → present)
    g.items_tab.base = menu::construct(true, 30);
}

/// `public final boolean handleKey(int action, int keyCode)` (`ay.a:(II)Z => []`):
/// child forward + non-wrapping vertical nav; FIRE resolves the selected bag item and
/// pushes the equip/use/cannot/drop popup. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Items, action, key_code) {
        return true;
    }
    // if (moveCursorVerticalNoWrap(action, keyCode)) { ((Menu) this).parent.needsRepaint = true; return true; }
    if menu::move_cursor_vertical_no_wrap(&mut g.items_tab.base, action, key_code) {
        if let Some(parent) = menu::parent_of(g, MenuNode::Items) {
            menu::set_needs_repaint(g, parent, true);
        }
        return true;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // Item item = GameState.hero().bag.get((int) ((Menu) this).cursorIndex);
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let cursor = g.items_tab.base.cursor_index as i32;
    let item = item_bag::get(
        &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
        cursor,
    );
    // if (item == null) return true;
    let item = match item {
        None => return true,
        Some(it) => it,
    };
    // Snapshot the runtime class + fields (an Rc clone; drop the borrow before text.get).
    let (is_equipment, is_weapon, is_usable, is_quest, identified, kind) = {
        let b = item.borrow();
        (
            item::is_equipment(&b),
            b.class == ItemClass::Weapon,
            item::is_usable(&b),
            item::is_quest_item(&b),
            b.identified,
            b.r#type,
        )
    };
    let class_id = g.game_state.class_id;
    // if (!(item instanceof Equipment)) {
    if !is_equipment {
        // if (item.isUsable()) { showPopup(5, 2, {text.get(13), text.get(10)}); return true; }
        if is_usable {
            let l13 = character_menu::text_get(g, 13);
            let l10 = character_menu::text_get(g, 10);
            menu::show_popup(g, MenuNode::Items, 5, 2, vec![l13, l10]);
            return true;
        }
        // if (item.isQuestItem()) { showMessage({text.get(14)}); return true; }
        if is_quest {
            let l14 = character_menu::text_get(g, 14);
            menu::show_message(g, MenuNode::Items, vec![l14]);
            return true;
        }
        // showPopup(6, 2, {text.get(12)}); return true;
        let l12 = character_menu::text_get(g, 12);
        menu::show_popup(g, MenuNode::Items, 6, 2, vec![l12]);
        return true;
    }
    // Equipment equipment = (Equipment) item;
    // if (!equipment.identified) { showPopup(6, 2, {text.get(12)}); return true; }
    if !identified {
        let l12 = character_menu::text_get(g, 12);
        menu::show_popup(g, MenuNode::Items, 6, 2, vec![l12]);
        return true;
    }
    // if (!(equipment instanceof Weapon)) {
    if !is_weapon {
        // if (equipment.type != 3 || GameState.classId == 8) { showPopup(4, 2, {text.get(11), text.get(10)}); return true; }
        if kind != 3 || class_id == 8 {
            let l11 = character_menu::text_get(g, 11);
            let l10 = character_menu::text_get(g, 10);
            menu::show_popup(g, MenuNode::Items, 4, 2, vec![l11, l10]);
            return true;
        }
        // showPopup(6, 2, {text.get(12)}); return true;
        let l12 = character_menu::text_get(g, 12);
        menu::show_popup(g, MenuNode::Items, 6, 2, vec![l12]);
        return true;
    }
    // if ((classId==6 && type==0) || (classId==7 && type==2) || (classId==8 && type==1)) {
    //   showPopup(4, 2, {text.get(11), text.get(10)}); return true; }
    if (class_id == 6 && kind == 0) || (class_id == 7 && kind == 2) || (class_id == 8 && kind == 1)
    {
        let l11 = character_menu::text_get(g, 11);
        let l10 = character_menu::text_get(g, 10);
        menu::show_popup(g, MenuNode::Items, 4, 2, vec![l11, l10]);
        return true;
    }
    // Object[] messageLines = new Object[2];
    let mut message_lines: Vec<Vec<u16>> = vec![Vec::new(), Vec::new()];
    // if (type==0) messageLines[0]=text.get(8); else if (type==2) messageLines[0]=text.get(9);
    // else if (type==1) messageLines[0]=text.get(50);
    if kind == 0 {
        message_lines[0] = character_menu::text_get(g, 8);
    } else if kind == 2 {
        message_lines[0] = character_menu::text_get(g, 9);
    } else if kind == 1 {
        message_lines[0] = character_menu::text_get(g, 50);
    }
    // messageLines[1] = text.get(12);
    message_lines[1] = character_menu::text_get(g, 12);
    // showPopup(6, 2, messageLines); return true;
    menu::show_popup(g, MenuNode::Items, 6, 2, message_lines);
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`ay.a:(BB)V => []`):
/// runs the base dismiss (`super`), then commits the selected action. The equip
/// (`hero.equipItem`) and use (`hero.useItem`) commits are DEFERRED (`Hero` mutators
/// unported this lane); the drop (`bag.removeFromSlot`) and re-prompt (`showPopup(6, …)`)
/// paths are real.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Items, tag, result);
    // Hero hero = GameState.hero(); ItemBag bag = hero.bag;
    // if (tag == 4 && result == 0) { switch (((Equipment) bag.get(cursorIndex)).type) { ... hero.equipItem(cursorIndex, N); } return; }
    if tag == 4 && result == 0 {
        // (DEFERRED: the equip switch commits via Hero.equipItem(cursorIndex, {0|1|4|2|3}) —
        //  Hero.equipItem is unported this lane (hero.rs untouched). The switch reads
        //  ((Equipment) bag.get(cursorIndex)).type (modelled) only to pick the equip slot.)
        return;
    }
    // if (tag == 5 && result == 0) { hero.useItem(bag.get(cursorIndex)); return; }
    if tag == 5 && result == 0 {
        // (DEFERRED: Hero.useItem(bag.get(cursorIndex)) — Hero.useItem is unported this lane.)
        return;
    }
    // if ((tag==4 && result==1) || (tag==5 && result==1)) showPopup(6, 2, {text.get(12)});
    if (tag == 4 && result == 1) || (tag == 5 && result == 1) {
        let l12 = character_menu::text_get(g, 12);
        menu::show_popup(g, MenuNode::Items, 6, 2, vec![l12]);
    // else if (tag == 6 && result == 0) bag.removeFromSlot(cursorIndex, (byte) 1);
    } else if tag == 6 && result == 0 {
        let cursor = g.items_tab.base.cursor_index;
        item_bag::remove_from_slot(
            &mut g.entity_arena[g
                .game_state
                .hero
                .expect("NullPointerException: GameState.hero()")]
            .as_hero_mut()
            .expect("Hero node")
            .bag,
            cursor,
            1,
        );
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`ay.a:(…Graphics;II)V`): the paginated bag list. The `drawListPage` grid and the
/// empty-selection `text.get(15)` label are drawn; the per-slot item icons
/// (`Menu.drawItemIcon`), the selected-item info panel (`Menu.drawItemInfo`), and the
/// header (`BaseCanvas.drawLabelBox(AssetCache.commonText.get(2))`) are DEFERRED.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int panelX = x + 2; int panelY = y + 15;
    let panel_x = x.wrapping_add(2);
    let panel_y = y.wrapping_add(15);
    // ItemBag bag = GameState.hero().bag;
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    // Item selectedItem = bag.get((int) cursorIndex);   (only the null test is ported)
    let cursor = g.items_tab.base.cursor_index as i32;
    let selected_is_some = item_bag::get(
        &g.entity_arena[hero_id].as_hero().expect("Hero node").bag,
        cursor,
    )
    .is_some();
    // Read text.get(15) up front (only used on the empty-selection path) to avoid a
    // borrow of `g` while the framebuffer is split out.
    let text15 = if !selected_is_some {
        character_menu::text_get(g, 15)
    } else {
        Vec::new()
    };
    let base = g.items_tab.base.clone();

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // BaseCanvas.drawLabelBox(graphics, AssetCache.commonText.get(2), panelX + 5, panelY);
    // (DEFERRED: BaseCanvas.drawLabelBox + the AssetCache.commonText bank are unported.)
    // drawListPage(graphics, panelX, panelY, true);
    menu::draw_list_page(&mut graphics, &base, panel_x, panel_y, true);
    // for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++)
    //   if (bag.get(slot) != null) Menu.drawItemIcon(graphics, panelX+13, panelY+18+(23*(slot%5)), item, true);
    // (DEFERRED: Menu.drawItemIcon art is unported — the per-slot icon loop (the shape's
    //  irem/imul `23*(slot%5)` + the `slot++` iinc + icon geometry iadds) is skipped.)
    // Item selectedItem = bag.get((int) cursorIndex);
    // if (selectedItem != null) Menu.drawItemInfo(graphics, panelX+33, panelY+14, selectedItem);
    // else { graphics.setColor(16777215); FontManager.drawChars(graphics, panelX+33, panelY+14, text.get(15), 1); }
    if !selected_is_some {
        // graphics.setColor(16777215);
        graphics.set_color(16777215);
        // FontManager.drawChars(graphics, panelX + 33, panelY + 14, CharacterMenu.text.get(15), 1);
        font_manager::draw_chars(
            font_manager,
            &mut graphics,
            panel_x.wrapping_add(33),
            panel_y.wrapping_add(14),
            &text15,
            1,
        );
    }
    // (DEFERRED: the selectedItem != null branch → Menu.drawItemInfo (unported art).)
}
