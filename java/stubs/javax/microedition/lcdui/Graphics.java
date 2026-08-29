package javax.microedition.lcdui;

// Minimal compile-only stub of the MIDP Graphics (JSR-118 lcdui).
// Behaviour-free. Method set matches exactly what the baseline bytecode
// references on Graphics (clip/draw/fill/color/translate).
public class Graphics {

    // Standard lcdui anchor / stroke constants (structural only; JSR-118 values).
    public static final int HCENTER = 1;
    public static final int VCENTER = 2;
    public static final int LEFT = 4;
    public static final int RIGHT = 8;
    public static final int TOP = 16;
    public static final int BOTTOM = 32;
    public static final int BASELINE = 64;
    public static final int SOLID = 0;
    public static final int DOTTED = 1;

    Graphics() {
    }

    public int getColor() {
        return 0;
    }

    public void setColor(int rgb) {
    }

    public void setColor(int red, int green, int blue) {
    }

    public int getClipX() {
        return 0;
    }

    public int getClipY() {
        return 0;
    }

    public int getClipWidth() {
        return 0;
    }

    public int getClipHeight() {
        return 0;
    }

    public void clipRect(int x, int y, int width, int height) {
    }

    public void setClip(int x, int y, int width, int height) {
    }

    public void translate(int x, int y) {
    }

    public int getTranslateX() {
        return 0;
    }

    public int getTranslateY() {
        return 0;
    }

    public void drawLine(int x1, int y1, int x2, int y2) {
    }

    public void drawRect(int x, int y, int width, int height) {
    }

    public void fillRect(int x, int y, int width, int height) {
    }

    public void drawArc(int x, int y, int width, int height,
            int startAngle, int arcAngle) {
    }

    public void fillArc(int x, int y, int width, int height,
            int startAngle, int arcAngle) {
    }

    public void drawImage(Image img, int x, int y, int anchor) {
    }
}
