//! Transliterated from `java/src/main/java/defpackage/StatAllocMenu.java`
//! (original `bi.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The stat-point allocation dialog (`StatAllocMenu extends Menu`) pushed by
//! [`StatusPage`](crate::status_page) (page 3) when the hero has unspent points.
//! Left/right adjust the pending points on the selected stat (STR/VIT/AGI/SPR),
//! tracked in [`pending`](StatAllocMenuState::pending) without touching the hero until
//! confirmed; OK asks for confirmation and, on yes, commits the deltas via
//! [`on_popup_result`] and recomputes derived stats.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `<init>`/`handleKey`/`onPopupResult`/`adjustStat` are all
//! real — the confirm-and-commit flow drives `Hero.strength/vitality/agility/spirit` +
//! `statPoints` and calls the ported `Hero.recomputeStats`. In `paint` the ported
//! primitives are drawn (the `Menu.fillOutlinedRect` header box, the interior
//! `fillRect`, and the per-row selection colour); the stat labels
//! (`AssetCache.heroText`), the pending/base `statValue` numbers
//! (`BaseCanvas.drawNumberAt`), the two `BaseCanvas.drawLabelBox` captions, and the
//! `AssetCache.cursorArrow`/`slotFrame` art are DEFERRED.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `bi.<init>:(Lq;)V => []`,
//! `bi.a:(II)Z => []` (handleKey),
//! `bi.a:(BB)V => [iadd,i2s,iadd,i2s,iadd,i2s,iadd,i2s]` (onPopupResult — the four
//! `stat + pending[i]` commits), `bi.a:(…Graphics;II)V => [iinc,iinc,iadd×…,imul,…]`
//! (paint — the ported adds are the box geometry; the `imul` + remaining adds feed the
//! DEFERRED per-row stat draws),
//! `bi.b:(B)V => [iadd,i2s,isub,i2s,isub,i2s,iadd,i2s]` (adjustStat).

use crate::character_menu;
use crate::game::Game;
use crate::hero;
use crate::menu::{self, MenuChild, MenuNode};

/// Java `bi` / `StatAllocMenu` state — the `Menu` (`cb`) base + the two per-instance
/// allocation fields. `StatAllocMenu` has no `static` fields (no
/// `java/reconstruction/ownership.tsv` rows).
#[derive(Debug, Default, Clone)]
pub struct StatAllocMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private short remainingPoints;` (obf `a`, `S`) — points still available to
    /// spend (starts at the hero's balance).
    pub remaining_points: i16,
    /// `private short[] pending;` (obf `a`, `[S`) — pending points queued onto each of
    /// the four base stats.
    pub pending: Vec<i16>,
}

/// `public StatAllocMenu(StatusPage statusPage)` (`bi.<init>:(Lq;)V => []`): starts the
/// allocation with the hero's current point balance and an empty pending vector.
pub fn construct(g: &mut Game) {
    // super(statusPage, (byte) 4);   (parent is the StatusPage → non-null → present)
    g.stat_alloc_menu.base = menu::construct(true, 4);
    // this.remainingPoints = GameState.hero().statPoints;
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let stat_points = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .stat_points;
    g.stat_alloc_menu.remaining_points = stat_points;
    // this.pending = new short[4];
    g.stat_alloc_menu.pending = vec![0i16; 4];
}

/// `public final boolean handleKey(int action, int keyCode)` (`bi.a:(II)Z => []`):
/// child forward + non-wrapping vertical nav; LEFT (`52`/action 2) refunds a point,
/// RIGHT (`54`/action 5) spends one; FIRE (`53`/action 8) asks to confirm; Back (`-8`)
/// cancels back to the parent. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::StatAlloc, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.stat_alloc_menu.base, action, key_code)
    {
        return true;
    }
    // if (keyCode == 52 || action == 2) { adjustStat((byte) 3); return true; }
    if key_code == 52 || action == 2 {
        adjust_stat(g, 3);
        return true;
    }
    // if (keyCode == 54 || action == 5) { adjustStat((byte) 4); return true; }
    if key_code == 54 || action == 5 {
        adjust_stat(g, 4);
        return true;
    }
    // if (keyCode != 53 && action != 8) {
    if key_code != 53 && action != 8 {
        // if (keyCode != -8) return true;
        if key_code != -8 {
            return true;
        }
        // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        let parent = menu::parent_of(g, MenuNode::StatAlloc)
            .expect("NullPointerException: StatAllocMenu.parent");
        menu::on_popup_result(g, parent, -1, -1);
        // return true;
        return true;
    }
    // if (pending[0]==0 && pending[1]==0 && pending[2]==0 && pending[3]==0) {
    let pending = &g.stat_alloc_menu.pending;
    if pending[0] == 0 && pending[1] == 0 && pending[2] == 0 && pending[3] == 0 {
        // showPopup((byte) 1, (byte) 1, new Object[]{CharacterMenu.text.get(34), CharacterMenu.text.get(35)});
        let l34 = character_menu::text_get(g, 34);
        let l35 = character_menu::text_get(g, 35);
        menu::show_popup(g, MenuNode::StatAlloc, 1, 1, vec![l34, l35]);
        // return true;
        return true;
    }
    // showPopup((byte) 2, (byte) 2, new Object[]{CharacterMenu.text.get(33)});
    let l33 = character_menu::text_get(g, 33);
    menu::show_popup(g, MenuNode::StatAlloc, 2, 2, vec![l33]);
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)`
/// (`bi.a:(BB)V => [iadd,i2s,iadd,i2s,iadd,i2s,iadd,i2s]`): dismisses the confirm popup
/// (`super`); when the yes-confirm (`tag == 2 && result == 0`) came from a `PopupMenu`,
/// commits the pending deltas onto the hero, recomputes derived stats, and pops back to
/// the parent.
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // Menu previousChild = ((Menu) this).child;
    let previous_child = g.stat_alloc_menu.base.child;
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::StatAlloc, tag, result);
    // if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
    if previous_child == MenuChild::Popup && tag == 2 && result == 0 {
        // Hero hero = GameState.hero();
        let hero_id = g
            .game_state
            .hero
            .expect("NullPointerException: GameState.hero()");
        let pending = g.stat_alloc_menu.pending.clone();
        let remaining = g.stat_alloc_menu.remaining_points;
        {
            let h = g.entity_arena[hero_id].as_hero_mut().expect("Hero node");
            // hero.strength = (short) (hero.strength + this.pending[0]);
            h.strength = (h.strength as i32).wrapping_add(pending[0] as i32) as i16;
            // hero.vitality = (short) (hero.vitality + this.pending[1]);
            h.vitality = (h.vitality as i32).wrapping_add(pending[1] as i32) as i16;
            // hero.agility = (short) (hero.agility + this.pending[2]);
            h.agility = (h.agility as i32).wrapping_add(pending[2] as i32) as i16;
            // hero.spirit = (short) (hero.spirit + this.pending[3]);
            h.spirit = (h.spirit as i32).wrapping_add(pending[3] as i32) as i16;
            // hero.statPoints = this.remainingPoints;
            h.stat_points = remaining;
        }
        // hero.recomputeStats();
        hero::recompute_stats(g, hero_id);
        // ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        let parent = menu::parent_of(g, MenuNode::StatAlloc)
            .expect("NullPointerException: StatAllocMenu.parent");
        menu::on_popup_result(g, parent, -1, -1);
    }
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`bi.a:(…Graphics;II)V`): the allocation box. See the module header for the
/// ported/DEFERRED split.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int boxX = x + 36; int boxY = y + 37;
    let box_x = x.wrapping_add(36);
    let box_y = y.wrapping_add(37);
    // Hero hero = GameState.hero();   — read only for the DEFERRED per-row statValue base stats.
    // byte cursor = ((Menu) this).cursorIndex;
    let cursor = g.stat_alloc_menu.base.cursor_index as i32;
    let Game { screen, .. } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // Menu.fillOutlinedRect(graphics, boxX, boxY, 101, 26, 4136767);
    menu::fill_outlined_rect(&mut graphics, box_x, box_y, 101, 26, 4136767);
    // graphics.setColor(16777215);
    graphics.set_color(16777215);
    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(36), boxX+3, boxY+3);
    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(37), boxX+3, boxY+14);
    // BaseCanvas.drawNumberAt(graphics, this.remainingPoints, boxX+65, boxY+14, 8);
    // (DEFERRED: BaseCanvas.drawLabelBox/drawNumberAt unported; CharacterMenu.text captions.)
    // graphics.setColor(6242111);
    graphics.set_color(6242111);
    // graphics.fillRect(boxX, boxY + 30, 101, 62);
    graphics.fill_rect(box_x, box_y.wrapping_add(30), 101, 62);
    // for (byte statIndex = 0; statIndex < 4; statIndex = (byte) (statIndex + 1)) {
    let mut stat_index: i8 = 0;
    while (stat_index as i32) < 4 {
        // if (cursorIndex == statIndex) { setColor(16777215); drawImage(cursorArrow, boxX+2, boxY+35+(statIndex*15), 20); }
        // else { setColor(14663551); }
        if cursor == (stat_index as i32) {
            graphics.set_color(16777215);
            // (DEFERRED: AssetCache.cursorArrow selection-arrow art unported.)
        } else {
            graphics.set_color(14663551);
        }
        // int statValue = this.pending[statIndex]; switch (statIndex) { case 0..3: statValue += base stat + bonus; }
        // FontManager.drawChars(graphics, boxX+10, boxY+35+(statIndex*15), AssetCache.heroText.get(9+statIndex), 1);
        // graphics.drawImage(AssetCache.slotFrame, boxX+70, boxY+35+(statIndex*15), 20);
        // BaseCanvas.drawNumberAt(graphics, statValue, boxX+90, boxY+35+(statIndex*15), 8);
        // graphics.drawImage(AssetCache.cursorArrow, boxX+92, boxY+35+(statIndex*15), 20);
        // (DEFERRED: the statValue = pending + base-stat switch feeds the unported
        //  BaseCanvas.drawNumberAt; the stat label uses the unported AssetCache.heroText;
        //  the slotFrame/cursorArrow art is unported.)
        stat_index = (stat_index as i32).wrapping_add(1) as i8;
    }
}

/// `private void adjustStat(byte direction)`
/// (`bi.b:(B)V => [iadd,i2s,isub,i2s,isub,i2s,iadd,i2s]`): adjusts the selected stat —
/// `direction` 4 spends a point (if any remain), 3 refunds one (if any pending).
fn adjust_stat(g: &mut Game, direction: i8) {
    // byte stat = ((Menu) this).cursorIndex;
    let cursor = g.stat_alloc_menu.base.cursor_index;
    // if (direction == 4 && this.remainingPoints > 0) {
    if direction == 4 && (g.stat_alloc_menu.remaining_points as i32) > 0 {
        // pending[stat] = (short) (pending[stat] + 1);
        let v = g.stat_alloc_menu.pending[cursor as usize];
        g.stat_alloc_menu.pending[cursor as usize] = (v as i32).wrapping_add(1) as i16;
        // this.remainingPoints = (short) (this.remainingPoints - 1);
        g.stat_alloc_menu.remaining_points =
            (g.stat_alloc_menu.remaining_points as i32).wrapping_sub(1) as i16;
        // return;
        return;
    }
    // if (direction != 3 || this.pending[cursorIndex] <= 0) return;
    if direction != 3 || (g.stat_alloc_menu.pending[cursor as usize] as i32) <= 0 {
        return;
    }
    // pending[stat] = (short) (pending[stat] - 1);
    let v = g.stat_alloc_menu.pending[cursor as usize];
    g.stat_alloc_menu.pending[cursor as usize] = (v as i32).wrapping_sub(1) as i16;
    // this.remainingPoints = (short) (this.remainingPoints + 1);
    g.stat_alloc_menu.remaining_points =
        (g.stat_alloc_menu.remaining_points as i32).wrapping_add(1) as i16;
}
