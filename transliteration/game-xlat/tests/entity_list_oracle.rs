//! EntityList cross-check oracle ("two implementations, one truth").
//!
//! The strict transliteration's intrusive doubly-linked list
//! ([`heroes_lore_wind_of_soltia_game_xlat::entity_list`], from `aq.class`) is
//! driven in lock-step with an INDEPENDENT reference that implements the same
//! endpoint/relocation semantics over a plain `Vec<EntityId>` (array scan + insert
//! instead of pointer splicing). After every operation both must present the same
//! head→tail order and the same count, and the linked list must stay a consistent
//! doubly-linked list (tail→prev is the reverse of head→next). A negative control
//! proves the comparison bites.

use heroes_lore_wind_of_soltia_game_xlat::entity_list::{
    add_back, add_front, remove, reorder_by_depth, EntityArena, EntityId, EntityListState,
};

/// `node.halfH + node.pixelY` — an independent transcription of the depth formula
/// the list sorts by (also cross-checks that sum).
fn dep(arena: &EntityArena, id: EntityId) -> i32 {
    let n = arena.node(id);
    (n.half_h as i32) + (n.pixel_y as i32)
}

/// Head→tail traversal of the linked list into a `Vec`.
fn forward(s: &EntityListState, arena: &EntityArena) -> Vec<EntityId> {
    let mut out = Vec::new();
    let mut cur = s.head;
    while let Some(id) = cur {
        out.push(id);
        cur = arena.node(id).next;
    }
    out
}

/// Tail→head traversal (must be the reverse of [`forward`] for a consistent list).
fn backward(s: &EntityListState, arena: &EntityArena) -> Vec<EntityId> {
    let mut out = Vec::new();
    let mut cur = s.tail;
    while let Some(id) = cur {
        out.push(id);
        cur = arena.node(id).prev;
    }
    out
}

/// Assert the linked list's ORDER matches the reference and it is a consistent
/// doubly-linked list (forward reversed == backward). Count is asserted separately
/// (see [`ref_reorder`] — reorder has a count-inflation defect).
fn assert_order(s: &EntityListState, arena: &EntityArena, reference: &[EntityId]) {
    let fwd = forward(s, arena);
    let mut bwd = backward(s, arena);
    bwd.reverse();
    assert_eq!(
        fwd, bwd,
        "doubly-linked list is inconsistent (fwd != rev(bwd))"
    );
    assert_eq!(
        fwd, reference,
        "linked-list order diverged from the reference"
    );
}

/// Like [`assert_order`] but also asserts `count == reference.len()` — valid for
/// add/remove, which maintain `count` correctly.
fn assert_consistent(s: &EntityListState, arena: &EntityArena, reference: &[EntityId]) {
    assert_order(s, arena, reference);
    assert_eq!(
        s.count as usize,
        reference.len(),
        "count diverged from the reference length"
    );
}

/// Independent reference relocation of `node`, mirroring `reorderByDepth`'s single
/// directional move over the ordered `Vec` (toward head if it now precedes its
/// predecessor, toward tail if it now follows its successor). Returns `true` when
/// the move lands at an extreme end via the `addFront`/`addBack` path — the case
/// the transliteration's PRESERVED DEFECT inflates `count` by one (the node was
/// unlinked without `count--`, then re-added with `count++`).
fn ref_reorder(order: &mut Vec<EntityId>, arena: &EntityArena, node: EntityId) -> bool {
    let pos = order.iter().position(|&x| x == node).expect("node in list");
    let dn = dep(arena, node);
    if pos > 0 && dn < dep(arena, order[pos - 1]) {
        // Toward head: insert after the nearest preceding node with depth <= dn.
        order.remove(pos);
        let mut j = pos as i32 - 1;
        while j >= 0 && dn < dep(arena, order[j as usize]) {
            j -= 1;
        }
        let at = (j + 1) as usize;
        order.insert(at, node);
        at == 0 // backward == null → addFront (count inflated)
    } else if pos + 1 < order.len() && dn > dep(arena, order[pos + 1]) {
        // Toward tail: insert before the first following node with depth >= dn.
        order.remove(pos);
        let reduced_len = order.len();
        let mut k = pos;
        while k < order.len() && dn > dep(arena, order[k]) {
            k += 1;
        }
        order.insert(k, node);
        k == reduced_len // forward == null → addBack (count inflated)
    } else {
        false // already in place — no move, count unchanged
    }
}

/// Tiny deterministic PRNG (xorshift64*).
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn range(&mut self, n: i32) -> i32 {
        (self.next_u64() >> 33) as i32 % n
    }
}

#[test]
fn add_front_back_and_remove_track_the_reference() {
    let mut ops = 0usize;
    let mut rng = Rng(0xEA71_1157_0000_1234);

    for trial in 0..40 {
        let mut s = EntityListState::new();
        let mut arena = EntityArena::new();
        let mut reference: Vec<EntityId> = Vec::new();

        // Random adds at both ends.
        let n = 3 + (trial % 12);
        for _ in 0..n {
            let half_h = (rng.range(41) - 20) as i8; // -20..20
            let pixel_y = (rng.range(4001) - 2000) as i16; // -2000..2000
            let id = arena.spawn(half_h, pixel_y);
            if rng.range(2) == 0 {
                add_front(&mut s, &mut arena, id);
                reference.insert(0, id);
            } else {
                add_back(&mut s, &mut arena, id);
                reference.push(id);
            }
            assert_consistent(&s, &arena, &reference);
            ops += 1;
        }
        assert!(s.count >= 3, "count floor: list built");

        // Remove a present node and (harmlessly) an absent one.
        if !reference.is_empty() {
            let victim = reference[rng.range(reference.len() as i32) as usize];
            let got = remove(&mut s, &mut arena, victim);
            assert_eq!(got, Some(victim), "remove must return the unlinked node");
            reference.retain(|&x| x != victim);
            assert_consistent(&s, &arena, &reference);

            // Removing a never-linked node returns null and changes nothing.
            let ghost = arena.spawn(0, 0);
            assert_eq!(remove(&mut s, &mut arena, ghost), None);
            assert_consistent(&s, &arena, &reference);
            ops += 1;
        }
    }
    assert!(
        ops >= 100,
        "only {ops} operations — below the liveness floor"
    );
}

#[test]
fn reorder_by_depth_matches_reference_and_restores_sort() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut reorders = 0usize;

    for trial in 0..60 {
        // Build a sorted list (ascending depth) via addBack.
        let mut s = EntityListState::new();
        let mut arena = EntityArena::new();
        let mut reference: Vec<EntityId> = Vec::new();

        let n = 5 + (trial % 15);
        let mut ids = Vec::new();
        for k in 0..n {
            // Strictly increasing depth (half_h 0, pixel_y = 10*k) → sorted.
            let id = arena.spawn(0, (10 * k) as i16);
            add_back(&mut s, &mut arena, id);
            reference.push(id);
            ids.push(id);
        }
        assert_consistent(&s, &arena, &reference);
        // Count is correct after the adds; reorder may inflate it (preserved defect).
        let mut expected_count = s.count;

        // Perturb ONE node's depth, then reorder both implementations.
        let victim = ids[rng.range(ids.len() as i32) as usize];
        let new_pixel_y = (rng.range(400) - 50) as i16; // may move either way or not
        arena[victim].pixel_y = new_pixel_y;

        reorder_by_depth(&mut s, &mut arena, victim);
        let inflated = ref_reorder(&mut reference, &arena, victim);
        if inflated {
            // PRESERVED DEFECT: an extreme relocation over-counts by one.
            expected_count = expected_count.wrapping_add(1);
        }

        assert_order(&s, &arena, &reference);
        assert_eq!(
            s.count, expected_count,
            "count did not match the preserved-defect expectation"
        );

        // Semantic check: after a single-node reorder of a previously-sorted list,
        // the whole list is sorted again by depth.
        let order = forward(&s, &arena);
        for w in order.windows(2) {
            assert!(
                dep(&arena, w[0]) <= dep(&arena, w[1]),
                "reorderByDepth did not restore ascending depth order"
            );
        }
        reorders += 1;
    }
    assert!(reorders >= 40, "only {reorders} reorders — below floor");
}

#[test]
fn negative_control_has_teeth() {
    // A concrete move: three nodes sorted [A=0, B=10, C=20]; bump A's depth to 15
    // and reorder — A must slide to between B and C.
    let mut s = EntityListState::new();
    let mut arena = EntityArena::new();
    let a = arena.spawn(0, 0);
    let b = arena.spawn(0, 10);
    let c = arena.spawn(0, 20);
    for id in [a, b, c] {
        add_back(&mut s, &mut arena, id);
    }
    let before = forward(&s, &arena);
    assert_eq!(before, vec![a, b, c]);

    arena[a].pixel_y = 15;
    reorder_by_depth(&mut s, &mut arena, a);
    let after = forward(&s, &arena);

    // Teeth 1: a real move happened.
    assert_ne!(
        after, before,
        "reorder that should have moved a node did nothing"
    );
    // Teeth 2: it landed in the correct place, not some other permutation.
    assert_eq!(after, vec![b, a, c]);
    // Teeth 3: the assertion is not vacuous against a deliberately-wrong order.
    assert_ne!(after, vec![a, b, c]);
    assert_eq!(s.count, 3);
}
