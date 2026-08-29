package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bk */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:StartTraitMenu.class */
/**
 * New-game starting-guardian picker, pushed by {@link ClassConfirmMenu} once a
 * class has been chosen. The player toggles exactly two of the three starting
 * guardians ({@link #guardianSelected}); reaching two selections flips the screen
 * into a yes/no confirmation ({@link #confirming}/{@link #confirmYes}), and
 * confirming calls {@link #startGame()} to launch {@link GameState#newGame} with
 * the chosen {@link #classId} and guardian mask. The selected guardian icon bobs
 * via a small triangle-wave animation ({@link #bounceOffset}/{@link #bounceUp}).
 */
public final class StartTraitMenu extends Menu {
    /* renamed from: c */
    /** True once two guardians are picked and the yes/no confirmation is shown. */
    private boolean confirming;
    /* renamed from: d */
    /** In the confirmation, whether the "start" option (vs. "cancel") is highlighted. */
    private boolean confirmYes;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Chosen character class id, forwarded to {@link GameState#newGame}. */
    private byte classId;
    /* renamed from: b */
    /** Per-guardian selection flags (exactly two may be set). */
    private boolean[] guardianSelected;
    /* renamed from: e */
    /** Bob-animation direction: {@code true} while the offset is rising. */
    private boolean bounceUp;

    /* JADX INFO: renamed from: d, reason: collision with other field name */
    /** Vertical bob offset (0..3) of the highlighted guardian icon. */
    private byte bounceOffset;

    public StartTraitMenu(ClassConfirmMenu parentMenu, byte classId) {
        super(parentMenu, (byte) 3);
        this.guardianSelected = new boolean[3];
        this.bounceUp = true;
        this.bounceOffset = (byte) 0;
        this.confirming = false;
        this.confirmYes = false;
        this.classId = classId;
        showMessage(new Object[]{AssetCache.commonText.get(16), AssetCache.commonText.get(13)});
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (!this.confirming) {
            if (moveCursorHorizontal(action, keyCode)) {
                this.bounceOffset = (byte) 0;
                return true;
            }
            if (keyCode == 53 || action == 8) {
                this.guardianSelected[((Menu) this).cursorIndex] = !this.guardianSelected[((Menu) this).cursorIndex];
                byte selectedCount = 0;
                for (int i = 0; i < 3; i++) {
                    if (this.guardianSelected[i]) {
                        selectedCount = (byte) (selectedCount + 1);
                    }
                }
                if (selectedCount == 2) {
                    this.confirming = true;
                    this.confirmYes = false;
                }
            }
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
            return true;
        }
        switch (action) {
            case 2:
            case 5:
                this.confirmYes = !this.confirmYes;
                break;
            case 8:
                if (this.confirmYes) {
                    startGame();
                } else {
                    this.guardianSelected = new boolean[3];
                    this.confirming = false;
                }
                break;
            default:
                switch (keyCode) {
                    case -8:
                        this.guardianSelected = new boolean[3];
                        this.confirming = false;
                        break;
                    case 52:
                    case 54:
                        this.confirmYes = !this.confirmYes;
                        break;
                    case 53:
                        if (this.confirmYes) {
                            startGame();
                        } else {
                            this.guardianSelected = new boolean[3];
                            this.confirming = false;
                        }
                        break;
                }
                break;
        }
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        graphics.setColor(4136767);
        graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
        MainMenu.drawTitlePlate(graphics, originX, originY);
        FontManager.drawMenuItem(graphics, 1, (originX + 155) >> 1, originY + 5);
        MainMenu.drawMenuPanel(graphics, originX, originY + 24, 3);
        int panelX = originX + 15;
        int panelY = originY + 10;
        graphics.drawImage(AssetCache.menuFrames[19], panelX + 11, panelY + 82, 20);
        byte guardian = 0;
        while (true) {
            byte g = guardian;
            if (g >= 3) {
                break;
            }
            if (this.guardianSelected[g]) {
                graphics.drawImage(AssetCache.menuGuardianPreview[g][1], panelX + 22 + (g * 34), (panelY + 66) - 5, 3);
            } else {
                graphics.drawImage(AssetCache.menuGuardianPreview[g][0], panelX + 22 + (g * 34), ((panelY + 59) - 5) + (((Menu) this).cursorIndex == g ? this.bounceOffset : (byte) 0), 3);
            }
            guardian = (byte) (g + 1);
        }
        if (!this.confirming) {
            graphics.drawImage(AssetCache.menuFrames[20], panelX + 19 + (((Menu) this).cursorIndex * 34), panelY + 73, 20);
        }
        graphics.setColor(0);
        if (this.confirming) {
            graphics.drawImage(AssetCache.menuFrames[17], panelX + 60 + (this.confirmYes ? 0 : 28), panelY + 118, 20);
            FontManager.drawChars(graphics, panelX + 11, panelY + 104, AssetCache.commonText.get(17), 1);
            if (this.confirmYes) {
                graphics.setColor(16777215);
            } else {
                graphics.setColor(0);
            }
            FontManager.drawChars(graphics, panelX + 64, panelY + 121, AssetCache.commonText.get(14), 1);
            if (this.confirmYes) {
                graphics.setColor(0);
            } else {
                graphics.setColor(16777215);
            }
            FontManager.drawChars(graphics, panelX + 92, panelY + 121, AssetCache.commonText.get(15), 1);
        } else {
            FontManager.drawChars(graphics, panelX + 11, panelY + 94, AssetCache.guardianText.get(((Menu) this).cursorIndex), 1);
            FontManager.drawWrappedText(graphics, panelX + 11, panelY + 109, 100, 1, AssetCache.guardianText.get(12 + ((Menu) this).cursorIndex));
        }
        if (this.bounceOffset == 0) {
            this.bounceOffset = (byte) (this.bounceOffset + 1);
            this.bounceUp = true;
        } else if (this.bounceOffset == 3) {
            this.bounceOffset = (byte) (this.bounceOffset - 1);
            this.bounceUp = false;
        } else if (this.bounceUp) {
            this.bounceOffset = (byte) (this.bounceOffset + 1);
        } else {
            this.bounceOffset = (byte) (this.bounceOffset - 1);
        }
        if (((Menu) this).child == null) {
            ((Menu) this).needsRepaint = true;
        }
        FontManager.drawSoftKeys(graphics, FontManager.labelOk, FontManager.labelBack);
    }

    /* renamed from: d */
    /** Launches a new game with the chosen class and guardian selection mask. */
    private void startGame() {
        GameState.newGame(false, this.classId, this.guardianSelected);
    }
}
