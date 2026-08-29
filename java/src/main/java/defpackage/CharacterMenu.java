package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Graphics;

/* renamed from: ai */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ai.class */
/**
 * The in-game character menu: a six-tab panel (status / items / equipment /
 * guardian / skill / system) reached by pausing in the world. Each tab is a
 * pushed sub-screen ({@link StatusPage}, {@link ItemsTab}, {@link EquipTab},
 * {@link GuardianTab}, {@link SkillTab}, {@link SystemTab}); left/right switch
 * tabs via the {@link #moveCursor} override.
 *
 * <p>{@link #open()} snapshots the equipped-gear sprite ids
 * ({@link #equipSnapshot}) and the active guardian ({@link #guardianSnapshot})
 * so that {@link #closeMenu(boolean)} can diff them and reload only the sprites
 * that actually changed before returning to the world (or trigger a guardian
 * summon if the active guardian changed).
 */
public final class CharacterMenu extends Menu implements Directions {
    /* renamed from: a */
    /** Centered panel origin X. */
    public static int panelX;
    /** Centered panel origin Y. */
    public static int panelY;

    /* renamed from: h */
    /** Equipped-sprite ids captured on {@link #open()} (weapon/armour/acc1/acc2/-). */
    private byte[] equipSnapshot;

    /* renamed from: c */
    /** Active guardian type captured on {@link #open()} (-1 = none). */
    private byte guardianSnapshot;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** {@code /sgui/gm} game-menu string table (shared by every tab). */
    public static TextTable text;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Lazily-created singleton instance. */
    private static CharacterMenu singleton;

    /* renamed from: a */
    /** Returns (creating on first use, opened on the status tab) the character-menu singleton. */
    public static final CharacterMenu instance() {
        if (singleton == null) {
            singleton = new CharacterMenu();
            ((Menu) singleton).child = new StatusPage(singleton);
            panelX = BaseCanvas.halfW - 77;
            panelY = BaseCanvas.halfH - 85;
        }
        return singleton;
    }

    private CharacterMenu() {
        super(null, (byte) 6);
        this.equipSnapshot = new byte[5];
    }

    /* renamed from: d */
    /** Opens the menu: snapshots equipped sprites and guardian, loads strings, speeds the loop. */
    public final void open() {
        Hero hero = GameState.hero();
        Guardian activeGuardian = hero.getActiveGuardian();
        for (byte slot = 0; slot < 5; slot = (byte) (slot + 1)) {
            this.equipSnapshot[slot] = -1;
        }
        if (hero.getWeapon() != null) {
            this.equipSnapshot[0] = AssetLoader.weaponAnim[GameState.classId - 6][((Item) hero.getWeapon()).subId];
        }
        if (hero.getArmor() != null) {
            this.equipSnapshot[1] = AssetLoader.shieldAnim[((Item) hero.getArmor()).subId];
        }
        if (hero.getAccessory1() != null) {
            this.equipSnapshot[2] = AssetLoader.armorAnim[GameState.classId - 6][hero.getAccessory1().subId];
        }
        if (hero.getAccessory2() != null) {
            this.equipSnapshot[3] = AssetLoader.headAnim[hero.getAccessory2().subId];
        }
        this.guardianSnapshot = (byte) -1;
        if (activeGuardian != null) {
            this.guardianSnapshot = activeGuardian.type;
        }
        try {
            text = new TextTable("/sgui/gm");
        } catch (IOException e) {
            e.printStackTrace();
        }
        GameLoop.instance.setFastFps();
    }

    /* renamed from: e */
    /** Jumps straight to the system tab's quit prompt (used by the hardware back/quit action). */
    public final void openSystemQuit() {
        ((Menu) this).cursorIndex = (byte) 6;
        SystemTab systemTab = new SystemTab(this);
        systemTab.promptExit();
        ((Menu) this).child = systemTab;
        ((Menu) this).child.cursorIndex = (byte) 1;
    }

    /* renamed from: a */
    /** Closes the menu; when {@code applyChanges}, diffs and reloads changed equip sprites, then resumes the world. */
    public final void closeMenu(boolean applyChanges) {
        Hero hero = GameState.hero();
        Guardian activeGuardian = hero.getActiveGuardian();
        singleton = null;
        if (applyChanges) {
            if (hero.getArmor() != null && this.equipSnapshot[1] != AssetLoader.shieldAnim[((Item) hero.getArmor()).subId]) {
                hero.reloadEquipSprite((byte) 1);
            }
            if (hero.getAccessory1() != null && this.equipSnapshot[2] != AssetLoader.armorAnim[GameState.classId - 6][hero.getAccessory1().subId]) {
                hero.reloadEquipSprite((byte) 2);
            }
            if (hero.getAccessory2() != null && this.equipSnapshot[3] != AssetLoader.headAnim[hero.getAccessory2().subId]) {
                hero.reloadEquipSprite((byte) 3);
            }
            if (activeGuardian == null || this.guardianSnapshot == activeGuardian.type) {
                if (hero.getWeapon() != null && this.equipSnapshot[0] != AssetLoader.weaponAnim[GameState.classId - 6][((Item) hero.getWeapon()).subId]) {
                    hero.reloadEquipSprite((byte) 0);
                }
                GameState.setScreen(2);
                GameLoop.instance.applyDifficultyFps();
            } else {
                hero.beginGuardianSummon();
            }
            ((Menu) this).child = null;
            singleton = null;
            this.equipSnapshot = null;
            text = null;
            GameLoop.gameScreen.markRedraw();
        }
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (keyCode != -8) {
            return moveCursorHorizontal(action, keyCode);
        }
        GameState.requestState((byte) 14, (byte) 0);
        return true;
    }

    /* renamed from: a */
    /** Draws the whole character-menu tree at the centered panel origin. */
    public final void draw(Graphics graphics) {
        render(graphics, panelX, panelY);
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        if (((Menu) this).child != null) {
            FontManager.clearScreen(graphics);
            FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
        }
        graphics.setColor(4136767);
        graphics.fillRect(x, y, 155, 176);
        Menu.drawInsetPanel(graphics, x + 2, y + 15, 151, 160);
        graphics.setColor(16768959);
        graphics.fillRect(x + 5 + (((Menu) this).cursorIndex * 16) + 1, y, 14, 1);
        graphics.fillRect(x + 5 + (((Menu) this).cursorIndex * 16), y + 1, 1, 16);
        graphics.setColor(12558207);
        graphics.fillRect(x + 5 + (((Menu) this).cursorIndex * 16) + 15, y + 1, 1, 15);
        graphics.setColor(14663551);
        graphics.fillRect(x + 5 + (((Menu) this).cursorIndex * 16) + 1, y + 1, 14, 16);
        int iconX = x + 7;
        for (byte tab = 0; tab < 6; tab = (byte) (tab + 1)) {
            graphics.drawImage(AssetCache.menuTabIcons[tab], iconX, y + 1, 20);
            iconX += 16;
        }
    }

    @Override // defpackage.cb
    public final void moveCursor(byte direction) {
        super.moveCursor(direction);
        ((Menu) this).child = null;
        switch (((Menu) this).cursorIndex) {
            case 0:
                ((Menu) this).child = new StatusPage(this);
                break;
            case 1:
                ((Menu) this).child = new ItemsTab(this);
                break;
            case 2:
                ((Menu) this).child = new EquipTab(this);
                break;
            case 3:
                ((Menu) this).child = new GuardianTab(this);
                break;
            case 4:
                ((Menu) this).child = new SkillTab(this);
                break;
            case 5:
                ((Menu) this).child = new SystemTab(this);
                break;
        }
        ((Menu) this).needsRepaint = true;
    }
}
