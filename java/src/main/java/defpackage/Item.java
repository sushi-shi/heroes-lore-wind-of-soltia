package defpackage;

import java.util.Vector;

/* renamed from: ad */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ad.class */
/**
 * Base class for every carryable item and the root of the item hierarchy
 * {@code Item -> Equipment -> Armor -> Weapon}. An item is identified by a
 * {@link #type}/{@link #subId} pair and carries a display {@link #name},
 * {@link #description}, {@link #price} and stack {@link #quantity}.
 *
 * <p>Item definitions live in the {@code itm/NN} tables (one file per type):
 * each table is a run of {@code [u8 recLen][recLen bytes]} records, and the
 * record body is decoded here into fields. The name and description fields are
 * <em>indirections</em>: each is stored as {@code [u8 len][len ASCII digits]}, a
 * decimal string id that {@link FontManager#m38a} resolves against the loaded
 * language string-table into the actual {@code char[]} text. Type-level names
 * come from the {@link #typeNames} table ({@code itm/itmtp}).
 *
 * <p>The static factories build the right subclass for a {@link #type} and
 * (de)serialize the 10-byte save form.
 */
public class Item {
    /* renamed from: b */
    /** Type-level display names ({@code itm/itmtp}), indexed by {@link #type}. */
    public static TextTable typeNames;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Whether each item {@link #type} stacks in a single inventory slot. */
    public static final boolean[] STACKABLE = {false, false, false, false, false, false, false, true, true, true, true, true, true, true, true, true, true, true, false, false, false, false, true, true};
    /** Whether each item {@link #type} is a quick-use consumable. */
    public static final boolean[] QUICK_USABLE = {false, false, false, false, false, false, false, true, false, true, false, false, true, true, true, true, true, false, false, false, false, false, false, false};

    /* renamed from: f */
    /** Item category (0-2 weapon, 3 armor, 4-6 accessory, 7-10 usable, ...). */
    public byte type;

    /* renamed from: g */
    /** Sub-index within {@link #type} (selects the concrete item / sprite). */
    public byte subId;

    /* renamed from: a */
    /** Resolved display name (from the record's name lang-id). */
    public char[] name;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Resolved description text (from the record's description lang-id). */
    public char[] description;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Buy price, packed little-endian in the record. */
    public int price;

    /* renamed from: h */
    /** Stack quantity (1..99). */
    public byte quantity = 1;

    public Item(byte type, byte subId) {
        this.type = type;
        this.subId = subId;
    }

    /* renamed from: a */
    /** Loads and decodes this item's {@code itm} record from {@link AssetCache}. */
    public final void load(boolean rollEnchants) {
        parseRecord(rollEnchants, AssetCache.loadItemRecord(this.type, this.subId), 1);
    }

    /* renamed from: a */
    /** Decodes the base record fields (name, description, price) from {@code data}. */
    public int parseRecord(boolean rollEnchants, byte[] data, int offset) {
        int afterName = offset + parseName(data, offset);
        int afterDesc = afterName + parseDescription(data, afterName);
        return afterDesc + parsePrice(data, afterDesc);
    }

    /* renamed from: a */
    /** Reads the {@code [u8 len][ASCII lang-id]} name field into {@link #name}. */
    public final int parseName(byte[] data, int offset) {
        byte len = data[offset];
        this.name = FontManager.getStringChars(new String(data, offset + 1, (int) len));
        return 1 + len;
    }

    /* renamed from: b */
    /** Reads the {@code [u8 len][ASCII lang-id]} description into {@link #description}. */
    public final int parseDescription(byte[] data, int offset) {
        byte len = data[offset];
        this.description = FontManager.getStringChars(new String(data, offset + 1, (int) len));
        return 1 + len;
    }

    /* renamed from: c */
    /** Reads the 4-byte little-endian {@link #price}. */
    public final int parsePrice(byte[] data, int offset) {
        this.price += (data[offset + 3] & 255) * 16777216;
        this.price += (data[offset + 2] & 255) * 65536;
        this.price += (data[offset + 1] & 255) * 256;
        this.price += data[offset] & 255;
        return 4;
    }

    /* renamed from: a */
    /** Serializes the item to its 10-byte save form (base writes type/sub/qty). */
    public byte[] serialize() {
        byte[] out = new byte[10];
        out[0] = this.type;
        out[1] = this.subId;
        out[2] = this.quantity;
        return out;
    }

    /* renamed from: a */
    /** Adds {@code amount} to the stack. */
    public final void addQuantity(byte amount) {
        this.quantity = (byte) (this.quantity + amount);
    }

    /* renamed from: b */
    /** Removes {@code amount} from the stack. */
    public final void removeQuantity(byte amount) {
        this.quantity = (byte) (this.quantity - amount);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the type-level name for this item. */
    public final char[] typeName() {
        return typeNames.get(this.type);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** True for usable/consumable types (7-10). */
    public final boolean isUsable() {
        return this.type == 10 || this.type == 7 || this.type == 8 || this.type == 9;
    }

    /* renamed from: b */
    /** True for quest/key item types (18-21). */
    public final boolean isQuestItem() {
        return this.type == 18 || this.type == 19 || this.type == 20 || this.type == 21;
    }

    /* renamed from: a */
    /** Creates the concrete item for {@code type}/{@code subId}, optionally loading its record. */
    public static final Item create(byte type, byte subId, boolean parse, boolean rollEnchants) {
        Item item;
        switch (type) {
            case 0:
            case 1:
            case 2:
                item = new Weapon(type, subId);
                break;
            case 3:
                item = new Armor(type, subId);
                break;
            case 4:
            case 5:
            case 6:
                item = new Equipment(type, subId);
                break;
            default:
                item = new Item(type, subId);
                break;
        }
        if (parse) {
            item.load(rollEnchants);
        }
        item.quantity = (byte) 1;
        return item;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Creates the concrete item from a record at {@code offset} in {@code data}. */
    public static final Item createFromBytes(byte[] data, int offset, boolean parse, boolean rollEnchants) {
        Item item;
        int p = offset + 1;
        byte type = data[offset];
        int p2 = p + 1;
        byte subId = data[p];
        switch (type) {
            case 0:
            case 1:
            case 2:
                item = new Weapon(type, subId);
                break;
            case 3:
                item = new Armor(type, subId);
                break;
            case 4:
            case 5:
            case 6:
                item = new Equipment(type, subId);
                break;
            default:
                item = new Item(type, subId);
                break;
        }
        if (parse) {
            item.parseRecord(rollEnchants, data, p2);
        }
        return item;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Reconstructs an item from its 10-byte save form (see {@link #serialize}). */
    public static final Item deserialize(byte[] saved) {
        Item item = create(saved[0], saved[1], true, true);
        item.quantity = saved[2];
        if (item instanceof Equipment) {
            ((Equipment) item).identified = saved[3] == 1;
            ((Equipment) item).refineLevel = saved[4];
            ((Equipment) item).setEnchant(saved[5], saved[6], saved[7], saved[8]);
        }
        if (item instanceof Armor) {
            ((Armor) item).attribute = saved[9];
        }
        return item;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Builds the shop's purchasable stock from {@code itm/forshop}, grouping items
     * into six category vectors (0 usables, 1 weapons, 2 armor, 3 boots, 4 gloves,
     * 5 helmets) with each equipment marked {@link Equipment#identified}.
     */
    public static final Vector[] buildShopStock() {
        Vector[] categories = new Vector[6];
        for (int i = 0; i < 6; i++) {
            categories[i] = new Vector();
        }
        byte[] data = AssetCache.loadShopItemData();
        int pos = 0;
        while (pos < data.length) {
            byte recLen = data[pos];
            Item item = createFromBytes(data, pos + 1, true, false);
            pos += 1 + recLen;
            switch (item.type) {
                case 0:
                case 1:
                case 2:
                    ((Equipment) item).identified = true;
                    categories[1].addElement(item);
                    break;
                case 3:
                    ((Equipment) item).identified = true;
                    categories[2].addElement(item);
                    break;
                case 4:
                    ((Equipment) item).identified = true;
                    categories[5].addElement(item);
                    break;
                case 5:
                    ((Equipment) item).identified = true;
                    categories[3].addElement(item);
                    break;
                case 6:
                    ((Equipment) item).identified = true;
                    categories[4].addElement(item);
                    break;
                case 7:
                case 9:
                case 10:
                    categories[0].addElement(item);
                    break;
            }
        }
        for (int i = 0; i < 6; i++) {
            categories[i].trimToSize();
        }
        return categories;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Looks up the {@code itm/mixtbl} crafting recipe matched by the (up to three)
     * ingredient items and returns the freshly-created result, or null when no
     * recipe exactly matches the given ingredient set.
     */
    public static final Item craft(Item ingredientA, Item ingredientB, Item ingredientC) {
        int ingredientCount = ingredientA != null ? 0 + 1 : 0;
        if (ingredientB != null) {
            ingredientCount++;
        }
        if (ingredientC != null) {
            ingredientCount++;
        }
        byte[] table = AssetCache.readResource("/itm/mixtbl");
        int pos = 0;
        while (pos < table.length) {
            Item[] ingredients = new Item[3];
            ingredients[0] = ingredientA;
            ingredients[1] = ingredientB;
            ingredients[2] = ingredientC;
            byte recipeCount = table[pos];
            pos++;
            boolean allMatched = true;
            for (int n = 0; n < recipeCount; n++) {
                byte reqType = table[pos];
                pos++;
                byte reqSub = table[pos];
                pos++;
                boolean found = false;
                for (int k = 0; k < 3; k++) {
                    if (ingredients[k] != null && ingredients[k].type == reqType && ingredients[k].subId == reqSub) {
                        found = true;
                        ingredients[k] = null;
                        break;
                    }
                }
                if (!found) {
                    allMatched = false;
                }
            }
            byte resultType = table[pos];
            pos++;
            byte resultSub = table[pos];
            pos++;
            if (recipeCount != ingredientCount) {
                allMatched = false;
            }
            if (allMatched) {
                return create(resultType, resultSub, true, true);
            }
        }
        return null;
    }
}
