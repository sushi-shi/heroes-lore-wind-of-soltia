package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: bn */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:bn.class */
/**
 * Guardian skill-equip sub-panel, pushed by {@link GuardianTab} for one
 * {@link Guardian}. The top bar shows the guardian's two skill slots; left/right
 * toggle which slot ({@link #editingFirstSlot}) is being edited, and the list
 * below shows the three learnable skills with unlock levels
 * ({@link Guardian#skillUnlockLevel}). Selecting a skill calls
 * {@link Guardian#equipSkill}, with popups for the empty row, an
 * already-equipped skill, or a still-locked skill.
 */
public final class GuardianSkillPanel extends Menu {
    /* renamed from: c */
    /** Which slot the cursor edits: {@code true} = first (left) slot, {@code false} = second. */
    private boolean editingFirstSlot;

    /* renamed from: a */
    /** The guardian whose skills are being assigned. */
    private Guardian guardian;

    public GuardianSkillPanel(GuardianTab guardianTab, Guardian guardian) {
        super(guardianTab, (byte) 3);
        this.editingFirstSlot = true;
        this.guardian = guardian;
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode == 52 || keyCode == 54 || action == 2 || action == 5) {
            this.editingFirstSlot = !this.editingFirstSlot;
            return true;
        }
        if (keyCode != 53 && action != 8) {
            if (keyCode != -8) {
                return true;
            }
            ((Menu) this).parent.close();
            return true;
        }
        if (((Menu) this).cursorIndex == 3) {
            showPopup((byte) 1, (byte) 0, new Object[]{CharacterMenu.text.get(59), CharacterMenu.text.get(60)});
            return true;
        }
        if (this.guardian.skillSlotA == ((Menu) this).cursorIndex || this.guardian.skillSlotB == ((Menu) this).cursorIndex) {
            showPopup((byte) 1, (byte) 0, new Object[]{CharacterMenu.text.get(61)});
            return true;
        }
        if (this.guardian.equipSkill(this.editingFirstSlot, ((Menu) this).cursorIndex, false)) {
            return true;
        }
        showPopup((byte) 1, (byte) 0, new Object[]{CharacterMenu.text.get(62), CharacterMenu.text.get(63)});
        return true;
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int headerY = y + 10;
        Menu.drawPanelFrame(graphics, panelX, headerY, 132, 22);
        Menu.fillPanelInterior(graphics, panelX, headerY, 132, 22);
        int listTop = headerY + 19;
        Menu.drawBevelBox(graphics, panelX, listTop, 149, 140, 2039615, 10452799, 4144959);
        Menu.fillInset2(graphics, panelX, listTop, 149, 140, 6242111);
        int slotBarY = listTop - 19;
        int slotBarX = panelX + (this.editingFirstSlot ? 0 : 66);
        graphics.setColor(6242111);
        graphics.fillRect(slotBarX + 2, slotBarY + 2, 62, 19);
        graphics.setColor(4144959);
        graphics.fillRect(slotBarX + 44 + 20, slotBarY + 1, 1, 19);
        graphics.setColor(2039615);
        graphics.fillRect(slotBarX + 45 + 20, slotBarY + 1, 1, 17);
        graphics.fillRect(slotBarX, slotBarY + 1, 1, 18);
        graphics.fillRect(slotBarX + 1, slotBarY, 64, 1);
        graphics.setColor(10452799);
        graphics.fillRect(slotBarX + 1, slotBarY + 2, 1, 18);
        graphics.fillRect(slotBarX + 1, slotBarY + 1, 63, 1);
        int slotAY = slotBarY + 5 + (this.editingFirstSlot ? 0 : 2);
        int slotALabelRight = BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(38), panelX + 6, slotAY);
        BaseCanvas.drawNumberAt(graphics, 1, slotALabelRight + 3, slotAY, 4);
        if (this.guardian.skillSlotA != -1) {
            graphics.drawImage(AssetCache.guardianSkillIcons[(this.guardian.type * 4) + this.guardian.skillSlotA], slotALabelRight + 13, slotAY - 2, 20);
        }
        int slotBY = slotBarY + 5 + (this.editingFirstSlot ? 2 : 0);
        int slotBLabelRight = BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(38), panelX + 51 + 20, slotBY);
        BaseCanvas.drawNumberAt(graphics, 3, slotBLabelRight + 3, slotBY, 4);
        if (this.guardian.skillSlotB != -1) {
            graphics.drawImage(AssetCache.guardianSkillIcons[(this.guardian.type * 4) + this.guardian.skillSlotB], slotBLabelRight + 13, slotBY - 2, 20);
        }
        graphics.drawImage(AssetCache.slotFrame, panelX, slotBarY + 7, 20);
        graphics.drawImage(AssetCache.cursorArrow, panelX + 90 + 40, slotBarY + 7, 20);
        graphics.setColor(4136767);
        Menu.fillOutlinedRect(graphics, panelX + 28, slotBarY + 26, 114, 127, 4136767);
        Menu.fillOutlinedRect(graphics, panelX + 3, slotBarY + 31 + (((Menu) this).cursorIndex * 20), 26, 19, 4136767);
        for (int skill = 0; skill < 3; skill++) {
            graphics.drawImage(AssetCache.guardianSkillIcons[(this.guardian.type * 4) + skill], panelX + 5, slotBarY + 48 + (skill * 20), 36);
        }
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, panelX + 34, slotBarY + 29, AssetCache.guardianSkillText.get((this.guardian.type * 8) + (((Menu) this).cursorIndex * 2)), 1);
        BaseCanvas.drawNumberAt(graphics, Guardian.skillUnlockLevel[((Menu) this).cursorIndex], BaseCanvas.drawLabelBox(graphics, FontManager.levelPrefix, panelX + 34, slotBarY + 44) + 3, slotBarY + 44, 4);
        graphics.setColor(14663551);
        FontManager.drawWrappedText(graphics, panelX + 34, slotBarY + 53, 100, 1, AssetCache.guardianSkillText.get((this.guardian.type * 8) + (((Menu) this).cursorIndex * 2) + 1));
    }
}
