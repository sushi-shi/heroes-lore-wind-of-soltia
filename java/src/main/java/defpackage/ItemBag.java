package defpackage;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.util.Vector;

/* renamed from: g */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:g.class */
/**
 * A fixed-capacity item store, used both for the hero's main {@code bag} and the
 * quick-item bar. Items live in the {@link #slots} array; stackable types share a
 * slot up to 99 and are coalesced by {@link #mergeStacks}, non-stackable types
 * take one slot each. It also tracks {@link #gold}.
 *
 * <p>The quick-item bar cycles through the consumable types in {@link #QUICK_TYPES}
 * ({@link #cycleQuickType}); {@link #quickTypeCursor} is the current type and
 * {@link #quickSlot} the slot backing it (-1 when none), kept consistent by
 * {@link #revalidateQuickSlot}/{@link #ensureQuickSlot}. The store (de)serializes
 * to a byte blob for the RMS save.
 */
public final class ItemBag {
    /* renamed from: a */
    /** Consumable item types the quick bar cycles through. */
    private static final byte[] QUICK_TYPES = {7, 8, 9, 10};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Backing slot array (capacity fixed at construction). */
    private Item[] slots;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Slot currently backing the quick bar (-1 = none). */
    private byte quickSlot;
    /** Index into {@link #QUICK_TYPES} of the selected quick type. */
    private byte quickTypeCursor;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Carried gold. */
    public int gold;

    public ItemBag(byte capacity) {
        this.slots = new Item[capacity];
        resetQuickSelection();
    }

    /* renamed from: a */
    /** Resets the quick-bar cursor and selects the first available quick item. */
    public final void resetQuickSelection() {
        this.quickSlot = (byte) -1;
        this.quickTypeCursor = (byte) -1;
        cycleQuickType();
    }

    /* renamed from: a */
    /** Returns the item in slot {@code index} (may be null). */
    public final Item get(int index) {
        return this.slots[index];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the indices of all occupied slots. */
    public final byte[] occupiedSlots() {
        int count = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null) {
                count++;
            }
        }
        byte[] out = new byte[count];
        int w = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null) {
                out[w++] = (byte) i;
            }
        }
        return out;
    }

    /**
     * Returns the indices of equipment slots, optionally restricted to armor
     * ({@code armorOnly}) and filtered by identified state ({@code identifiedFilter}
     * = 1 only identified, -1 only unidentified, 0 either).
     */
    public final byte[] equipmentSlots(boolean armorOnly, byte identifiedFilter) {
        int count = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && (this.slots[i] instanceof Equipment) && ((!armorOnly || (this.slots[i] instanceof Armor)) && ((identifiedFilter != 1 || ((Equipment) this.slots[i]).identified) && (identifiedFilter != -1 || !((Equipment) this.slots[i]).identified)))) {
                count++;
            }
        }
        byte[] out = new byte[count];
        int w = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && (this.slots[i] instanceof Equipment) && ((!armorOnly || (this.slots[i] instanceof Armor)) && ((identifiedFilter != 1 || ((Equipment) this.slots[i]).identified) && (identifiedFilter != -1 || !((Equipment) this.slots[i]).identified)))) {
                out[w++] = (byte) i;
            }
        }
        return out;
    }

    /** Returns the indices of all slots holding an item of type {@code type}. */
    public final byte[] slotsOfType(byte type) {
        int count = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type) {
                count++;
            }
        }
        byte[] out = new byte[count];
        int w = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type) {
                out[w++] = (byte) i;
            }
        }
        return out;
    }

    /* renamed from: b */
    /** Returns the indices of all quick-usable ({@link Item#QUICK_USABLE}) items. */
    public final byte[] quickUsableSlots() {
        int count = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && Item.QUICK_USABLE[this.slots[i].type]) {
                count++;
            }
        }
        byte[] out = new byte[count];
        int w = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && Item.QUICK_USABLE[this.slots[i].type]) {
                out[w++] = (byte) i;
            }
        }
        return out;
    }

    /* renamed from: a */
    /** Puts {@code item} in slot {@code slot}, returning the previous occupant. */
    public final Item replaceAt(Item item, byte slot) {
        Item previous = this.slots[slot];
        this.slots[slot] = item;
        return previous;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the item currently selected on the quick bar, or null. */
    public final Item currentQuickItem() {
        if (this.quickSlot == -1) {
            return null;
        }
        return this.slots[this.quickSlot];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the currently selected quick-bar item type. */
    public final byte currentQuickType() {
        return QUICK_TYPES[this.quickTypeCursor];
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    /** Advances to the next quick-bar type (wrapping) and re-selects its slot. */
    public final void cycleQuickType() {
        this.quickTypeCursor = (byte) (this.quickTypeCursor + 1);
        if (this.quickTypeCursor == 4) {
            this.quickTypeCursor = (byte) 0;
        }
        this.quickSlot = findSlot(QUICK_TYPES[this.quickTypeCursor], (byte) 0);
    }

    /* renamed from: f */
    /** Fixes {@link #quickSlot} if its item was consumed or changed type. */
    private final void revalidateQuickSlot() {
        if (this.quickSlot != -1) {
            if (this.slots[this.quickSlot] == null) {
                this.quickSlot = (byte) -1;
            } else if (this.slots[this.quickSlot].type != QUICK_TYPES[this.quickTypeCursor]) {
                this.quickSlot = findSlot(QUICK_TYPES[this.quickTypeCursor], (byte) 0);
            }
        }
    }

    /* renamed from: g */
    /** Selects a quick slot for the current type if none is selected. */
    private final void ensureQuickSlot() {
        if (this.quickSlot == -1) {
            this.quickSlot = findSlot(QUICK_TYPES[this.quickTypeCursor], (byte) 0);
        }
    }

    /* renamed from: a */
    /**
     * Adds {@code count} of {@code item}, stacking onto an existing stack when the
     * type is stackable and there is room. Returns false if it does not fit.
     */
    public final boolean add(Item item, int count) {
        if (!canAdd(item.type, item.subId, count)) {
            return false;
        }
        byte free = firstFreeSlot();
        if (!Item.STACKABLE[item.type]) {
            if (free == -1) {
                return false;
            }
            Debug.assertTrue(item.quantity == 1);
            this.slots[free] = item;
            return true;
        }
        Item[] stacks = findAllItems(item.type, item.subId);
        for (int i = 0; i < stacks.length; i++) {
            if (stacks[i].quantity + count <= 99) {
                stacks[i].addQuantity((byte) count);
                mergeStacks();
                ensureQuickSlot();
                return true;
            }
        }
        if (free == -1) {
            return false;
        }
        item.quantity = (byte) count;
        this.slots[free] = item;
        mergeStacks();
        ensureQuickSlot();
        return true;
    }

    /** Removes {@code count} items of the given type/subId across all stacks. */
    public final void removeItems(byte type, byte subId, byte count) {
        for (int guard = this.slots.length; guard > 0 && count > 0; guard--) {
            Item found = findItem(type, subId);
            if (found != null) {
                byte take = found.quantity < count ? found.quantity : count;
                decrementItem(found, take);
                count = (byte) (count - take);
            }
        }
        Debug.assertTrue(count == 0);
        mergeStacks();
    }

    /** Removes {@code count} from the item in slot {@code slot}. */
    public final void removeFromSlot(byte slot, byte count) {
        decrementItem(this.slots[slot], count);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Decrements {@code item} by {@code count}, removing it when the stack empties. */
    public final void decrementItem(Item item, byte count) {
        Debug.assertTrue(item.quantity >= count);
        item.removeQuantity(count);
        if (item.quantity < 1) {
            removeItem(item);
        }
        revalidateQuickSlot();
        mergeStacks();
    }

    /* renamed from: a */
    /** Finds and removes {@code item} from the store, compacting the gap. */
    private void removeItem(Item item) {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] == item) {
                this.slots[i] = null;
                compact();
                return;
            }
        }
    }

    /* renamed from: c */
    /** Removes all quest/key items from the store. */
    public final void removeQuestItems() {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].isQuestItem()) {
                this.slots[i] = null;
                compact();
            }
        }
    }

    /* renamed from: d */
    /** Shifts items down to fill the first empty slot. */
    public final void compact() {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] == null) {
                int j = i;
                while (j < this.slots.length - 1) {
                    this.slots[j] = this.slots[j + 1];
                    j++;
                }
                this.slots[j] = null;
                return;
            }
        }
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the first item of the given type/subId, or null. */
    public final Item findItem(byte type, byte subId) {
        byte slot = findSlot(type, subId);
        if (slot == -1) {
            return null;
        }
        return this.slots[slot];
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns every item stack of the given type/subId. */
    public final Item[] findAllItems(byte type, byte subId) {
        Vector matches = new Vector(2);
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type && this.slots[i].subId == subId) {
                matches.addElement(this.slots[i]);
            }
        }
        Item[] out = new Item[matches.size()];
        for (int i = 0; i < matches.size(); i++) {
            out[i] = (Item) matches.elementAt(i);
        }
        return out;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the total quantity held of the given type/subId. */
    public final int totalQuantity(byte type, byte subId) {
        int total = 0;
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type && this.slots[i].subId == subId) {
                total += this.slots[i].quantity;
            }
        }
        return total;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the slot holding {@code item}, or -1. */
    public final byte slotOf(Item item) {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] == item) {
                return (byte) i;
            }
        }
        return (byte) -1;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the first slot holding the given type/subId, or -1. */
    public final byte findSlot(byte type, byte subId) {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type && this.slots[i].subId == subId) {
                return (byte) i;
            }
        }
        return (byte) -1;
    }

    /**
     * Returns true when {@code count} of the given type/subId can be added (there
     * is a free slot, or room on an existing stack of a stackable type).
     */
    public final boolean canAdd(byte type, byte subId, int count) {
        mergeStacks();
        byte free = firstFreeSlot();
        if (count > 99) {
            return false;
        }
        if (free != -1) {
            return true;
        }
        if (!Item.STACKABLE[type]) {
            return false;
        }
        for (Item stack : findAllItems(type, subId)) {
            if (stack.quantity + count <= 99) {
                return true;
            }
        }
        return false;
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    /** Returns the first empty slot index, or -1. */
    private final byte firstFreeSlot() {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] == null) {
                return (byte) i;
            }
        }
        return (byte) -1;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** True when the store holds at least {@code count} of the given type/subId. */
    public final boolean hasAtLeast(byte type, byte subId, byte count) {
        for (int i = 0; i < this.slots.length; i++) {
            if (this.slots[i] != null && this.slots[i].type == type && this.slots[i].subId == subId && this.slots[i].quantity >= count) {
                return true;
            }
        }
        return false;
    }

    /* renamed from: e */
    /** Coalesces adjacent stacks of the same stackable item (capping stacks at 99). */
    public final void mergeStacks() {
        for (int i = 0; i < this.slots.length - 1; i++) {
            if (this.slots[i] != null && Item.STACKABLE[this.slots[i].type] && this.slots[i].quantity < 99) {
                for (int j = i + 1; j < this.slots.length; j++) {
                    if (this.slots[j] != null && this.slots[j].type == this.slots[i].type && this.slots[j].subId == this.slots[i].subId) {
                        byte have = this.slots[i].quantity;
                        byte other = this.slots[j].quantity;
                        if (have + other <= 99) {
                            this.slots[i].addQuantity(other);
                            this.slots[j] = null;
                            if (this.quickSlot == j) {
                                this.quickSlot = (byte) i;
                            }
                        } else {
                            byte moved = (byte) (99 - have);
                            this.slots[i].addQuantity(moved);
                            this.slots[j].removeQuantity(moved);
                        }
                    }
                }
            }
        }
    }

    /* JADX INFO: renamed from: c, reason: collision with other method in class */
    /** Serializes gold and every slot to a byte blob for the RMS save. */
    public final byte[] serialize() {
        ByteArrayOutputStream byteStream = new ByteArrayOutputStream();
        DataOutputStream out = new DataOutputStream(byteStream);
        try {
            out.writeInt(this.gold);
            for (int i = 0; i < this.slots.length; i++) {
                if (this.slots[i] == null) {
                    out.writeByte(0);
                } else {
                    out.writeByte(1);
                    out.write(this.slots[i].serialize());
                }
            }
            byte[] result = byteStream.toByteArray();
            try {
                out.close();
                byteStream.close();
            } catch (IOException unused) {
            }
            return result;
        } catch (IOException e) {
            e.printStackTrace();
            try {
                out.close();
                byteStream.close();
            } catch (IOException unused) {
            }
            return null;
        }
    }

    /** Restores gold and the slot contents from a serialized blob. */
    public final void deserialize(byte[] data) {
        ByteArrayInputStream byteStream = new ByteArrayInputStream(data);
        DataInputStream in = new DataInputStream(byteStream);
        try {
            this.gold = in.readInt();
            int w = 0;
            for (int i = 0; i < this.slots.length; i++) {
                if (in.readByte() != 0) {
                    byte[] itemBytes = new byte[10];
                    in.read(itemBytes);
                    this.slots[w++] = Item.deserialize(itemBytes);
                }
            }
            try {
                in.close();
                byteStream.close();
            } catch (IOException unused) {
            }
        } catch (IOException e) {
            e.printStackTrace();
            try {
                in.close();
                byteStream.close();
            } catch (IOException unused) {
            }
        }
        resetQuickSelection();
    }
}
