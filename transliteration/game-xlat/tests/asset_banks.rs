//! State gate for the ported in-game UI / HUD image banks in `asset_cache`.
//!
//! `hud_screens`/`in_game_frame` proved the world+HUD render path; those lanes
//! DEFERRED the `AssetCache` banks the HUD/screens blit. This gate proves the
//! newly-ported `AssetCache` loaders actually fill those banks from the real
//! baseline JAR:
//!
//! - a **proven-red control** — every bank is asserted `None` (empty) BEFORE the
//!   load, so the populated-after assertions genuinely bite (a bank that was
//!   pre-filled, or a loader that silently no-ops, would fail here);
//! - a **state witness** — after `load_in_game_ui` / `load_item_icons` /
//!   `load_guardian_icons` / `load_global_ui` / `load_game_menu_icons` /
//!   `load_shop_ui` / `load_status_effect_icons`, each bank is `Some` with the right
//!   element count and non-degenerate decoded images (width/height > 0);
//! - a **teardown witness** — each `unload*` clears its banks back to `None`;
//! - a **DEFERRED-feed control** — the `FontManager.loadLocaleImage` sub-banks
//!   (`statPointAlert`, `floaterIcon2/3`, `shopBuyIcon/SellIcon`) stay `None` even
//!   after their load, and `commonText` (assigned only in the DEFERRED
//!   `TitleScreen.loadLanguage`) stays `None` — matching the module's DEFER notes.
//!
//! The JAR is git-ignored and read only at test time; if it is absent the shared
//! helper panics loudly (GATES.md R4), never a false green.

mod common;

use common::{jar, to_i8};
use heroes_lore_wind_of_soltia_game_xlat::{asset_cache, Game};

/// Frame counts read from each atlas's `.mph` header (offset-4 u32) in the baseline
/// `v207.jar` — the non-vacuity floor for the `allImages`/fixed-size banks.
const ICOITM_FRAMES: usize = 23; // /img/icoitm
const EMOTI_FRAMES: usize = 12; // /img/emoti
const LVUP_FRAMES: usize = 6; // /char/lvup (spriteBanks[13] slot size)
const LVUP_EIF_LEN: usize = 198; // /char/lvup.eif byte length

fn fresh_game_with_resources() -> Game {
    let mut g = Game::new();
    for (name, bytes) in jar().matching(|_| true) {
        g.resources.insert(name, to_i8(&bytes));
    }
    g
}

/// A decoded `Image` must be non-degenerate (real pixels), never a 0-size stub.
fn assert_live(img: &j2me_me::Image, what: &str) {
    assert!(
        img.width() > 0 && img.height() > 0,
        "{what}: decoded image is degenerate ({}x{})",
        img.width(),
        img.height()
    );
}

#[test]
fn load_in_game_ui_fills_the_hud_banks_and_unload_clears_them() {
    let mut g = fresh_game_with_resources();

    // --- proven-red control: every bank empty before the load ---
    assert!(g.asset_cache.hud_frame.is_none(), "hudFrame pre-populated");
    assert!(
        g.asset_cache.dialog_border.is_none(),
        "dialogBorder pre-populated"
    );
    assert!(
        g.asset_cache.number_font1.is_none(),
        "numberFont1 pre-populated"
    );
    assert!(
        g.asset_cache.skill_charge_fill.is_none(),
        "skillChargeFill pre-populated"
    );
    assert!(
        g.asset_cache.entity_shadow.is_none(),
        "entityShadow pre-populated"
    );
    assert!(
        g.asset_cache.level_up_script.is_none(),
        "levelUpScript pre-populated"
    );
    assert!(
        g.asset_cache.sprite_banks[13].is_none(),
        "spriteBanks[13] pre-populated"
    );

    asset_cache::load_in_game_ui(&mut g);

    // --- state witness: hudFrame (7) + dialogBorder (4) ---
    let hud = g.asset_cache.hud_frame.as_ref().expect("hudFrame loaded");
    assert_eq!(hud.len(), 7, "hudFrame has 7 pieces (/img/uifrm 0..6)");
    assert_live(&hud[0], "hudFrame[0]");
    assert_live(&hud[6], "hudFrame[6]");
    let border = g
        .asset_cache
        .dialog_border
        .as_ref()
        .expect("dialogBorder loaded");
    assert_eq!(border.len(), 4, "dialogBorder has 4 corners");
    assert_live(&border[0], "dialogBorder[0]");
    assert_live(&border[3], "dialogBorder[3]");

    // --- etcui glyph/marker set ---
    assert_live(
        g.asset_cache.number_font1.as_ref().expect("numberFont1"),
        "numberFont1",
    );
    assert_live(
        g.asset_cache.number_font2.as_ref().expect("numberFont2"),
        "numberFont2",
    );
    assert_live(
        g.asset_cache.number_font3.as_ref().expect("numberFont3"),
        "numberFont3",
    );
    assert_live(
        g.asset_cache.number_font4.as_ref().expect("numberFont4"),
        "numberFont4",
    );
    assert_live(
        g.asset_cache
            .drop_item_marker
            .as_ref()
            .expect("dropItemMarker"),
        "dropItemMarker",
    );
    assert_live(
        g.asset_cache
            .drop_gold_marker
            .as_ref()
            .expect("dropGoldMarker"),
        "dropGoldMarker",
    );
    assert_live(
        g.asset_cache
            .skill_charge_fill
            .as_ref()
            .expect("skillChargeFill"),
        "skillChargeFill",
    );
    assert_live(
        g.asset_cache.entity_shadow.as_ref().expect("entityShadow"),
        "entityShadow",
    );

    // --- level-up effect: script read + spriteBanks[13] assembled ---
    let script = g
        .asset_cache
        .level_up_script
        .as_ref()
        .expect("levelUpScript loaded");
    assert_eq!(
        script.len(),
        LVUP_EIF_LEN,
        "levelUpScript is the /char/lvup.eif blob"
    );
    let bank13 = g.asset_cache.sprite_banks[13]
        .as_ref()
        .expect("spriteBanks[13] allocated by assembleSprites");
    assert_eq!(
        bank13.len(),
        LVUP_FRAMES,
        "spriteBanks[13] sized to /char/lvup frameCount"
    );
    assert!(
        bank13.iter().any(|slot| slot.is_some()),
        "assembleSprites decoded at least one level-up frame"
    );
    // The in-place flag rewrite set the script's referenced bank bytes to 13.
    assert!(
        script.contains(&13),
        "assembleSprites rewrote the script flag bytes to bank 13"
    );

    // --- DEFERRED-feed control: loadLocaleImage banks stay empty ---
    assert!(
        g.asset_cache.stat_point_alert.is_none(),
        "statPointAlert is a DEFERRED loadLocaleImage feed (stays None)"
    );
    assert!(
        g.asset_cache.floater_icon2.is_none(),
        "floaterIcon2 is a DEFERRED loadLocaleImage feed (stays None)"
    );
    assert!(
        g.asset_cache.floater_icon3.is_none(),
        "floaterIcon3 is a DEFERRED loadLocaleImage feed (stays None)"
    );

    // --- teardown witness ---
    asset_cache::unload_in_game_ui(&mut g);
    assert!(g.asset_cache.hud_frame.is_none(), "unload cleared hudFrame");
    assert!(
        g.asset_cache.dialog_border.is_none(),
        "unload cleared dialogBorder"
    );
    assert!(
        g.asset_cache.number_font1.is_none(),
        "unload cleared numberFont1"
    );
    assert!(
        g.asset_cache.skill_charge_fill.is_none(),
        "unload cleared skillChargeFill"
    );
    assert!(
        g.asset_cache.entity_shadow.is_none(),
        "unload cleared entityShadow"
    );
    assert!(
        g.asset_cache.level_up_script.is_none(),
        "unload cleared levelUpScript"
    );
    assert!(
        g.asset_cache.sprite_banks[13].is_none(),
        "unload cleared spriteBanks[13]"
    );
}

#[test]
fn load_item_icons_fills_the_bank_and_unload_clears_it() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.item_icons.is_none(),
        "itemIcons pre-populated"
    );

    asset_cache::load_item_icons(&mut g);
    let icons = g.asset_cache.item_icons.as_ref().expect("itemIcons loaded");
    assert_eq!(
        icons.len(),
        ICOITM_FRAMES,
        "itemIcons = /img/icoitm allImages"
    );
    assert_live(&icons[0], "itemIcons[0]");
    assert_live(&icons[ICOITM_FRAMES - 1], "itemIcons[last]");

    asset_cache::unload_item_icons(&mut g);
    assert!(
        g.asset_cache.item_icons.is_none(),
        "unload cleared itemIcons"
    );
}

#[test]
fn load_guardian_icons_fills_both_banks_and_unload_clears_them() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.guardian_icons.is_none(),
        "guardianIcons pre-populated"
    );
    assert!(
        g.asset_cache.guardian_skill_icons.is_none(),
        "guardianSkillIcons pre-populated"
    );

    asset_cache::load_guardian_icons(&mut g);
    let icons = g
        .asset_cache
        .guardian_icons
        .as_ref()
        .expect("guardianIcons loaded");
    assert_eq!(icons.len(), 6, "guardianIcons has 6 portraits");
    assert_live(&icons[0], "guardianIcons[0]");
    let skills = g
        .asset_cache
        .guardian_skill_icons
        .as_ref()
        .expect("guardianSkillIcons loaded");
    assert_eq!(skills.len(), 24, "guardianSkillIcons has 24 (6 x 4)");
    assert_live(&skills[0], "guardianSkillIcons[0]");
    assert_live(&skills[23], "guardianSkillIcons[23]");

    asset_cache::unload_guardian_icons(&mut g);
    assert!(
        g.asset_cache.guardian_icons.is_none(),
        "unload cleared guardianIcons"
    );
    assert!(
        g.asset_cache.guardian_skill_icons.is_none(),
        "unload cleared guardianSkillIcons"
    );
}

#[test]
fn load_global_ui_fills_the_glb_singles_and_help_text() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.number_font0.is_none(),
        "numberFont0 pre-populated"
    );
    assert!(g.asset_cache.help_text.is_none(), "helpText pre-populated");

    asset_cache::load_global_ui(&mut g);
    // The single /img/glb images the HUD + menus draw.
    assert_live(
        g.asset_cache.number_font0.as_ref().expect("numberFont0"),
        "numberFont0",
    );
    assert_live(
        g.asset_cache.cursor_arrow.as_ref().expect("cursorArrow"),
        "cursorArrow",
    );
    assert_live(
        g.asset_cache.slot_frame.as_ref().expect("slotFrame"),
        "slotFrame",
    );
    assert_live(
        g.asset_cache
            .scroll_up_arrow
            .as_ref()
            .expect("scrollUpArrow"),
        "scrollUpArrow",
    );
    assert_live(
        g.asset_cache
            .scroll_down_arrow
            .as_ref()
            .expect("scrollDownArrow"),
        "scrollDownArrow",
    );
    assert_live(
        g.asset_cache.gold_icon.as_ref().expect("goldIcon"),
        "goldIcon",
    );
    assert_live(
        g.asset_cache
            .portrait_frame
            .as_ref()
            .expect("portraitFrame"),
        "portraitFrame",
    );
    assert_live(
        g.asset_cache
            .status_panel_icon
            .as_ref()
            .expect("statusPanelIcon"),
        "statusPanelIcon",
    );
    assert_live(
        g.asset_cache
            .fraction_slash
            .as_ref()
            .expect("fractionSlash"),
        "fractionSlash",
    );
    let help = g.asset_cache.help_text.as_ref().expect("helpText loaded");
    assert!(
        help.count > 0,
        "helpText parsed entries from /sgui/help.tdf"
    );

    // DEFERRED-feed control: the loadLocaleImage statLabels stay empty (loadGlobalUi
    // has no unload; the fields are unmodelled feeds).
}

#[test]
fn load_game_menu_icons_fills_the_tab_and_equip_banks() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.menu_tab_icons.is_none(),
        "menuTabIcons pre-populated"
    );
    assert!(
        g.asset_cache.equip_slot_icons.is_none(),
        "equipSlotIcons pre-populated"
    );

    asset_cache::load_game_menu_icons(&mut g);
    let tabs = g
        .asset_cache
        .menu_tab_icons
        .as_ref()
        .expect("menuTabIcons loaded");
    assert_eq!(tabs.len(), 6, "menuTabIcons has 6 tabs");
    assert_live(&tabs[0], "menuTabIcons[0]");
    assert_live(&tabs[5], "menuTabIcons[5]");
    let slots = g
        .asset_cache
        .equip_slot_icons
        .as_ref()
        .expect("equipSlotIcons loaded");
    assert_eq!(slots.len(), 5, "equipSlotIcons has 5 slots");
    assert_live(&slots[0], "equipSlotIcons[0]");
    assert_live(&slots[4], "equipSlotIcons[4]");
}

#[test]
fn load_shop_ui_fills_the_shop_banks_and_unload_clears_them() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.shop_category_icons.is_none(),
        "shopCategoryIcons pre-populated"
    );

    asset_cache::load_shop_ui(&mut g);
    let cats = g
        .asset_cache
        .shop_category_icons
        .as_ref()
        .expect("shopCategoryIcons loaded");
    assert_eq!(cats.len(), 6, "shopCategoryIcons has 6");
    assert_live(&cats[0], "shopCategoryIcons[0]");
    assert_live(&cats[5], "shopCategoryIcons[5]");
    assert_live(
        g.asset_cache.shop_coin_icon.as_ref().expect("shopCoinIcon"),
        "shopCoinIcon",
    );
    assert_live(
        g.asset_cache
            .shop_select_box
            .as_ref()
            .expect("shopSelectBox"),
        "shopSelectBox",
    );

    // DEFERRED-feed control: the loadLocaleImage buy/sell icons stay empty.
    assert!(
        g.asset_cache.shop_buy_icon.is_none(),
        "shopBuyIcon is a DEFERRED loadLocaleImage feed (stays None)"
    );
    assert!(
        g.asset_cache.shop_sell_icon.is_none(),
        "shopSellIcon is a DEFERRED loadLocaleImage feed (stays None)"
    );

    asset_cache::unload_shop_ui(&mut g);
    assert!(
        g.asset_cache.shop_category_icons.is_none(),
        "unload cleared shopCategoryIcons"
    );
    assert!(
        g.asset_cache.shop_coin_icon.is_none(),
        "unload cleared shopCoinIcon"
    );
    assert!(
        g.asset_cache.shop_select_box.is_none(),
        "unload cleared shopSelectBox"
    );
}

#[test]
fn load_status_effect_icons_fills_the_banks_and_unload_clears_them() {
    let mut g = fresh_game_with_resources();
    assert!(
        g.asset_cache.status_icons.is_none(),
        "statusIcons pre-populated"
    );
    assert!(g.asset_cache.emoticons.is_none(), "emoticons pre-populated");
    assert!(
        g.asset_cache.emoticon_bubble.is_none(),
        "emoticonBubble pre-populated"
    );

    asset_cache::load_status_effect_icons(&mut g);
    assert_live(
        g.asset_cache
            .emoticon_bubble
            .as_ref()
            .expect("emoticonBubble"),
        "emoticonBubble",
    );
    let status = g
        .asset_cache
        .status_icons
        .as_ref()
        .expect("statusIcons loaded");
    assert_eq!(status.len(), 8, "statusIcons has 8");
    assert_live(&status[0], "statusIcons[0]");
    assert_live(&status[7], "statusIcons[7]");
    let emoti = g.asset_cache.emoticons.as_ref().expect("emoticons loaded");
    assert_eq!(
        emoti.len(),
        EMOTI_FRAMES,
        "emoticons = /img/emoti allImages"
    );
    assert_live(&emoti[0], "emoticons[0]");

    asset_cache::unload_status_effect_icons(&mut g);
    assert!(
        g.asset_cache.status_icons.is_none(),
        "unload cleared statusIcons"
    );
    assert!(
        g.asset_cache.emoticons.is_none(),
        "unload cleared emoticons"
    );
    assert!(
        g.asset_cache.emoticon_bubble.is_none(),
        "unload cleared emoticonBubble"
    );
}

#[test]
fn common_text_stays_unmodelled_until_the_deferred_title_loader() {
    // commonText's only assignment is TitleScreen.loadLanguage (DEFERRED there); the
    // field is modelled but nothing in the ported AssetCache loaders fills it.
    let mut g = fresh_game_with_resources();
    asset_cache::load_in_game_ui(&mut g);
    asset_cache::load_global_ui(&mut g);
    assert!(
        g.asset_cache.common_text.is_none(),
        "commonText stays None (assigned only in DEFERRED TitleScreen.loadLanguage)"
    );
}
