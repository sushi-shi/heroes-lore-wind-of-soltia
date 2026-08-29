package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: s */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:s.class */
/**
 * Class-skill tab (tab 4) of {@link CharacterMenu}. The constructor scans the
 * class's skills (via {@link GameState#classStartFlags} and the learned-skill
 * flag bits) and builds {@link #skillEntries}, a list of
 * {@code (skillIndex, variant)} pairs for the skills to display. The selected
 * skill's name/description come from the {@code grd}-style skill text table
 * {@link AssetCache#f228f}; OK confirms learning the skill via a
 * {@link ConfirmDialog}.
 */
public final class SkillTab extends Menu {
    /* renamed from: h */
    /** Flattened list of {@code (skillIndex, variant)} pairs for the visible skills. */
    private byte[] skillEntries;

    /* renamed from: c */
    /** Class-skill index of the highlighted entry. */
    private byte skillIndex;

    /* renamed from: d */
    /** Variant of the highlighted entry (0 = basic, 2 = advanced). */
    private byte skillVariant;

    /* renamed from: a */
    /** Localized name of the highlighted skill. */
    private char[] skillName;

    /* renamed from: b */
    /** Localized description of the highlighted skill. */
    private char[] skillDesc;

    public SkillTab(Menu parentMenu) {
        super(parentMenu, (byte) 0);
        int skillCount = GameState.classStartFlags[GameState.classId - 6].length;
        int writePos = 0;
        this.skillEntries = new byte[skillCount * 2];
        for (byte skill = 0; skill < skillCount; skill = (byte) (skill + 1)) {
            if (!GameState.isFlag(1 + (skill * 3) + 1)) {
                if (GameState.isFlag(1 + (skill * 3) + 2)) {
                    if (GameState.isFlag(1 + (skill * 3)) && AssetCache.classSkillText.get((skill * 7) + 2).length > 0) {
                        this.skillEntries[writePos] = skill;
                        this.skillEntries[writePos + 1] = 2;
                        writePos += 2;
                    }
                } else if (GameState.isFlag(1 + (skill * 3)) && AssetCache.classSkillText.get(skill * 7).length > 0) {
                    this.skillEntries[writePos] = skill;
                    this.skillEntries[writePos + 1] = 0;
                    writePos += 2;
                }
            }
        }
        ((Menu) this).itemCount = (byte) (writePos / 2);
        loadSelectedSkill();
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode)) {
            return true;
        }
        if (moveCursorVerticalNoWrap(action, keyCode)) {
            ((Menu) this).parent.needsRepaint = true;
            loadSelectedSkill();
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        if (((Menu) this).itemCount <= 0) {
            return true;
        }
        ((Menu) this).child = new ConfirmDialog(this, this.skillName, this.skillDesc, (byte) 0);
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        super.onPopupResult(tag, result);
        if (tag == 0 && result == 1) {
            ((Menu) this).child = new ConfirmDialog(this, CharacterMenu.text.get(54), AssetCache.classSkillText.get((this.skillIndex * 7) + 6), (byte) 1);
        }
    }

    /* renamed from: d */
    /** Refreshes {@link #skillIndex}/{@link #skillVariant} and the name/desc for the cursor. */
    private final void loadSelectedSkill() {
        this.skillIndex = this.skillEntries[((Menu) this).cursorIndex * 2];
        this.skillVariant = this.skillEntries[(((Menu) this).cursorIndex * 2) + 1];
        if (this.skillVariant == 2) {
            int base = (this.skillIndex * 7) + 2;
            this.skillName = AssetCache.classSkillText.get(base);
            this.skillDesc = AssetCache.classSkillText.get(base + 1);
        } else {
            int base2 = (this.skillIndex * 7) + 0;
            this.skillName = AssetCache.classSkillText.get(base2);
            this.skillDesc = AssetCache.classSkillText.get(base2 + 1);
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int x, int y) {
        int panelX = x + 2;
        int panelY = y + 15;
        BaseCanvas.drawLabelBox(graphics, CharacterMenu.text.get(39), panelX + 5, panelY);
        if (((Menu) this).itemCount <= 0) {
            Menu.drawBevelBox(graphics, panelX + 4, panelY + 10, 143, 137, 4136767, 10452799, 4144959);
            Menu.fillInset2(graphics, panelX + 4, panelY + 10, 143, 137, 6242111);
            graphics.setColor(16777215);
            FontManager.drawWrappedText(graphics, panelX + 10, panelY + 15, 96, 1, CharacterMenu.text.get(58));
            return;
        }
        drawListPage(graphics, panelX, panelY, true);
        for (int slot = pageFirstIndex(); slot <= pageLastIndex(); slot++) {
            graphics.drawImage(AssetCache.itemIcons[18], panelX + 13, panelY + 18 + (23 * (slot % 5)), 3);
        }
        int nameY = panelY + 14;
        graphics.setColor(16777215);
        FontManager.drawWrappedText(graphics, panelX + 33, nameY, 105, 1, this.skillName);
        int descY = nameY + (FontManager.lineCount(this.skillName, 105) * 15);
        graphics.setColor(14663551);
        FontManager.drawChars(graphics, panelX + 33, descY, GameState.classStartFlags[GameState.classId - 6][this.skillIndex] ? CharacterMenu.text.get(55) : CharacterMenu.text.get(56), 1);
        int lineY = descY + 15;
        FontManager.drawChars(graphics, panelX + 33, lineY, CharacterMenu.text.get(57), 1);
        int nextY = lineY + 15;
        if (AssetCache.classSkillText.get((this.skillIndex * 7) + 4).length > 0) {
            graphics.setColor(16777215);
            FontManager.drawChars(graphics, panelX + 33, nextY, AssetCache.classSkillText.get((this.skillIndex * 7) + 4), 1);
            nextY += 15;
        }
        if (AssetCache.classSkillText.get((this.skillIndex * 7) + 5).length > 0) {
            graphics.setColor(16777215);
            FontManager.drawChars(graphics, panelX + 33, nextY, AssetCache.classSkillText.get((this.skillIndex * 7) + 5), 1);
        }
    }
}
