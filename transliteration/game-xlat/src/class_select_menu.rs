//! Transliterated from `java/src/main/java/defpackage/ClassSelectMenu.java`
//! (original `c.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The starting-class selection screen (`ClassSelectMenu extends Menu`), reached
//! from [`MainMenu`](crate::main_menu) (New Game). It shows three class portraits
//! from `AssetCache.classFaces`, highlighting the one under the cursor; the two
//! side classes stay locked until a character has been created
//! (`GameLoop.hasCreatedCharacter`), otherwise picking them just shows a hint
//! message. Confirming a class opens `ClassConfirmMenu` for
//! `classId = 6 + (2 - cursorIndex)`.
//!
//! ## ANTI-BOG boundary
//!
//! This increment ports the constructor, `handleKey`, and a **PARTIAL** `paint`
//! (the shared title plate + menu panel + heading only — the class-portrait faces,
//! the class-name labels, and the soft keys cross into as-yet-unported statics
//! (`AssetCache.classFaces` / `AssetCache.commonText` / `FontManager.labelBack`)
//! and the class-select art is not oracle-captured, so they are DEFERRED). In
//! `handleKey`, the two `child = new ClassConfirmMenu(...)` sites are now **real**
//! (the flat model materialises the [`ClassConfirmMenu`](crate::class_confirm_menu)
//! child state and links it via the [`MenuChild`] discriminant); the `showMessage`
//! popup sets its discriminant faithfully but DEFERs the child construction
//! (`PopupMenu` is next-lane); `onPopupResult` and `parent.close()` are DEFERRED too.
//!
//! `ClassSelectMenu` has **no `static` fields** (its `Menu` base fields are
//! per-instance), so it contributes no `java/reconstruction/ownership.tsv` rows.
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`): `c.<init>:(Lbf;)V => []`
//! (constructor), `c.a:(II)Z => [isub,iadd,i2b,isub,iadd,i2b]` (handleKey — the two
//! `(byte)(6 + (2 - cursorIndex))` classIds), `c.a:(…Graphics;II)V => [iadd×22,ishr]`
//! (paint — the ported subset is the `(x+155)>>1` heading centre + the `y+…`
//! offsets; the remaining iadds live in the DEFERRED faces/labels).

use crate::class_confirm_menu;
use crate::font_manager;
use crate::game::Game;
use crate::main_menu;
use crate::menu::{self, MenuChild, MenuNode};
use j2me_jvm::ishr;

/// Java `c` / `ClassSelectMenu` instance state — the `Menu` (`cb`) base fields.
/// It adds no fields of its own beyond the base.
#[derive(Debug, Default, Clone)]
pub struct ClassSelectMenuState {
    /// The `Menu` (`cb`) base instance fields.
    pub base: menu::MenuBase,
}

/// `public ClassSelectMenu(MainMenu parent)` (`c.<init>:(Lbf;)V => []`):
/// `super(parent, (byte) 3); cursorIndex = (byte) 2;`.
pub fn construct(g: &mut Game) {
    // super(parent, (byte) 3);   (parent is the MainMenu → non-null → present)
    g.class_select_menu.base = menu::construct(true, 3);
    // ((Menu) this).cursorIndex = (byte) 2;
    g.class_select_menu.base.cursor_index = 2;
}

/// `public final boolean handleKey(int action, int keyCode)`
/// (`c.a:(II)Z => [isub,iadd,i2b,isub,iadd,i2b]`): horizontal class navigation, the
/// side-class lock (`cursorIndex` 0/1 locked until `GameLoop.hasCreatedCharacter`),
/// and FIRE → confirm the class. The child menus it pushes (`ClassConfirmMenu` /
/// `PopupMenu`) are DEFERRED — the discriminant is set faithfully, the construction
/// is next-lane. Returns whether the key was consumed.
pub fn handle_key(g: &mut Game, action: i32, key_code: i32) -> bool {
    // if (passKeyToChild(action, keyCode) || moveCursorHorizontal(action, keyCode)) return true;
    if menu::pass_key_to_child(g, MenuNode::ClassSelect, action, key_code)
        || menu::move_cursor_horizontal(&mut g.class_select_menu.base, action, key_code)
    {
        return true;
    }
    // if (keyCode != 53 && action != 8) { if (keyCode != -8) return true; parent.close(); return true; }
    if key_code != 53 && action != 8 {
        if key_code != -8 {
            return true;
        }
        // ((Menu) this).parent.close();
        // (DEFERRED: Menu.close — drops the child + GameScreen.activate + invalidateUp;
        // close()/GameScreen.activate not ported. Back is not on the wired FIRE path.)
        return true;
    }
    // if (cursorIndex != 0 && cursorIndex != 1) { child = new ClassConfirmMenu(this, (byte)(6 + (2 - cursorIndex))); return true; }
    if (g.class_select_menu.base.cursor_index as i32) != 0
        && (g.class_select_menu.base.cursor_index as i32) != 1
    {
        // (byte) (6 + (2 - cursorIndex))
        let class_id: i8 = 6i32
            .wrapping_add(2i32.wrapping_sub(g.class_select_menu.base.cursor_index as i32))
            as i8;
        // ((Menu) this).child = new ClassConfirmMenu(this, classId);
        // — materialise the child state (with the computed classId), then link it.
        class_confirm_menu::construct(g, class_id);
        g.class_select_menu.base.child = MenuChild::ClassConfirm;
        return true;
    }
    // if (GameLoop.instance.hasCreatedCharacter) { child = new ClassConfirmMenu(this, (byte)(6 + (2 - cursorIndex))); return true; }
    if g.game_loop.has_created_character {
        // (byte) (6 + (2 - cursorIndex))
        let class_id: i8 = 6i32
            .wrapping_add(2i32.wrapping_sub(g.class_select_menu.base.cursor_index as i32))
            as i8;
        // ((Menu) this).child = new ClassConfirmMenu(this, classId);
        // — materialise the child state (with the computed classId), then link it.
        class_confirm_menu::construct(g, class_id);
        g.class_select_menu.base.child = MenuChild::ClassConfirm;
        return true;
    }
    // showMessage(new Object[]{AssetCache.commonText.get(6), AssetCache.commonText.get(7)});
    // (DEFERRED: PopupMenu message — showMessage / AssetCache.commonText not ported;
    // discriminant set faithfully.)
    g.class_select_menu.base.child = MenuChild::Popup;
    // return true;
    true
}

/// `public final void paint(Graphics graphics, int x, int y)`
/// (`c.a:(…Graphics;II)V`): **PARTIAL** — the parchment fill + the shared title
/// plate + the "select class" heading + the menu panel. The class-portrait faces
/// (`AssetCache.classFaces`), the class-name labels (`AssetCache.commonText`), and
/// the soft keys (`FontManager.labelBack`) are DEFERRED (those statics are
/// unported and the class-select art is not oracle-captured).
pub fn paint(g: &mut Game, x: i32, y: i32) {
    let Game {
        screen,
        asset_cache,
        base_canvas,
        font_manager,
        ..
    } = &mut *g;
    let target = screen.as_mut().expect("framebuffer");
    let mut graphics = j2me_me::Graphics::new(target);

    // graphics.setColor(4136767);
    graphics.set_color(4136767);
    // graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
    graphics.fill_rect(0, 0, base_canvas.width, base_canvas.height);
    // MainMenu.drawTitlePlate(graphics, x, y);
    main_menu::draw_title_plate(asset_cache, &mut graphics, x, y);
    // FontManager.drawMenuItem(graphics, 1, (x + 155) >> 1, y + 5);
    font_manager::draw_menu_item(
        font_manager,
        &mut graphics,
        base_canvas,
        1,
        ishr(x.wrapping_add(155), 1),
        y.wrapping_add(5),
    );
    // MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
    main_menu::draw_menu_panel(asset_cache, &mut graphics, x, y.wrapping_add(24), 3);

    // (DEFERRED — the class-portrait faces, class-name labels, and soft keys cross
    // into as-yet-unported statics. Faithful full form:
    //   int baseX = x + 15; int baseY = y + 10;
    //   the six cursorIndex-gated drawImage(AssetCache.classFaces[5|4|3|2|1|0]) portraits,
    //   graphics.setColor(0);
    //   FontManager.drawChars(graphics, baseX+11, baseY+104, AssetCache.commonText.get(12), 1);
    //   FontManager.drawChars(graphics, baseX+11, baseY+119, AssetCache.commonText.get(13), 1);
    //   FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    // AssetCache.classFaces / AssetCache.commonText / FontManager.labelBack are unported.)
}
