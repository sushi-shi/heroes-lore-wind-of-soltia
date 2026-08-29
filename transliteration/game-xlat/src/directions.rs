//! Transliterated from `java/src/main/java/defpackage/Directions.java`
//! (original `u.class` in `Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar`).
//!
//! Implementation #1: strict transliteration. See `docs/TRANSLITERATION.md`.
//!
//! Shared lookup tables for the tile-grid direction system (a Java `interface`
//! whose `public static final` constants every entity/menu class inherits). No
//! methods, no mutable state — the `<clinit>` only builds the constant arrays, so
//! these become module-level `pub const` tables carrying the reviewed names.
//!
//! Two direction encodings coexist:
//! * **step direction** 0-8 (index into [`DIR_DX`]/[`DIR_DY`]): 0=none, 1=up,
//!   2=down, 3=left, 4=right, 5=up-left, 6=up-right, 7=down-left, 8=down-right
//!   (x grows right, y grows down, so "up" is `(0,-1)`).
//! * **facing** 0-4 (none/up/down/left/right): indexes [`FACING_IS_HORIZONTAL`]
//!   and the rotation tables.
//!
//! Opcode shape (R8, `_reference/numeric_shapes.json`): `u.<clinit>:()V => []`
//! (pure array construction — no arithmetic). Every value below is byte-verified
//! against `javap -c -p u.class`.

/// `byte[][] elementDamageMultiplier` (`u.a:[[B`). Element damage multiplier ×10,
/// indexed `[attackerElement][defenderElement]` (both 0-3): 10 neutral, 13 strong
/// (×1.3), 6 weak (×0.6). Combat computes `damage * m[atk][def] / 10`.
pub const ELEMENT_DAMAGE_MULTIPLIER: [[i8; 4]; 4] = [
    [10, 10, 10, 10],
    [10, 10, 6, 13],
    [10, 13, 10, 6],
    [10, 6, 13, 10],
];

/// `byte[] dirDx` (`u.a:[B`). X step per step-direction 0-8.
pub const DIR_DX: [i8; 9] = [0, 0, 0, -1, 1, -1, 1, -1, 1];

/// `byte[] dirDy` (`u.b:[B`). Y step per step-direction 0-8 (positive = downward).
pub const DIR_DY: [i8; 9] = [0, -1, 1, 0, 0, -1, -1, 1, 1];

/// `boolean[] facingIsHorizontal` (`u.a:[Z`). True for horizontal facings
/// (left/right); false for none/up/down. Indexed by facing 0-4.
pub const FACING_IS_HORIZONTAL: [bool; 5] = [false, false, false, true, true];

/// `byte[] diagCW` (`u.c:[B`). Facing 0-4 → the forward diagonal step-direction
/// 45° clockwise (up → up-right).
pub const DIAG_CW: [i8; 5] = [0, 6, 7, 5, 8];

/// `byte[] diagCCW` (`u.d:[B`). Facing 0-4 → the forward diagonal step-direction
/// 45° counter-clockwise (up → up-left).
pub const DIAG_CCW: [i8; 5] = [0, 5, 8, 7, 6];

/// `byte[] rotateCW` (`u.e:[B`). Facing 0-4 rotated 90° clockwise on screen
/// (up → right → down → left).
pub const ROTATE_CW: [i8; 5] = [0, 4, 3, 1, 2];

/// `byte[] rotateCCW` (`u.f:[B`). Facing 0-4 rotated 90° counter-clockwise on
/// screen (up → left → down → right).
pub const ROTATE_CCW: [i8; 5] = [0, 3, 4, 2, 1];

/// `byte[] reverse` (`u.g:[B`). Facing 0-4 reversed 180° (up↔down, left↔right).
pub const REVERSE: [i8; 5] = [0, 2, 1, 4, 3];
