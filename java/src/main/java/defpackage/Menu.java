package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: cb */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:cb.class */
/**
 * Abstract base of every in-game menu, panel and dialog. It models a stack of
 * nested screens via the {@link #parent}/{@link #child} links: a menu pushes a
 * sub-screen by assigning {@link #child}, and input/painting are delegated down
 * that chain by {@link #passKeyToChild} and {@link #render}.
 *
 * <p>Each menu holds a linear cursor ({@link #cursorIndex}) over
 * {@link #itemCount} entries, moved by the shared navigation helpers
 * ({@link #moveCursorVertical}/{@link #moveCursorHorizontal}/{@link #stepCursor})
 * with optional wrap-around, and a {@link #needsRepaint} dirty flag driving the
 * lazy repaint in {@link #render}. For lists longer than a page the cursor is
 * paginated five entries at a time ({@link #currentPage}/{@link #pageCount}/
 * {@link #drawListPage}).
 *
 * <p>The remainder of the class is a static <b>draw kit</b> shared by all menu
 * subclasses: beveled boxes ({@link #drawBevelBox}/{@link #drawPanelFrame}/
 * {@link #drawInsetPanel}), buttons ({@link #drawButton}/{@link #drawTabButton}/
 * {@link #drawSelectableBox}), text fields ({@link #drawTextField}) and
 * item/gold widgets ({@link #drawItemIcon}/{@link #drawItemInfo}/
 * {@link #drawQuickSlotRow}/{@link #drawGold}).
 */
public abstract class Menu implements Directions {
    /* renamed from: a */
    /** Enclosing menu this screen was pushed from ({@code null} at the root). */
    public Menu parent;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Number of selectable entries (tabs/rows) the cursor ranges over. */
    public byte itemCount;

    /* renamed from: b */
    /** Sub-screen currently pushed on top of this one ({@code null} if none). */
    public Menu child = null;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** One-shot flag forcing a first paint even when {@link #needsRepaint} is clear. */
    private boolean pendingInitialPaint = true;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Dirty flag: when set, {@link #render} repaints this screen. */
    public boolean needsRepaint = true;

    /* JADX INFO: renamed from: b, reason: collision with other field name */
    /** Zero-based index of the highlighted entry. */
    public byte cursorIndex = 0;

    public Menu(Menu parentMenu, byte itemCount) {
        this.parent = parentMenu;
        this.itemCount = itemCount;
    }

    /* renamed from: a */
    /** Handles a key press; {@code action} is the game action, {@code keyCode} the raw key. */
    public abstract boolean handleKey(int action, int keyCode);

    /* renamed from: b */
    /** Forwards a key to the pushed {@link #child}; returns whether the child consumed it. */
    public final boolean passKeyToChild(int action, int keyCode) {
        if (this.child != null && this.child.handleKey(action, keyCode)) {
            return true;
        }
        this.needsRepaint = true;
        return false;
    }

    /** Paints this screen's own content (implemented per subclass). */
    public abstract void paint(Graphics graphics, int originX, int originY);

    /* renamed from: b */
    /** Lazily repaints this screen when dirty, then recurses into the pushed {@link #child}. */
    public final void render(Graphics graphics, int originX, int originY) {
        boolean painted = false;
        if (this.needsRepaint) {
            this.needsRepaint = false;
            paint(graphics, originX, originY);
            painted = true;
        }
        if (this.child != null) {
            this.child.render(graphics, originX, originY);
        } else if (this.pendingInitialPaint) {
            if (!painted) {
                paint(graphics, originX, originY);
            }
            this.pendingInitialPaint = false;
        }
    }

    /* renamed from: a */
    /** Moves the cursor up/down (keys 2/8 or UP/DOWN actions); {@code wrap} allows wrap-around. */
    public final boolean moveCursorVertical(int action, int keyCode, boolean wrap) {
        switch (keyCode) {
            case 50:
                stepCursor((byte) 3, wrap);
                return true;
            case 56:
                stepCursor((byte) 4, wrap);
                return true;
            default:
                switch (action) {
                    case 1:
                        stepCursor((byte) 3, wrap);
                        return true;
                    case 6:
                        stepCursor((byte) 4, wrap);
                        return true;
                    default:
                        return false;
                }
        }
    }

    /* renamed from: c */
    /** {@link #moveCursorVertical} without wrap-around (stops at the ends). */
    public final boolean moveCursorVerticalNoWrap(int action, int keyCode) {
        return moveCursorVertical(action, keyCode, false);
    }

    /* renamed from: d */
    /** Moves the cursor left/right (keys 4/6 or LEFT/RIGHT actions), with wrap-around. */
    public final boolean moveCursorHorizontal(int action, int keyCode) {
        switch (keyCode) {
            case 52:
                moveCursor((byte) 3);
                return true;
            case 54:
                moveCursor((byte) 4);
                return true;
            default:
                switch (action) {
                    case 2:
                        moveCursor((byte) 3);
                        return true;
                    case 5:
                        moveCursor((byte) 4);
                        return true;
                    default:
                        return false;
                }
        }
    }

    /** Steps the cursor one entry in {@code direction} (4 = forward, else backward), with wrap. */
    public void moveCursor(byte direction) {
        stepCursor(direction, true);
    }

    /* renamed from: a */
    /** Core cursor step: {@code direction} 4 advances, otherwise retreats; {@code wrap} enables wrap-around. */
    public final void stepCursor(byte direction, boolean wrap) {
        if (direction != 4) {
            this.cursorIndex = (byte) (this.cursorIndex - 1);
            if (this.cursorIndex < 0) {
                if (wrap) {
                    this.cursorIndex = (byte) (this.itemCount - 1);
                    return;
                } else {
                    this.cursorIndex = (byte) 0;
                    return;
                }
            }
            return;
        }
        this.cursorIndex = (byte) (this.cursorIndex + 1);
        if (this.cursorIndex >= this.itemCount) {
            if (wrap) {
                this.cursorIndex = (byte) 0;
                return;
            }
            this.cursorIndex = (byte) (this.itemCount - 1);
            if (this.cursorIndex < 0) {
                this.cursorIndex = (byte) 0;
            }
        }
    }

    /* renamed from: a */
    /** Popup-result callback ({@code tag}/{@code result}); default dismisses the child and resumes the game. */
    public void onPopupResult(byte tag, byte result) {
        this.child = null;
        if (GameLoop.gameScreen != null) {
            GameLoop.gameScreen.activate();
        }
        invalidateUp();
    }

    /* renamed from: a */
    /** Closes this screen: drops the pushed child and reactivates the game screen. */
    public final void close() {
        this.child = null;
        if (GameLoop.gameScreen != null) {
            GameLoop.gameScreen.activate();
        }
        invalidateUp();
    }

    /* renamed from: a */
    /** Pushes a {@link PopupMenu} of {@code style}/{@code tag} carrying {@code lines}. */
    public final void showPopup(byte style, byte tag, Object[] lines) {
        this.child = new PopupMenu(this, style, tag, lines, null, null);
    }

    /* renamed from: a */
    /** Pushes a {@link PopupMenu} with custom yes/no button labels. */
    public final void showPopup(byte style, byte tag, Object[] lines, char[] okLabel, char[] cancelLabel) {
        this.child = new PopupMenu(this, style, tag, lines, okLabel, cancelLabel);
    }

    /* renamed from: a */
    /** Pushes a plain message {@link PopupMenu} (style 1) showing {@code lines}. */
    public final void showMessage(Object[] lines) {
        this.child = new PopupMenu(this, (byte) 1, (byte) 0, lines, null, null);
    }

    /* renamed from: b */
    /** Marks this screen and every ancestor as needing a repaint. */
    public final void invalidateUp() {
        if (this.parent != null) {
            this.parent.invalidateUp();
        }
        this.needsRepaint = true;
    }

    /* renamed from: c */
    /** Marks this screen and every pushed descendant as needing a repaint. */
    public final void invalidateDown() {
        if (this.child != null) {
            this.child.invalidateDown();
        }
        this.needsRepaint = true;
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** One-based page number the cursor currently sits on (five entries per page). */
    public final int currentPage() {
        return (this.cursorIndex / 5) + 1;
    }

    /* JADX INFO: renamed from: b, reason: collision with other method in class */
    /** Total number of five-entry pages. */
    public final int pageCount() {
        return ((this.itemCount - 1) / 5) + 1;
    }

    /* JADX INFO: renamed from: c, reason: collision with other method in class */
    /** Index of the first entry shown on the current page. */
    public final int pageFirstIndex() {
        return (currentPage() - 1) * 5;
    }

    /* renamed from: d */
    /** Index of the last entry shown on the current page (clamped to the list end). */
    public final int pageLastIndex() {
        int last = (currentPage() * 5) - 1;
        return last > this.itemCount - 1 ? this.itemCount - 1 : last;
    }

    /* renamed from: a */
    /** Draws the current page of a scrolling five-slot list plus up/down arrows when {@code arrows}. */
    public final void drawListPage(Graphics graphics, int x, int y, boolean arrows) {
        byte selected = (byte) (this.cursorIndex % 5);
        int remaining = this.itemCount - ((currentPage() - 1) * 5);
        int rowsOnPage = remaining;
        if (remaining > 5) {
            rowsOnPage = 5;
        }
        for (byte row = 0; row < rowsOnPage; row = (byte) (row + 1)) {
            if (row != selected) {
                drawTabButton(graphics, x, y, row, false);
            }
        }
        drawBevelBox(graphics, x + 27, y + 10, 120, 137, 4136767, 10452799, 4144959);
        fillInset2(graphics, x + 27, y + 10, 120, 137, 6242111);
        drawTabButton(graphics, x, y, selected, true);
        if (arrows) {
            if (currentPage() > 1) {
                graphics.drawImage(AssetCache.scrollUpArrow, x + 70, y + 4, 20);
            }
            if (currentPage() < pageCount()) {
                graphics.drawImage(AssetCache.scrollDownArrow, x + 70, y + 148, 20);
            }
        }
    }

    /* renamed from: a */
    /** Draws a two-tone rectangle outline: {@code lightColor} top/left, {@code darkColor} bottom/right. */
    private static final void drawBevelOutline(Graphics graphics, int x, int y, int width, int height, int lightColor, int darkColor) {
        graphics.setColor(lightColor);
        graphics.drawLine(x + 1, y, (x + width) - 2, y);
        graphics.drawLine(x, y + 1, x, (y + height) - 2);
        graphics.setColor(darkColor);
        graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 1);
        graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
    }

    /* renamed from: c */
    /** Fills the interior of a {@code width}&times;{@code height} box inset by one pixel with {@code color}. */
    private static final void fillInset1(Graphics graphics, int x, int y, int width, int height, int color) {
        graphics.setColor(color);
        graphics.fillRect(x + 1, y + 1, width - 2, height - 2);
    }

    /* renamed from: a */
    /** Draws a raised beveled box outline: {@code frame} edges, {@code highlight} inner top/left, {@code shadow} inner bottom/right. */
    public static final void drawBevelBox(Graphics graphics, int x, int y, int width, int height, int frame, int highlight, int shadow) {
        graphics.setColor(frame);
        graphics.drawLine(x + 1, y, (x + width) - 2, y);
        graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 2);
        graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
        graphics.drawLine(x, y + 1, x, (y + height) - 2);
        graphics.setColor(highlight);
        graphics.drawLine(x + 1, y + 1, (x + width) - 3, y + 1);
        graphics.drawLine(x + 1, y + 1, x + 1, (y + height) - 3);
        graphics.setColor(shadow);
        graphics.drawLine((x + width) - 2, y + 1, (x + width) - 2, (y + height) - 3);
        graphics.drawLine(x + 1, (y + height) - 2, (x + width) - 2, (y + height) - 2);
    }

    /* JADX INFO: renamed from: a, reason: collision with other method in class */
    /** Fills the interior of a {@code width}&times;{@code height} box inset by two pixels with {@code color}. */
    public static final void fillInset2(Graphics graphics, int x, int y, int width, int height, int color) {
        graphics.setColor(color);
        graphics.fillRect(x + 2, y + 2, width - 4, height - 4);
    }

    /* renamed from: b */
    /** Draws a single-color rectangle outline and fills its interior with {@code color}. */
    public static final void fillOutlinedRect(Graphics graphics, int x, int y, int width, int height, int color) {
        graphics.setColor(color);
        graphics.drawLine(x + 1, y, (x + width) - 2, y);
        graphics.drawLine(x, y + 1, x, (y + height) - 2);
        graphics.drawLine((x + width) - 1, y + 1, (x + width) - 1, (y + height) - 2);
        graphics.drawLine(x + 1, (y + height) - 1, (x + width) - 2, (y + height) - 1);
        graphics.fillRect(x + 1, y + 1, width - 2, height - 2);
    }

    /* renamed from: a */
    /** Draws a beveled panel box using the standard menu palette. */
    public static final void drawPanelFrame(Graphics graphics, int x, int y, int width, int height) {
        drawBevelBox(graphics, x, y, width, height, 2039615, 6242111, 2039615);
    }

    /* renamed from: b */
    /** Fills a panel interior (inset two pixels) with the standard menu-blue background. */
    public static final void fillPanelInterior(Graphics graphics, int x, int y, int width, int height) {
        fillInset2(graphics, x, y, width, height, 4136767);
    }

    /* renamed from: a */
    /** Draws the selection cursor slot for tab/row {@code index}; {@code selected} chooses the lit palette. */
    public static final void drawTabButton(Graphics graphics, int x, int y, byte index, boolean selected) {
        int slotX = x + 3;
        int slotY = y + 10 + (index * 23);
        graphics.setColor(selected ? 4136767 : 6242111);
        graphics.fillRect(slotX + 1, slotY, 24, 1);
        graphics.fillRect(slotX, slotY + 1, 1, 16);
        graphics.fillRect(slotX + 1, slotY + 17, 24, 1);
        graphics.setColor(selected ? 10452799 : 14663551);
        graphics.fillRect(slotX + 1, slotY + 1, 24, 1);
        graphics.fillRect(slotX + 1, slotY + 1, 1, 16);
        graphics.setColor(selected ? 4144959 : 8347519);
        graphics.fillRect(slotX + 2, slotY + 16, 23, 1);
        graphics.setColor(selected ? 6242111 : 10452863);
        graphics.fillRect(slotX + 2, slotY + 2, 24, 14);
    }

    /* renamed from: a */
    /** Draws a right-aligned {@code amount} of gold with the coin icon at ({@code x},{@code y}). */
    public static final void drawGold(Graphics graphics, int x, int y, int amount) {
        BaseCanvas.drawNumberAt(graphics, amount, x, y, 8);
        graphics.drawImage(AssetCache.goldIcon, x - BaseCanvas.numberWidth(amount), y, 24);
    }

    /* renamed from: a */
    /** Draws {@code item}'s icon and, when {@code showQty} and stacked, its quantity. */
    public static final void drawItemIcon(Graphics graphics, int x, int y, Item item, boolean showQty) {
        graphics.drawImage(AssetCache.itemIcons[item.type], x, y + 1, 3);
        if (!showQty || item.quantity <= 1) {
            return;
        }
        BaseCanvas.drawNumberAt(graphics, item.quantity, x + 11, y + 2, 8);
    }

    /* renamed from: a */
    /** Draws an item detail block: plain items show name + description, equipment shows its stat panel. */
    public static final void drawItemInfo(Graphics graphics, int x, int y, Item item) {
        if (!(item instanceof Equipment)) {
            graphics.setColor(16777215);
            int textBottom = (y + FontManager.drawWrappedText(graphics, x, y, 115, 1, item.name)) - (FontManager.lineHeight() + 2);
            graphics.setColor(14663551);
            if (BaseCanvas.width > 128) {
                FontManager.drawWrappedText(graphics, x, textBottom + 15, 110, 1, item.description);
                return;
            } else {
                FontManager.drawWrappedText(graphics, x, textBottom + 15, 75, 1, item.description);
                return;
            }
        }
        Equipment equipment = (Equipment) item;
        if (!equipment.identified) {
            graphics.setColor(14663551);
            int nameBottom = (y + FontManager.drawWrappedText(graphics, x, y, 115, 1, equipment.typeName())) - (FontManager.lineHeight() + 2);
            graphics.setColor(16777215);
            FontManager.drawWrappedText(graphics, x, nameBottom + 16, 115, 1, AssetCache.commonText.get(5));
            return;
        }
        graphics.setColor(16777215);
        int cursorY = (y + FontManager.drawWrappedText(graphics, x, y, 115, 1, ((Item) equipment).name)) - (FontManager.lineHeight() + 2);
        graphics.setColor(14663551);
        FontManager.drawChars(graphics, x, cursorY + 25, AssetCache.commonText.get(equipment instanceof Weapon ? 4 : 46), 1);
        BaseCanvas.drawNumberAt(graphics, equipment.value, (x + 155) - 47, cursorY + 25, 8);
        if (item instanceof Armor) {
            Armor armor = (Armor) item;
            if (armor.attribute != -1) {
                graphics.setColor(16711680);
                cursorY = (cursorY + FontManager.drawWrappedText(graphics, x + 55, cursorY + 10, 115, 1, Armor.attributeNames.get(armor.attribute))) - (FontManager.lineHeight() + 2);
            }
        }
        graphics.setColor(14663551);
        FontManager.drawChars(graphics, x, cursorY + 40, AssetCache.commonText.get(3), 1);
        BaseCanvas.drawFraction(graphics, (x + 155) - 47, cursorY + 40, equipment.refineLevel, equipment.levelReq);
        StringBuffer stringBuffer = new StringBuffer();
        for (int stat = 0; stat < equipment.enchant.length; stat++) {
            if (equipment.enchant[stat] > 0) {
                stringBuffer.append(FontManager.charsToString(AssetCache.heroText.get(9 + stat))).append("+").append((int) equipment.enchant[stat]).append("  ");
            }
        }
        stringBuffer.append(FontManager.charsToString(((Item) equipment).description));
        char[] descChars = stringBuffer.toString().toCharArray();
        if (BaseCanvas.width > 128) {
            FontManager.drawWrappedText(graphics, x, cursorY + 55, 110, 1, descChars);
        } else {
            FontManager.drawWrappedText(graphics, x, cursorY + 55, 75, 1, descChars);
        }
    }

    /* renamed from: a */
    /** Draws a quick-slot row: hotkey number {@code hotkey}, its label {@code label}, and {@code item} (highlighted when {@code selected}). */
    public static final void drawQuickSlotRow(Graphics graphics, int x, int y, Item item, byte hotkey, char[] label, boolean selected) {
        fillOutlinedRect(graphics, x, y + 1, 28, 31, 12558207);
        BaseCanvas.drawNumberAt(graphics, hotkey, BaseCanvas.drawLabelBox(graphics, AssetCache.commonText.get(2), x + 2, y + 1) + 2, y + 1, 4);
        graphics.setColor(16777215);
        FontManager.drawCharsCentered(graphics, x + 90, y + 2, label, 1);
        if (selected) {
            drawBevelBox(graphics, x + 30, y + 14, 117, 19, 4136767, 10452799, 4144959);
            fillInset2(graphics, x + 30, y + 14, 117, 19, 6233919);
        } else {
            drawBevelBox(graphics, x + 30, y + 14, 117, 19, 6242111, 14663551, 8347519);
            fillInset2(graphics, x + 30, y + 14, 117, 19, 10452863);
        }
        if (item != null) {
            graphics.drawImage(AssetCache.itemIcons[item.type], x + 14, y + 19, 3);
            graphics.setColor(16777215);
            if (!(item instanceof Equipment) || ((Equipment) item).identified) {
                FontManager.drawChars(graphics, x + 34, y + 20, item.name, 1);
            } else {
                FontManager.drawChars(graphics, x + 34, y + 20, item.typeName(), 1);
            }
        }
    }

    /* renamed from: a */
    /** Draws a {@code width}-wide beveled button showing centered {@code label} (highlighted when {@code selected}). */
    public static final void drawButton(Graphics graphics, int x, int y, int width, char[] label, boolean selected) {
        if (selected) {
            drawBevelBox(graphics, x, y, width, 19, 4136767, 10452799, 4144959);
            fillInset2(graphics, x, y, width, 19, 6233919);
        } else {
            drawBevelBox(graphics, x, y, width, 19, 6242111, 14663551, 8347519);
            fillInset2(graphics, x, y, width, 19, 10452863);
        }
        if (label != null) {
            graphics.setColor(16777215);
            FontManager.drawChars(graphics, (x + (width / 2)) - (FontManager.stringWidth(label) / 2), y + 5, label, 1);
        }
    }

    /* renamed from: a */
    /** Draws an unlabeled selectable box; {@code selected} swaps between the two menu palettes. */
    public static final void drawSelectableBox(Graphics graphics, int x, int y, int width, int height, boolean selected) {
        if (selected) {
            drawBevelBox(graphics, x, y, width, height, 6242111, 14663551, 8347519);
        } else {
            drawBevelBox(graphics, x, y, width, height, 4136767, 10452799, 4144959);
        }
    }

    /* renamed from: a */
    /** Fills a {@code fillColor} rectangle and draws {@code text} in {@code textColor}, centered when {@code align}==1, inset by {@code pad}. */
    public static final void drawTextField(Graphics graphics, int x, int y, int width, int height, char[] text, int pad, int align, int fillColor, int textColor) {
        graphics.setColor(fillColor);
        graphics.fillRect(x, y, width, height);
        if (text == null) {
            return;
        }
        int innerWidth = (width - pad) - pad;
        graphics.setColor(textColor);
        if (align == 1) {
            FontManager.drawWrappedBlockCentered(graphics, x + pad + (innerWidth >> 1), y + 1, innerWidth, 1, text, 0, 0, text.length);
        } else {
            FontManager.drawWrappedText(graphics, x + pad, y + 1, innerWidth, 1, text);
        }
    }

    /* renamed from: c */
    /** Draws an inset panel: a two-tone outline over a filled interior. */
    public static final void drawInsetPanel(Graphics graphics, int x, int y, int width, int height) {
        drawBevelOutline(graphics, x, y, width, height, 16768959, 12558207);
        fillInset1(graphics, x, y, width, height, 14663551);
    }
}
