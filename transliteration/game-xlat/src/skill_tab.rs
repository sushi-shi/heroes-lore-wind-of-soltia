//! Transliterated from `java/src/main/java/defpackage/SkillTab.java`
//! (original `s.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Class-skill tab (tab 4) of [`CharacterMenu`](crate::character_menu). The
//! constructor scans the class's skills (via [`GameState.classStartFlags`](crate::game_state)
//! and the learned-skill flag bits) and builds `skillEntries`, a list of
//! `(skillIndex, variant)` pairs for the skills to display. The selected skill's
//! name/description come from the `grd`-style skill text table
//! (`AssetCache.classSkillText`); OK confirms learning the skill via a
//! [`ConfirmDialog`](crate::confirm_dialog).
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. The constructor's skill-scan is real over the modelled
//! `GameState.classStartFlags`/`GameState.isFlag` — but the entry-add gate
//! `AssetCache.classSkillText.get(idx).length > 0` is DEFERRED (that skill-text bank is
//! unported); it is the second operand of an `&&` guarded by the learned-skill flag
//! `isFlag(1 + skill*3)`, so a fresh hero (no learned skills) never evaluates it and
//! `itemCount` stays `0`. `handleKey` is fully real (FIRE pushes a `ConfirmDialog`, or
//! returns early on `itemCount <= 0`). `onPopupResult` pushes the follow-up
//! `ConfirmDialog` (its body line — `classSkillText.get(skillIndex*7+6)` — DEFERRED to
//! empty). `loadSelectedSkill` computes `skillIndex`/`skillVariant` from the modelled
//! `skillEntries`, and DEFERS the `classSkillText` name/desc reads (empty). In `paint`
//! the empty-list branch (`Menu.drawBevelBox`/`fillInset2` + `drawWrappedText(text.get(58))`)
//! is drawn; the populated branch (`classSkillText`/`AssetCache.itemIcons` heavy) and
//! the `BaseCanvas.drawLabelBox` header are DEFERRED (unreachable this slice —
//! `itemCount` is always `0`).
//!
//! `SkillTab`'s fields (`skillEntries`/`skillIndex`/`skillVariant`/`skillName`/
//! `skillDesc`) are all per-INSTANCE (no `static`s), so it contributes no
//! `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `s.<init>:(Lcb;)V => [isub,imul,imul,iadd,iadd,imul,iadd,iadd,imul,iadd,imul,iadd,
//! iinc,iinc,imul,iadd,imul,iinc,iinc,iadd,i2b,idiv,i2b]` (the ported `classId-6` (isub),
//! `skillCount*2` (imul), the `1+skill*3+{0,1,2}` flag-index arithmetic, the `writePos`
//! writes, `skill++` and `writePos/2` (idiv); the two `skill*7(+2)` (imul,iadd) feed the
//! DEFERRED classSkillText length checks), `s.a:(II)Z => []` (handleKey),
//! `s.a:(BB)V => [imul,iadd]` (onPopupResult — the `skillIndex*7+6` index feeds the
//! DEFERRED classSkillText.get body line), `s.d:()V => [imul,imul,iadd,imul,iadd,iadd,
//! imul,iadd,iadd]` (loadSelectedSkill — the ported prefix `skillEntries[cursorIndex*2]`
//! / `[cursorIndex*2+1]` is `[imul,imul,iadd]`; the `base`/`base2` `skillIndex*7(+N)` feed
//! the DEFERRED classSkillText name/desc), `s.a:(…Graphics;II)V => [...]` (paint — the
//! ported empty-list `panelX+4`/`panelY+10`/`panelX+10`/`panelY+15` iadds; the rest is
//! the DEFERRED populated branch).

use crate::character_menu;
use crate::confirm_dialog;
use crate::font_manager;
use crate::game::Game;
use crate::game_state;
use crate::menu::{self, MenuChild, MenuNode};
use j2me_jvm::java_div;

/// Java `s` / `SkillTab` state — the `Menu` (`cb`) base + the five per-instance skill
/// fields.
#[derive(Debug, Default, Clone)]
pub struct SkillTabState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte[] skillEntries;` (obf `h`) — flattened `(skillIndex, variant)`
    /// pairs for the visible skills.
    pub skill_entries: Vec<i8>,
    /// `private byte skillIndex;` (obf `c`) — class-skill index of the highlighted entry.
    pub skill_index: i8,
    /// `private byte skillVariant;` (obf `d`) — variant (0 = basic, 2 = advanced).
    pub skill_variant: i8,
    /// `private char[] skillName;` (obf `a`) — localized name (DEFERRED classSkillText → empty).
    pub skill_name: Vec<u16>,
    /// `private char[] skillDesc;` (obf `b`) — localized description (DEFERRED → empty).
    pub skill_desc: Vec<u16>,
}

/// `AssetCache.classSkillText.get(index).length > 0` — DEFERRED: the `classSkillText`
/// skill-text bank is unported. This operand is only reached when the guarding
/// `GameState.isFlag(1 + skill*3)` (the skill is learned/available) holds; a fresh hero
/// has learned no skills, so in this slice it is never evaluated. Panics if it is.
fn class_skill_text_len_nonzero(_index: i32) -> bool {
    unreachable!("DEFERRED: AssetCache.classSkillText.get(index).length (classSkillText unported)")
}

/// `public SkillTab(Menu parentMenu)` (`s.<init>:(Lcb;)V`): scans the class's skills
/// and builds the visible-entry list. See the module header for the DEFERRED
/// classSkillText length gate (leaving `itemCount = 0` for a fresh hero).
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 0);   (parent is the CharacterMenu → non-null → present)
    g.skill_tab.base = menu::construct(true, 0);
    // int skillCount = GameState.classStartFlags[GameState.classId - 6].length;
    let skill_count = g.game_state.class_start_flags
        [(g.game_state.class_id as i32).wrapping_sub(6) as usize]
        .len() as i32;
    // int writePos = 0;
    let mut write_pos: i32 = 0;
    // this.skillEntries = new byte[skillCount * 2];
    g.skill_tab.skill_entries = vec![0i8; skill_count.wrapping_mul(2) as usize];
    // for (byte skill = 0; skill < skillCount; skill = (byte) (skill + 1)) {
    let mut skill: i8 = 0;
    while (skill as i32) < skill_count {
        let s = skill as i32;
        // if (!GameState.isFlag(1 + (skill*3) + 1)) {
        if !game_state::is_flag(g, 1i32.wrapping_add(s.wrapping_mul(3)).wrapping_add(1)) {
            // if (GameState.isFlag(1 + (skill*3) + 2)) {
            if game_state::is_flag(g, 1i32.wrapping_add(s.wrapping_mul(3)).wrapping_add(2)) {
                // if (GameState.isFlag(1 + (skill*3)) && classSkillText.get((skill*7)+2).length > 0) {
                if game_state::is_flag(g, 1i32.wrapping_add(s.wrapping_mul(3)))
                    && class_skill_text_len_nonzero(s.wrapping_mul(7).wrapping_add(2))
                {
                    // this.skillEntries[writePos] = skill; this.skillEntries[writePos+1] = 2; writePos += 2;
                    g.skill_tab.skill_entries[write_pos as usize] = skill;
                    g.skill_tab.skill_entries[write_pos.wrapping_add(1) as usize] = 2;
                    write_pos = write_pos.wrapping_add(2);
                }
            // } else if (GameState.isFlag(1 + (skill*3)) && classSkillText.get(skill*7).length > 0) {
            } else if game_state::is_flag(g, 1i32.wrapping_add(s.wrapping_mul(3)))
                && class_skill_text_len_nonzero(s.wrapping_mul(7))
            {
                // this.skillEntries[writePos] = skill; this.skillEntries[writePos+1] = 0; writePos += 2;
                g.skill_tab.skill_entries[write_pos as usize] = skill;
                g.skill_tab.skill_entries[write_pos.wrapping_add(1) as usize] = 0;
                write_pos = write_pos.wrapping_add(2);
            }
        }
        // skill = (byte) (skill + 1);
        skill = (skill as i32).wrapping_add(1) as i8;
    }
    // ((Menu) this).itemCount = (byte) (writePos / 2);
    g.skill_tab.base.item_count = java_div(write_pos, 2).expect("writePos / 2") as i8;
    // loadSelectedSkill();
    load_selected_skill(g);
}

/// `public final boolean handleKey(int action, int keyCode)` (`s.a:(II)Z => []`):
/// child forward + non-wrapping vertical nav (reloading the highlighted skill); FIRE
/// pushes a learn-skill [`ConfirmDialog`](crate::confirm_dialog) when there is a skill
/// to learn. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Skill, action, key_code) {
        return true;
    }
    // if (moveCursorVerticalNoWrap(action, keyCode)) { parent.needsRepaint = true; loadSelectedSkill(); return true; }
    if menu::move_cursor_vertical_no_wrap(&mut g.skill_tab.base, action, key_code) {
        if let Some(parent) = menu::parent_of(g, MenuNode::Skill) {
            menu::set_needs_repaint(g, parent, true);
        }
        load_selected_skill(g);
        return true;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // if (((Menu) this).itemCount <= 0) return true;
    if (g.skill_tab.base.item_count as i32) <= 0 {
        return true;
    }
    // ((Menu) this).child = new ConfirmDialog(this, this.skillName, this.skillDesc, (byte) 0);
    let name = g.skill_tab.skill_name.clone();
    let desc = g.skill_tab.skill_desc.clone();
    confirm_dialog::construct(g, name, desc, 0);
    g.skill_tab.base.child = MenuChild::Confirm;
    // return true;
    true
}

/// `public final void onPopupResult(byte tag, byte result)` (`s.a:(BB)V => [imul,iadd]`):
/// runs the base dismiss (`super`), then — on a confirmed learn (tag 0, result 1) —
/// pushes a follow-up [`ConfirmDialog`](crate::confirm_dialog). The body line
/// (`classSkillText.get(skillIndex*7+6)`) is DEFERRED (classSkillText unported → empty).
pub fn on_popup_result(g: &mut Game, tag: i8, result: i8) {
    // super.onPopupResult(tag, result);
    menu::on_popup_result_base(g, MenuNode::Skill, tag, result);
    // if (tag == 0 && result == 1) {
    if tag == 0 && result == 1 {
        // ((Menu) this).child = new ConfirmDialog(this, CharacterMenu.text.get(54),
        //   AssetCache.classSkillText.get((this.skillIndex * 7) + 6), (byte) 1);
        let l54 = character_menu::text_get(g, 54);
        // (DEFERRED: AssetCache.classSkillText.get((skillIndex*7)+6) — classSkillText
        //  unported; the [imul,iadd] index feeds the DEFERRED get, so the body line is empty.)
        let desc: Vec<u16> = Vec::new();
        confirm_dialog::construct(g, l54, desc, 1);
        g.skill_tab.base.child = MenuChild::Confirm;
    }
}

/// `private final void loadSelectedSkill()` (`s.d:()V`): refreshes
/// `skillIndex`/`skillVariant` from the modelled `skillEntries`; the name/desc reads
/// (`classSkillText`) are DEFERRED (empty).
fn load_selected_skill(g: &mut Game) {
    // this.skillIndex = this.skillEntries[((Menu) this).cursorIndex * 2];
    let ci = g.skill_tab.base.cursor_index as i32;
    g.skill_tab.skill_index = g.skill_tab.skill_entries[ci.wrapping_mul(2) as usize];
    // this.skillVariant = this.skillEntries[(((Menu) this).cursorIndex * 2) + 1];
    g.skill_tab.skill_variant =
        g.skill_tab.skill_entries[ci.wrapping_mul(2).wrapping_add(1) as usize];
    // if (skillVariant == 2) { base=(skillIndex*7)+2; skillName=classSkillText.get(base); skillDesc=classSkillText.get(base+1); }
    // else { base2=(skillIndex*7)+0; skillName=classSkillText.get(base2); skillDesc=classSkillText.get(base2+1); }
    // (DEFERRED: AssetCache.classSkillText unported — the base/base2 `skillIndex*7(+N)`
    //  index arithmetic feeds the DEFERRED get lookups; skillName/skillDesc stay empty.)
    g.skill_tab.skill_name = Vec::new();
    g.skill_tab.skill_desc = Vec::new();
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`s.a:(…Graphics;II)V`): the class-skill panel. The empty-list branch (bevel box +
/// `drawWrappedText(text.get(58))`) is drawn; the populated branch (classSkillText /
/// itemIcons heavy) and the `BaseCanvas.drawLabelBox` header are DEFERRED (unreachable
/// this slice — `itemCount` is always `0`).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int panelX = x + 2; int panelY = y + 15;
    let panel_x = x.wrapping_add(2);
    let panel_y = y.wrapping_add(15);
    let item_count = g.skill_tab.base.item_count as i32;
    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(39), panelX + 5, panelY);
    // (DEFERRED: BaseCanvas.drawLabelBox is unported.)
    // if (((Menu) this).itemCount <= 0) { ... drawWrappedText(text.get(58)) ... return; }
    if item_count <= 0 {
        let text58 = character_menu::text_get(g, 58);
        let Game {
            screen,
            font_manager,
            base_canvas,
            ..
        } = &mut *g;
        let target = screen.as_mut().expect("framebuffer");
        let mut graphics = j2me_me::Graphics::new(target);
        // Menu.drawBevelBox(graphics, panelX+4, panelY+10, 143, 137, 4136767, 10452799, 4144959);
        menu::draw_bevel_box(
            &mut graphics,
            panel_x.wrapping_add(4),
            panel_y.wrapping_add(10),
            143,
            137,
            4136767,
            10452799,
            4144959,
        );
        // Menu.fillInset2(graphics, panelX+4, panelY+10, 143, 137, 6242111);
        menu::fill_inset2(
            &mut graphics,
            panel_x.wrapping_add(4),
            panel_y.wrapping_add(10),
            143,
            137,
            6242111,
        );
        // graphics.setColor(16777215);
        graphics.set_color(16777215);
        // FontManager.drawWrappedText(graphics, panelX+10, panelY+15, 96, 1, CharacterMenu.text.get(58));
        font_manager::draw_wrapped_text(
            font_manager,
            &mut graphics,
            base_canvas,
            panel_x.wrapping_add(10),
            panel_y.wrapping_add(15),
            96,
            1,
            &text58,
        );
        // return;   (the Java `return` ends paint here; the populated branch below is
        //  comment-only DEFERRED, so control naturally ends — no explicit `return` needed.)
    }
    // The populated (itemCount > 0) branch — drawListPage + the itemIcons/classSkillText
    // skill name/desc/status draws — is DEFERRED (AssetCache.classSkillText + itemIcons
    // banks unported). Unreachable this slice: itemCount is always 0 (see construct).
}
