package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: be */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:OptionsMenu.class */
/**
 * Options screen — volume, difficulty, auto-advance-text and camera-follow
 * rows. It is reached both from the in-game {@link SystemTab}
 * ({@code inGame == true}, drawn as an inset panel) and from the title
 * {@link MainMenu} ({@code inGame == false}, drawn full screen). Left/right on a
 * row mutates that setting on the shared {@link GameLoop} ({@link #gameLoop});
 * Back persists the settings via {@link GameLoop#saveOptions} and closes.
 */
public final class OptionsMenu extends Menu {
    /* renamed from: c */
    /** True when opened in-game (inset panel); false when opened from the title menu. */
    private boolean inGame;

    /* renamed from: a */
    /** The shared game-loop whose option fields this screen edits. */
    private GameLoop gameLoop;

    public OptionsMenu(Menu parent, boolean inGame) {
        super(parent, (byte) 4);
        this.inGame = inGame;
        this.gameLoop = GameLoop.instance;
    }

    /* JADX WARN: Multi-variable type inference failed */
    /* JADX WARN: Type inference failed for: r0v50 */
    /* JADX WARN: Type inference failed for: r0v51, types: [java.lang.Throwable] */
    /* JADX WARN: Type inference failed for: r0v55, types: [bs] */
    /* JADX WARN: Type inference failed for: r0v58 */
    /* JADX WARN: Type inference failed for: r0v59 */
    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVertical(action, keyCode, false)) {
            return true;
        }
        if (keyCode == 52 || action == 2 || keyCode == 54 || action == 5) {
            switch (((Menu) this).cursorIndex) {
                case 0:
                    if (keyCode == 52 || action == 2) {
                        this.gameLoop.volume = 0;
                    } else if (keyCode == 54 || action == 5) {
                        this.gameLoop.volume = AudioManager.maxVolume;
                    }
                    AudioManager.setVolume(this.gameLoop.volume);
                    if (this.gameLoop.volume == 0) {
                        AudioManager.stopSfx();
                    }
                    break;
                case 1:
                    if (keyCode == 52 || action == 2) {
                        GameLoop loop = this.gameLoop;
                        loop.difficulty = (byte) (loop.difficulty - 1);
                        if (this.gameLoop.difficulty < 0) {
                            this.gameLoop.difficulty = (byte) 2;
                        }
                    }
                    if (keyCode == 54 || action == 5) {
                        GameLoop loop2 = this.gameLoop;
                        loop2.difficulty = (byte) (loop2.difficulty + 1);
                        if (this.gameLoop.difficulty > 2) {
                            this.gameLoop.difficulty = (byte) 0;
                        }
                    }
                    this.gameLoop.setDifficulty(this.gameLoop.difficulty);
                    break;
                case 2:
                    this.gameLoop.autoTextAdvance = !this.gameLoop.autoTextAdvance;
                    break;
                case 3:
                    this.gameLoop.cameraFollow = !this.gameLoop.cameraFollow;
                    break;
            }
        }
        if (keyCode != -8) {
            return true;
        }
        boolean follow = this.gameLoop.cameraFollow;
        if (follow) {
            GameState.camX = GameState.camTargetX;
            int camTargetY = GameState.camTargetY;
            GameState.camY = camTargetY;
        }
        try {
            GameLoop.instance.saveOptions();
        } catch (Exception e) {
            e.printStackTrace();
        }
        ((Menu) this).parent.close();
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int labelX;
        int firstRowY;
        int dimColor = 0;
        int valueColor = 0;
        if (this.inGame) {
            int panelX = x + 6;
            int panelY = y + 25;
            Menu.drawPanelFrame(graphics, panelX, panelY, 143, 139);
            Menu.fillPanelInterior(graphics, panelX, panelY, 143, 139);
            dimColor = 10452799;
            valueColor = 16777215;
            FontManager.drawSoftKeys(graphics, FontManager.blankLabel, FontManager.labelBack);
            labelX = panelX + 5 + 15;
            firstRowY = panelY + 15 + 10;
        } else {
            graphics.setColor(4136767);
            graphics.fillRect(0, 0, BaseCanvas.width, BaseCanvas.height);
            MainMenu.drawTitlePlate(graphics, x, y);
            FontManager.drawMenuItem(graphics, 5, (x + 155) >> 1, y + 5);
            MainMenu.drawMenuPanel(graphics, x, y + 24, 3);
            labelX = x + 15 + 12;
            firstRowY = y + 10 + 46;
            graphics.drawImage(AssetCache.menuFrames[19], labelX + 1, firstRowY + 16, 20);
            graphics.drawImage(AssetCache.menuFrames[19], labelX + 1, firstRowY + 36, 20);
            graphics.drawImage(AssetCache.menuFrames[19], labelX + 1, firstRowY + 56, 20);
            FontManager.drawSoftKeys(graphics, (char[]) null, FontManager.labelBack);
        }
        int volumeY = firstRowY;
        byte cursor = ((Menu) this).cursorIndex;
        graphics.setColor(cursor == 0 ? 16777215 : dimColor);
        FontManager.drawChars(graphics, labelX, volumeY, AssetCache.commonText.get(18), 1);
        graphics.setColor(valueColor);
        if (this.gameLoop.volume == 0) {
            FontManager.drawCharsCentered(graphics, labelX + 70, volumeY, StringTable.instance.get(3945).toCharArray(), 0);
        } else {
            FontManager.drawCharsCentered(graphics, labelX + 70, volumeY, StringTable.instance.get(3944).toCharArray(), 0);
        }
        int difficultyY = volumeY + 20;
        graphics.setColor(cursor == 1 ? 16777215 : dimColor);
        FontManager.drawChars(graphics, labelX, difficultyY, AssetCache.commonText.get(19), 1);
        graphics.setColor(valueColor);
        FontManager.drawCharsCentered(graphics, labelX + 70, difficultyY, AssetCache.commonText.get(60 + this.gameLoop.difficulty), 0);
        int autoTextY = difficultyY + 20;
        graphics.setColor(cursor == 2 ? 16777215 : dimColor);
        FontManager.drawChars(graphics, labelX, autoTextY, AssetCache.commonText.get(20), 1);
        graphics.setColor(valueColor);
        FontManager.drawCharsCentered(graphics, labelX + 70, autoTextY, (this.gameLoop.autoTextAdvance ? StringTable.instance.get(3942) : StringTable.instance.get(3943)).toCharArray(), 0);
        int cameraY = autoTextY + 20;
        graphics.setColor(cursor == 3 ? 16777215 : dimColor);
        FontManager.drawChars(graphics, labelX, cameraY, AssetCache.commonText.get(21), 1);
        graphics.setColor(valueColor);
        FontManager.drawCharsCentered(graphics, labelX + 70, cameraY, (this.gameLoop.cameraFollow ? StringTable.instance.get(3944) : StringTable.instance.get(3945)).toCharArray(), 0);
        byte row = 0;
        while (true) {
            byte rowIndex = row;
            if (rowIndex >= ((Menu) this).itemCount) {
                return;
            }
            graphics.drawImage(AssetCache.slotFrame, labelX + 42, firstRowY + (rowIndex * 20), 20);
            graphics.drawImage(AssetCache.cursorArrow, labelX + 92, firstRowY + (rowIndex * 20), 20);
            row = (byte) (rowIndex + 1);
        }
    }
}
