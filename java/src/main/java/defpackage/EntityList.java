package defpackage;

/* renamed from: aq */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:aq.class */
/**
 * Intrusive doubly-linked list of {@link Entity} nodes kept sorted by draw
 * depth. Nodes carry their own {@link Entity#next}/{@link Entity#prev} links, so
 * the list only stores the {@link #head}/{@link #tail} endpoints and a
 * {@link #count}. {@link GameMap} owns one such list per map and paints it front
 * to back; depth is the entity's foot line {@code halfH + pixelY}, and
 * {@link #reorderByDepth} slides a single moved node back into sorted position.
 */
public final class EntityList {

    /* renamed from: a */
    /** First node (lowest depth), or null when empty. */
    public Entity head = null;
    /** Last node (highest depth), or null when empty. */
    public Entity tail = null;
    /** Number of nodes currently linked. */
    public int count = 0;

    /* renamed from: a */
    /** Links {@code node} at the head of the list. */
    public final void addFront(Entity node) {
        node.next = this.head;
        node.prev = null;
        if (this.head != null) {
            this.head.prev = node;
        }
        this.head = node;
        if (this.tail == null) {
            this.tail = this.head;
        }
        this.count++;
    }

    /* renamed from: b */
    /** Links {@code node} at the tail of the list. */
    public final void addBack(Entity node) {
        node.prev = this.tail;
        node.next = null;
        if (this.tail != null) {
            this.tail.next = node;
        }
        this.tail = node;
        if (this.head == null) {
            this.head = this.tail;
        }
        this.count++;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Unlinks the node equal to {@code target} (by {@link Object#equals}) and
     * returns it, or null when it is not present.
     */
    public final Entity remove(Entity target) {
        Entity node;
        Entity cursor = this.head;
        while (true) {
            node = cursor;
            if (node == null || node.equals(target)) {
                break;
            }
            cursor = node.next;
        }
        if (node == null) {
            return null;
        }
        if (node.prev != null) {
            node.prev.next = node.next;
        } else {
            this.head = node.next;
        }
        if (node.next != null) {
            node.next.prev = node.prev;
        } else {
            this.tail = node.prev;
        }
        this.count--;
        return node;
    }

    /* renamed from: c */
    /**
     * Re-sorts a single {@code node} whose depth just changed, moving it toward
     * the head if it now sits in front of its predecessor, or toward the tail if
     * it now sits behind its successor. Depth is the foot line
     * {@code halfH + pixelY}.
     */
    public final void reorderByDepth(Entity node) {
        Entity backward;
        Entity forward;
        if (node.prev != null && node.halfH + node.pixelY < node.prev.halfH + node.prev.pixelY) {
            node.prev.next = node.next;
            if (node.next == null) {
                this.tail = node.prev;
            } else {
                node.next.prev = node.prev;
            }
            Entity scan = node.prev;
            while (true) {
                backward = scan;
                if (backward == null || node.halfH + node.pixelY >= backward.halfH + backward.pixelY) {
                    break;
                } else {
                    scan = backward.prev;
                }
            }
            if (backward == null) {
                addFront(node);
                return;
            }
            backward.next.prev = node;
            node.next = backward.next;
            backward.next = node;
            node.prev = backward;
            return;
        }
        if (node.next == null || node.halfH + node.pixelY <= node.next.halfH + node.next.pixelY) {
            return;
        }
        node.removed = true;
        node.next.prev = node.prev;
        if (node.prev == null) {
            this.head = node.next;
        } else {
            node.prev.next = node.next;
        }
        Entity scan = node.next;
        while (true) {
            forward = scan;
            if (forward == null || node.halfH + node.pixelY <= forward.halfH + forward.pixelY) {
                break;
            } else {
                scan = forward.next;
            }
        }
        if (forward == null) {
            addBack(node);
            return;
        }
        forward.prev.next = node;
        node.prev = forward.prev;
        forward.prev = node;
        node.next = forward;
    }
}
