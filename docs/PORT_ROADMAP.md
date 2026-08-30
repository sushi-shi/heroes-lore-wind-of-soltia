# Full-transliteration roadmap

Goal: **completely** transliterate all 90 classes (no deferred slices) **and**
decide **every** AST node in the per-node crosswalk (0 undecided across the whole
corpus), while lifting generic phone/device-platform code into the shared `j2me-*`
crates in the home `_template/`. Playable-New-Game (boot → title → menu chain →
walkable world with a moving, visible hero) is a landed milestone, not the finish.

Drive it: **2 lanes/round, menu-lane ‖ entity-lane** (disjoint primary files —
`menu.rs` vs `entity.rs`/`battler.rs`/`game_map.rs`; only `asset_cache.rs`/`game.rs`/
`lib.rs`/`ownership.tsv` overlap append-only). Port-phase first (complete every
class), then crosswalk-phase (expand the CORPUS to 90, burn every node to 0 like the
original burn-down). Every lane: `git reset --hard master` FIRST (worktree may branch
from an old commit); keep existing oracles green; DEFER only genuinely-unported
cross-class calls with a named comment; don't run the crosswalk gate during porting.

## Status (as of the campaign start)
- **15 fully ported** (no work): GameMIDlet, Adler32, Crc32, Directions, Debug,
  ByteUtil, SaveCipher, RmsFile, EntityList, Equipment, Weapon, ItemBag, SoundPlayer,
  AudioManager, PngMerger.
- **25 partial** — biggest gaps: **GameScreen ~5%** (only screen==9 arm; every other
  screen/HUD/drawFrame deferred — gates all screen rendering), **Menu ~14%** (the
  static draw kit drawBevelBox/drawButton/drawInsetPanel + popup machinery
  showPopup/onPopupResult/close — gates every concrete menu), **GameState ~13%**
  (only requestState case 2; startNewMap/warpMap/save/load/other cases deferred),
  **AssetCache ~14%** (~33 load*/unload* deferred), **GameMap ~50%** (the whole
  `/m/<classId>/<NN>.evt` parse — collision/objects/npcs/enemies/triggers — deferred),
  Hero (combat FSM + guardian/equipment in init_class), Battler (the update/move FSM +
  the OverlayRef placeholder), plus AssetLoader, BaseCanvas (loading HUD kit),
  FontManager (in-game labels + wrapped-text family), BitmapFont (drawString/drawLines),
  WrapFont (wrap/wrapInto), StringTable (resolveLocale), TitleScreen (boot/loadLanguage),
  GameLoop (options persistence), the 4 class-select menus (art), Item/Armor (typeNames
  TextTable owner gap), AppConfig (cross-owner hooks).
- **50 unported** — see the dependency order below.

## Unported dependency order (leaf-first)
- **Combat/entities** (touch entity.rs/battler.rs/game_map.rs): Overlay → StatusIcon →
  Floater → Effect → Projectile → EnemyType → **Enemy (XL)** → Boss → GuardianCastFx →
  **Guardian (XL)** → RockyBoss → Geb{Core,Head,HandLeft,HandRight} (one batch) →
  Nord{Body1,Body2,Healer,Tentacle} (one batch).
- **Dialogs/info** (touch menu.rs): **PopupMenu (highest leverage — un-defers 3 partials)**
  → AboutScreen, ContinueMenu → ScrollCaption → ConfirmDialog → HelpPage+HelpMenu →
  OptionsMenu.
- **Character menus** (menu.rs): CharacterMenu (shell first) → SystemTab → StatusPage+
  StatAllocMenu → SkillTab → ItemsTab → EquipTab → GuardianTab+GuardianSkillPanel.
- **Shop/craft** (menu.rs): ItemPickerList → SellList → BuySellDialog+ShopItemList+
  ShopMenu (batch) → CostConfirmDialog → CombineMenu+EnchantMenu+RefineMenu (batch) →
  IdentifyMenu+UpgradeMenu+BlacksmithMenu (batch).
- **World**: Npc (leaf) → **EventScript (XL)** (gates the .evt parse + boss cutscenes).

Shared match-site discipline: each Menu subclass adds a `MenuChild`+`MenuNode` variant
and arms in child_node/node_base/dispatch_handle_key/paint_node/render_node (menu.rs);
each Entity/Battler subclass adds an `EntityData` variant (entity.rs) / a real Overlay
union member (battler.rs); each wired screen adds a `GameScreen::paint`/`keyPressed`
arm and a `GameState::process_state_request` case (currently only 9 / 2 exist).

## Round 0 (prerequisite): complete Menu's draw kit + popup machinery; make
GameScreen/GameState dispatch extensible (stub arms). Unblocks nearly every round.

## Batch plan (menu lane ‖ entity lane), ~14 rounds
1. PopupMenu ‖ Overlay, StatusIcon
2. AboutScreen, ContinueMenu ‖ Floater
3. ItemPickerList, SellList ‖ Npc
4. CostConfirmDialog ‖ Effect, Projectile
5. BuySellDialog+ShopItemList+ShopMenu ‖ EnemyType
6. SystemTab, CharacterMenu(shell) ‖ Enemy (XL, maybe own round)
7. SkillTab+ConfirmDialog ‖ Boss
8. CombineMenu+EnchantMenu+RefineMenu ‖ EventScript (XL, own round)
9. IdentifyMenu+UpgradeMenu+BlacksmithMenu ‖ GuardianCastFx
10. ItemsTab, EquipTab ‖ Guardian (XL, own round)
11. StatusPage+StatAllocMenu ‖ RockyBoss
12. HelpPage+HelpMenu ‖ Geb batch (4)
13. OptionsMenu ‖ Nord batch (4)
14. GuardianTab+GuardianSkillPanel (needs Guardian from R10)
Plus, interleaved: complete the big partials (GameScreen screens, GameState cases,
AssetCache banks, GameMap .evt parse, Hero combat, Battler FSM, FontManager wrapped
text) as their dependent classes land.

## Generic platform-lifts → `_template/crates/j2me-*` (the kit source)
The peer maintains `j2me-preservation-kit` (WoS pins it as a git-dep; source in
`_template/crates/`). Lift these generic pieces (coordinate; don't clobber peer work).
Each lift = add to the kit source, re-publish/re-pin, switch game-xlat to the generic API.
1. Nokia negative keycodes (apps/linux keymap.rs `nokia`) → `j2me-me::canvas`.
2. winit input adapter (keymap.rs `nokia_code`) + winit/softbuffer presenter (shell.rs) → `j2me-platform-native`.
3. `java.util.Random` LCG + `Math.abs` (byte_util.rs JavaRandom/java_math_abs) → `j2me-jvm`.
4. `ResourceBank` — the whole `getResourceAsStream` classpath seam (resources.rs, entire file) → `j2me-me`. (its own doc says it's a device fact — cleanest lift)
5. JAR→classpath loader (apps/linux jar.rs, whole file) → `j2me-platform-native`.
6. DataInput/DataOutput big-endian int helpers (duplicated in string_table.rs + item_bag.rs) → route through `j2me-codec::Reader`; add `Writer::write_i32_be`.
7. `DataInputStream.readUTF` modified-UTF-8 decode (string_table.rs read_utf) → `j2me-codec` (missing).
8. ByteArrayIn/OutputStream (rms_file.rs) → `j2me-me`.
9. MIDlet JAD property reader (app_config.rs get_app_property) → `j2me-me`; `String.trim`/`indexOf` (app_config.rs) → `j2me-jvm`.
Already-correct (do not touch): j2me-me media.rs (MMAPI), rms.rs (RecordStore) — consumed via the host seam.
