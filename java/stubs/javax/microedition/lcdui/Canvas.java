package javax.microedition.lcdui;

// Minimal compile-only stub of the MIDP Canvas (JSR-118 lcdui).
// Behaviour-free. Signatures cover exactly what the baseline references
// (getGameAction/getWidth/getHeight/keyReleased/repaint/setFullScreenMode and
// the <init>) plus the overridable event/paint hooks the baseline subclasses
// (r -> as, bg) override: paint, keyPressed, keyReleased, showNotify,
// hideNotify. `paint` is a concrete no-op here (not abstract) so the stub never
// forces an override and the tree compiles regardless of which subclass is
// concrete; in the real API it is `protected abstract`.
public abstract class Canvas extends Displayable {

    // MIDP game-action codes (referenced structurally; values match JSR-118).
    public static final int UP = 1;
    public static final int DOWN = 6;
    public static final int LEFT = 2;
    public static final int RIGHT = 5;
    public static final int FIRE = 8;
    public static final int GAME_A = 9;
    public static final int GAME_B = 10;
    public static final int GAME_C = 11;
    public static final int GAME_D = 12;

    public static final int KEY_NUM0 = 48;
    public static final int KEY_NUM1 = 49;
    public static final int KEY_NUM2 = 50;
    public static final int KEY_NUM3 = 51;
    public static final int KEY_NUM4 = 52;
    public static final int KEY_NUM5 = 53;
    public static final int KEY_NUM6 = 54;
    public static final int KEY_NUM7 = 55;
    public static final int KEY_NUM8 = 56;
    public static final int KEY_NUM9 = 57;
    public static final int KEY_STAR = 42;
    public static final int KEY_POUND = 35;

    protected Canvas() {
    }

    public int getGameAction(int keyCode) {
        return 0;
    }

    public int getKeyCode(int gameAction) {
        return 0;
    }

    public boolean hasPointerEvents() {
        return false;
    }

    public boolean hasPointerMotionEvents() {
        return false;
    }

    public boolean hasRepeatEvents() {
        return false;
    }

    public boolean isDoubleBuffered() {
        return false;
    }

    public void setFullScreenMode(boolean mode) {
    }

    public final void repaint() {
    }

    public final void repaint(int x, int y, int width, int height) {
    }

    public final void serviceRepaints() {
    }

    protected void paint(Graphics g) {
    }

    protected void keyPressed(int keyCode) {
    }

    protected void keyReleased(int keyCode) {
    }

    protected void keyRepeated(int keyCode) {
    }

    protected void pointerPressed(int x, int y) {
    }

    protected void pointerReleased(int x, int y) {
    }

    protected void pointerDragged(int x, int y) {
    }

    protected void showNotify() {
    }

    protected void hideNotify() {
    }

    protected void sizeChanged(int w, int h) {
    }
}
