package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: af */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:af.class */
/**
 * General-purpose popup / dialog spawned by {@link Menu#showPopup} and
 * {@link Menu#showMessage}. Its {@link #type} code selects the layout:
 * <ul>
 *   <li>1 / 11 — OK message (single softkey);</li>
 *   <li>2 / 6 / 12 — yes-no confirm;</li>
 *   <li>3 / 4 / 5 / 8 — a selectable option list (8 keeps row 0 as a header);</li>
 *   <li>9 — a bare message with no softkeys.</li>
 * </ul>
 * Message layouts draw {@link #joinedText} (all options joined by newlines);
 * list layouts draw {@link #options} row by row with the cursor arrow. The two
 * softkey labels {@link #positiveLabel} / {@link #negativeLabel} default per
 * type. Answers are reported back to the parent through
 * {@code onPopupResult(type, result)} where {@code result} is the chosen row,
 * {@code 0} for OK/Yes, or {@code 99} for cancel/Back. {@link #boxHeight} is the
 * panel height pre-measured in the constructor.
 */
public final class PopupMenu extends Menu {
    /* renamed from: c */
    /** Layout/behavior selector — see the class doc for the type codes. */
    private byte type;

    /* renamed from: a */
    /** The option rows as passed in (each element is a {@code char[]}). */
    private Object[] options;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** {@link #options} joined by newlines into one block, for message layouts. */
    private char[] joinedText;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Pre-measured panel height for the current type and content. */
    private int boxHeight;

    /* renamed from: b */
    /** Left/confirm softkey label (OK/Yes/…), defaulted from {@link #type}. */
    private char[] positiveLabel;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Right/cancel softkey label (Back/No/…), defaulted from {@link #type}. */
    private char[] negativeLabel;

    public PopupMenu(Menu parent, byte type, byte itemCount, Object[] options, char[] positiveLabel, char[] negativeLabel) {
        super(parent, itemCount);
        this.type = type;
        StringBuffer joined = new StringBuffer();
        for (int i = 0; i < options.length; i++) {
            char[] option = (char[]) options[i];
            if (option.length > 0) {
                joined.append(option);
                if (i != options.length - 1) {
                    joined.append('\n');
                }
            }
        }
        this.joinedText = joined.toString().toCharArray();
        this.options = options;
        if (type == 2 || type == 12) {
            positiveLabel = positiveLabel == null ? FontManager.labelYes : positiveLabel;
            if (negativeLabel == null) {
                negativeLabel = FontManager.labelNo;
            }
        } else if (type == 1 || type == 11) {
            if (positiveLabel == null) {
                positiveLabel = FontManager.labelOk;
            }
        } else if (type != 9) {
            positiveLabel = positiveLabel == null ? FontManager.labelOk : positiveLabel;
            if (negativeLabel == null) {
                negativeLabel = FontManager.labelBack;
            }
        }
        this.positiveLabel = positiveLabel;
        this.negativeLabel = negativeLabel;
        switch (type) {
            case 2:
            case 6:
                this.boxHeight = 8 + FontManager.measureBlockHeight(FontManager.percentOf(BaseCanvas.width, 80) - 10, 1, this.joinedText, 0, 0, this.joinedText.length);
                break;
            case 3:
            case 4:
            case 5:
            case 8:
                this.boxHeight = 12;
                for (int optionIndex = 0; optionIndex < this.options.length; optionIndex++) {
                    char[] option = (char[]) this.options[optionIndex];
                    this.boxHeight += 3 + FontManager.measureBlockHeight(FontManager.percentOf(BaseCanvas.width, 80) - 10, 1, option, 0, 0, option.length);
                }
                break;
            case 7:
            case 9:
            case 10:
            default:
                this.boxHeight = 22 + FontManager.measureBlockHeight(FontManager.percentOf(BaseCanvas.width, 80) - 10, 1, this.joinedText, 0, 0, this.joinedText.length);
                break;
            case 11:
            case 12:
                this.boxHeight = 8 + FontManager.measureBlockHeight(FontManager.percentOf(BaseCanvas.width, 80) - 10, 1, this.joinedText, 0, 0, this.joinedText.length);
                break;
        }
        if (type == 6) {
            ((Menu) this).cursorIndex = (byte) 1;
        }
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        switch (this.type) {
            case 1:
            case 11:
                if (keyCode == 53 || action == 8) {
                    ((Menu) this).parent.onPopupResult(this.type, (byte) 0);
                }
                break;
            case 2:
            case 6:
            case 12:
                if (keyCode == 53) {
                    ((Menu) this).parent.onPopupResult(this.type, (byte) 0);
                } else if (keyCode == -8) {
                    ((Menu) this).parent.onPopupResult(this.type, (byte) 99);
                }
                break;
            case 3:
            case 4:
            case 5:
            case 8:
                if (!moveCursorVerticalNoWrap(action, keyCode)) {
                    if (keyCode == 53 || action == 8) {
                        ((Menu) this).parent.onPopupResult(this.type, ((Menu) this).cursorIndex);
                    } else if (keyCode == -8) {
                        ((Menu) this).parent.onPopupResult(this.type, (byte) 99);
                    }
                    break;
                }
                break;
        }
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        FontManager.clearScreen(graphics);
        int boxWidth = FontManager.percentOf(BaseCanvas.width, 80);
        int boxX = BaseCanvas.halfW - (boxWidth >> 1);
        int boxY = BaseCanvas.halfH - (this.boxHeight >> 1);
        Menu.drawPanelFrame(graphics, boxX, boxY, boxWidth, this.boxHeight);
        Menu.fillPanelInterior(graphics, boxX, boxY, boxWidth, this.boxHeight);
        switch (this.type) {
            case 1:
            case 9:
                graphics.setColor(16777215);
                FontManager.drawWrappedText(graphics, boxX + 5, boxY + 5, boxWidth - 10, 1, this.joinedText);
                if (this.type != 9) {
                    FontManager.drawSoftKeys(graphics, this.positiveLabel, (char[]) null);
                }
                break;
            case 2:
            case 6:
                graphics.setColor(16777215);
                FontManager.drawWrappedText(graphics, boxX + 5, boxY + 5, boxWidth - 10, 1, this.joinedText);
                break;
            case 3:
            case 4:
            case 5:
                graphics.setColor(16777215);
                int rowY = boxY + 7;
                for (byte optionIndex = 0; optionIndex < this.options.length; optionIndex = (byte) (optionIndex + 1)) {
                    if (optionIndex == ((Menu) this).cursorIndex) {
                        graphics.drawImage(AssetCache.cursorArrow, boxX + 5, rowY, 20);
                    }
                    rowY += 3 + FontManager.drawWrappedText(graphics, boxX + 12, rowY, boxWidth - 10, 1, (char[]) this.options[optionIndex]);
                }
                break;
            case 8:
                graphics.setColor(16777215);
                int headerY = boxY + 5;
                int listY = headerY + 3 + FontManager.drawWrappedText(graphics, boxX + 5, headerY, boxWidth - 10, 1, (char[]) this.options[0]);
                for (byte optionIndex2 = 1; optionIndex2 < this.options.length; optionIndex2 = (byte) (optionIndex2 + 1)) {
                    if (optionIndex2 == ((Menu) this).cursorIndex + 1) {
                        graphics.drawImage(AssetCache.cursorArrow, boxX + 5, listY, 20);
                    }
                    listY += 3 + FontManager.drawWrappedText(graphics, boxX + 12, listY, boxWidth - 10, 1, (char[]) this.options[optionIndex2]);
                }
                break;
            case 11:
                FontManager.drawWrappedText(graphics, boxX + 5, boxY + 5, boxWidth - 10, 1, this.joinedText);
                break;
            case 12:
                FontManager.drawWrappedText(graphics, boxX + 5, boxY + 5, boxWidth - 10, 1, this.joinedText);
                break;
        }
        FontManager.drawSoftKeys(graphics, this.positiveLabel, this.negativeLabel);
    }
}
