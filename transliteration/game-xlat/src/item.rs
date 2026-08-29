//! Transliterated from `java/src/main/java/defpackage/Item.java`
//! (original `ad.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! The root of the item hierarchy `Item -> Equipment -> Armor -> Weapon`
//! (`ad -> e -> t -> l`). Java uses real inheritance and stores items
//! polymorphically (`ItemBag` keeps an `Item[]` with `instanceof`/virtual
//! dispatch), so the four classes are flattened into ONE [`Item`] struct carrying
//! every subclass's fields plus an [`ItemClass`] discriminant. Each field is
//! documented with its originating class; virtual methods (`parseRecord`,
//! `serialize`) are modelled as dispatchers ([`parse_record`], [`serialize`]) that
//! switch on the discriminant, and `super.method()` calls are direct calls to the
//! specific superclass function ([`parse_record_base`], [`crate::equipment`], ...).
//! `instanceof Equipment`/`instanceof Armor` become [`is_equipment`]/[`is_armor`].
//!
//! ## Deferred cross-class boundaries
//!
//! - **`name`/`description` id indirection.** `parseName`/`parseDescription` read a
//!   `[u8 len][ASCII-decimal digits]` field, then Java resolves it to display text
//!   via `FontManager.getStringChars` → `StringTable` (the loaded lang blob). Per
//!   the contract and the task spec the id stays an **indirection** — this module
//!   stores the ASCII-decimal id chars in `name`/`description` and marks the
//!   `FontManager`/`StringTable` resolution as DEFERRED (not resolved here).
//! - **`typeNames` (`itm/itmtp`) + `typeName()`.** `Item.typeNames` is a static
//!   `TextTable` (`z`, `itm/itmtp.tdf`) resolved through the lang table. `TextTable`
//!   is not ported; the static and `typeName()` are DEFERRED (not needed by the
//!   record-parse cross-check or `ItemBag`). Same for `Armor.attributeNames`.
//!
//! ## Opcode shapes (R8, `_reference/numeric_shapes.json`)
//!
//! `ad.a:(Z[BI)I => iadd,iadd,iadd` (parseRecord), `ad.a:([BI)I => iinc,iadd`
//! (parseName), `ad.b:([BI)I => iinc,iadd` (parseDescription),
//! `ad.c:([BI)I => iadd,iand,imul,iadd,iadd,iand,imul,iadd,iadd,iand,imul,iadd,iand,iadd`
//! (parsePrice — little-endian, `* 2^k` not `<<`), `ad.a:(B)V => iadd,i2b`
//! (addQuantity), `ad.b:(B)V => isub,i2b` (removeQuantity),
//! `ad.a:([BIZZ)Lad; => iinc,iinc` (createFromBytes),
//! `ad.a:(Lad;Lad;Lad;)Lad; => iinc×10` (craft),
//! `ad.a:()[Ljava/util/Vector; => iinc,iinc,iadd,iinc` (buildShopStock). All
//! reconcile with the decompiled Java (iinc↔`x+1` rendering aside).

use crate::asset_cache;
use crate::byte_util::ByteUtilState;
use crate::game::Game;

/// Which concrete class an [`Item`] is — the flattened stand-in for the Java
/// `Item -> Equipment -> Armor -> Weapon` runtime type (used for virtual dispatch
/// and `instanceof`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemClass {
    /// `Item` (`ad`) — base carryable / usable item.
    Item,
    /// `Equipment` (`e`) — equippable accessory.
    Equipment,
    /// `Armor` (`t`) — equipment carrying a combat attribute.
    Armor,
    /// `Weapon` (`l`) — wieldable armor with accuracy/crit.
    Weapon,
}

/// The flattened item: every field of `Item`/`Equipment`/`Armor`/`Weapon`, tagged
/// by [`ItemClass`]. Field defaults reproduce the Java field initializers (all 0 /
/// false / empty; `quantity = 1`; Equipment+ allocate `enchant = new byte[4]`).
#[derive(Debug, Clone)]
pub struct Item {
    /// The concrete runtime class (dispatch + `instanceof`).
    pub class: ItemClass,

    // --- Item (`ad`) fields ---
    /// `public byte type;` — item category (0-2 weapon, 3 armor, 4-6 accessory, ...).
    pub r#type: i8,
    /// `public byte subId;` — sub-index within `type`.
    pub sub_id: i8,
    /// `public char[] name;` — the record's name lang-id (ASCII-decimal chars; the
    /// text resolution via `FontManager`/`StringTable` is DEFERRED, see module doc).
    pub name: Vec<u16>,
    /// `public char[] description;` — the record's description lang-id (as `name`).
    pub description: Vec<u16>,
    /// `public int price;` — buy price, packed little-endian in the record.
    pub price: i32,
    /// `public byte quantity = 1;` — stack quantity.
    pub quantity: i8,

    // --- Equipment (`e`) fields ---
    /// `public short value;` — buy/sell value.
    pub value: i16,
    /// `public byte levelReq;` — minimum hero level to equip.
    pub level_req: i8,
    /// `public boolean needsIdentify;`
    pub needs_identify: bool,
    /// `public boolean identified;`
    pub identified: bool,
    /// `public byte refineLevel;`
    pub refine_level: i8,
    /// `public byte[] enchant;` — four rolled enchant values (`new byte[4]` for
    /// Equipment+; empty for a base `Item`).
    pub enchant: Vec<i8>,

    // --- Armor (`t`) field ---
    /// `public byte attribute;` — combat attribute / proc index (-1 = none).
    pub attribute: i8,

    // --- Weapon (`l`) fields ---
    /// `public byte accuracy;` — added to the hero's critical-hit chance.
    pub accuracy: i8,
    /// `public byte critBonus;` — critical-hit damage bonus (tenths of damage).
    pub crit_bonus: i8,
}

/// `public static final boolean[] STACKABLE` — whether each item `type` stacks.
pub const STACKABLE: [bool; 24] = [
    false, false, false, false, false, false, false, true, true, true, true, true, true, true,
    true, true, true, true, false, false, false, false, true, true,
];

/// `public static final boolean[] QUICK_USABLE` — whether each item `type` is a
/// quick-use consumable.
pub const QUICK_USABLE: [bool; 24] = [
    false, false, false, false, false, false, false, true, false, true, false, false, true, true,
    true, true, true, false, false, false, false, false, false, false,
];

/// The `switch (type)` that selects the concrete subclass (shared by `create` and
/// `createFromBytes`), returning a freshly-constructed, unparsed item. `pub` so the
/// record cross-check oracle can build a typed item then drive [`parse_record`] with
/// injected bytes (the same dispatch the game reaches via `create`/`load`).
// The OR arms mirror the Java `switch` `case` labels verbatim (not folded to ranges).
#[allow(clippy::manual_range_patterns)]
pub fn construct_for_type(r#type: i8, sub_id: i8) -> Item {
    match r#type {
        0 | 1 | 2 => new_weapon(r#type, sub_id),
        3 => new_armor(r#type, sub_id),
        4 | 5 | 6 => new_equipment(r#type, sub_id),
        _ => new_item(r#type, sub_id),
    }
}

fn new_base(r#type: i8, sub_id: i8, class: ItemClass) -> Item {
    // Item(byte type, byte subId): this.type = type; this.subId = subId;
    //   (quantity = 1 from the `public byte quantity = 1;` field initializer.)
    Item {
        class,
        r#type,
        sub_id,
        name: Vec::new(),
        description: Vec::new(),
        price: 0,
        quantity: 1,
        value: 0,
        level_req: 0,
        needs_identify: false,
        identified: false,
        refine_level: 0,
        enchant: Vec::new(),
        attribute: 0,
        accuracy: 0,
        crit_bonus: 0,
    }
}

/// `new Item(type, subId)`.
pub fn new_item(r#type: i8, sub_id: i8) -> Item {
    new_base(r#type, sub_id, ItemClass::Item)
}

/// `new Equipment(type, subId)` — `super(...)` then `this.enchant = new byte[4];`.
pub fn new_equipment(r#type: i8, sub_id: i8) -> Item {
    let mut it = new_base(r#type, sub_id, ItemClass::Equipment);
    it.enchant = vec![0i8; 4];
    it
}

/// `new Armor(type, subId)` — chains `Equipment(...)` (so `enchant = new byte[4]`).
pub fn new_armor(r#type: i8, sub_id: i8) -> Item {
    let mut it = new_base(r#type, sub_id, ItemClass::Armor);
    it.enchant = vec![0i8; 4];
    it
}

/// `new Weapon(type, subId)` — chains `Armor(...)`/`Equipment(...)`.
pub fn new_weapon(r#type: i8, sub_id: i8) -> Item {
    let mut it = new_base(r#type, sub_id, ItemClass::Weapon);
    it.enchant = vec![0i8; 4];
    it
}

/// `true` iff `item instanceof Equipment` (Equipment/Armor/Weapon).
pub fn is_equipment(item: &Item) -> bool {
    matches!(
        item.class,
        ItemClass::Equipment | ItemClass::Armor | ItemClass::Weapon
    )
}

/// `true` iff `item instanceof Armor` (Armor/Weapon).
pub fn is_armor(item: &Item) -> bool {
    matches!(item.class, ItemClass::Armor | ItemClass::Weapon)
}

/// `public final void load(boolean rollEnchants)`
/// (`ad.a:(Z)V => []`): `parseRecord(rollEnchants, AssetCache.loadItemRecord(type, subId), 1)`.
/// `loadItemRecord` is the (previously deferred) `AssetCache` byte gateway.
pub fn load(g: &mut Game, item: &mut Item, roll_enchants: bool) {
    // parseRecord(rollEnchants, AssetCache.loadItemRecord(this.type, this.subId), 1);
    let data = asset_cache::load_item_record(g, item.r#type, item.sub_id)
        .expect("AssetCache.loadItemRecord returned null (item record absent)");
    parse_record(item, &mut g.byte_util, roll_enchants, &data, 1);
}

/// VIRTUAL `parseRecord` dispatcher — the observable `item.parseRecord(...)` call
/// (`ad.a`/`e.a`/`t.a`/`l.a`, all `(Z[BI)I`). `super.parseRecord` calls target the
/// specific superclass function directly, never this dispatcher.
pub fn parse_record(
    item: &mut Item,
    byte_util: &mut ByteUtilState,
    roll_enchants: bool,
    data: &[i8],
    offset: i32,
) -> i32 {
    match item.class {
        ItemClass::Item => parse_record_base(item, data, offset),
        ItemClass::Equipment => {
            crate::equipment::parse_record(item, byte_util, roll_enchants, data, offset)
        }
        ItemClass::Armor => {
            crate::armor::parse_record(item, byte_util, roll_enchants, data, offset)
        }
        ItemClass::Weapon => {
            crate::weapon::parse_record(item, byte_util, roll_enchants, data, offset)
        }
    }
}

/// `Item.parseRecord` (`ad.a:(Z[BI)I => iadd,iadd,iadd`) — the base body, which
/// `Equipment.parseRecord`'s `super.parseRecord` calls. `rollEnchants` is unused
/// in the base (present only for the overriding signature).
pub fn parse_record_base(item: &mut Item, data: &[i8], offset: i32) -> i32 {
    // int afterName = offset + parseName(data, offset);
    let after_name = offset.wrapping_add(parse_name(item, data, offset));
    // int afterDesc = afterName + parseDescription(data, afterName);
    let after_desc = after_name.wrapping_add(parse_description(item, data, after_name));
    // return afterDesc + parsePrice(data, afterDesc);
    after_desc.wrapping_add(parse_price(item, data, after_desc))
}

/// `public final int parseName(byte[] data, int offset)` (`ad.a:([BI)I => iinc,iadd`).
/// Reads `[u8 len][ASCII lang-id]`; the id-to-text resolution is DEFERRED (module doc).
pub fn parse_name(item: &mut Item, data: &[i8], offset: i32) -> i32 {
    // byte len = data[offset];
    let len: i8 = data[offset as usize];
    // this.name = FontManager.getStringChars(new String(data, offset + 1, (int) len));
    //   DEFERRED text resolution: keep the ASCII-decimal id chars (indirection).
    item.name = ascii_string_chars(data, offset.wrapping_add(1), len as i32);
    // return 1 + len;
    (1i32).wrapping_add(len as i32)
}

/// `public final int parseDescription(byte[] data, int offset)`
/// (`ad.b:([BI)I => iinc,iadd`). As [`parse_name`] into `description`.
pub fn parse_description(item: &mut Item, data: &[i8], offset: i32) -> i32 {
    // byte len = data[offset];
    let len: i8 = data[offset as usize];
    // this.description = FontManager.getStringChars(new String(data, offset + 1, (int) len));
    item.description = ascii_string_chars(data, offset.wrapping_add(1), len as i32);
    // return 1 + len;
    (1i32).wrapping_add(len as i32)
}

/// `public final int parsePrice(byte[] data, int offset)`
/// (`ad.c:([BI)I => iadd,iand,imul,iadd,...`). Little-endian 4-byte price via `* 2^k`.
pub fn parse_price(item: &mut Item, data: &[i8], offset: i32) -> i32 {
    // this.price += (data[offset + 3] & 255) * 16777216;
    item.price = item.price.wrapping_add(
        ((data[offset.wrapping_add(3) as usize] as i32) & 255).wrapping_mul(16777216),
    );
    // this.price += (data[offset + 2] & 255) * 65536;
    item.price = item
        .price
        .wrapping_add(((data[offset.wrapping_add(2) as usize] as i32) & 255).wrapping_mul(65536));
    // this.price += (data[offset + 1] & 255) * 256;
    item.price = item
        .price
        .wrapping_add(((data[offset.wrapping_add(1) as usize] as i32) & 255).wrapping_mul(256));
    // this.price += data[offset] & 255;
    item.price = item
        .price
        .wrapping_add((data[offset as usize] as i32) & 255);
    // return 4;
    4
}

/// `Item.serialize` (`ad.a:()[B`) — the base 10-byte save form (type/sub/qty). This
/// is the body `Equipment.serialize`'s `super.serialize()` calls.
pub fn serialize_base(item: &Item) -> Vec<i8> {
    // byte[] out = new byte[10];
    let mut out: Vec<i8> = vec![0i8; 10];
    // out[0] = this.type; out[1] = this.subId; out[2] = this.quantity;
    out[0] = item.r#type;
    out[1] = item.sub_id;
    out[2] = item.quantity;
    out
}

/// VIRTUAL `serialize` dispatcher — `item.serialize()`. `Weapon` has no override, so
/// it inherits `Armor.serialize` (`Armor.serialize` is `final`).
pub fn serialize(item: &Item) -> Vec<i8> {
    match item.class {
        ItemClass::Item => serialize_base(item),
        ItemClass::Equipment => crate::equipment::serialize(item),
        ItemClass::Armor | ItemClass::Weapon => crate::armor::serialize(item),
    }
}

/// `public final void addQuantity(byte amount)` (`ad.a:(B)V => iadd,i2b`).
pub fn add_quantity(item: &mut Item, amount: i8) {
    // this.quantity = (byte) (this.quantity + amount);
    item.quantity = (item.quantity as i32).wrapping_add(amount as i32) as i8;
}

/// `public final void removeQuantity(byte amount)` (`ad.b:(B)V => isub,i2b`).
pub fn remove_quantity(item: &mut Item, amount: i8) {
    // this.quantity = (byte) (this.quantity - amount);
    item.quantity = (item.quantity as i32).wrapping_sub(amount as i32) as i8;
}

/// `public final boolean isUsable()` (`ad.a:()Z => []`).
pub fn is_usable(item: &Item) -> bool {
    // return this.type == 10 || this.type == 7 || this.type == 8 || this.type == 9;
    item.r#type == 10 || item.r#type == 7 || item.r#type == 8 || item.r#type == 9
}

/// `public final boolean isQuestItem()` (`ad.b:()Z => []`).
pub fn is_quest_item(item: &Item) -> bool {
    // return this.type == 18 || this.type == 19 || this.type == 20 || this.type == 21;
    item.r#type == 18 || item.r#type == 19 || item.r#type == 20 || item.r#type == 21
}

/// `public static final Item create(byte type, byte subId, boolean parse, boolean rollEnchants)`
/// (`ad.a:(BBZZ)Lad; => []`).
pub fn create(g: &mut Game, r#type: i8, sub_id: i8, parse: bool, roll_enchants: bool) -> Item {
    // switch (type) { ... } — pick the concrete subclass.
    let mut item = construct_for_type(r#type, sub_id);
    // if (parse) item.load(rollEnchants);
    if parse {
        load(g, &mut item, roll_enchants);
    }
    // item.quantity = (byte) 1;
    item.quantity = 1;
    item
}

/// `public static final Item createFromBytes(byte[] data, int offset, boolean parse, boolean rollEnchants)`
/// (`ad.a:([BIZZ)Lad; => iinc,iinc`). Reads `type`/`subId` inline, then parses at
/// `offset + 2`. Unlike [`create`] it does NOT reset quantity.
pub fn create_from_bytes(
    byte_util: &mut ByteUtilState,
    data: &[i8],
    offset: i32,
    parse: bool,
    roll_enchants: bool,
) -> Item {
    // int p = offset + 1; byte type = data[offset]; int p2 = p + 1; byte subId = data[p];
    let p = offset.wrapping_add(1);
    let r#type = data[offset as usize];
    let p2 = p.wrapping_add(1);
    let sub_id = data[p as usize];
    let mut item = construct_for_type(r#type, sub_id);
    // if (parse) item.parseRecord(rollEnchants, data, p2);
    if parse {
        parse_record(&mut item, byte_util, roll_enchants, data, p2);
    }
    item
}

/// `public static final Item deserialize(byte[] saved)` (`ad.a:([B)Lad; => []`) —
/// reconstructs an item from its 10-byte save form (see [`serialize`]).
pub fn deserialize(g: &mut Game, saved: &[i8]) -> Item {
    // Item item = create(saved[0], saved[1], true, true);
    let mut item = create(g, saved[0], saved[1], true, true);
    // item.quantity = saved[2];
    item.quantity = saved[2];
    // if (item instanceof Equipment) { ... }
    if is_equipment(&item) {
        item.identified = saved[3] == 1;
        item.refine_level = saved[4];
        crate::equipment::set_enchant(&mut item, saved[5], saved[6], saved[7], saved[8]);
    }
    // if (item instanceof Armor) { ((Armor) item).attribute = saved[9]; }
    if is_armor(&item) {
        item.attribute = saved[9];
    }
    item
}

/// `public static final Vector[] buildShopStock()` (`ad.a:()[Ljava/util/Vector; =>
/// iinc,iinc,iadd,iinc`). Six category lists; each equipment marked `identified`.
/// (`Vector` → `Vec`; `trimToSize` is a capacity-only no-op, elided.)
// The OR arms mirror the Java `switch` `case` labels verbatim (not folded to ranges).
#[allow(clippy::manual_range_patterns)]
pub fn build_shop_stock(g: &mut Game) -> Vec<Vec<Item>> {
    // Vector[] categories = new Vector[6]; for (i=0;i<6;i++) categories[i] = new Vector();
    let mut categories: Vec<Vec<Item>> = (0..6).map(|_| Vec::new()).collect();
    // byte[] data = AssetCache.loadShopItemData();
    let data = asset_cache::load_shop_item_data(g).expect("loadShopItemData returned null");
    // int pos = 0; while (pos < data.length) { ... }
    let mut pos: i32 = 0;
    while pos < data.len() as i32 {
        // byte recLen = data[pos];
        let rec_len = data[pos as usize];
        // Item item = createFromBytes(data, pos + 1, true, false);
        let item = create_from_bytes(&mut g.byte_util, &data, pos.wrapping_add(1), true, false);
        // pos += 1 + recLen;
        pos = pos.wrapping_add((1i32).wrapping_add(rec_len as i32));
        // switch (item.type) { ... categories[k].addElement(item); }
        match item.r#type {
            0 | 1 | 2 => {
                let mut item = item;
                item.identified = true;
                categories[1].push(item);
            }
            3 => {
                let mut item = item;
                item.identified = true;
                categories[2].push(item);
            }
            4 => {
                let mut item = item;
                item.identified = true;
                categories[5].push(item);
            }
            5 => {
                let mut item = item;
                item.identified = true;
                categories[3].push(item);
            }
            6 => {
                let mut item = item;
                item.identified = true;
                categories[4].push(item);
            }
            7 | 9 | 10 => {
                categories[0].push(item);
            }
            _ => {}
        }
    }
    // for (i=0;i<6;i++) categories[i].trimToSize();   — capacity-only, elided.
    categories
}

/// `public static final Item craft(Item a, Item b, Item c)`
/// (`ad.a:(Lad;Lad;Lad;)Lad; => iinc×10`). Matches the `itm/mixtbl` recipe whose
/// ingredient multiset equals the (up to three) given ingredients.
// The `? 0 + 1 :` identity is preserved from the Java source (not simplified).
#[allow(clippy::identity_op)]
pub fn craft(
    g: &mut Game,
    ingredient_a: Option<Item>,
    ingredient_b: Option<Item>,
    ingredient_c: Option<Item>,
) -> Option<Item> {
    // int ingredientCount = ingredientA != null ? 0 + 1 : 0;
    let mut ingredient_count: i32 = if ingredient_a.is_some() { 0 + 1 } else { 0 };
    // if (ingredientB != null) ingredientCount++;
    if ingredient_b.is_some() {
        ingredient_count = ingredient_count.wrapping_add(1);
    }
    // if (ingredientC != null) ingredientCount++;
    if ingredient_c.is_some() {
        ingredient_count = ingredient_count.wrapping_add(1);
    }
    // byte[] table = AssetCache.readResource("/itm/mixtbl");
    let table = asset_cache::read_resource(g, "/itm/mixtbl").expect("mixtbl absent");
    // (type, subId) snapshot of the three ingredients — the loop nulls matched slots.
    let src: [Option<(i8, i8)>; 3] = [
        ingredient_a.as_ref().map(|it| (it.r#type, it.sub_id)),
        ingredient_b.as_ref().map(|it| (it.r#type, it.sub_id)),
        ingredient_c.as_ref().map(|it| (it.r#type, it.sub_id)),
    ];
    // int pos = 0; while (pos < table.length) { ... }
    let mut pos: i32 = 0;
    while pos < table.len() as i32 {
        // Item[] ingredients = { a, b, c };  (reset each recipe)
        let mut ingredients: [Option<(i8, i8)>; 3] = src;
        // byte recipeCount = table[pos]; pos++;
        let recipe_count = table[pos as usize];
        pos = pos.wrapping_add(1);
        // boolean allMatched = true;
        let mut all_matched = true;
        // for (int n = 0; n < recipeCount; n++) { ... }
        let mut n: i32 = 0;
        while n < recipe_count as i32 {
            // byte reqType = table[pos]; pos++; byte reqSub = table[pos]; pos++;
            let req_type = table[pos as usize];
            pos = pos.wrapping_add(1);
            let req_sub = table[pos as usize];
            pos = pos.wrapping_add(1);
            // boolean found = false;
            let mut found = false;
            // for (int k = 0; k < 3; k++) { if (ingredients[k] != null && type&&sub match) { found=true; ingredients[k]=null; break; } }
            let mut k: usize = 0;
            while k < 3 {
                if let Some((t, s)) = ingredients[k] {
                    if t == req_type && s == req_sub {
                        found = true;
                        ingredients[k] = None;
                        break;
                    }
                }
                k += 1;
            }
            // if (!found) allMatched = false;
            if !found {
                all_matched = false;
            }
            n = n.wrapping_add(1);
        }
        // byte resultType = table[pos]; pos++; byte resultSub = table[pos]; pos++;
        let result_type = table[pos as usize];
        pos = pos.wrapping_add(1);
        let result_sub = table[pos as usize];
        pos = pos.wrapping_add(1);
        // if (recipeCount != ingredientCount) allMatched = false;
        if recipe_count as i32 != ingredient_count {
            all_matched = false;
        }
        // if (allMatched) return create(resultType, resultSub, true, true);
        if all_matched {
            return Some(create(g, result_type, result_sub, true, true));
        }
    }
    // return null;
    None
}

/// `new String(data, offset, len)` for ASCII payloads → `char[]` (`Vec<u16>`), the
/// unresolved id chars. Faithful for the ASCII-decimal ids these fields hold; each
/// byte zero-extends to a `char` (`& 255`).
fn ascii_string_chars(data: &[i8], offset: i32, len: i32) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::with_capacity(len.max(0) as usize);
    let mut i: i32 = 0;
    while i < len {
        out.push((data[offset.wrapping_add(i) as usize] as i32 & 255) as u16);
        i = i.wrapping_add(1);
    }
    out
}
