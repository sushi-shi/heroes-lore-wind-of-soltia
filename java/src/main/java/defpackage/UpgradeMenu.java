package defpackage;

import javax.microedition.lcdui.Graphics;

/* renamed from: at */
/* JADX INFO: loaded from: Heroes-Lore-Wind-of-Soltia_J2ME_EN_v207.jar:at.class */
/**
 * The blacksmith's equipment-refine (upgrade) screen, pushed by
 * {@link BlacksmithMenu}. The player picks an identified {@link #equipment}
 * piece; refining it consumes {@link #materialCost} upgrade stones (item type 11)
 * and {@link #goldCost} gold, then rolls a success chance that rises as the piece
 * nears its refine cap. On success {@link Equipment#refineLevel} goes up; on
 * failure the piece is destroyed. The outcome plays through a small
 * {@link #resultPhase} state machine (with a "processing" pause), and
 * {@link #success} records whether the roll passed.
 */
public final class UpgradeMenu extends Menu {
    /* renamed from: a */
    /** The equipment piece selected for refining. */
    private Equipment equipment;

    /* JADX INFO: renamed from: a, reason: collision with other field name */
    /** Gold cost of the next refine ({@code refineLevel * 100}). */
    private int goldCost;
    /* renamed from: c */
    /** Number of upgrade stones (item type 11) the next refine consumes. */
    private byte materialCost;
    /* renamed from: d */
    /** Result-animation phase: 0 idle, 2 show "processing", 1 apply outcome. */
    private byte resultPhase;

    /* JADX INFO: renamed from: c, reason: collision with other field name */
    /** Whether the last refine roll succeeded. */
    private boolean success;

    public UpgradeMenu(Menu parent) {
        super(parent, (byte) 2);
    }

    @Override // defpackage.cb
    public final boolean handleKey(int action, int keyCode) {
        if (passKeyToChild(action, keyCode) || moveCursorVerticalNoWrap(action, keyCode)) {
            return true;
        }
        if (keyCode != 53 && action != 8) {
            return false;
        }
        Hero hero = GameState.hero();
        if (((Menu) this).cursorIndex == 0) {
            byte[] slots = hero.bag.equipmentSlots(false, (byte) 0);
            if (slots.length > 0) {
                ((Menu) this).child = new ItemPickerList(this, slots, ((Menu) this).cursorIndex, BlacksmithMenu.text.get(3));
                return true;
            }
            showMessage(new Object[]{StringTable.instance.get(3936).toCharArray()});
            return true;
        }
        if (((Menu) this).cursorIndex != 1) {
            return true;
        }
        int materialCount = hero.bag.totalQuantity((byte) 11, (byte) 0);
        byte materialSlot = hero.bag.findSlot((byte) 11, (byte) 0);
        if (this.equipment == null) {
            showMessage(new Object[]{BlacksmithMenu.text.get(3)});
            return true;
        }
        if (this.equipment.refineLevel >= this.equipment.levelReq) {
            showMessage(new Object[]{BlacksmithMenu.text.get(6), BlacksmithMenu.text.get(7)});
            return true;
        }
        if (materialSlot < 0 || materialCount < this.materialCost) {
            showMessage(new Object[]{BlacksmithMenu.text.get(5)});
            return true;
        }
        if (hero.bag.gold < this.goldCost) {
            showMessage(new Object[]{BlacksmithMenu.text.get(8)});
            return true;
        }
        showPopup((byte) 2, (byte) 2, new Object[]{BlacksmithMenu.text.get(9)});
        return true;
    }

    @Override // defpackage.cb
    public final void onPopupResult(byte tag, byte result) {
        Menu previousChild = ((Menu) this).child;
        super.onPopupResult(tag, result);
        if ((previousChild instanceof PopupMenu) && tag == 2 && result == 0) {
            Hero hero = GameState.hero();
            Debug.assertTrue(hero.bag.findSlot((byte) 11, (byte) 0) != -1);
            hero.bag.removeItems((byte) 11, (byte) 0, this.materialCost);
            hero.bag.gold -= this.goldCost;
            if (((100 * (this.equipment.levelReq - this.equipment.refineLevel)) / this.equipment.levelReq) + 30 < ByteUtil.randRange(1, 100)) {
                this.resultPhase = (byte) 2;
                this.success = false;
                return;
            } else {
                this.resultPhase = (byte) 2;
                this.success = true;
                return;
            }
        }
        if ((previousChild instanceof ItemPickerList) && tag == 0) {
            Item picked = result >= 100 ? GameState.hero().getEquip(result - 100) : GameState.hero().bag.get((int) result);
            Debug.assertTrue(picked instanceof Equipment);
            if (!((Equipment) picked).identified) {
                showMessage(new Object[]{BlacksmithMenu.text.get(11), BlacksmithMenu.text.get(12)});
                return;
            }
            this.equipment = (Equipment) picked;
            this.goldCost = this.equipment.refineLevel * 100;
            this.materialCost = (byte) (this.equipment.refineLevel + 1);
        }
    }

    @Override // defpackage.cb
    public final void paint(Graphics graphics, int originX, int originY) {
        Hero hero = GameState.hero();
        graphics.setColor(4136767);
        graphics.fillRect(originX, originY, 155, 170);
        Menu.drawInsetPanel(graphics, originX + 2, originY + 4, 151, 162);
        BaseCanvas.drawLabelBox(graphics, BlacksmithMenu.text.get(13), originX + 3, originY - 2);
        Menu.fillOutlinedRect(graphics, originX + 3, originY + 7, 149, 17, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, originX + 6, originY + 11, BlacksmithMenu.text.get(14), 1);
        Menu.drawQuickSlotRow(graphics, originX + 4, originY + 30, this.equipment, (byte) 1, BlacksmithMenu.text.get(15), ((Menu) this).cursorIndex == 0);
        Menu.drawGold(graphics, (originX + 155) - 8, originY + 65, hero.bag.gold);
        Menu.fillOutlinedRect(graphics, originX + 4, originY + 73, 147, 38, 10452863);
        graphics.setColor(16777215);
        FontManager.drawChars(graphics, originX + 8, originY + 80, BlacksmithMenu.text.get(16), 1);
        if (this.equipment != null) {
            Menu.drawGold(graphics, (originX + 155) - 8, originY + 80, this.goldCost);
            FontManager.drawChars(graphics, originX + 8, originY + 93, ByteUtil.concat(BlacksmithMenu.text.get(17), new StringBuffer().append(" : ").append((int) this.materialCost).append("개").toString().toCharArray()), 1);
        }
        if (this.resultPhase == 2) {
            this.resultPhase = (byte) 1;
            int popupX = BaseCanvas.halfW - 55;
            int popupY = BaseCanvas.halfH - 11;
            Menu.drawPanelFrame(graphics, popupX, popupY, 110, 22);
            Menu.fillPanelInterior(graphics, popupX, popupY, 110, 22);
            graphics.setColor(16777215);
            FontManager.drawChars(graphics, popupX + 5, popupY + 5, BlacksmithMenu.text.get(28), 1);
            ((Menu) this).needsRepaint = true;
        } else if (this.resultPhase == 1) {
            this.resultPhase = (byte) 0;
            try {
                Thread.sleep(500L);
                if (this.success) {
                    Thread.sleep(1000L);
                    Equipment equip = this.equipment;
                    equip.refineLevel = (byte) (equip.refineLevel + 1);
                    ((Menu) this).child = new ItemPickerList(this, new byte[]{hero.slotOf((Item) this.equipment)}, (byte) 10, BlacksmithMenu.text.get(10));
                    this.goldCost = this.equipment.refineLevel * 100;
                    this.materialCost = (byte) (this.equipment.refineLevel + 1);
                } else {
                    GameState.hero().bag.decrementItem((Item) this.equipment, (byte) 1);
                    this.equipment = null;
                    GameState.saveGame();
                    showMessage(new Object[]{BlacksmithMenu.text.get(26), BlacksmithMenu.text.get(29)});
                }
            } catch (Exception unused) {
            }
        }
        int buttonWidth = FontManager.percentOf(155, 80);
        Menu.drawButton(graphics, originX + ((155 - buttonWidth) >> 1), originY + 138, buttonWidth, BlacksmithMenu.text.get(18), ((Menu) this).cursorIndex == 1);
    }
}
