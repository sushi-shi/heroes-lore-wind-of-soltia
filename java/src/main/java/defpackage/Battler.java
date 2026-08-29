package defpackage;

import java.util.Vector;
import javax.microedition.lcdui.Graphics;

/* renamed from: o */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:o.class */
/**
 * Abstract base for every mobile combatant that walks the tile grid: the player
 * ({@link Hero}), hostile actors ({@link Enemy}/{@link Boss}) and town folk
 * ({@link Npc}). On top of {@link Entity} (pixel/tile position + occupancy
 * links) it adds the movement/animation finite-state machine ({@link #state},
 * {@link #facing}, {@link #moveDir}, {@link #animFrame}), the sub-tile stepping
 * mover ({@link #move}), an AI path-choice routine that steers toward a target
 * with side-step fallbacks ({@link #approach}), and two per-actor overlay lists:
 * floating combat text ({@link #floaters}) and active status effects
 * ({@link #statuses}). Tiles are 16px and actors step 8px at a time, so a tile
 * takes two frames; the off-grid flags on {@link Entity} track the half-tile
 * phase.
 */
public abstract class Battler extends Entity implements Directions {
    /* renamed from: a */
    /** Floating-text overlays ({@link Floater}/{@link Overlay}) drawn on this actor. */
    public Vector floaters;

    /* renamed from: b */
    /** Active status-effect icons ({@link StatusIcon}) drawn under this actor. */
    public Vector statuses;

    /* renamed from: h */
    /** FSM: 1 idle/walk, 2 stepping, 3 attacking, 4 knockback, 5 dying, 6 dead. */
    public byte state;

    /* renamed from: i */
    /** Direction the actor faces (1 up, 2 down, 3 left, 4 right; see {@link Directions}). */
    public byte facing;

    /* renamed from: j */
    /** Direction committed for the in-progress sub-tile step. */
    public byte moveDir;

    /* renamed from: k */
    /** Animation frame counter (starts at -1; advanced per tick). */
    public byte animFrame;

    /* renamed from: l */
    /** Knockback/recoil countdown (state 4); returns to idle when it expires. */
    public byte knockbackTimer;

    public Battler(short pixelX, short pixelY, byte halfWidth, byte halfHeight) {
        super(pixelX, pixelY, halfWidth, halfHeight);
        this.knockbackTimer = (byte) 0;
        init();
    }

    /* renamed from: a */
    /** (Re)initialises the overlay lists and the movement/animation state. */
    public void init() {
        if (this.floaters == null) {
            this.floaters = new Vector(2);
        }
        if (this.statuses == null) {
            this.statuses = new Vector(3);
        }
        this.state = (byte) 1;
        this.facing = (byte) 2;
        this.moveDir = (byte) 2;
        this.animFrame = (byte) -1;
    }

    /* renamed from: c */
    /** Discards all pending floating-text overlays. */
    public final void clearFloaters() {
        this.floaters = new Vector(2);
    }

    /* renamed from: a */
    /** Enters {@code newState}, resetting the animation frame counter. */
    public void setState(byte newState) {
        this.animFrame = (byte) -1;
        this.state = newState;
    }

    /* renamed from: b */
    /** Faces {@code dir} and commits it as the next step direction. */
    public final void setFacing(byte dir) {
        this.facing = dir;
        this.moveDir = dir;
    }

    /* renamed from: d */
    /** Per-tick update (base: advance any in-progress step). */
    public void update() {
        stepIfMoving();
    }

    /* renamed from: e */
    /** While stepping (state 2/4), stops at a blocked tile then advances 8px. */
    public final void stepIfMoving() {
        if (this.state == 2 || this.state == 4) {
            tryStepForward();
        }
        if (this.state == 2 || this.state == 4) {
            move(8);
        }
    }

    /* renamed from: f */
    /** Clears this actor's footprint from the map occupancy grid. */
    public final void clearOccupancy() {
        GameMap map = GameState.map;
        byte colOffset = 0;
        while (true) {
            byte col = colOffset;
            if (col >= ((Entity) this).layer) {
                return;
            }
            map.occupancy[((Entity) this).tileY][((Entity) this).tileX + col] = null;
            if (((Entity) this).offGridY) {
                map.occupancy[((Entity) this).tileY + 1][((Entity) this).tileX + col] = null;
            } else if (((Entity) this).offGridX) {
                map.occupancy[((Entity) this).tileY][((Entity) this).tileX + 1 + col] = null;
            }
            colOffset = (byte) (col + 1);
        }
    }

    /* renamed from: g */
    /** Writes this actor's footprint into the map occupancy grid. */
    public final void setOccupancy() {
        GameMap map = GameState.map;
        byte colOffset = 0;
        while (true) {
            byte col = colOffset;
            if (col >= ((Entity) this).layer) {
                return;
            }
            map.occupancy[((Entity) this).tileY][((Entity) this).tileX + col] = this;
            if (((Entity) this).offGridY) {
                map.occupancy[((Entity) this).tileY + 1][((Entity) this).tileX + col] = this;
            } else if (((Entity) this).offGridX) {
                map.occupancy[((Entity) this).tileY][((Entity) this).tileX + 1 + col] = this;
            }
            colOffset = (byte) (col + 1);
        }
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * If aligned to the grid and the tile ahead is blocked, halts (state 1) and
     * reports {@code true}; otherwise keeps moving and reports {@code false}.
     */
    public boolean tryStepForward() {
        GameMap map = GameState.map;
        if (((Entity) this).offGridX || ((Entity) this).offGridY || map.canStep(this, this.facing)) {
            return false;
        }
        setState((byte) 1);
        return true;
    }

    /* renamed from: a */
    /**
     * Advances the actor {@code stepPixels} in its facing direction, toggling the
     * off-grid half-tile flags and re-registering it in the occupancy grid. A
     * plain 8px sub-tile step skips {@link #syncTile()} (the flags are updated by
     * hand here); any other step re-derives the tile from pixels.
     */
    public void move(int stepPixels) {
        clearOccupancy();
        switch (this.facing) {
            case 1:
                Debug.assertTrue(((Entity) this).pixelY > 0);
                ((Entity) this).pixelY = (short) (((Entity) this).pixelY - stepPixels);
                if (!((Entity) this).offGridY) {
                    ((Entity) this).offGridY = true;
                    ((Entity) this).tileY = (byte) (((Entity) this).tileY - 1);
                } else {
                    ((Entity) this).offGridY = false;
                }
                break;
            case 2:
                Debug.assertTrue(((Entity) this).pixelY < GameState.map.heightPx - 16);
                ((Entity) this).pixelY = (short) (((Entity) this).pixelY + stepPixels);
                if (!((Entity) this).offGridY) {
                    ((Entity) this).offGridY = true;
                } else {
                    ((Entity) this).offGridY = false;
                    ((Entity) this).tileY = (byte) (((Entity) this).tileY + 1);
                }
                break;
            case 3:
                Debug.assertTrue(((Entity) this).pixelX > 0);
                ((Entity) this).pixelX = (short) (((Entity) this).pixelX - stepPixels);
                if (!((Entity) this).offGridX) {
                    ((Entity) this).offGridX = true;
                    ((Entity) this).tileX = (byte) (((Entity) this).tileX - 1);
                } else {
                    ((Entity) this).offGridX = false;
                }
                break;
            case 4:
                Debug.assertTrue(((Entity) this).pixelX < GameState.map.widthPx - 16);
                ((Entity) this).pixelX = (short) (((Entity) this).pixelX + stepPixels);
                if (!((Entity) this).offGridX) {
                    ((Entity) this).offGridX = true;
                } else {
                    ((Entity) this).offGridX = false;
                    ((Entity) this).tileX = (byte) (((Entity) this).tileX + 1);
                }
                break;
        }
        if (stepPixels != 8) {
            syncTile();
        }
        setOccupancy();
    }

    /* renamed from: a */
    /**
     * AI path-choice: picks a step direction that closes on {@code target} within
     * reach {@code range}. When already adjacent (or the target is straight ahead)
     * it just faces the target and stops; otherwise it steps along the dominant
     * axis, falling back to side-steps ({@link #findSidestepDir}) and finally a
     * random direction when fully boxed in.
     */
    public final void approach(Entity target, byte range) {
        byte faceDir;
        byte chosenDir;
        byte finalDir;
        byte targetTileX = target.tileX;
        byte targetTileY = target.tileY;
        GameMap map = GameState.map;
        int dxTiles = 100;
        byte scanCol = 0;
        while (true) {
            byte col = scanCol;
            if (col >= ((Entity) this).layer) {
                break;
            }
            if (Math.abs(dxTiles) > Math.abs(targetTileX - (((Entity) this).tileX + col))) {
                dxTiles = targetTileX - (((Entity) this).tileX + col);
            }
            scanCol = (byte) (col + 1);
        }
        int dyTiles = targetTileY - ((Entity) this).tileY;
        int absDx = Math.abs(dxTiles);
        int absDy = Math.abs(dyTiles);
        int randTie = Entity.rng.nextInt();
        if ((absDx + absDy <= range && absDx * absDy == 0) || target == entityInDir(this.moveDir, target)) {
            if (dyTiles != 0) {
                faceDir = dyTiles < 0 ? (byte) 1 : (byte) 2;
            } else {
                faceDir = dxTiles < 0 ? (byte) 3 : (byte) 4;
            }
            setFacing(faceDir);
            return;
        }
        if (absDy == absDx) {
            byte horizDir = dxTiles > 0 ? (byte) 4 : (byte) 3;
            byte vertDir = dyTiles > 0 ? (byte) 2 : (byte) 1;
            boolean canHoriz = map.canStep(this, horizDir);
            boolean canVert = map.canStep(this, vertDir);
            if (canHoriz && canVert) {
                chosenDir = Entity.rng.nextInt() > 0 ? horizDir : vertDir;
            } else {
                chosenDir = canHoriz ? horizDir : vertDir;
            }
        } else if (absDy > absDx) {
            chosenDir = dyTiles > 0 ? (byte) 2 : (byte) 1;
        } else {
            chosenDir = dxTiles > 0 ? (byte) 4 : (byte) 3;
        }
        if ((absDx <= range || absDy <= range) && absDx != absDy) {
            if (absDx > range || absDy >= absDx) {
                if (absDy <= range && absDy > absDx) {
                    if (dxTiles > 0 && map.canStep(this, (byte) 4)) {
                        chosenDir = 4;
                    } else if (dxTiles < 0 && map.canStep(this, (byte) 3)) {
                        chosenDir = 3;
                    }
                }
            } else if (dyTiles > 0 && map.canStep(this, (byte) 2)) {
                chosenDir = 2;
            } else if (dyTiles < 0 && map.canStep(this, (byte) 1)) {
                chosenDir = 1;
            }
        }
        boolean committed = false;
        if (map.canStep(this, chosenDir)) {
            finalDir = chosenDir;
            committed = true;
        } else {
            boolean sidestepClockwise = true;
            if ((chosenDir == 1 && dxTiles > 0) || ((chosenDir == 2 && dxTiles < 0) || ((chosenDir == 3 && dyTiles < 0) || (chosenDir == 4 && dyTiles > 0)))) {
                sidestepClockwise = false;
            }
            byte sidestep = findSidestepDir(target, chosenDir, sidestepClockwise);
            finalDir = sidestep;
            if (sidestep != 0) {
                committed = true;
            } else {
                byte sidestepBack = findSidestepDir(target, chosenDir, !sidestepClockwise);
                finalDir = sidestepBack;
                if (sidestepBack != 0) {
                    committed = true;
                } else if (sidestepClockwise && map.canStep(this, Directions.rotateCCW[chosenDir])) {
                    finalDir = Directions.rotateCCW[chosenDir];
                    committed = true;
                } else if (!sidestepClockwise && map.canStep(this, Directions.rotateCW[chosenDir])) {
                    finalDir = Directions.rotateCW[chosenDir];
                    committed = true;
                }
            }
        }
        if (!committed) {
            finalDir = (byte) (((randTie & 255) % 4) + 1);
        }
        setState((byte) 2);
        setFacing(finalDir);
    }

    /* renamed from: a */
    /**
     * Scans the perpendicular band beside direction {@code dir} for a walkable
     * side-step that still keeps {@code target} reachable, searching either the
     * clockwise or counter-clockwise turn table. Returns the side-step direction,
     * or 0 when none exists.
     */
    private final byte findSidestepDir(Entity target, byte dir, boolean clockwise) {
        byte[] turnDir;
        byte[] diagDir;
        GameMap map = GameState.map;
        if (clockwise) {
            turnDir = Directions.rotateCCW;
            diagDir = Directions.diagCCW;
        } else {
            turnDir = Directions.rotateCW;
            diagDir = Directions.diagCW;
        }
        byte scanRadius = (dir == 1 || dir == 2) ? ((Entity) this).layer : (byte) 1;
        for (int offset = (-scanRadius) + 1; offset < scanRadius; offset++) {
            if (map.canOccupy(this, ((Entity) this).tileX + offset + Directions.dirDx[turnDir[dir]], ((Entity) this).tileY + Directions.dirDy[turnDir[dir]]) && (map.canOccupy(this, ((Entity) this).tileX + offset + Directions.dirDx[diagDir[dir]], ((Entity) this).tileY + Directions.dirDy[diagDir[dir]]) || target == map.occupancy[((Entity) this).tileY + Directions.dirDy[diagDir[dir]]][((Entity) this).tileX + offset + Directions.dirDx[diagDir[dir]]])) {
                return turnDir[dir];
            }
        }
        return (byte) 0;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the {@link Enemy} on the tile straight ahead, or {@code null}. */
    public final Enemy enemyAhead() {
        Entity ahead = entityInDir(this.facing, (Entity) null);
        if (ahead instanceof Enemy) {
            return (Enemy) ahead;
        }
        return null;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Returns the {@link Enemy} on the adjacent tile in {@code dir}, or {@code null}. */
    public final Enemy enemyInDir(byte dir) {
        Entity found = entityInDir(dir, (Entity) null);
        if (found instanceof Enemy) {
            return (Enemy) found;
        }
        return null;
    }

    /* renamed from: a */
    /**
     * Scans the {@code layer} tiles adjacent in direction {@code dir}. With
     * {@code wanted == null} returns the first occupant found; otherwise returns
     * {@code wanted} iff it occupies one of those tiles (else {@code null}).
     */
    public final Entity entityInDir(byte dir, Entity wanted) {
        GameMap map = GameState.map;
        for (int col = 0; col < ((Entity) this).layer; col++) {
            int scanX = ((Entity) this).tileX + Directions.dirDx[dir] + col;
            int scanY = ((Entity) this).tileY + Directions.dirDy[dir];
            Debug.assertTrue(scanX >= 0);
            Debug.assertTrue(scanX < map.widthTiles);
            Debug.assertTrue(scanY >= 0);
            Debug.assertTrue(scanY < map.heightTiles);
            Entity occupant = map.occupancy[scanY][scanX];
            if (occupant != this) {
                if (wanted == null && occupant != null) {
                    return occupant;
                }
                if (wanted != null && occupant == wanted) {
                    return occupant;
                }
            }
        }
        return null;
    }

    /* renamed from: a */
    /** Pushes a floating-text overlay above this actor. */
    public final void addFloater(Overlay overlay) {
        this.floaters.addElement(overlay);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /**
     * Applies status effect {@code statusKind} (0..7): refreshes an existing icon
     * of that kind, or adds a new one. Returns {@code true} if it was a refresh.
     */
    public final boolean applyStatus(byte statusKind) {
        Debug.assertTrue(statusKind >= 0 && statusKind < 8);
        boolean refreshed = false;
        for (int i = 0; i < this.statuses.size(); i++) {
            StatusIcon icon = (StatusIcon) this.statuses.elementAt(i);
            if (!((Overlay) icon).finished && icon.kind == statusKind) {
                icon.reset();
                refreshed = true;
                break;
            }
        }
        if (!refreshed) {
            this.statuses.addElement(new StatusIcon(statusKind));
        }
        return refreshed;
    }

    /* renamed from: b */
    /** Draws (and reaps finished) floating-text overlays at screen origin. */
    public final void drawFloaters(Graphics graphics, int originX, int originY) {
        for (int i = this.floaters.size() - 1; i >= 0; i--) {
            Overlay overlay = (Overlay) this.floaters.elementAt(i);
            overlay.paint(graphics, originX, originY);
            if (overlay.finished) {
                this.floaters.removeElementAt(i);
            }
        }
    }

    /* renamed from: c */
    /** Draws the row of active status-effect icons centred at the origin. */
    public final void drawStatusIcons(Graphics graphics, int originX, int originY) {
        int offsetX = (-6) * (this.statuses.size() - 1);
        for (int i = this.statuses.size() - 1; i >= 0; i--) {
            ((StatusIcon) this.statuses.elementAt(i)).paint(graphics, originX + offsetX, originY);
            offsetX += 12;
        }
    }
}
