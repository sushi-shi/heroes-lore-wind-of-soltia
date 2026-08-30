//! Transliterated from `java/src/main/java/defpackage/StatusPage.java`
//! (original `q.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The character-status sub-panel (`StatusPage extends Menu`), tab 0 of
//! [`CharacterMenu`](crate::character_menu) (also opened by the level-up flow). Four
//! vertical pages selected by the cursor: 0 = summary (name/level/exp/HP/MP), 1 = the
//! six stats, 2 = class lore, 3 = a stat-point box that pushes
//! [`StatAllocMenu`](crate::stat_alloc_menu) when the hero has unspent points.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. `<init>`/`handleKey` are real: FIRE on page 3 pushes
//! `StatAllocMenu` when `statPoints > 0`, else a `showMessage` popup. In `paint` the
//! statements over ported primitives are drawn — the paginated `drawListPage`, the
//! page-0 exp bar (`(exp * 70) / expToNext` → two `fillRect`s), and the page-3
//! `Menu.fillOutlinedRect` boxes + the three `CharacterMenu.text` stat-label
//! `drawChars`. The rest is DEFERRED: `className`/`classDesc` and the page-1/2 labels
//! come from the unported `AssetCache.heroText` bank; `BaseCanvas.drawLabelBox`/
//! `drawNumberAt`/`drawFraction`, `Menu.drawGold`, and the `AssetCache` icon/label art
//! (`statusPanelIcon`/`portraitFrame`/`statLabel*`) are all unported.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `q.<init>:(Lcb;)V => [isub,iadd,isub,iinc,iinc]` (the `classId-6` + `descIndex`
//! computation, all inside the DEFERRED `AssetCache.heroText.get` className/classDesc
//! lookups), `q.a:(II)Z => []` (handleKey), `q.a:(…Graphics;II)V => [iinc,iinc,iadd×…,
//! imul,idiv,…]` (paint — the ported `idiv`/`imul` are the exp bar; the remaining adds
//! feed the DEFERRED art/label draws).

use crate::character_menu;
use crate::font_manager;
use crate::game::Game;
use crate::menu::{self, MenuChild, MenuNode};
use crate::stat_alloc_menu;
use j2me_jvm::java_div;

/// Java `q` / `StatusPage` state — the `Menu` (`cb`) base + the two per-instance
/// localized-text fields. `StatusPage` has no `static` fields (no
/// `java/reconstruction/ownership.tsv` rows).
#[derive(Debug, Default, Clone)]
pub struct StatusPageState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private char[] className;` (obf `a`) — localized class name (from the unported
    /// `AssetCache.heroText`; DEFERRED → empty).
    pub class_name: Vec<u16>,
    /// `private char[] classDesc;` (obf `b`) — localized class flavour line (DEFERRED → empty).
    pub class_desc: Vec<u16>,
}

/// `public StatusPage(Menu parentMenu)` (`q.<init>:(Lcb;)V`): the status tab.
pub fn construct(g: &mut Game) {
    // super(parentMenu, (byte) 4);   (parent is the CharacterMenu → non-null → present)
    g.status_page.base = menu::construct(true, 4);
    // this.className = AssetCache.heroText.get(GameState.classId - 6);
    // int descIndex = (3 + GameState.classId) - 6;
    // if (GameState.clearCount == 1) descIndex += 15; else if (GameState.clearCount >= 2) descIndex += 18;
    // this.classDesc = AssetCache.heroText.get(descIndex);
    // (DEFERRED: AssetCache.heroText is unported — the `classId - 6` / `descIndex`
    //  arithmetic ([isub,iadd,isub,iinc,iinc]) feeds the unported class-text lookups;
    //  className/classDesc stay empty.)
    g.status_page.class_name = Vec::new();
    g.status_page.class_desc = Vec::new();
}

/// `public final boolean handleKey(int action, int keyCode)` (`q.a:(II)Z => []`):
/// child forward + non-wrapping vertical nav; FIRE on page 3 pushes
/// [`StatAllocMenu`](crate::stat_alloc_menu) when `statPoints > 0`, else shows a "no
/// points" message. Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Status, action, key_code)
        || menu::move_cursor_vertical_no_wrap(&mut g.status_page.base, action, key_code)
    {
        return true;
    }
    // if (((Menu) this).cursorIndex != 3) return false;
    if (g.status_page.base.cursor_index as i32) != 3 {
        return false;
    }
    // if (keyCode != 53 && action != 8) return false;
    if key_code != 53 && action != 8 {
        return false;
    }
    // if (GameState.hero().statPoints > 0) { child = new StatAllocMenu(this); return true; }
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let stat_points = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .stat_points;
    if (stat_points as i32) > 0 {
        // ((Menu) this).child = new StatAllocMenu(this);
        stat_alloc_menu::construct(g);
        g.status_page.base.child = MenuChild::StatAlloc;
        return true;
    }
    // showMessage(new Object[]{CharacterMenu.text.get(0), CharacterMenu.text.get(1)});
    let l0 = character_menu::text_get(g, 0);
    let l1 = character_menu::text_get(g, 1);
    menu::show_message(g, MenuNode::Status, vec![l0, l1]);
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`q.a:(…Graphics;II)V`): the four-page status panel. See the module header for the
/// ported/DEFERRED split.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // int panelX = x + 2; int panelY = y + 15;
    let panel_x = x.wrapping_add(2);
    let panel_y = y.wrapping_add(15);
    // Hero hero = GameState.hero();   (page-0 exp bar reads exp/expToNext)
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    let (exp, exp_to_next) = {
        let h = g.entity_arena[hero_id].as_hero().expect("Hero node");
        (h.exp, h.exp_to_next)
    };
    // Snapshot the StatusPage base (for drawListPage) and the page-3 stat-help labels
    // (CharacterMenu.text.get(5/6/7) — ported, only read on page 3 to avoid a spurious NPE).
    let base = g.status_page.base.clone();
    let cursor = base.cursor_index as i32;
    let (text5, text6, text7) = if cursor == 3 {
        (
            character_menu::text_get(g, 5),
            character_menu::text_get(g, 6),
            character_menu::text_get(g, 7),
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };

    let Game {
        screen,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(2), panelX + 5, panelY);
    // Menu.drawGold(graphics, panelX + 110, panelY + 2, hero.bag.gold);
    // (DEFERRED: BaseCanvas.drawLabelBox + Menu.drawGold are unported.)
    // drawListPage(graphics, panelX, panelY, false);
    menu::draw_list_page(&mut graphics, &base, panel_x, panel_y, false);
    // BaseCanvas.drawNumberAt(graphics, 1/2/3, panelX + 12, panelY + 16 (+23/+46), 4);   ×3
    // graphics.drawImage(AssetCache.statusPanelIcon, panelX + 10, panelY + 83, 20);
    // if (hero.statPoints > 0) graphics.drawImage(AssetCache.portraitFrame, panelX + 3, panelY + 87, 36);
    // (DEFERRED: BaseCanvas.drawNumberAt + AssetCache.statusPanelIcon/portraitFrame art unported.)
    match cursor {
        // case 0: summary
        0 => {
            // graphics.setColor(16777215); FontManager.drawChars(graphics, panelX+35, panelY+18, className, 1);
            // graphics.setColor(14663551); FontManager.drawChars(graphics, panelX+33, panelY+35, classDesc, 1);
            // graphics.drawImage(AssetCache.statLabel3, panelX+35, panelY+52, 20);
            // BaseCanvas.drawNumberAt(graphics, hero.level, panelX+52, panelY+52, 4);
            // graphics.drawImage(AssetCache.statLabel1, panelX+34, panelY+70, 20);
            // BaseCanvas.drawNumberAt(graphics, hero.exp, panelX+102, panelY+70, 8);
            // (DEFERRED: className/classDesc come from the unported AssetCache.heroText;
            //  statLabel* art + BaseCanvas.drawNumberAt unported.)
            // graphics.setColor(4136767); graphics.fillRect(panelX+34, panelY+79, 72, 3);
            graphics.set_color(4136767);
            graphics.fill_rect(panel_x.wrapping_add(34), panel_y.wrapping_add(79), 72, 3);
            // graphics.setColor(16777215); graphics.fillRect(panelX+35, panelY+80, (hero.exp * 70) / hero.expToNext, 1);
            graphics.set_color(16777215);
            let bar_width =
                java_div(exp.wrapping_mul(70), exp_to_next).expect("(exp * 70) / expToNext");
            graphics.fill_rect(
                panel_x.wrapping_add(34).wrapping_add(1),
                panel_y.wrapping_add(79).wrapping_add(1),
                bar_width,
                1,
            );
            // graphics.drawImage(AssetCache.statLabel4, panelX+38, panelY+84, 20);
            // BaseCanvas.drawNumberAt(graphics, hero.expToNext, panelX+102, panelY+84, 8);
            // graphics.drawImage(AssetCache.statLabel2, panelX+34, panelY+97, 20);
            // BaseCanvas.drawFraction(graphics, panelX+102, panelY+96, hero.hp, hero.maxHp);
            // graphics.drawImage(AssetCache.statLabel5, panelX+34, panelY+106, 20);
            // BaseCanvas.drawFraction(graphics, panelX+102, panelY+105, hero.mp, hero.maxMp);
            // (DEFERRED: statLabel* art + BaseCanvas.drawNumberAt/drawFraction unported.)
        }
        // case 1: the six stats
        1 => {
            // graphics.setColor(14663551);
            graphics.set_color(14663551);
            // for (int stat = 0; stat < 6; stat++) FontManager.drawChars(graphics, panelX+38, panelY+21+(stat*15), AssetCache.heroText.get(9+stat), 1);
            // BaseCanvas.drawNumberAt(graphics, hero.strength+strengthBonus, panelX+100, panelY+22, 8);   (+the other five rows)
            // (DEFERRED: the stat labels come from the unported AssetCache.heroText; the
            //  six BaseCanvas.drawNumberAt value draws are unported.)
        }
        // case 2: class lore
        2 => {
            // graphics.setColor(14663551); FontManager.drawChars(graphics, panelX+34, panelY+18, className, 1);
            // graphics.setColor(16777215);
            // char[] loreText = AssetCache.heroText.get(GameState.classId);
            // if (BaseCanvas.width > 128) FontManager.drawWrappedText(graphics, panelX+34, panelY+30, 110, 1, loreText);
            // else FontManager.drawWrappedText(graphics, panelX+34, panelY+30, 75, 1, loreText);
            // (DEFERRED: className + loreText come from the unported AssetCache.heroText.)
        }
        // case 3: the stat-point box
        3 => {
            // Menu.fillOutlinedRect(graphics, panelX+34, panelY+22, 100, 26, 4136767);
            menu::fill_outlined_rect(
                &mut graphics,
                panel_x.wrapping_add(34),
                panel_y.wrapping_add(22),
                100,
                26,
                4136767,
            );
            // graphics.setColor(16777215);
            graphics.set_color(16777215);
            // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(3), panelX+37, panelY+25);
            // BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(4), panelX+37, panelY+36);
            // BaseCanvas.drawNumberAt(graphics, GameState.hero().statPoints, panelX+99, panelY+36, 8);
            // (DEFERRED: BaseCanvas.drawLabelBox/drawNumberAt unported.)
            // Menu.fillOutlinedRect(graphics, panelX+34, panelY+62, 100, 33, 4136767);
            menu::fill_outlined_rect(
                &mut graphics,
                panel_x.wrapping_add(34),
                panel_y.wrapping_add(62),
                100,
                33,
                4136767,
            );
            // graphics.setColor(16777215);
            graphics.set_color(16777215);
            // FontManager.drawChars(graphics, panelX+40, panelY+72, CharacterMenu.text.get(5), 1);
            font_manager::draw_chars(
                font_manager,
                &mut graphics,
                panel_x.wrapping_add(40),
                panel_y.wrapping_add(72),
                &text5,
                1,
            );
            // graphics.setColor(14663551);
            graphics.set_color(14663551);
            // FontManager.drawChars(graphics, panelX+60, panelY+67, CharacterMenu.text.get(6), 1);
            font_manager::draw_chars(
                font_manager,
                &mut graphics,
                panel_x.wrapping_add(60),
                panel_y.wrapping_add(67),
                &text6,
                1,
            );
            // FontManager.drawChars(graphics, panelX+60, panelY+80, CharacterMenu.text.get(7), 1);
            font_manager::draw_chars(
                font_manager,
                &mut graphics,
                panel_x.wrapping_add(60),
                panel_y.wrapping_add(80),
                &text7,
                1,
            );
        }
        _ => {}
    }
}
