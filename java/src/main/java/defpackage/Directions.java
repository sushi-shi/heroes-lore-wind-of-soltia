package defpackage;

/* renamed from: u */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:u.class */
/**
 * Shared lookup tables for the game's tile-grid direction system, implemented by
 * every entity/menu class that needs them (they read the tables as inherited
 * interface constants).
 *
 * <p>Two direction encodings coexist:
 * <ul>
 *   <li><b>step direction</b> 0-8, indexing {@link #dirDx}/{@link #dirDy}:
 *       0 = none, 1 = up, 2 = down, 3 = left, 4 = right, 5 = up-left,
 *       6 = up-right, 7 = down-left, 8 = down-right (x grows right, y grows
 *       down, so "up" is {@code (0,-1)}).</li>
 *   <li><b>facing</b> 0-4 (the cardinal subset: none/up/down/left/right), used
 *       to index {@link #facingIsHorizontal} and the rotation tables
 *       {@link #rotateCW}/{@link #rotateCCW}/{@link #reverse} (which return a
 *       cardinal facing) and {@link #diagCW}/{@link #diagCCW} (which return a
 *       diagonal step direction).</li>
 * </ul>
 */
public interface Directions {
    /* renamed from: a */
    /**
     * Element damage multiplier &times;10, indexed {@code [attackerElement][defenderElement]}
     * (both 0-3). Combat computes {@code damage * elementDamageMultiplier[atk][def] / 10},
     * so 10 = neutral, 13 = strong (&times;1.3), 6 = weak (&times;0.6).
     */
    public static final byte[][] elementDamageMultiplier = {new byte[]{10, 10, 10, 10}, new byte[]{10, 10, 6, 13}, new byte[]{10, 13, 10, 6}, new byte[]{10, 6, 13, 10}};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** X step per step-direction 0-8 (see class doc). */
    public static final byte[] dirDx = {0, 0, 0, -1, 1, -1, 1, -1, 1};
    /* renamed from: b */
    /** Y step per step-direction 0-8 (positive = downward). */
    public static final byte[] dirDy = {0, -1, 1, 0, 0, -1, -1, 1, 1};

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** True for horizontal facings (left/right); false for none/up/down. Indexed by facing 0-4. */
    public static final boolean[] facingIsHorizontal = {false, false, false, true, true};
    /* renamed from: c */
    /** Facing (0-4) &rarr; the forward diagonal step-direction 45&deg; clockwise (up&rarr;up-right). */
    public static final byte[] diagCW = {0, 6, 7, 5, 8};
    /* renamed from: d */
    /** Facing (0-4) &rarr; the forward diagonal step-direction 45&deg; counter-clockwise (up&rarr;up-left). */
    public static final byte[] diagCCW = {0, 5, 8, 7, 6};
    /* renamed from: e */
    /** Facing (0-4) rotated 90&deg; clockwise on screen (up&rarr;right&rarr;down&rarr;left). */
    public static final byte[] rotateCW = {0, 4, 3, 1, 2};
    /* renamed from: f */
    /** Facing (0-4) rotated 90&deg; counter-clockwise on screen (up&rarr;left&rarr;down&rarr;right). */
    public static final byte[] rotateCCW = {0, 3, 4, 2, 1};
    /* renamed from: g */
    /** Facing (0-4) reversed 180&deg; (up&harr;down, left&harr;right). */
    public static final byte[] reverse = {0, 2, 1, 4, 3};
}
