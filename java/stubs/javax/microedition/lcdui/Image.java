package javax.microedition.lcdui;

import java.io.IOException;

// Minimal compile-only stub of the MIDP Image (JSR-118 lcdui).
// Behaviour-free. Factory methods and accessors match what the baseline
// references. createImage(String) declares `throws IOException` to match the
// real API (bh.m43a propagates it); the byte[]/int factory overloads do not.
public class Image {

    Image() {
    }

    public static Image createImage(int width, int height) {
        return new Image();
    }

    public static Image createImage(byte[] imageData, int imageOffset, int imageLength) {
        return new Image();
    }

    public static Image createImage(String name) throws IOException {
        return new Image();
    }

    public static Image createImage(Image source) {
        return new Image();
    }

    public Graphics getGraphics() {
        return new Graphics();
    }

    public int getWidth() {
        return 0;
    }

    public int getHeight() {
        return 0;
    }

    public boolean isMutable() {
        return true;
    }
}
