package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: q */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:q.class */
/**
 * Character-status sub-panel, tab 0 of {@link CharacterMenu} (also opened by the
 * level-up flow). Four vertical pages selected by the cursor: 0 = summary
 * (name/level/exp/HP/MP), 1 = the six stats (STR/VIT/AGI/SPR + attack/defense
 * with equipment bonuses), 2 = class lore, 3 = a stat-point box that pushes
 * {@link StatAllocMenu} when the hero has unspent points.
 */
public final class StatusPage extends Menu {
    /* renamed from: a */
    /** Localized class name (from {@link AssetCache#f192a}). */
    private char[] className;

    /* renamed from: b */
    /** Localized class flavour/description line. */
    private char[] classDesc;

    public StatusPage(Menu parentMenu) {
        super(parentMenu, (byte) 4);
        this.className = AssetCache.heroText.get(GameState.classId - 6);
        int descIndex = (3 + GameState.classId) - 6;
        if (GameState.clearCount == 1) {
            descIndex += 15;
        } else if (GameState.clearCount >= 2) {
            descIndex += 18;
        }
        this.classDesc = AssetCache.heroText.get(descIndex);
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (((Menu) this).cursorIndex != 3) {
            return false;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        if (GameState.hero().statPoints > 0) {
            ((Menu) this).child = new StatAllocMenu(this);
            return true;
        }
        showMessage(new Object[]{CharacterMenu.text.get(0), CharacterMenu.text.get(1)});
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int panelY = y + 15;
        Hero hero = GameState.hero();
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(2), panelX + 5, panelY);
        Menu.drawGold(graphics, panelX + 110, panelY + 2, hero.bag.gold);
        drawListPage(graphics, panelX, panelY, false);
        BaseCanvas.drawNumberAt(graphics, 1, panelX + 12, panelY + 16, 4);
        BaseCanvas.drawNumberAt(graphics, 2, panelX + 12, panelY + 16 + 23, 4);
        BaseCanvas.drawNumberAt(graphics, 3, panelX + 12, panelY + 16 + 46, 4);
        graphics.drawImage(AssetCache.statusPanelIcon, panelX + 10, panelY + 14 + 69, 20);
        if (hero.statPoints > 0) {
            graphics.drawImage(AssetCache.portraitFrame, panelX + 3, panelY + 18 + 69, 36);
        }
        switch (((Menu) this).cursorIndex) {
            case 0:
                graphics.setColor(16777215);
                FontManager.drawChars(graphics, panelX + 35, panelY + 18, this.className, 1);
                graphics.setColor(14663551);
                FontManager.drawChars(graphics, panelX + 33, panelY + 35, this.classDesc, 1);
                graphics.drawImage(AssetCache.statLabel3, panelX + 35, panelY + 52, 20);
                BaseCanvas.drawNumberAt(graphics, hero.level, panelX + 52, panelY + 52, 4);
                graphics.drawImage(AssetCache.statLabel1, panelX + 34, panelY + 70, 20);
                BaseCanvas.drawNumberAt(graphics, hero.exp, panelX + 102, panelY + 70, 8);
                graphics.setColor(4136767);
                graphics.fillRect(panelX + 34, panelY + 79, 72, 3);
                graphics.setColor(16777215);
                graphics.fillRect(panelX + 34 + 1, panelY + 79 + 1, (hero.exp * 70) / hero.expToNext, 1);
                graphics.drawImage(AssetCache.statLabel4, panelX + 38, panelY + 84, 20);
                BaseCanvas.drawNumberAt(graphics, hero.expToNext, panelX + 102, panelY + 84, 8);
                graphics.drawImage(AssetCache.statLabel2, panelX + 34, panelY + 97, 20);
                BaseCanvas.drawFraction(graphics, panelX + 102, panelY + 96, hero.hp, hero.maxHp);
                graphics.drawImage(AssetCache.statLabel5, panelX + 34, panelY + 106, 20);
                BaseCanvas.drawFraction(graphics, panelX + 102, panelY + 105, hero.mp, hero.maxMp);
                break;
            case 1:
                graphics.setColor(14663551);
                for (int stat = 0; stat < 6; stat++) {
                    FontManager.drawChars(graphics, panelX + 38, panelY + 21 + (stat * 15), AssetCache.heroText.get(9 + stat), 1);
                }
                BaseCanvas.drawNumberAt(graphics, hero.strength + hero.strengthBonus, panelX + 100, panelY + 22, 8);
                BaseCanvas.drawNumberAt(graphics, hero.vitality + hero.vitalityBonus, panelX + 100, panelY + 22 + 15, 8);
                BaseCanvas.drawNumberAt(graphics, hero.agility + hero.agilityBonus, panelX + 100, panelY + 22 + 30, 8);
                BaseCanvas.drawNumberAt(graphics, hero.spirit + hero.spiritBonus, panelX + 100, panelY + 22 + 45, 8);
                BaseCanvas.drawNumberAt(graphics, hero.attack, panelX + 100, panelY + 22 + 60, 8);
                BaseCanvas.drawNumberAt(graphics, hero.defense, panelX + 100, panelY + 22 + 75, 8);
                break;
            case 2:
                graphics.setColor(14663551);
                FontManager.drawChars(graphics, panelX + 34, panelY + 18, this.className, 1);
                graphics.setColor(16777215);
                char[] loreText = AssetCache.heroText.get(GameState.classId);
                if (BaseCanvas.width > 128) {
                    FontManager.drawWrappedText(graphics, panelX + 34, panelY + 30, 110, 1, loreText);
                } else {
                    FontManager.drawWrappedText(graphics, panelX + 34, panelY + 30, 75, 1, loreText);
                }
                break;
            case 3:
                Menu.fillOutlinedRect(graphics, panelX + 34, panelY + 22, 100, 26, 4136767);
                graphics.setColor(16777215);
                BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(3), panelX + 37, panelY + 25);
                BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(4), panelX + 37, panelY + 32 + 4);
                BaseCanvas.drawNumberAt(graphics, GameState.hero().statPoints, panelX + 99, panelY + 32 + 4, 8);
                Menu.fillOutlinedRect(graphics, panelX + 34, panelY + 62, 100, 33, 4136767);
                graphics.setColor(16777215);
                FontManager.drawChars(graphics, panelX + 40, panelY + 72, CharacterMenu.text.get(5), 1);
                graphics.setColor(14663551);
                FontManager.drawChars(graphics, panelX + 60, panelY + 67, CharacterMenu.text.get(6), 1);
                FontManager.drawChars(graphics, panelX + 60, panelY + 80, CharacterMenu.text.get(7), 1);
                break;
        }
    }
}
