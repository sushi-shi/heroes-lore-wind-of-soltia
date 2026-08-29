package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: ah */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ah.class */
/**
 * The static event/cutscene VM that interprets a map's {@code .evt} bytecode:
 * tile/action/facing trigger matching, a program counter over the opcode
 * script, and the textbox / yes-no / message / screen-fx sub-states with their
 * scripted-camera and typewriter-text machinery. All state is static (one event
 * runs at a time).
 */
public final class EventScript implements Directions {

    /* renamed from: b */
    /** The opcode script currently executing (rows of {op, arg1, arg2}). */
    private static byte[][] script;
    /* renamed from: h */
    /** Remaining screen-shake camera offsets, consumed two at a time. */
    private static byte[] shakeOffsets;

    /* renamed from: e */
    /** Flash-effect countdown (SCR_FLAS). */
    private static byte flashTimer;

    /* renamed from: b */
    /** First line of the modal message box. */
    private static char[] messageLine1;

    /* renamed from: c */
    /** Second line of the modal message box. */
    private static char[] messageLine2;

    /* renamed from: f */
    /** Set when the player dismisses the message box. */
    private static boolean messageConfirmed;
    /** True while the yes/no box is a save-game confirmation. */
    private static boolean isSaveDialog;
    /** VM sub-state (0=exec, 1=movement, 2=textbox, 3=yes/no, 4=message, 5=fx). */
    private static byte state = 0;

    /* renamed from: a */
    /** Program counter into {@link #script}. */
    private static int pc = 0;
    /** Movement/opcode delay countdown. */
    private static int delay = 0;

    /* renamed from: b */
    /** Set when the player advances the textbox. */
    private static boolean textAdvance = false;

    /* renamed from: a */
    /** Current dialogue text buffer. */
    private static char[] text = null;
    /** Start index of the current textbox page. */
    private static int textStart = 0;
    /** Character length of the current page. */
    private static int lineLen = 0;
    /** Characters currently revealed (typewriter). */
    private static int charShown = 0;
    /** Characters revealed on the previous frame. */
    private static int prevShown = 0;

    /* renamed from: c */
    /** Current yes/no highlight (true = yes). */
    private static boolean yesSelected = true;

    /* renamed from: d */
    /** Set when the player confirms a yes/no choice. */
    private static boolean choiceMade = false;

    /* renamed from: e */
    /** Scripted camera should re-center on the hero. */
    private static boolean cameraFollowHero = true;

    /* renamed from: b */
    /** Npc id the scripted camera follows, or -1. */
    private static byte cameraFollowNpc = -1;

    /* renamed from: c */
    /** Scripted-camera fixed-offset direction. */
    private static byte cameraOffsetDir = 0;

    /* renamed from: d */
    /** Scripted-camera fixed-offset distance. */
    private static byte cameraOffsetDist = 0;

    /* renamed from: f */
    /** World fill mode during events (0=map, 1=black, 2=white). */
    private static byte fillMode = 0;

    /* renamed from: a */
    /** Set to fast-forward (skip) the current event. */
    public static boolean skip = false;

    private EventScript() {
    }

    public static final boolean checkTileTrigger(Hero aoVar) {
        byte b2;
        if (((Entity) aoVar).offGridX || ((Entity) aoVar).offGridY || (b2 = GameState.map.collisionGrid[((Entity) aoVar).tileY][((Entity) aoVar).tileX]) == 0) {
            return false;
        }
        return findTrigger((byte) 0, b2);
    }

    public static final boolean checkActionTrigger() {
        Hero aoVarM100a = GameState.hero();
        if (((Entity) aoVarM100a).offGridX || ((Entity) aoVarM100a).offGridY) {
            return false;
        }
        byte bAbs = (byte) Math.abs((int) GameState.map.collisionGrid[((Entity) aoVarM100a).tileY][((Entity) aoVarM100a).tileX]);
        if (bAbs >= 1 && bAbs <= 127 && findTrigger((byte) 1, bAbs)) {
            return true;
        }
        byte bAbs2 = (byte) Math.abs((int) GameState.map.collisionGrid[((Entity) aoVarM100a).tileY + Directions.dirDy[((Battler) aoVarM100a).facing]][((Entity) aoVarM100a).tileX + Directions.dirDx[((Battler) aoVarM100a).facing]]);
        return bAbs2 >= 1 && bAbs2 <= 127 && findTrigger((byte) 2, bAbs2);
    }

    public static final boolean checkFacingTrigger() {
        byte bAbs;
        Hero aoVarM100a = GameState.hero();
        if (((Entity) aoVarM100a).offGridX || ((Entity) aoVarM100a).offGridY || (bAbs = (byte) Math.abs((int) GameState.map.collisionGrid[((Entity) aoVarM100a).tileY + Directions.dirDy[((Battler) aoVarM100a).facing]][((Entity) aoVarM100a).tileX + Directions.dirDx[((Battler) aoVarM100a).facing]])) < 1 || bAbs > 127) {
            return false;
        }
        return findTrigger((byte) 3, bAbs);
    }

    public static final void fire(byte b2) {
        findTrigger((byte) 0, b2);
    }

    private static final boolean findTrigger(byte b2, byte b3) {
        byte[] bArr = null;
        for (byte[] bArr2 : (byte[][]) GameState.map.triggers[b3 - 1]) {
            bArr = bArr2;
            if (((bArr2[0] >> 6) & 3) == b2 && conditionMet(bArr)) {
                break;
            }
            bArr = null;
        }
        if (bArr == null || bArr[6] == -1) {
            return false;
        }
        run((byte[][]) GameState.map.eventScripts[bArr[6]]);
        return true;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public static final boolean hasTrigger(byte b2) {
        for (byte[] bArr : (byte[][]) GameState.map.triggers[b2 - 1]) {
            if (conditionMet(bArr)) {
                return true;
            }
        }
        return false;
    }

    private static final boolean conditionMet(byte[] bArr) {
        Hero aoVarM100a = GameState.hero();
        int i = ((bArr[0] & 12) << 6) | (bArr[1] & 255);
        int i2 = ((bArr[0] & 3) << 8) | (bArr[2] & 255);
        boolean z = (((bArr[0] & 255) >> 5) & 1) == 0;
        boolean z2 = (((bArr[0] & 255) >> 4) & 1) == 0;
        Debug.assertTrue(i != -1);
        Debug.assertTrue(i2 != -1);
        if (z && !GameState.isSwitch(i)) {
            return false;
        }
        if (!z && !GameState.isFlag(i)) {
            return false;
        }
        if (z2 && !GameState.isSwitch(i2)) {
            return false;
        }
        if (z2 || GameState.isFlag(i2)) {
            return bArr[3] == -1 || aoVarM100a.bag.hasAtLeast(bArr[3], bArr[4], bArr[5]);
        }
        return false;
    }

    private static final void setState(byte b2) {
        delay = 0;
        state = b2;
    }

    private static final void run(byte[][] bArr) {
        skip = false;
        script = bArr;
        GameState.setScreen(4);
        GameState.clearPendingHeroAction();
        pc = 0;
        setState((byte) 0);
        cameraFollowNpc = (byte) -1;
        cameraFollowHero = true;
        fillMode = (byte) 0;
    }

    private static final void endEvent() {
        state = (byte) 0;
        script = (byte[][]) null;
        skip = false;
        cameraFollowNpc = (byte) -1;
        cameraFollowHero = true;
        GameLoop.gameScreen.markRedraw();
        Npc[] acVarArr = GameState.map.npcs;
        for (int i = 0; i < acVarArr.length; i++) {
            if (acVarArr[i] != null && acVarArr[i].visible) {
                acVarArr[i].setOccupancy();
            }
        }
        AudioManager.stopBgm2();
    }

    public static final void handleKey(int i, int i2) {
        if (i2 == -8) {
            skip = true;
        }
        switch (state) {
            case 2:
                if (i == 8 || i2 == 53) {
                    textAdvance = true;
                }
                break;
            case 3:
                if (i == 6 || i == 1 || i2 == 50 || i2 == 56) {
                    yesSelected = !yesSelected;
                } else if (i == 8 || i2 == 53) {
                    choiceMade = true;
                }
                break;
            case 4:
                if (i == 8 || i2 == 53) {
                    messageConfirmed = true;
                }
                break;
        }
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    public static final void step() {
        if (state != 3 && skip) {
            textAdvance = true;
            messageConfirmed = true;
        }
        Hero aoVarM100a = GameState.hero();
        if (state == 0) {
            switch (script[pc][0]) {
                case 1:
                    if (!skip) {
                        text = (char[]) GameState.map.dialogueStrings[script[pc][1]];
                        textStart = 0;
                        lineLen = FontManager.charsInLine(text, textStart, BaseCanvas.width - 28, 3);
                        charShown = 0;
                        prevShown = 0;
                        setState((byte) 2);
                    } else {
                        pc++;
                    }
                    break;
                case 2:
                    choiceMade = false;
                    yesSelected = true;
                    skip = false;
                    setState((byte) 3);
                    break;
                case 3:
                case 4:
                case 5:
                case 6:
                case 15:
                case 16:
                case 17:
                case 34:
                case 35:
                case 36:
                case 37:
                case 38:
                case 39:
                    setState((byte) 1);
                    break;
                case 7:
                    opMapChange();
                    endEvent();
                    break;
                case 8:
                    opSetHeroPos();
                    break;
                case 9:
                    opSetSwitch();
                    break;
                case 10:
                    opChangeMoney();
                    break;
                case 11:
                    opGiveItem();
                    break;
                case 12:
                    opChangeExp();
                    break;
                case 13:
                    opChangeHp();
                    break;
                case 14:
                    opChangeSp();
                    break;
                case 18:
                    opSummonGuardian();
                    break;
                case 19:
                    opSetCombo();
                    break;
                case 20:
                    endEvent();
                    GameLoop.gameScreen.loadEnding();
                    GameState.setScreen(12);
                    break;
                case 22:
                    fillMode = (byte) 1;
                    pc++;
                    break;
                case 23:
                    fillMode = (byte) 0;
                    pc++;
                    break;
                case 24:
                    flashTimer = (byte) 5;
                    setState((byte) 5);
                    break;
                case 25:
                    shakeOffsets = new byte[6];
                    byte b2 = 0;
                    while (true) {
                        byte b3 = b2;
                        if (b3 >= 6) {
                            setState((byte) 5);
                        } else {
                            shakeOffsets[b3] = (byte) defpackage.ByteUtil.randRange(-5, 5);
                            b2 = (byte) (b3 + 1);
                        }
                        break;
                    }
                    break;
                case 26:
                case 29:
                    byte b4 = script[pc][1];
                    System.out.println(new StringBuffer().append("=====[EVENT BGM] ").append((int) b4).toString());
                    if (b4 == 5 || b4 == 6 || b4 == 7) {
                        AudioManager.playBgm2(b4);
                    } else if (b4 == 8) {
                        AudioManager.playSfx(b4, false);
                    }
                    pc++;
                    break;
                case 27:
                    AudioManager.stopBgm2();
                    pc++;
                    break;
                case 30:
                    opQueueSwitch();
                    break;
                case 31:
                    pc += script[pc][1];
                    break;
                case 32:
                    pc -= script[pc][1];
                    break;
                case 40:
                    opChangeObject();
                    break;
                case 41:
                    opChangeNpcFace();
                    break;
                case 42:
                    opChangeTile();
                    break;
                case 44:
                    GameState.requestState((byte) 11, (byte) 2);
                    endEvent();
                    return;
                case 45:
                    GameState.requestState((byte) 11, (byte) 0);
                    endEvent();
                    return;
                case 46:
                    Npc[] acVarArr = GameState.map.npcs;
                    byte[][] bArr = script;
                    int i = pc;
                    pc = i + 1;
                    Npc acVar = acVarArr[bArr[i][1]];
                    acVar.visible = false;
                    acVar.clearOccupancy();
                    break;
                case 47:
                    Npc[] acVarArr2 = GameState.map.npcs;
                    byte[][] bArr2 = script;
                    int i2 = pc;
                    pc = i2 + 1;
                    Npc acVar2 = acVarArr2[bArr2[i2][1]];
                    acVar2.visible = true;
                    acVar2.setPixelPos(((Entity) acVar2).pixelX, ((Entity) acVar2).pixelY);
                    break;
                case 48:
                    GameState.requestState((byte) 11, (byte) 1);
                    endEvent();
                    return;
                case 49:
                    byte b5 = script[pc][1];
                    aoVarM100a.clearFloaters();
                    if (b5 != 0) {
                        aoVarM100a.addFloater(new Floater((byte) 10, (short) -1, (short) (b5 - 1)));
                    }
                    pc++;
                    break;
                case 50:
                    Npc acVar3 = GameState.map.npcs[script[pc][1]];
                    byte b6 = script[pc][2];
                    acVar3.clearFloaters();
                    if (b6 != 0) {
                        acVar3.addFloater(new Floater((byte) 10, (short) -1, (short) (b6 - 1)));
                    }
                    pc++;
                    break;
                case 99:
                    endEvent();
                    GameState.setScreen(2);
                    return;
            }
        }
        switch (state) {
            case 1:
                runMovement();
                GameState.updateHero();
                GameState.map.updateNpcs();
                break;
            case 2:
                runTextbox();
                break;
            case 3:
                runYesNo();
                break;
            case 4:
                runMessage();
                break;
            case 5:
                runScreenFx();
                break;
        }
        if (script == null || pc < script.length) {
            return;
        }
        endEvent();
        GameState.setScreen(2);
    }

    private static final void runMovement() {
        if (delay > 0) {
            delay--;
            if (delay == 0) {
                pc++;
                return;
            }
            return;
        }
        Hero aoVarM100a = GameState.hero();
        while (pc < script.length && script[pc][0] != 4) {
            switch (script[pc][0]) {
                case 3:
                    aoVarM100a.setState((byte) 2);
                    aoVarM100a.setFacing(script[pc][1]);
                    break;
                case 4:
                case 7:
                case 8:
                case 9:
                case 10:
                case 11:
                case 12:
                case 13:
                case 14:
                case 18:
                case 19:
                case 20:
                case 21:
                case 22:
                case 23:
                case 24:
                case 25:
                case 26:
                case 27:
                case 28:
                case 29:
                case 30:
                case 31:
                case 32:
                case 33:
                default:
                    setState((byte) 0);
                    return;
                case 5:
                    aoVarM100a.setState((byte) 1);
                    break;
                case 6:
                    aoVarM100a.setFacing(script[pc][1]);
                    break;
                case 15:
                    Npc acVar = GameState.map.npcs[script[pc][1]];
                    acVar.setState((byte) 2);
                    acVar.setFacing(script[pc][2]);
                    break;
                case 16:
                    GameState.map.npcs[script[pc][1]].setState((byte) 1);
                    break;
                case 17:
                    GameState.map.npcs[script[pc][1]].setFacing(script[pc][2]);
                    break;
                case 34:
                    resetCamera();
                    cameraFollowHero = true;
                    applyCamera();
                    GameState.camX = GameState.camTargetX;
                    GameState.camY = GameState.camTargetY;
                    break;
                case 35:
                    resetCamera();
                    break;
                case 36:
                    resetCamera();
                    cameraFollowNpc = script[pc][1];
                    break;
                case 37:
                    resetCamera();
                    cameraOffsetDir = script[pc][1];
                    cameraOffsetDist = script[pc][2];
                    break;
                case 38:
                    resetCamera();
                    break;
                case 39:
                    resetCamera();
                    GameState.camTargetX = (-(script[pc][1] * 16)) + BaseCanvas.halfW;
                    GameState.camTargetY = (-(script[pc][2] * 16)) + BaseCanvas.halfH;
                    break;
            }
            pc++;
        }
        if (pc != script.length) {
            delay = script[pc][1] - 1;
            if (delay == 0) {
                pc++;
            }
        }
    }

    private static final void runTextbox() {
        if (charShown < lineLen) {
            if (textAdvance || GameLoop.instance.autoTextAdvance) {
                charShown = lineLen;
            } else {
                charShown = FontManager.advanceWord(text, textStart, charShown);
                if (charShown + 1 >= lineLen) {
                    charShown = lineLen;
                }
            }
        } else if (textStart + lineLen >= text.length && textAdvance) {
            text = null;
            pc++;
            setState((byte) 0);
        } else if (textAdvance) {
            textStart += lineLen;
            lineLen = FontManager.charsInLine(text, textStart, BaseCanvas.width - 28, 3);
            charShown = 0;
            prevShown = 0;
        }
        textAdvance = false;
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r0v10, types: [boolean] */
    /* JADX WARN: Type inference failed for: r0v11, types: [java.lang.Throwable] */
    /* JADX WARN: Type inference failed for: r0v12, types: [java.io.PrintStream] */
    private static final void runYesNo() {
        if (!isSaveDialog) {
            if (choiceMade) {
                if (yesSelected) {
                    pc++;
                    setState((byte) 0);
                    return;
                } else {
                    pc += script[pc][2];
                    setState((byte) 0);
                    return;
                }
            }
            return;
        }
        if (choiceMade) {
            if (yesSelected) {
                try {
                    System.out.println("save!!!!!!");
                    GameState.saveGame();
                } catch (Exception e2) {
                    e2.printStackTrace();
                }
            }
            opMapChange();
            endEvent();
            isSaveDialog = false;
        }
    }

    private static final void runMessage() {
        if (messageConfirmed) {
            messageLine1 = null;
            messageLine2 = null;
            endEvent();
            GameState.setScreen(2);
        }
    }

    private static final void runScreenFx() {
        switch (script[pc][0]) {
            case 24:
                if (flashTimer <= 0) {
                    setState((byte) 0);
                    pc++;
                    fillMode = (byte) 0;
                } else {
                    if (flashTimer % 2 == 1) {
                        fillMode = (byte) 2;
                    } else {
                        fillMode = (byte) 0;
                    }
                    flashTimer = (byte) (flashTimer - 1);
                }
                break;
            case 25:
                if (shakeOffsets != null && shakeOffsets.length > 0) {
                    GameState.map.cameraShiftX = shakeOffsets[0];
                    GameState.map.cameraShiftY = shakeOffsets[1];
                    byte[] bArr = new byte[shakeOffsets.length - 2];
                    System.arraycopy(shakeOffsets, 2, bArr, 0, bArr.length);
                    shakeOffsets = bArr;
                } else {
                    setState((byte) 0);
                    pc++;
                }
                break;
        }
    }

    private static final void opMapChange() {
        byte b2 = script[pc][1];
        byte b3 = script[pc][2];
        pc++;
        if (script[pc][0] != 8) {
            System.out.println("[ERROR:EventScript] No hero position after map change.");
        } else {
            GameState.requestMapWarp(b2, b3, script[pc][1], script[pc][2]);
        }
    }

    private static final void opSetSwitch() {
        int i = (script[pc][1] & 255) | (((script[pc][2] & 255) << 2) & 768);
        switch (script[pc][2] & 3) {
            case 0:
                GameState.setSwitch(i);
                break;
            case 1:
                GameState.clearSwitch(i);
                break;
            case 2:
                GameState.toggleSwitch(i);
                break;
        }
        pc++;
    }

    private static final void opQueueSwitch() {
        int i = (script[pc][1] & 255) | (((script[pc][2] & 255) << 2) & 768);
        switch (script[pc][2] & 3) {
            case 0:
                GameState.setFlag(i);
                break;
            case 1:
                GameState.clearFlag(i);
                break;
            case 2:
                GameState.toggleFlag(i);
                break;
        }
        pc++;
    }

    private static final void opSetCombo() {
        GameState.hero().setComboDepth(script[pc][1]);
        pc++;
    }

    private static final void opChangeHp() {
        GameState.hero().addHp((script[pc][2] & 255) | ((script[pc][1] & 255) << 8));
        pc++;
    }

    private static final void opChangeSp() {
        GameState.hero().addMp((script[pc][2] & 255) | ((script[pc][1] & 255) << 8));
        pc++;
    }

    private static final void opChangeExp() {
        GameState.hero().addExp((script[pc][2] & 255) | ((script[pc][1] & 255) << 8));
        pc++;
    }

    private static final void opChangeMoney() {
        GameState.hero().addMoney((script[pc][2] & 255) | ((script[pc][1] & 255) << 8));
        pc++;
    }

    private static final void opGiveItem() {
        byte b2 = script[pc][1];
        byte b3 = script[pc][2];
        pc++;
        if (script[pc][0] != 21) {
            System.out.println("[ERROR:EventScript] No CMD_HANDLE_ITEM_NUM after CMD_HANDLE_ITEM.");
            return;
        }
        Hero aoVarM100a = GameState.hero();
        byte b4 = script[pc][1];
        pc++;
        if (b4 > 0) {
            Item adVarA = Item.create(b2, b3, true, true);
            if (adVarA instanceof Equipment) {
                ((Equipment) adVarA).identified = true;
            }
            if (aoVarM100a.bag.add(adVarA, (int) b4)) {
                return;
            }
            showMessage(StringTable.instance.get(3938).toCharArray(), "".toCharArray());
            return;
        }
        if (b4 < 0) {
            byte b5 = (byte) (-b4);
            if (aoVarM100a.bag.totalQuantity(b2, b3) >= b5) {
                aoVarM100a.bag.removeItems(b2, b3, b5);
            } else {
                showMessage(StringTable.instance.get(3939).toCharArray(), "".toCharArray());
            }
        }
    }

    private static final void opSummonGuardian() {
        byte b2;
        switch (script[pc][1]) {
            case 0:
                b2 = 4;
                break;
            case 1:
                b2 = 3;
                break;
            case 2:
                b2 = 5;
                break;
            default:
                return;
        }
        pc++;
        GameState.hero().setState(b2);
    }

    private static final void opSetHeroPos() {
        Hero aoVarM100a = GameState.hero();
        aoVarM100a.clearOccupancy();
        aoVarM100a.setPixelPos((short) (script[pc][1] * 16), (short) (script[pc][2] * 16));
        aoVarM100a.setOccupancy();
        pc++;
    }

    private static final void opChangeTile() {
        GameMap aeVar = GameState.map;
        byte b2 = script[pc][1];
        byte b3 = script[pc][2];
        pc++;
        if (script[pc][0] != 43) {
            System.out.println("[ERROR:EventScript] No CMD_TILE_PROPERTY after CMD_CHANGE_TILE.");
            return;
        }
        byte b4 = script[pc][1];
        byte b5 = script[pc][2];
        pc++;
        aeVar.collisionGrid[b3][b2] = b5;
        aeVar.tileGrid[b3][b2] = b4;
    }

    private static final void opChangeObject() {
        GameMap aeVar = GameState.map;
        byte b2 = script[pc][1];
        byte b3 = script[pc][2];
        pc++;
        aeVar.objects[b2].image = AssetCache.mapObjects[b3];
    }

    private static final void opChangeNpcFace() {
        GameMap aeVar = GameState.map;
        byte b2 = script[pc][1];
        byte b3 = script[pc][2];
        pc++;
        aeVar.npcs[b2].kind = b3;
    }

    public static final void paint(Graphics graphics) {
        if (state == 0) {
        }
        GameState.scrollCamera(false, false);
        if (state == 3 || !skip) {
            if (fillMode == 0) {
                GameState.map.paint(graphics);
            } else {
                if (fillMode == 1) {
                    graphics.setColor(0);
                } else if (fillMode == 2) {
                    graphics.setColor(16777215);
                }
                graphics.fillRect(0, 0, GameScreen.width, GameScreen.worldHeight);
            }
            GameLoop.gameScreen.drawHud(graphics);
            switch (state) {
                case 2:
                    paintTextbox(graphics);
                    GameLoop.gameScreen.markRedraw();
                    FontManager.drawSoftKeys(graphics, FontManager.labelNext, FontManager.labelSkip);
                    break;
                case 3:
                    paintYesNo(graphics);
                    FontManager.drawSoftKeys(graphics, FontManager.labelOk, (char[]) null);
                    break;
                case 4:
                    paintMessageBox(graphics, (BaseCanvas.width >> 1) - 60, (BaseCanvas.height >> 1) - 25, 120, 45, messageLine1, messageLine2);
                    FontManager.drawSoftKeys(graphics, FontManager.labelOk, (char[]) null);
                    break;
            }
        }
    }

    private static final void paintTextbox(Graphics graphics) {
        int i = BaseCanvas.width - 8;
        int i2 = BaseCanvas.halfW - (i / 2);
        int i3 = (BaseCanvas.height - 41) - 10;
        graphics.translate(i2, i3);
        graphics.setColor(2039615);
        graphics.drawRect(0, 0, i - 1, 40);
        graphics.setColor(6250367);
        graphics.drawRect(1, 1, i - 3, 38);
        graphics.setColor(0);
        graphics.fillRect(2, 2, i - 4, 37);
        graphics.drawImage(AssetCache.dialogBorder[0], 2, 2, 20);
        graphics.drawImage(AssetCache.dialogBorder[1], (0 + i) - 2, 2, 24);
        graphics.drawImage(AssetCache.dialogBorder[2], 2, 39, 36);
        graphics.drawImage(AssetCache.dialogBorder[3], (0 + i) - 2, 39, 40);
        graphics.setColor(16777215);
        FontManager.drawWrappedBlockPartial(graphics, 10, 5, i - 20, 1, text, textStart, prevShown, charShown);
        prevShown = charShown;
        graphics.translate(-i2, -i3);
        graphics.setClip(0, 0, BaseCanvas.width, BaseCanvas.height);
        byte b2 = script[pc][2];
        if (b2 > 0) {
            graphics.drawImage(AssetCache.dialoguePortraits[b2 - 1], i2, i3, 36);
        } else if (b2 < 0) {
            graphics.drawImage(AssetCache.dialoguePortraits[(-b2) - 1], i2 + i, i3, 40);
        }
    }

    private static final void paintYesNo(Graphics graphics) {
        try {
            Object[] objArr = {isSaveDialog ? StringTable.instance.get(3940).toCharArray() : (char[]) GameState.map.dialogueStrings[script[pc][1]], StringTable.instance.get(3915).toCharArray(), StringTable.instance.get(3916).toCharArray()};
            int iA = FontManager.percentOf(BaseCanvas.width, 70);
            paintListBox(graphics, BaseCanvas.halfW - (iA >> 1), BaseCanvas.halfH - 30, iA, 60, objArr, 6, 1, yesSelected ? 1 : 2);
        } catch (Exception unused) {
        }
    }

    public static final void paintListBox(Graphics graphics, int i, int i2, int i3, int i4, Object[] objArr, int i5, int i6, int i7) {
        Menu.drawPanelFrame(graphics, i, i2, i3, i4);
        Menu.fillPanelInterior(graphics, i, i2, i3, i4);
        graphics.setColor(255, 255, 255);
        int iA = i2 + 6;
        for (int i8 = 0; i8 < objArr.length; i8++) {
            if (i8 < i6 || i6 == -1) {
                iA = iA + FontManager.drawWrappedText(graphics, i + i5, iA, (i3 - i5) - i5, 1, (char[]) objArr[i8]) + 5;
            } else {
                FontManager.drawChars(graphics, i + i5 + 9, iA, (char[]) objArr[i8], 1);
                if (i8 == i7) {
                    graphics.drawImage(AssetCache.cursorArrow, i + i5, iA, 20);
                }
                iA += FontManager.lineHeight() + 3;
            }
        }
    }

    public static final void paintMessageBox(Graphics graphics, int i, int i2, int i3, int i4, char[] cArr, char[] cArr2) {
        Menu.drawPanelFrame(graphics, i, i2, i3, i4);
        Menu.fillPanelInterior(graphics, i, i2, i3, i4);
        graphics.setColor(255, 255, 255);
        FontManager.drawChars(graphics, i + 6, i2 + 7, cArr, 1);
        FontManager.drawChars(graphics, i + 6, i2 + 23, cArr2, 1);
    }

    private static void resetCamera() {
        cameraFollowHero = false;
        cameraFollowNpc = (byte) -1;
        cameraOffsetDir = (byte) 0;
        cameraOffsetDist = (byte) 0;
    }

    private static final void showMessage(char[] cArr, char[] cArr2) {
        messageLine1 = cArr;
        messageLine2 = cArr2;
        messageConfirmed = false;
        setState((byte) 4);
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    public static final void applyCamera() {
        if (cameraFollowHero) {
            GameState.centerCamera();
        } else {
            if (cameraFollowHero || cameraFollowNpc != -1 || cameraOffsetDir == 0) {
                return;
            }
            GameState.camTargetX -= cameraOffsetDist * Directions.dirDx[cameraOffsetDir];
            GameState.camTargetY -= cameraOffsetDist * Directions.dirDy[cameraOffsetDir];
        }
    }

    static {
        String[] strArr = {"CMD_IDLE", "TALKTEXT", "YES/NO  ", "MV_H_MOV", "MV_DELAY", "MV_H_STP", "MV_H_DIR", "MAP_CHNG", "MAP_HERO", "SWI_DEF ", "MONEY   ", "ITEM    ", "EXP     ", "HP      ", "SP      ", "MV_N_MOV", "MV_N_STP", "MV_N_DIR", "GUARDIAN", "COMBO   ", "GAMEOVER", "ITEM_NUM", "SCR_DEL ", "SCR_SHOW", "SCR_FLAS", "SCR_SHAK", "BGM_PLAY", "BGM_STOP", "SYSBGM  ", "SOUND   ", "SWI_QUE ", "GOTO_FOR", "GOTO_BAK", "SWI_MAP ", "MV_FO_HE", "MV_FO_NO", "MV_FO_NP", "MV_CA_MV", "MV_CA_ST", "MV_CA_XY", "CHG_OBJ ", "CHG_NPC ", "CHGTL_XY", "CHGTL_VA", "OPN_BLAK", "OPEN_SHP", "HIDE_NPC", "SHOW_NPC", "OPN_REFI", "EMO_HERO", "EMO_NPC ", null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, "END_EVNT"};
    }
}
