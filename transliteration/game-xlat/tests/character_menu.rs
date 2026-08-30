//! STATE oracle for the character-stats menu cluster ported this lane:
//! `CharacterMenu` (`ai`), `StatusPage` (`q`) and `StatAllocMenu` (`bi`).
//!
//! * `CharacterMenu.instance()` builds the six-tab singleton over a New-Game hero,
//!   centres the panel at `halfW-77`/`halfH-85`, and opens on the status tab
//!   (`child == StatusPage`); the overriding `moveCursor` switches tabs (only tab 0's
//!   `StatusPage` is ported — the other five are DEFERRED, so their tabs leave
//!   `child == None`), rebuilding the status child on the way back.
//! * `StatusPage` page 3 (FIRE) pushes a `StatAllocMenu` when the hero has unspent
//!   points, and is inert on the other pages.
//! * `StatAllocMenu` queues pending points on the four base stats without touching the
//!   hero (LEFT refunds, RIGHT spends), and — on a confirmed yes (`onPopupResult`,
//!   tag 2, result 0, from a `PopupMenu`) — commits the deltas onto the hero,
//!   recomputes the derived stats, and pops back to the status page.
//!
//! These are STATE assertions, not pixel diffs: the character-menu art is partial
//! (DEFERRED — it crosses into unported `AssetCache.heroText`/icon banks +
//! `BaseCanvas`/`Menu` label widgets). The `open()` load of the shared `/sgui/gm`
//! label table is checked against the real JAR.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::entity::EntityId;
use heroes_lore_wind_of_soltia_game_xlat::menu::MenuChild;
use heroes_lore_wind_of_soltia_game_xlat::{
    byte_util, character_menu, hero, stat_alloc_menu, status_page, Game,
};

/// Deterministic RNG seed used by the other state tests (`new Random()` is time-seeded
/// on device; a fixed seed keeps the port reproducible).
const GAME_RNG_SEED: i64 = 305_419_896;
/// The warrior class (class ids run 6..8).
const CLASS_WARRIOR: i8 = 6;

/// FIRE / select — `keyCode 53` (KEY_NUM5).
const KEY_FIRE: i32 = 53;
/// DOWN — `keyCode 56` (KEY_NUM8).
const KEY_DOWN: i32 = 56;
/// RIGHT — `keyCode 54` (KEY_NUM6) — spends a point on the selected stat.
const KEY_RIGHT: i32 = 54;
/// LEFT — `keyCode 52` (KEY_NUM4) — refunds a point.
const KEY_LEFT: i32 = 52;

/// A New-Game hero of `class_id` placed as `GameState.hero`, with `stat_points`
/// unspent level-up points granted (a fresh `initClass` hero has 0). Returns its id.
fn new_game_hero(g: &mut Game, class_id: i8, stat_points: i16) -> EntityId {
    g.byte_util = byte_util::ByteUtilState::seeded(GAME_RNG_SEED);
    // GameState.classId is read by StatusPage's className/lore lookups (DEFERRED) and
    // by CharacterMenu.open's equip-anim indices (DEFERRED); set it faithfully.
    g.game_state.class_id = class_id;
    // new Hero(0, 0, 8, 8, classId); GameState.hero = hero; hero.initClass(classId).
    let id = hero::new_hero(&mut g.entity_arena, &g.clock, 0, 0, 8, 8, class_id);
    g.game_state.hero = Some(id);
    hero::init_class(g, id, class_id);
    // Grant unspent points (as a real level-up would) so the alloc path is reachable.
    g.entity_arena[id]
        .as_hero_mut()
        .expect("Hero node")
        .stat_points = stat_points;
    id
}

/// `CharacterMenu.instance()` opens on the status tab; page-3 FIRE with unspent points
/// opens `StatAllocMenu`; RIGHT queues a pending point; a confirmed yes commits it onto
/// the hero and recomputes derived stats.
#[test]
fn character_menu_status_tab_and_stat_allocation() {
    let mut g = Game::new();
    let hero_id = new_game_hero(&mut g, CLASS_WARRIOR, 5);
    // A concrete centred origin so panelX/panelY are checkable.
    g.base_canvas.half_w = 88;
    g.base_canvas.half_h = 104;

    // --- CharacterMenu.instance(): the six-tab singleton, opened on the status tab. ---
    character_menu::instance(&mut g);
    assert!(
        g.character_menu.singleton,
        "instance() created the singleton"
    );
    assert_eq!(
        g.character_menu.base.item_count, 6,
        "super(null, (byte) 6) → six tabs"
    );
    assert!(
        !g.character_menu.base.parent,
        "the character menu is a root (super(null, …) → no parent)"
    );
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Status,
        "instance() pushed a StatusPage child (tab 0)"
    );
    assert_eq!(g.character_menu.panel_x, 88 - 77, "panelX = halfW - 77");
    assert_eq!(g.character_menu.panel_y, 104 - 85, "panelY = halfH - 85");
    // The pushed StatusPage: a four-page panel, parented to the character menu.
    assert!(
        g.status_page.base.parent,
        "StatusPage super(parentMenu, …) → parent present"
    );
    assert_eq!(
        g.status_page.base.item_count, 4,
        "StatusPage super(parent, (byte) 4) → four pages"
    );
    assert_eq!(
        g.status_page.base.child,
        MenuChild::None,
        "the status page has no child yet"
    );

    // --- The tabbed shell: switch to a DEFERRED tab and back to the status tab. ---
    // moveCursor(4) advances tab 0 → 1 (ItemsTab, DEFERRED → child cleared to None).
    character_menu::move_cursor(&mut g, 4);
    assert_eq!(g.character_menu.base.cursor_index, 1, "tab cursor 0 → 1");
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::None,
        "tab 1 (ItemsTab) is DEFERRED → no child"
    );
    // moveCursor(3) retreats tab 1 → 0, rebuilding the StatusPage child.
    character_menu::move_cursor(&mut g, 3);
    assert_eq!(g.character_menu.base.cursor_index, 0, "tab cursor 1 → 0");
    assert_eq!(
        g.character_menu.base.child,
        MenuChild::Status,
        "returning to tab 0 rebuilt the StatusPage child"
    );

    // --- StatusPage: FIRE is inert off page 3, and opens StatAllocMenu on page 3. ---
    // FIRE on page 0 is a no-op (cursorIndex != 3 → returns false, no child pushed).
    assert!(
        !status_page::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE off page 3 is not consumed"
    );
    assert_eq!(
        g.status_page.base.child,
        MenuChild::None,
        "FIRE off page 3 pushes nothing"
    );
    // Navigate to page 3 (three non-wrapping DOWN steps: 0 → 1 → 2 → 3).
    for _ in 0..3 {
        assert!(
            status_page::handle_key(&mut g, 0, KEY_DOWN),
            "DOWN is consumed"
        );
    }
    assert_eq!(g.status_page.base.cursor_index, 3, "status page cursor → 3");
    // FIRE on page 3 with statPoints > 0 pushes the allocation dialog.
    assert!(
        status_page::handle_key(&mut g, 0, KEY_FIRE),
        "FIRE on page 3 with points is consumed"
    );
    assert_eq!(
        g.status_page.base.child,
        MenuChild::StatAlloc,
        "FIRE on page 3 pushed a StatAllocMenu"
    );

    // --- StatAllocMenu: starts at the hero's balance with empty pending. ---
    assert_eq!(
        g.stat_alloc_menu.remaining_points, 5,
        "remainingPoints seeded from the hero's statPoints"
    );
    assert_eq!(
        g.stat_alloc_menu.pending,
        vec![0i16; 4],
        "pending starts empty (four base stats)"
    );
    assert!(
        g.stat_alloc_menu.base.parent,
        "StatAllocMenu super(statusPage, …) → parent present"
    );
    assert_eq!(
        g.stat_alloc_menu.base.item_count, 4,
        "StatAllocMenu super(parent, (byte) 4) → four stat rows"
    );

    // RIGHT spends a point on stat 0 (STR); LEFT refunds it; RIGHT spends it again.
    assert!(
        stat_alloc_menu::handle_key(&mut g, 0, KEY_RIGHT),
        "RIGHT is consumed"
    );
    assert_eq!(g.stat_alloc_menu.pending[0], 1, "RIGHT queued +1 on STR");
    assert_eq!(g.stat_alloc_menu.remaining_points, 4, "one point spent");
    assert!(
        stat_alloc_menu::handle_key(&mut g, 0, KEY_LEFT),
        "LEFT is consumed"
    );
    assert_eq!(
        g.stat_alloc_menu.pending[0], 0,
        "LEFT refunded the STR point"
    );
    assert_eq!(g.stat_alloc_menu.remaining_points, 5, "the point returned");
    stat_alloc_menu::handle_key(&mut g, 0, KEY_RIGHT);
    assert_eq!(g.stat_alloc_menu.pending[0], 1, "re-spent on STR");

    // The hero is UNTOUCHED until the deltas are confirmed.
    let (str_before, attack_before, points_before) = {
        let h = g.entity_arena[hero_id].as_hero().expect("Hero node");
        (h.strength, h.attack, h.stat_points)
    };
    assert_eq!(
        str_before, 8,
        "class-6 base STR is 8 (unchanged while pending)"
    );
    assert_eq!(points_before, 5, "statPoints unchanged while pending");

    // --- Confirm (a PopupMenu yes): commit the deltas + recompute + pop back. ---
    // The confirm popup is the StatAllocMenu's child (as showPopup(2, 2, …) would set);
    // its yes result routes back here as onPopupResult(2, 0) with the popup as the child.
    g.stat_alloc_menu.base.child = MenuChild::Popup;
    stat_alloc_menu::on_popup_result(&mut g, 2, 0);

    let h = g.entity_arena[hero_id].as_hero().expect("Hero node");
    assert_eq!(h.strength, str_before + 1, "STR committed (+1 pending)");
    assert_eq!(h.stat_points, 4, "statPoints set to the remaining balance");
    assert_ne!(
        h.attack, attack_before,
        "recomputeStats reran — the derived attack changed with STR"
    );
    // The dialog popped: StatAllocMenu dismissed and StatusPage's child cleared.
    assert_eq!(
        g.stat_alloc_menu.base.child,
        MenuChild::None,
        "the confirm popup was dismissed"
    );
    assert_eq!(
        g.status_page.base.child,
        MenuChild::None,
        "onPopupResult popped the StatAllocMenu off the status page"
    );

    // NEGATIVE CONTROL: a hero with no unspent points has nothing to allocate — a
    // StatAllocMenu over it starts at 0 and RIGHT (spend) is a no-op (so the "one point
    // spent" assertions above cannot read as fixed constants).
    let mut g2 = Game::new();
    new_game_hero(&mut g2, CLASS_WARRIOR, 0);
    stat_alloc_menu::construct(&mut g2);
    assert_eq!(
        g2.stat_alloc_menu.remaining_points, 0,
        "a 0-point hero seeds remainingPoints 0"
    );
    stat_alloc_menu::handle_key(&mut g2, 0, KEY_RIGHT);
    assert_eq!(
        g2.stat_alloc_menu.pending[0], 0,
        "spending is blocked with 0 remaining points"
    );
    assert_eq!(
        g2.stat_alloc_menu.remaining_points, 0,
        "remainingPoints stays 0 when spending is blocked"
    );
}

/// `CharacterMenu.open()` snapshots the (null-in-this-slice) equip/guardian state and
/// loads the shared `/sgui/gm` label table from the real JAR.
#[test]
fn character_menu_open_loads_strings_and_snapshots() {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    new_game_hero(&mut g, CLASS_WARRIOR, 3);
    character_menu::instance(&mut g);

    character_menu::open(&mut g);

    // The equip snapshot is all -1 (no equipment in this slice); guardian -1 (none).
    assert_eq!(
        g.character_menu.equip_snapshot,
        vec![-1i8; 5],
        "open() reset every equip-snapshot slot to -1"
    );
    assert_eq!(
        g.character_menu.guardian_snapshot, -1,
        "no active guardian → guardianSnapshot -1"
    );
    // open() loaded the /sgui/gm game-menu label table.
    let text = g
        .character_menu
        .text
        .as_ref()
        .expect("open() set CharacterMenu.text");
    assert!(text.count > 0, "the /sgui/gm table has entries");
}
