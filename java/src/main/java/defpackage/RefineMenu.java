package defpackage;

import java.io.IOException;
import javax.microedition.lcdui.Graphics;

/* renamed from: ax */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:ax.class */
/**
 * The item-refinery hub screen (world screen 7), reached from the world via
 * event op 11/1 ({@link GameState#requestState}, which does {@code setScreen(7)}
 * then {@link #open()}). It is a lazily-created singleton that shows a two-choice
 * popup — enchant or combine — over the {@code /sgui/refi} label table
 * ({@link #text}); {@link #onPopupResult} pushes the chosen child
 * ({@link EnchantMenu} or {@link CombineMenu}). Both children read their strings
 * from this class's shared {@link #text} table. The screen paints itself at the
 * centered panel origin ({@link #panelX}/{@link #panelY}) via {@link #draw}.
 */
public final class RefineMenu extends Menu {
    /* renamed from: a */
    /** Centered panel origin X. */
    public static int panelX;
    /* renamed from: b */
    /** Centered panel origin Y. */
    public static int panelY;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** {@code /sgui/refi} label strings, shared with the enchant/combine children. */
    public static TextTable text;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Lazily-created singleton instance. */
    private static RefineMenu singleton;

    /* renamed from: a */
    /** Returns (creating on first use) the refinery singleton. */
    public static final RefineMenu instance() {
        if (singleton == null) {
            singleton = new RefineMenu();
            panelX = BaseCanvas.halfW - 77;
            panelY = BaseCanvas.halfH - 85;
        }
        return singleton;
    }

    public RefineMenu() {
        super(null, (byte) 0);
    }

    /* renamed from: d */
    /** Loads the {@code /sgui/refi} label table and shows the enchant/combine choice popup. */
    public final void open() {
        try {
            text = new TextTable("/sgui/refi");
        } catch (IOException e) {
            e.printStackTrace();
        }
        showPopup((byte) 8, (byte) 2, new Object[]{text.get(0), text.get(1), text.get(2)});
    }

    /* renamed from: e */
    /** Tears the refinery down and returns to the world screen. */
    public final void closeRefine() {
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
        closeRefine();
        return false;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if (tag == 8 && result == 0) {
            ((Menu) this).child = new EnchantMenu(this);
        } else if (tag == 8 && result == 1) {
            ((Menu) this).child = new CombineMenu(this);
        } else {
            closeRefine();
        }
    }

    /* renamed from: a */
    /** Draws the whole refinery screen tree at the centered panel origin. */
    public final void draw(Graphics graphics) {
        render(graphics, panelX, panelY);
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        FontManager.clearScreen(graphics);
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }
}
