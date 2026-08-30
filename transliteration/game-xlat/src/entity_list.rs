//! Transliterated from `java/src/main/java/defpackage/EntityList.java`
//! (original `aq.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Intrusive doubly-linked list of `Entity` (`ck`) nodes kept sorted by draw
//! depth. Nodes carry their own `next`/`prev` links, so the list stores only the
//! `head`/`tail` endpoints and a `count`. `GameMap` owns one such list per map and
//! paints it front to back; depth is the entity's foot line `halfH + pixelY`, and
//! [`reorder_by_depth`] slides a single moved node back into sorted position.
//!
//! An **instance** class (no `static` fields → no `ownership.tsv` rows). The shared
//! `Entity` (`ck`) heap seam — [`EntityArena`] / [`EntityNode`] / [`EntityId`] — now
//! lives in [`crate::entity`] (the ported `Entity` base and its subclass records)
//! and is re-exported here unchanged; this module keeps only the list
//! endpoint/relocation logic, which reads the base fields `next`, `prev`, `halfH`,
//! `pixelY`, `removed`. A Java reference is a slab **index** ([`EntityId`]);
//! `node.equals(target)` (Object identity — `Entity` overrides nothing) becomes
//! index equality. The methods take `&mut EntityListState` and `&mut EntityArena`,
//! never `self` (contract: *Statics and ownership*).
//!
//! Opcode shapes (R8, `_reference/numeric_shapes.json`):
//! `aq.a:(Lck;)V (addFront) => ["iadd"]` (count++),
//! `aq.b:(Lck;)V (addBack) => ["iadd"]` (count++),
//! `aq.a:(Lck;)Lck; (remove) => ["isub"]` (count-- on a field → isub),
//! `aq.c:(Lck;)V (reorderByDepth) => ["iadd" ×8]` (the four `halfH + pixelY`
//! depth sums, each side of the four comparisons — recomputed, never hoisted, to
//! preserve the multiset).

// The shared `Entity` heap seam ([`EntityArena`] / [`EntityNode`] / [`EntityId`])
// lives in [`crate::entity`]; re-exported so this module's public surface (and the
// EntityList oracle) is unchanged.
pub use crate::entity::{EntityArena, EntityId, EntityNode};

/// Java `aq` / `EntityList` instance state (all fields instance, not `static`).
#[derive(Debug, Default)]
pub struct EntityListState {
    /// `public Entity head;` — first node (lowest depth), or `None` when empty.
    pub head: Option<EntityId>,
    /// `public Entity tail;` — last node (highest depth), or `None` when empty.
    pub tail: Option<EntityId>,
    /// `public int count;` — number of nodes currently linked.
    pub count: i32,
}

impl EntityListState {
    /// `new EntityList()` — empty list.
    pub fn new() -> Self {
        Self::default()
    }
}

/// `node.halfH + node.pixelY` — the depth foot line. `byte` and `short` both
/// promote to `int` before the add (baload/saload sign-extend); one `iadd`.
/// Recomputed at every use, never hoisted, so the method's `iadd` multiset holds.
#[inline]
fn depth(arena: &EntityArena, id: EntityId) -> i32 {
    (arena[id].half_h as i32).wrapping_add(arena[id].pixel_y as i32)
}

/// `public final void addFront(Entity node)` (`aq.a:(Lck;)V`). Links `node` at the
/// head of the list.
pub fn add_front(s: &mut EntityListState, arena: &mut EntityArena, node: EntityId) {
    // node.next = this.head;
    arena[node].next = s.head;
    // node.prev = null;
    arena[node].prev = None;
    // if (this.head != null) this.head.prev = node;
    if let Some(head) = s.head {
        arena[head].prev = Some(node);
    }
    // this.head = node;
    s.head = Some(node);
    // if (this.tail == null) this.tail = this.head;
    if s.tail.is_none() {
        s.tail = s.head;
    }
    // this.count++;
    s.count = s.count.wrapping_add(1);
}

/// `public final void addBack(Entity node)` (`aq.b:(Lck;)V`). Links `node` at the
/// tail of the list.
pub fn add_back(s: &mut EntityListState, arena: &mut EntityArena, node: EntityId) {
    // node.prev = this.tail;
    arena[node].prev = s.tail;
    // node.next = null;
    arena[node].next = None;
    // if (this.tail != null) this.tail.next = node;
    if let Some(tail) = s.tail {
        arena[tail].next = Some(node);
    }
    // this.tail = node;
    s.tail = Some(node);
    // if (this.head == null) this.head = this.tail;
    if s.head.is_none() {
        s.head = s.tail;
    }
    // this.count++;
    s.count = s.count.wrapping_add(1);
}

/// `public final Entity remove(Entity target)` (`aq.a:(Lck;)Lck;`). Unlinks the
/// node equal to `target` (by identity) and returns it, or `None` when absent.
pub fn remove(
    s: &mut EntityListState,
    arena: &mut EntityArena,
    target: EntityId,
) -> Option<EntityId> {
    // Entity node; Entity cursor = this.head;
    // while (true) { node = cursor; if (node == null || node.equals(target)) break; cursor = node.next; }
    let mut cursor = s.head;
    let node: Option<EntityId>;
    loop {
        match cursor {
            None => {
                node = None;
                break;
            }
            Some(id) => {
                if id == target {
                    node = Some(id);
                    break;
                }
                cursor = arena[id].next;
            }
        }
    }
    // if (node == null) return null;
    let node = node?;
    let node_next = arena[node].next;
    let node_prev = arena[node].prev;
    // if (node.prev != null) node.prev.next = node.next; else this.head = node.next;
    match node_prev {
        Some(prev) => arena[prev].next = node_next,
        None => s.head = node_next,
    }
    // if (node.next != null) node.next.prev = node.prev; else this.tail = node.prev;
    match node_next {
        Some(next) => arena[next].prev = node_prev,
        None => s.tail = node_prev,
    }
    // this.count--;   (field decrement → getfield/iconst/isub/putfield)
    s.count = s.count.wrapping_sub(1);
    // return node;
    Some(node)
}

/// `public final void reorderByDepth(Entity node)` (`aq.c:(Lck;)V`). Re-sorts a
/// single `node` whose depth just changed, moving it toward the head if it now
/// sits in front of its predecessor, or toward the tail if it now sits behind its
/// successor. Depth is the foot line `halfH + pixelY`.
///
/// Preserved asymmetry (`EntityList.java`): the toward-tail branch sets
/// `node.removed = true`; the toward-head branch does not. Reproduced verbatim.
///
/// Preserved defect (`EntityList.java:112` / `:140`, byte-verified against
/// `javap -c -p aq.class` — no `isub`/`iinc` on `count` in `aq.c`, only the eight
/// depth `iadd`): the method unlinks `node` from its old position **without
/// decrementing `count`**, then, when the node relocates all the way to the head
/// or tail, calls [`add_front`] / [`add_back`] — each of which does `count++`. So
/// every reorder that slides a node to an extreme end erroneously **inflates
/// `count` by one** (the node total is unchanged; only the counter over-counts).
/// A reorder that lands via the manual mid-list splice leaves `count` correct.
/// Reproduced exactly; latent in practice (the game reads `count` for iteration
/// bounds where the padding is tolerated).
pub fn reorder_by_depth(s: &mut EntityListState, arena: &mut EntityArena, node: EntityId) {
    // if (node.prev != null && node.halfH + node.pixelY < node.prev.halfH + node.prev.pixelY) {
    if let Some(prev) = arena[node].prev {
        if depth(arena, node) < depth(arena, prev) {
            // node.prev.next = node.next;
            let node_next = arena[node].next;
            arena[prev].next = node_next;
            // if (node.next == null) this.tail = node.prev; else node.next.prev = node.prev;
            match node_next {
                None => s.tail = Some(prev),
                Some(next) => arena[next].prev = Some(prev),
            }
            // Entity scan = node.prev;
            // while (true) { backward = scan;
            //   if (backward == null || node.halfH+node.pixelY >= backward.halfH+backward.pixelY) break;
            //   else scan = backward.prev; }
            let mut scan = Some(prev);
            let backward: Option<EntityId>;
            loop {
                match scan {
                    None => {
                        backward = None;
                        break;
                    }
                    Some(b) => {
                        if depth(arena, node) >= depth(arena, b) {
                            backward = Some(b);
                            break;
                        }
                        scan = arena[b].prev;
                    }
                }
            }
            // if (backward == null) { addFront(node); return; }
            match backward {
                None => {
                    // Preserved defect: node was unlinked above without count--, so
                    // add_front's count++ inflates count by one (EntityList.java:112).
                    add_front(s, arena, node);
                    return;
                }
                Some(b) => {
                    // backward.next.prev = node;   (NPE if backward.next == null — unguarded, faithful)
                    let b_next = arena[b].next;
                    arena[b_next.expect("NullPointerException: backward.next")].prev = Some(node);
                    // node.next = backward.next;
                    arena[node].next = b_next;
                    // backward.next = node;
                    arena[b].next = Some(node);
                    // node.prev = backward;
                    arena[node].prev = Some(b);
                    return;
                }
            }
        }
    }
    // if (node.next == null || node.halfH + node.pixelY <= node.next.halfH + node.next.pixelY) return;
    let node_next = arena[node].next;
    let next = match node_next {
        None => return,
        Some(next) => next,
    };
    if depth(arena, node) <= depth(arena, next) {
        return;
    }
    // node.removed = true;
    arena[node].removed = true;
    // node.next.prev = node.prev;
    let node_prev = arena[node].prev;
    arena[next].prev = node_prev;
    // if (node.prev == null) this.head = node.next; else node.prev.next = node.next;
    match node_prev {
        None => s.head = Some(next),
        Some(prev) => arena[prev].next = Some(next),
    }
    // Entity scan = node.next;
    // while (true) { forward = scan;
    //   if (forward == null || node.halfH+node.pixelY <= forward.halfH+forward.pixelY) break;
    //   else scan = forward.next; }
    let mut scan = Some(next);
    let forward: Option<EntityId>;
    loop {
        match scan {
            None => {
                forward = None;
                break;
            }
            Some(f) => {
                if depth(arena, node) <= depth(arena, f) {
                    forward = Some(f);
                    break;
                }
                scan = arena[f].next;
            }
        }
    }
    // if (forward == null) { addBack(node); return; }
    match forward {
        None => {
            // Preserved defect: node was unlinked above without count--, so
            // add_back's count++ inflates count by one (EntityList.java:140).
            add_back(s, arena, node);
        }
        Some(f) => {
            // forward.prev.next = node;   (NPE if forward.prev == null — unguarded, faithful)
            let f_prev = arena[f].prev;
            arena[f_prev.expect("NullPointerException: forward.prev")].next = Some(node);
            // node.prev = forward.prev;
            arena[node].prev = f_prev;
            // forward.prev = node;
            arena[f].prev = Some(node);
            // node.next = forward;
            arena[node].next = Some(f);
        }
    }
}
