//! Directions constant-table oracle ("two implementations, one truth").
//!
//! The strict transliteration's `pub const` tables
//! ([`heroes_lore_wind_of_soltia_game_xlat::directions`], from `u.class`) are
//! byte-matched against the values disassembled independently from the shipped
//! bytecode with `javap -c -p u.class` (the R8 authority). The `EXPECTED_*`
//! literals below are transcribed directly from that `<clinit>` disassembly —
//! every `bastore` with its `bipush`/`iconst`/`iconst_m1` operand — so this test
//! is the transliteration literals vs. the bytecode literals, not a copy of the
//! module. A negative control proves the comparison bites.

use heroes_lore_wind_of_soltia_game_xlat::directions as d;

// --- Values read straight off `javap -c -p u.class` `<clinit>` (byte-verified) ---

// putstatic Field a:[[B — four `newarray byte` of {bipush 10 / 6 / 13} operands.
const EXPECTED_ELEMENT_DAMAGE_MULTIPLIER: [[i8; 4]; 4] = [
    [10, 10, 10, 10],
    [10, 10, 6, 13],
    [10, 13, 10, 6],
    [10, 6, 13, 10],
];
// putstatic Field a:[B — iconst_0/iconst_m1/iconst_1 operands, indices 0..8.
const EXPECTED_DIR_DX: [i8; 9] = [0, 0, 0, -1, 1, -1, 1, -1, 1];
// putstatic Field b:[B
const EXPECTED_DIR_DY: [i8; 9] = [0, -1, 1, 0, 0, -1, -1, 1, 1];
// putstatic Field a:[Z — newarray boolean, iconst_0/iconst_1 at indices 3,4.
const EXPECTED_FACING_IS_HORIZONTAL: [bool; 5] = [false, false, false, true, true];
// putstatic Field c:[B
const EXPECTED_DIAG_CW: [i8; 5] = [0, 6, 7, 5, 8];
// putstatic Field d:[B
const EXPECTED_DIAG_CCW: [i8; 5] = [0, 5, 8, 7, 6];
// putstatic Field e:[B
const EXPECTED_ROTATE_CW: [i8; 5] = [0, 4, 3, 1, 2];
// putstatic Field f:[B
const EXPECTED_ROTATE_CCW: [i8; 5] = [0, 3, 4, 2, 1];
// putstatic Field g:[B
const EXPECTED_REVERSE: [i8; 5] = [0, 2, 1, 4, 3];

#[test]
fn tables_byte_match_the_bytecode() {
    assert_eq!(
        d::ELEMENT_DAMAGE_MULTIPLIER,
        EXPECTED_ELEMENT_DAMAGE_MULTIPLIER,
        "elementDamageMultiplier diverges from u.class bytecode"
    );
    assert_eq!(d::DIR_DX, EXPECTED_DIR_DX, "dirDx diverges from bytecode");
    assert_eq!(d::DIR_DY, EXPECTED_DIR_DY, "dirDy diverges from bytecode");
    assert_eq!(
        d::FACING_IS_HORIZONTAL,
        EXPECTED_FACING_IS_HORIZONTAL,
        "facingIsHorizontal diverges from bytecode"
    );
    assert_eq!(
        d::DIAG_CW,
        EXPECTED_DIAG_CW,
        "diagCW diverges from bytecode"
    );
    assert_eq!(
        d::DIAG_CCW,
        EXPECTED_DIAG_CCW,
        "diagCCW diverges from bytecode"
    );
    assert_eq!(
        d::ROTATE_CW,
        EXPECTED_ROTATE_CW,
        "rotateCW diverges from bytecode"
    );
    assert_eq!(
        d::ROTATE_CCW,
        EXPECTED_ROTATE_CCW,
        "rotateCCW diverges from bytecode"
    );
    assert_eq!(
        d::REVERSE,
        EXPECTED_REVERSE,
        "reverse diverges from bytecode"
    );
}

/// Semantic cross-checks that would catch a transcription slip the byte-match
/// might share: the direction tables must be internally consistent.
#[test]
fn tables_are_internally_consistent() {
    // dirDx/dirDy have a step for each of the 9 step-directions.
    assert_eq!(d::DIR_DX.len(), 9);
    assert_eq!(d::DIR_DY.len(), 9);
    // "up" (dir 1) is (0,-1); "right" (dir 4) is (1,0).
    assert_eq!((d::DIR_DX[1], d::DIR_DY[1]), (0, -1));
    assert_eq!((d::DIR_DX[4], d::DIR_DY[4]), (1, 0));

    // Over the cardinal facings 1..=4, rotateCW and rotateCCW are inverse
    // permutations, and reverse is an involution.
    for f in 1..=4usize {
        let cw = d::ROTATE_CW[f] as usize;
        assert_eq!(d::ROTATE_CCW[cw], f as i8, "rotateCCW must undo rotateCW");
        let rev = d::REVERSE[f] as usize;
        assert_eq!(d::REVERSE[rev], f as i8, "reverse must be an involution");
    }

    // Element multiplier: the neutral row/self are all 10 (×1.0); the strong/weak
    // pairs are 13/6.
    for e in 0..4usize {
        assert_eq!(d::ELEMENT_DAMAGE_MULTIPLIER[0][e], 10, "row 0 is neutral");
        assert_eq!(d::ELEMENT_DAMAGE_MULTIPLIER[e][e], 10, "self is neutral");
    }
}

/// Negative control (R3): the byte-match has teeth — a single wrong expected value
/// must NOT compare equal to the transliterated table.
#[test]
fn negative_control_a_wrong_value_is_caught() {
    let mut wrong = EXPECTED_DIR_DX;
    wrong[3] = 1; // real value is -1
    assert_ne!(
        d::DIR_DX,
        wrong,
        "a one-entry perturbation compared equal — the oracle is blind"
    );

    let mut wrong_edm = EXPECTED_ELEMENT_DAMAGE_MULTIPLIER;
    wrong_edm[1][2] = 10; // real value is 6 (weak)
    assert_ne!(d::ELEMENT_DAMAGE_MULTIPLIER, wrong_edm);
}
