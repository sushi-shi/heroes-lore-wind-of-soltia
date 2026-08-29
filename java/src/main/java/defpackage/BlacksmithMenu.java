package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Graphics;

/* renamed from: aa */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:aa.class */
/**
 * The blacksmith hub screen (world screen 8), reached from the world via event
 * op 11/2 ({@link GameState#requestState}, which does {@code setScreen(8)} then
 * {@link #open()}). It is a lazily-created singleton that shows a two-choice
 * popup — upgrade or identify — over the {@code /sgui/blak} label table
 * ({@link #text}); {@link #onPopupResult} pushes the chosen child
 * ({@link UpgradeMenu} or {@link IdentifyMenu}). Both children read their strings
 * from this class's shared {@link #text} table. The screen paints itself at the
 * centered panel origin ({@link #panelX}/{@link #panelY}) via {@link #draw}.
 */
public final class BlacksmithMenu extends Menu {
    /* renamed from: a */
    /** Centered panel origin X. */
    public static int panelX;
    /* renamed from: b */
    /** Centered panel origin Y. */
    public static int panelY;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** {@code /sgui/blak} label strings, shared with the upgrade/identify children. */
    public static TextTable text;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Lazily-created singleton instance. */
    private static BlacksmithMenu singleton;

    /* renamed from: a */
    /** Returns (creating on first use) the blacksmith singleton. */
    public static final BlacksmithMenu instance() {
        if (singleton == null) {
            singleton = new BlacksmithMenu();
            panelX = BaseCanvas.halfW - 77;
            panelY = BaseCanvas.halfH - 85;
        }
        return singleton;
    }

    public BlacksmithMenu() {
        super(null, (byte) 0);
    }

    /* renamed from: d */
    /** Loads the {@code /sgui/blak} label table and shows the upgrade/identify choice popup. */
    public final void open() {
        try {
            text = new TextTable("/sgui/blak");
        } catch (IOException unused) {
        }
        showPopup((byte) 8, (byte) 2, new Object[]{text.get(0), text.get(1), text.get(2)});
    }

    /* renamed from: e */
    /** Tears the blacksmith down and returns to the world screen. */
    public final void closeBlacksmith() {
        singleton = null;
        text = null;
        ((Menu) this).child = null;
        GameState.setScreen(2);
        GameLoop.gameScreen.markRedraw();
        System.gc();
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (keyCode != -8) {
            return false;
        }
        closeBlacksmith();
        return false;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if (tag == 8 && result == 0) {
            ((Menu) this).child = new UpgradeMenu(this);
            ((Menu) this).child.showMessage(new Object[]{text.get(30), text.get(31), text.get(32), text.get(33)});
        } else if (tag == 8 && result == 1) {
            ((Menu) this).child = new IdentifyMenu(this);
        } else {
            closeBlacksmith();
        }
    }

    /* renamed from: a */
    /** Draws the whole blacksmith screen tree at the centered panel origin. */
    public final void draw(Graphics graphics) {
        render(graphics, panelX, panelY);
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        FontManager.clearScreen(graphics);
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }
}
