package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bi */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bi.class */
/**
 * Stat-point allocation dialog pushed by {@link StatusPage} (page 3) when the
 * hero has unspent points. Left/right adjust the pending points on the selected
 * stat (STR/VIT/AGI/SPR), tracked in {@link #pending} without touching the hero
 * until confirmed; OK asks for confirmation and, on yes, commits the deltas and
 * recomputes derived stats.
 */
public final class StatAllocMenu extends Menu {
    /* renamed from: a */
    /** Stat points still available to spend (starts at the hero's balance). */
    private short remainingPoints;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Pending points queued onto each of the four base stats. */
    private short[] pending;

    public StatAllocMenu(StatusPage statusPage) {
        super(statusPage, (byte) 4);
        this.remainingPoints = GameState.hero().statPoints;
        this.pending = new short[4];
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode == 52 || action == 2) {
            adjustStat((byte) 3);
            return true;
        }
        if (keyCode == 54 || action == 5) {
            adjustStat((byte) 4);
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
            return true;
        }
        if (this.pending[0] == 0 && this.pending[1] == 0 && this.pending[2] == 0 && this.pending[3] == 0) {
            showPopup((byte) 1, (byte) 1, new Object[]{CharacterMenu.text.get(34), CharacterMenu.text.get(35)});
            return true;
        }
        showPopup((byte) 2, (byte) 2, new Object[]{CharacterMenu.text.get(33)});
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int boxX = x + 36;
        int boxY = y + 37;
        Hero hero = GameState.hero();
        Menu.fillOutlinedRect(graphics, boxX, boxY, 101, 26, 4136767);
        graphics.setColor(16777215);
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(36), boxX + 3, boxY + 3);
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(37), boxX + 3, boxY + 10 + 4);
        BaseCanvas.drawNumberAt(graphics, this.remainingPoints, boxX + 65, boxY + 10 + 4, 8);
        graphics.setColor(6242111);
        graphics.fillRect(boxX, boxY + 30, 101, 62);
        for (byte statIndex = 0; statIndex < 4; statIndex = (byte) (statIndex + 1)) {
            if (((Menu) this).cursorIndex == statIndex) {
                graphics.setColor(16777215);
                graphics.drawImage(AssetCache.cursorArrow, boxX + 2, boxY + 35 + (statIndex * 15), 20);
            } else {
                graphics.setColor(14663551);
            }
            int statValue = this.pending[statIndex];
            switch (statIndex) {
                case 0:
                    statValue += hero.strength + hero.strengthBonus;
                    break;
                case 1:
                    statValue += hero.vitality + hero.vitalityBonus;
                    break;
                case 2:
                    statValue += hero.agility + hero.agilityBonus;
                    break;
                case 3:
                    statValue += hero.spirit + hero.spiritBonus;
                    break;
            }
            FontManager.drawChars(graphics, boxX + 10, boxY + 35 + (statIndex * 15), AssetCache.heroText.get(9 + statIndex), 1);
            graphics.drawImage(AssetCache.slotFrame, boxX + 45 + 25, boxY + 35 + (statIndex * 15), 20);
            BaseCanvas.drawNumberAt(graphics, statValue, boxX + 65 + 25, boxY + 35 + (statIndex * 15), 8);
            graphics.drawImage(AssetCache.cursorArrow, boxX + 67 + 25, boxY + 35 + (statIndex * 15), 20);
        }
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
            Hero hero = GameState.hero();
            hero.strength = (short) (hero.strength + this.pending[0]);
            hero.vitality = (short) (hero.vitality + this.pending[1]);
            hero.agility = (short) (hero.agility + this.pending[2]);
            hero.spirit = (short) (hero.spirit + this.pending[3]);
            hero.statPoints = this.remainingPoints;
            hero.recomputeStats();
            ((Menu) this).parent.onPopupResult((byte) -1, (byte) -1);
        }
    }

    /* renamed from: b */
    /** Adjusts the selected stat: {@code direction} 4 spends a point, 3 refunds one. */
    private void adjustStat(byte direction) {
        if (direction == 4 && this.remainingPoints > 0) {
            short[] pending = this.pending;
            byte stat = ((Menu) this).cursorIndex;
            pending[stat] = (short) (pending[stat] + 1);
            this.remainingPoints = (short) (this.remainingPoints - 1);
            return;
        }
        if (direction != 3 || this.pending[((Menu) this).cursorIndex] <= 0) {
            return;
        }
        short[] pending2 = this.pending;
        byte stat2 = ((Menu) this).cursorIndex;
        pending2[stat2] = (short) (pending2[stat2] - 1);
        this.remainingPoints = (short) (this.remainingPoints + 1);
    }
}
