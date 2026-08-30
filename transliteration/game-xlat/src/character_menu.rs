//! Transliterated from `java/src/main/java/defpackage/CharacterMenu.java`
//! (original `ai.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The in-game character menu (`CharacterMenu extends Menu`): a six-tab panel
//! (status / items / equipment / guardian / skill / system) reached by pausing in
//! the world. It is a lazily-created singleton root (`super(null, …)`), and each tab
//! is a pushed sub-screen switched by the overriding [`move_cursor`]. This lane
//! ports the tabbed **shell**: only tab 0's child ([`StatusPage`](crate::status_page))
//! is wired; the other five tab classes (`ItemsTab`/`EquipTab`/`GuardianTab`/
//! `SkillTab`/`SystemTab`) are not in this batch and their `child = new …Tab(this)`
//! constructions are DEFERRED with a named marker.
//!
//! ## Statics + singleton
//!
//! `CharacterMenu` is a lazily-created singleton like `MainMenu`/`ShopMenu`:
//! [`instance`] creates it on first use (opening on the status tab) and centres the
//! panel. Its `static` fields — `panelX`/`panelY` (the centred origin), `text` (the
//! `/sgui/gm` label table shared by every tab) and `singleton` (the presence flag) —
//! plus the per-instance `equipSnapshot`/`guardianSnapshot` fields live on
//! [`CharacterMenuState`] (`Game.character_menu`), the sole owner per
//! `java/reconstruction/ownership.tsv`.
//!
//! ## ANTI-BOG boundary
//!
//! Every method is ported. The STATE machinery — `instance`/`<init>`/`open`/
//! `closeMenu`/`openSystemQuit`/`handleKey`/the overriding `moveCursor`/`draw` — is
//! real. The genuinely-unported cross-class hops are DEFERRED with named markers:
//! the equip-snapshot diff reads `Hero.getWeapon`/`getArmor`/`getAccessory*` (simple
//! `equipment[]` reads) but their bodies index the unported `AssetLoader.weaponAnim`/
//! `shieldAnim`/`armorAnim`/`headAnim` tables and call the unported
//! `Hero.reloadEquipSprite`; equipment/guardian are null in this slice (item/guardian
//! creation DEFERRED in `Hero.initClass`), so those branches are unreachable.
//! `openSystemQuit` builds a `SystemTab` (not in this batch → DEFERRED). In `paint`
//! the six `AssetCache.menuTabIcons` tab-icon draws are DEFERRED (that art bank is
//! unported); the panel fill + inset panel + tab-cursor bevel + the clear/soft-keys
//! (with `labelBack` DEFERRED → `None`) are drawn.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `ai.<init>:()V => []`,
//! `ai.a:()Lai; => [isub,isub]` (instance — `halfW-77`/`halfH-85`),
//! `ai.d:()V => [iadd,i2b,isub,isub]` (open — the `slot++` loop step is `iadd,i2b`;
//! the two `classId-6` `isub`s live in the DEFERRED weapon/accessory1 branches),
//! `ai.a:(Z)V => [isub,isub]` (closeMenu — both `classId-6` `isub`s live in the
//! DEFERRED weapon/accessory1 reload checks), `ai.a:(II)Z => []` (handleKey),
//! `ai.a:(B)V => []` (moveCursor — child rebuild), `ai.e:()V => []` (openSystemQuit),
//! `ai.a:(…Graphics;)V => []` (draw),
//! `ai.a:(…Graphics;II)V => [iadd×…,imul×4,iadd,iinc]` (paint — the tab-cursor bevel
//! geometry; the `iinc` is the DEFERRED `iconX += 16` tab-icon walk).

use crate::game::Game;
use crate::game_loop;
use crate::game_screen;
use crate::game_state;
use crate::menu::{self, MenuChild, MenuNode};
use crate::status_page;
use crate::text_table::{self, TextTableState};

/// Java `ai` / `CharacterMenu` state — the `Menu` (`cb`) base + the two per-instance
/// snapshot fields + the class's four `static` fields (`panelX`, `panelY`, `text`,
/// `singleton`).
#[derive(Debug, Default, Clone)]
pub struct CharacterMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
    /// `private byte[] equipSnapshot;` (obf `h`) — equipped-sprite ids captured on
    /// [`open`] (weapon/armour/acc1/acc2/-). Empty == Java null.
    pub equip_snapshot: Vec<i8>,
    /// `private byte guardianSnapshot;` (obf `c`) — active guardian type captured on
    /// [`open`] (`-1` = none).
    pub guardian_snapshot: i8,
    /// `public static int panelX;` (obf `a`) — centred panel origin X.
    pub panel_x: i32,
    /// `public static int panelY;` (obf `b`) — centred panel origin Y.
    pub panel_y: i32,
    /// `public static TextTable text;` (obf `a`, `Lz;`) — the `/sgui/gm` game-menu
    /// label table shared by every tab (`None` == Java null, until [`open`]).
    pub text: Option<TextTableState>,
    /// `private static CharacterMenu singleton;` (obf `a`, `Lai;`) — presence flag.
    pub singleton: bool,
}

/// `CharacterMenu.text.get(index)` — resolves a `/sgui/gm` label through the loaded
/// [`TextTable`](crate::text_table). A null `text` (before [`open`]) panics, matching
/// the Java NPE.
pub(crate) fn text_get(g: &Game, index: i32) -> Vec<u16> {
    // CharacterMenu.text.get(index)
    text_table::get(
        g,
        g.character_menu
            .text
            .as_ref()
            .expect("NullPointerException: CharacterMenu.text (open not called)"),
        index,
    )
}

/// `public static final CharacterMenu instance()` (`ai.a:()Lai; => [isub,isub]`):
/// returns (creating on first use, opened on the status tab) the character-menu
/// singleton, centring the panel at `halfW - 77` / `halfH - 85`.
pub fn instance(g: &mut Game) {
    // if (singleton == null) {
    if !g.character_menu.singleton {
        // singleton = new CharacterMenu();
        construct(g);
        g.character_menu.singleton = true;
        // ((Menu) singleton).child = new StatusPage(singleton);
        status_page::construct(g);
        g.character_menu.base.child = MenuChild::Status;
        // panelX = BaseCanvas.halfW - 77;
        g.character_menu.panel_x = g.base_canvas.half_w.wrapping_sub(77);
        // panelY = BaseCanvas.halfH - 85;
        g.character_menu.panel_y = g.base_canvas.half_h.wrapping_sub(85);
    }
    // return singleton;   (identity in the flat model)
}

/// `private CharacterMenu()` (`ai.<init>:()V => []`): builds the six-tab panel root.
pub fn construct(g: &mut Game) {
    // super(null, (byte) 6);   (null parent → the character menu is a root)
    g.character_menu.base = menu::construct(false, 6);
    // this.equipSnapshot = new byte[5];
    g.character_menu.equip_snapshot = vec![0i8; 5];
}

/// `public final void open()` (`ai.d:()V => [iadd,i2b,isub,isub]`): snapshots the
/// equipped-gear sprite ids + the active guardian, loads the `/sgui/gm` strings, and
/// speeds the loop.
pub fn open(g: &mut Game) {
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    // Guardian activeGuardian = hero.getActiveGuardian();
    let has_active_guardian = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .active_guardian
        .is_some();
    // for (byte slot = 0; slot < 5; slot = (byte) (slot + 1)) this.equipSnapshot[slot] = -1;
    let mut slot: i8 = 0;
    while (slot as i32) < 5 {
        g.character_menu.equip_snapshot[slot as usize] = -1;
        slot = (slot as i32).wrapping_add(1) as i8;
    }
    // if (hero.getWeapon() != null) equipSnapshot[0] = AssetLoader.weaponAnim[classId-6][weapon.subId];
    if g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .equipment[0]
        .is_some()
    {
        // (DEFERRED: AssetLoader.weaponAnim[GameState.classId - 6][((Item) weapon).subId]
        //  — the weapon-anim atlas table is unported; equipment[0] is null in this
        //  slice (item creation DEFERRED in Hero.initClass), so this branch is unreachable.)
    }
    // if (hero.getArmor() != null) equipSnapshot[1] = AssetLoader.shieldAnim[armor.subId];
    if g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .equipment[1]
        .is_some()
    {
        // (DEFERRED: AssetLoader.shieldAnim[((Item) armor).subId] — unported; equipment[1]
        //  is null in this slice, so this branch is unreachable.)
    }
    // if (hero.getAccessory1() != null) equipSnapshot[2] = AssetLoader.armorAnim[classId-6][acc1.subId];
    if g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .equipment[2]
        .is_some()
    {
        // (DEFERRED: AssetLoader.armorAnim[GameState.classId - 6][accessory1.subId] —
        //  unported; equipment[2] is null in this slice, so this branch is unreachable.)
    }
    // if (hero.getAccessory2() != null) equipSnapshot[3] = AssetLoader.headAnim[acc2.subId];
    if g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .equipment[3]
        .is_some()
    {
        // (DEFERRED: AssetLoader.headAnim[accessory2.subId] — unported; equipment[3] is
        //  null in this slice, so this branch is unreachable.)
    }
    // this.guardianSnapshot = (byte) -1;
    g.character_menu.guardian_snapshot = -1;
    // if (activeGuardian != null) this.guardianSnapshot = activeGuardian.type;
    if has_active_guardian {
        // (DEFERRED: activeGuardian.type — Guardian is unported; activeGuardian is null
        //  in this slice, so this branch is unreachable.)
    }
    // try { text = new TextTable("/sgui/gm"); } catch (IOException e) { e.printStackTrace(); }
    let table = text_table::construct(g, "/sgui/gm");
    g.character_menu.text = Some(table);
    // GameLoop.instance.setFastFps();
    game_loop::set_fast_fps(g);
}

/// `public final void openSystemQuit()` (`ai.e:()V => []`): jumps straight to the
/// system tab's quit prompt. `SystemTab` is not in this batch, so the tab
/// construction + `promptExit` + child link are DEFERRED; only the `cursorIndex = 6`
/// tab selection lands.
pub fn open_system_quit(g: &mut Game) {
    // ((Menu) this).cursorIndex = (byte) 6;
    g.character_menu.base.cursor_index = 6;
    // SystemTab systemTab = new SystemTab(this); systemTab.promptExit();
    // ((Menu) this).child = systemTab; ((Menu) this).child.cursorIndex = (byte) 1;
    // (DEFERRED: SystemTab not yet ported — the quit-tab construction, promptExit, and
    //  child link are deferred.)
}

/// `public final void closeMenu(boolean applyChanges)` (`ai.a:(Z)V => [isub,isub]`):
/// closes the menu; when `applyChanges`, diffs and reloads changed equip sprites, then
/// resumes the world (screen 2). The equip-sprite reload diffs + the guardian-summon
/// branch are DEFERRED (equipment/guardian null in this slice; `reloadEquipSprite`/
/// `beginGuardianSummon` unported).
pub fn close_menu(g: &mut Game, apply_changes: bool) {
    // Hero hero = GameState.hero();
    let hero_id = g
        .game_state
        .hero
        .expect("NullPointerException: GameState.hero()");
    // Guardian activeGuardian = hero.getActiveGuardian();
    let has_active_guardian = g.entity_arena[hero_id]
        .as_hero()
        .expect("Hero node")
        .active_guardian
        .is_some();
    // singleton = null;
    g.character_menu.singleton = false;
    // if (applyChanges) {
    if apply_changes {
        // if (hero.getArmor() != null && equipSnapshot[1] != AssetLoader.shieldAnim[armor.subId]) hero.reloadEquipSprite(1);
        if g.entity_arena[hero_id]
            .as_hero()
            .expect("Hero node")
            .equipment[1]
            .is_some()
        {
            // (DEFERRED: the shieldAnim diff + Hero.reloadEquipSprite((byte) 1) — both
            //  unported; equipment[1] is null in this slice, so this is unreachable.)
        }
        // if (hero.getAccessory1() != null && equipSnapshot[2] != AssetLoader.armorAnim[classId-6][acc1.subId]) hero.reloadEquipSprite(2);
        if g.entity_arena[hero_id]
            .as_hero()
            .expect("Hero node")
            .equipment[2]
            .is_some()
        {
            // (DEFERRED: the armorAnim diff + Hero.reloadEquipSprite((byte) 2) — unported;
            //  equipment[2] is null in this slice, so this is unreachable.)
        }
        // if (hero.getAccessory2() != null && equipSnapshot[3] != AssetLoader.headAnim[acc2.subId]) hero.reloadEquipSprite(3);
        if g.entity_arena[hero_id]
            .as_hero()
            .expect("Hero node")
            .equipment[3]
            .is_some()
        {
            // (DEFERRED: the headAnim diff + Hero.reloadEquipSprite((byte) 3) — unported;
            //  equipment[3] is null in this slice, so this is unreachable.)
        }
        // if (activeGuardian == null || guardianSnapshot == activeGuardian.type) {
        //   activeGuardian is null in this slice (guardian DEFERRED), so the first
        //   disjunct holds; the `guardianSnapshot == activeGuardian.type` DEFERRED
        //   second disjunct is unreachable.
        if !has_active_guardian {
            // if (hero.getWeapon() != null && equipSnapshot[0] != AssetLoader.weaponAnim[classId-6][weapon.subId]) hero.reloadEquipSprite(0);
            if g.entity_arena[hero_id]
                .as_hero()
                .expect("Hero node")
                .equipment[0]
                .is_some()
            {
                // (DEFERRED: the weaponAnim diff + Hero.reloadEquipSprite((byte) 0) —
                //  unported; equipment[0] is null in this slice, so this is unreachable.)
            }
            // GameState.setScreen(2);
            game_state::set_screen(g, 2);
            // GameLoop.instance.applyDifficultyFps();
            game_loop::apply_difficulty_fps(g);
        } else {
            // hero.beginGuardianSummon();
            // (DEFERRED: Hero.beginGuardianSummon — guardian unported; unreachable, guardian null.)
        }
        // ((Menu) this).child = null;
        g.character_menu.base.child = MenuChild::None;
        // singleton = null;
        g.character_menu.singleton = false;
        // this.equipSnapshot = null;
        g.character_menu.equip_snapshot = Vec::new();
        // text = null;
        g.character_menu.text = None;
        // GameLoop.gameScreen.markRedraw();
        game_screen::mark_redraw(g);
    }
}

/// `public final boolean handleKey(int action, int keyCode)` (`ai.a:(II)Z => []`):
/// child forward; then horizontal tab nav (via the overriding [`move_cursor`]); Back
/// (`-8`) requests state 14 (return to the world). Returns whether consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::Character, action, key_code) {
        return true;
    }
    // if (keyCode != -8) return moveCursorHorizontal(action, keyCode);
    if key_code != -8 {
        return menu::move_cursor_horizontal_node(g, MenuNode::Character, action, key_code);
    }
    // GameState.requestState((byte) 14, (byte) 0);
    game_state::request_state_a0(g, 14, 0);
    // return true;
    true
}

/// `public final void draw(Graphics graphics)` (`ai.a:(…Graphics;)V => []`): draws the
/// whole character-menu tree at the centred panel origin.
pub fn draw(g: &mut Game) {
    // render(graphics, panelX, panelY);
    let (panel_x, panel_y) = (g.character_menu.panel_x, g.character_menu.panel_y);
    menu::render_at(g, MenuNode::Character, panel_x, panel_y);
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`ai.a:(…Graphics;II)V`): the panel fill + inset panel + selected-tab cursor bevel,
/// plus the clear/soft-keys when a tab child is pushed. The six `AssetCache.menuTabIcons`
/// tab-icon draws are DEFERRED (that art bank is unported); `labelBack` (unported) is
/// passed as `None`.
pub fn paint(g: &mut Game, x: i32, y: i32) {
    // if (child != null) { clearScreen; drawSoftKeys(labelOk, labelBack); }
    let has_child = g.character_menu.base.child != MenuChild::None;
    // int cursor = ((Menu) this).cursorIndex;
    let cursor = g.character_menu.base.cursor_index as i32;
    let label_ok = g.font_manager.label_ok.clone();
    let Game {
        screen,
        font_manager,
        base_canvas,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    if has_child {
        // FontManager.clearScreen(graphics);
        crate::font_manager::clear_screen(&mut graphics, base_canvas);
        // FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
        // (labelBack is unported → passed as None.)
        crate::font_manager::draw_soft_keys(
            font_manager,
            &mut graphics,
            base_canvas,
            label_ok.as_deref(),
            None,
        );
    }
    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(x, y, 155, 176);
    graphics.fill_rect(x, y, 155, 176);
    // Menu.drawInsetPanel(graphics, x + 2, y + 15, 151, 160);
    menu::draw_inset_panel(
        &mut graphics,
        x.wrapping_add(2),
        y.wrapping_add(15),
        151,
        160,
    );
    // graphics.setColor(16768959);
    graphics.set_color(16768959);
    // graphics.fillRect(x + 5 + (cursorIndex * 16) + 1, y, 14, 1);
    graphics.fill_rect(
        x.wrapping_add(5)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(1),
        y,
        14,
        1,
    );
    // graphics.fillRect(x + 5 + (cursorIndex * 16), y + 1, 1, 16);
    graphics.fill_rect(
        x.wrapping_add(5).wrapping_add(cursor.wrapping_mul(16)),
        y.wrapping_add(1),
        1,
        16,
    );
    // graphics.setColor(12558207);
    graphics.set_color(12558207);
    // graphics.fillRect(x + 5 + (cursorIndex * 16) + 15, y + 1, 1, 15);
    graphics.fill_rect(
        x.wrapping_add(5)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(15),
        y.wrapping_add(1),
        1,
        15,
    );
    // graphics.setColor(14663551);
    graphics.set_color(14663551);
    // graphics.fillRect(x + 5 + (cursorIndex * 16) + 1, y + 1, 14, 16);
    graphics.fill_rect(
        x.wrapping_add(5)
            .wrapping_add(cursor.wrapping_mul(16))
            .wrapping_add(1),
        y.wrapping_add(1),
        14,
        16,
    );
    // int iconX = x + 7;
    // for (byte tab = 0; tab < 6; tab++) { graphics.drawImage(AssetCache.menuTabIcons[tab], iconX, y + 1, 20); iconX += 16; }
    // (DEFERRED: AssetCache.menuTabIcons art is unported — the six tab-icon draws + the
    //  `iconX += 16` walk (the shape's `iinc`) are skipped.)
}

/// `public final void moveCursor(byte direction)` (`ai.a:(B)V => []`): steps the tab
/// cursor (base `moveCursor`) then rebuilds the child tab. Only tab 0
/// ([`StatusPage`](crate::status_page)) is ported; tabs 1..5 are DEFERRED (their tab
/// classes are not in this batch).
pub fn move_cursor(g: &mut Game, direction: i8) {
    // super.moveCursor(direction);
    menu::move_cursor(&mut g.character_menu.base, direction);
    // ((Menu) this).child = null;
    g.character_menu.base.child = MenuChild::None;
    // switch (((Menu) this).cursorIndex) { ... }
    match g.character_menu.base.cursor_index as i32 {
        // case 0: child = new StatusPage(this);
        0 => {
            status_page::construct(g);
            g.character_menu.base.child = MenuChild::Status;
        }
        // case 1: child = new ItemsTab(this);
        1 => {
            // DEFERRED: ItemsTab not yet ported
        }
        // case 2: child = new EquipTab(this);
        2 => {
            // DEFERRED: EquipTab not yet ported
        }
        // case 3: child = new GuardianTab(this);
        3 => {
            // DEFERRED: GuardianTab not yet ported
        }
        // case 4: child = new SkillTab(this);
        4 => {
            // DEFERRED: SkillTab not yet ported
        }
        // case 5: child = new SystemTab(this);
        5 => {
            // DEFERRED: SystemTab not yet ported
        }
        _ => {}
    }
    // ((Menu) this).needsRepaint = true;
    g.character_menu.base.needs_repaint = true;
}
