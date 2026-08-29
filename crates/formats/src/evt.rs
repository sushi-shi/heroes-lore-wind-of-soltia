//! `.evt` — the per-map event container, including the EventScript VM bytecode.
//!
//! # Ground truth
//!
//! Decoded from the Phase-2 decompile of the VM class `ah` (`EventScript`) and
//! its loader `ae` (`GameMap`), cross-checked byte-for-byte against all 210
//! baseline blobs under `m/6`, `m/7`, `m/8` in `…v207.jar`.
//!
//! ## The Phase-1 hypothesis was wrong
//!
//! Phase 1 assumed a flat `[opcode][operands…]*` stream terminated by
//! `END_EVNT`. It is **not**. An `.evt` file is a **seven-section container** for
//! one map (loaded by `ae.load()`), and the event scripts are only one section
//! inside it. The scripts themselves are **not** an opcode stream either: each
//! instruction is a fixed **3-byte row** `[opcode][a][b]`, and a script body is
//! **length-prefixed** by its instruction count — `END_EVNT` is a *runtime*
//! early-return, not the framing terminator (only 1163 of the corpus's 1245
//! scripts even end on `END_EVNT`; others end on `MAP_HERO`, `TALKTEXT`, etc.).
//!
//! ## Container layout (parsed in this exact order by `ae.load()`)
//!
//! The first section's length depends on the paired `/m/NN.map`'s dimensions, so
//! [`parse`] takes `width`/`height` (the map's `w`,`h`). Every other section is
//! self-describing (count-prefixed). All counts are unsigned bytes (`& 0xFF`)
//! **except** the three marked *(signed)*, which the VM reads as signed loop
//! bounds — a negative count means zero records (matching the Java `for` loop).
//!
//! | # | section          | framing                                                                 |
//! |---|------------------|-------------------------------------------------------------------------|
//! | 1 | collision grid   | `width * height` raw bytes (no count in-file)                           |
//! | 2 | objects          | `[nImg:u8] nImg×img:u8  [nObj:u8] nObj×(5 bytes)`                        |
//! | 3 | npcs             | `[nId:u8]  nId×id:u8    [nNpc:u8] nNpc×(3 bytes)`                        |
//! | 4 | enemies          | `[nType:u8] nType×id:u8 [nEnm:u8] nEnm×(3 bytes)`                       |
//! | 5 | faces            | `[nFace:i8 (signed)] nFace×id:u8`                                        |
//! | 6 | triggers+scripts+dialogue | see below                                                      |
//! | 7 | initial patches  | `[nCond:i8 (signed)] nCond×(3 bytes) [nGrp:i8 (signed)] nGrp×group`      |
//!
//! Section 6 (`ae.e()`):
//! * triggers  `[nTrig:u8]`, then per trigger `[nEntry:u8] nEntry×(7 bytes)`
//!   (`f23a`; a `0` entry-count leaves that trigger slot empty),
//! * scripts   `[nScript:u8]`, then per script `[nInstr:u8] nInstr×(3 bytes)`
//!   (`f24b` — the EventScript bodies decoded here),
//! * dialogue  `[nStr:u8]`, then per string `[len:u8] len×byte` (Latin-1 text,
//!   `f25c`).
//!
//! Section 7 (`ae.c()` / patch op-table 100–112) per group: `[nPatch:i8 (signed)]
//! nPatch×(4 bytes)`.
//!
//! The whole file must consume **exactly** to EOF; trailing bytes are rejected.
//!
//! # Opcode → operand-shape table (derived from `ah.m11a()` / `ah.d()` dispatch)
//!
//! Every row is 3 bytes on disk (`op`, `a`, `b`); `a`/`b` are **signed** (`i8`).
//! "unused" = the VM ignores that operand for this opcode (the byte is still
//! present in the row). Wider runtime values are noted where the VM combines the
//! two bytes. Multi-row opcodes read the immediately following row (verified: all
//! 811 such pairings in the corpus are consistent, though the VM only *warns* if
//! the follow-up row is wrong, so this decoder does not enforce the pairing).
//!
//! | op | name       | `a`                         | `b`                              | notes |
//! |----|------------|-----------------------------|----------------------------------|-------|
//! | 0  | `CMD_IDLE` | unused                      | unused                           | defined but never dispatched (would spin the PC); unused in corpus |
//! | 1  | `TALKTEXT` | dialogue index (`f25c`)     | face id (`>0` left, `<0` right, `0` none) | |
//! | 2  | `YES/NO`   | prompt dialogue index       | rows to jump on "no"             | |
//! | 3  | `MV_H_MOV` | hero move steps             | unused                           | movement block |
//! | 4  | `MV_DELAY` | delay frames (`-1` counted) | unused                           | movement block; ends the block |
//! | 5  | `MV_H_STP` | unused                      | unused                           | movement block |
//! | 6  | `MV_H_DIR` | hero facing dir             | unused                           | movement block |
//! | 7  | `MAP_CHNG` | target map                  | change param                     | **+ next row `MAP_HERO`** = dest tile |
//! | 8  | `MAP_HERO` | tile x (`×16` px)           | tile y (`×16` px)                | also the follow-up row of `MAP_CHNG` |
//! | 9  | `SWI_DEF`  | switch id low 8 bits        | bits0-1 = op(0/1/2), bits6-7 = id bits 8-9 | 10-bit switch id |
//! | 10 | `MONEY`    | amount hi byte              | amount lo byte                   | `u16` BE |
//! | 11 | `ITEM`     | item type                   | item sub-id                      | **+ next row `ITEM_NUM`** = qty |
//! | 12 | `EXP`      | exp hi byte                 | exp lo byte                      | `u16` BE |
//! | 13 | `HP`       | hp hi byte                  | hp lo byte                       | `u16` BE |
//! | 14 | `SP`       | sp hi byte                  | sp lo byte                       | `u16` BE; unused in corpus |
//! | 15 | `MV_N_MOV` | npc index                   | npc move steps                   | movement block |
//! | 16 | `MV_N_STP` | npc index                   | unused                           | movement block |
//! | 17 | `MV_N_DIR` | npc index                   | npc facing dir                   | movement block |
//! | 18 | `GUARDIAN` | guardian type (0/1/2)       | unused                           | |
//! | 19 | `COMBO`    | combo depth                 | unused                           | |
//! | 20 | `GAMEOVER` | unused                      | unused                           | |
//! | 21 | `ITEM_NUM` | quantity (`>0` give,`<0` take)| unused                         | follow-up row of `ITEM` |
//! | 22 | `SCR_DEL`  | unused                      | unused                           | screen → black fill |
//! | 23 | `SCR_SHOW` | unused                      | unused                           | screen → map fill |
//! | 24 | `SCR_FLAS` | unused                      | unused                           | fixed 5-frame flash |
//! | 25 | `SCR_SHAK` | unused                      | unused                           | random 6-frame shake |
//! | 26 | `BGM_PLAY` | bgm id                      | unused                           | |
//! | 27 | `BGM_STOP` | unused                      | unused                           | |
//! | 28 | `SYSBGM`   | *undetermined*              | *undetermined*                   | **no dispatch handler**; see assumptions |
//! | 29 | `SOUND`    | sound id                    | unused                           | shares `BGM_PLAY` handler |
//! | 30 | `SWI_QUE`  | switch id low 8 bits        | bits0-1 = op, bits6-7 = id hi    | like `SWI_DEF` |
//! | 31 | `GOTO_FOR` | forward jump (rows)         | unused                           | |
//! | 32 | `GOTO_BAK` | backward jump (rows)        | unused                           | unused in corpus |
//! | 33 | `SWI_MAP`  | *undetermined*              | *undetermined*                   | **no dispatch handler**; see assumptions |
//! | 34 | `MV_FO_HE` | unused                      | unused                           | camera follows hero |
//! | 35 | `MV_FO_NO` | unused                      | unused                           | camera follow off |
//! | 36 | `MV_FO_NP` | npc index                   | unused                           | camera follows npc; unused in corpus |
//! | 37 | `MV_CA_MV` | camera dir                  | camera speed                     | |
//! | 38 | `MV_CA_ST` | unused                      | unused                           | camera stop |
//! | 39 | `MV_CA_XY` | camera tile x               | camera tile y                    | |
//! | 40 | `CHG_OBJ`  | object index                | object image index               | |
//! | 41 | `CHG_NPC`  | npc index                   | npc face value                   | |
//! | 42 | `CHGTL_XY` | tile x                      | tile y                           | **+ next row `CHGTL_VA`** = tile/coll |
//! | 43 | `CHGTL_VA` | tile image value            | collision value                  | follow-up row of `CHGTL_XY` |
//! | 44 | `OPN_BLAK` | unused                      | unused                           | opens sub-screen 2, ends event |
//! | 45 | `OPEN_SHP` | unused                      | unused                           | opens shop, ends event |
//! | 46 | `HIDE_NPC` | npc index                   | unused                           | |
//! | 47 | `SHOW_NPC` | npc index                   | unused                           | |
//! | 48 | `OPN_REFI` | unused                      | unused                           | opens refine, ends event |
//! | 49 | `EMO_HERO` | emotion id                  | unused                           | |
//! | 50 | `EMO_NPC`  | npc index                   | emotion id                       | |
//! | 99 | `END_EVNT` | unused                      | unused                           | runtime terminator |
//!
//! Opcodes `51..=98` are `null` in the game's own opcode table and are rejected
//! as unknown.
//!
//! ## Undetermined operand widths (documented assumptions)
//!
//! `SYSBGM` (28) and `SWI_MAP` (33) are named in the opcode table but have **no
//! handler** in `ah.m11a()`/`ah.d()` in this build, so their operand semantics
//! cannot be derived from the dispatch. Neither appears in any of the 210
//! baseline blobs. They are decoded like any other row (2 ignored operand bytes,
//! since the on-disk row width is always 3) and accepted as defined opcodes; the
//! `a`/`b` fields hold the raw bytes should a future build reveal their meaning.

use crate::{FormatError, Reader};

/// A defined EventScript opcode. Values match the game's opcode table (the
/// `static{}` name array in `ah`): `0..=50` plus `99`. Any other byte is
/// rejected by [`OpCode::from_u8`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(missing_docs)] // names are the game's own opcode mnemonics; see module table
pub enum OpCode {
    CmdIdle = 0,
    TalkText = 1,
    YesNo = 2,
    MvHMov = 3,
    MvDelay = 4,
    MvHStp = 5,
    MvHDir = 6,
    MapChng = 7,
    MapHero = 8,
    SwiDef = 9,
    Money = 10,
    Item = 11,
    Exp = 12,
    Hp = 13,
    Sp = 14,
    MvNMov = 15,
    MvNStp = 16,
    MvNDir = 17,
    Guardian = 18,
    Combo = 19,
    GameOver = 20,
    ItemNum = 21,
    ScrDel = 22,
    ScrShow = 23,
    ScrFlas = 24,
    ScrShak = 25,
    BgmPlay = 26,
    BgmStop = 27,
    SysBgm = 28,
    Sound = 29,
    SwiQue = 30,
    GotoFor = 31,
    GotoBak = 32,
    SwiMap = 33,
    MvFoHe = 34,
    MvFoNo = 35,
    MvFoNp = 36,
    MvCaMv = 37,
    MvCaSt = 38,
    MvCaXy = 39,
    ChgObj = 40,
    ChgNpc = 41,
    ChgtlXy = 42,
    ChgtlVa = 43,
    OpnBlak = 44,
    OpenShp = 45,
    HideNpc = 46,
    ShowNpc = 47,
    OpnRefi = 48,
    EmoHero = 49,
    EmoNpc = 50,
    EndEvnt = 99,
}

impl OpCode {
    /// Map an opcode byte to its [`OpCode`], or `None` if it is not a defined
    /// opcode (i.e. `51..=98`, or `>99`).
    pub fn from_u8(v: u8) -> Option<OpCode> {
        use OpCode::*;
        Some(match v {
            0 => CmdIdle,
            1 => TalkText,
            2 => YesNo,
            3 => MvHMov,
            4 => MvDelay,
            5 => MvHStp,
            6 => MvHDir,
            7 => MapChng,
            8 => MapHero,
            9 => SwiDef,
            10 => Money,
            11 => Item,
            12 => Exp,
            13 => Hp,
            14 => Sp,
            15 => MvNMov,
            16 => MvNStp,
            17 => MvNDir,
            18 => Guardian,
            19 => Combo,
            20 => GameOver,
            21 => ItemNum,
            22 => ScrDel,
            23 => ScrShow,
            24 => ScrFlas,
            25 => ScrShak,
            26 => BgmPlay,
            27 => BgmStop,
            28 => SysBgm,
            29 => Sound,
            30 => SwiQue,
            31 => GotoFor,
            32 => GotoBak,
            33 => SwiMap,
            34 => MvFoHe,
            35 => MvFoNo,
            36 => MvFoNp,
            37 => MvCaMv,
            38 => MvCaSt,
            39 => MvCaXy,
            40 => ChgObj,
            41 => ChgNpc,
            42 => ChgtlXy,
            43 => ChgtlVa,
            44 => OpnBlak,
            45 => OpenShp,
            46 => HideNpc,
            47 => ShowNpc,
            48 => OpnRefi,
            49 => EmoHero,
            50 => EmoNpc,
            99 => EndEvnt,
            _ => return None,
        })
    }

    /// The opcode's mnemonic as it appears in the game's opcode name table.
    pub fn name(self) -> &'static str {
        use OpCode::*;
        match self {
            CmdIdle => "CMD_IDLE",
            TalkText => "TALKTEXT",
            YesNo => "YES/NO",
            MvHMov => "MV_H_MOV",
            MvDelay => "MV_DELAY",
            MvHStp => "MV_H_STP",
            MvHDir => "MV_H_DIR",
            MapChng => "MAP_CHNG",
            MapHero => "MAP_HERO",
            SwiDef => "SWI_DEF",
            Money => "MONEY",
            Item => "ITEM",
            Exp => "EXP",
            Hp => "HP",
            Sp => "SP",
            MvNMov => "MV_N_MOV",
            MvNStp => "MV_N_STP",
            MvNDir => "MV_N_DIR",
            Guardian => "GUARDIAN",
            Combo => "COMBO",
            GameOver => "GAMEOVER",
            ItemNum => "ITEM_NUM",
            ScrDel => "SCR_DEL",
            ScrShow => "SCR_SHOW",
            ScrFlas => "SCR_FLAS",
            ScrShak => "SCR_SHAK",
            BgmPlay => "BGM_PLAY",
            BgmStop => "BGM_STOP",
            SysBgm => "SYSBGM",
            Sound => "SOUND",
            SwiQue => "SWI_QUE",
            GotoFor => "GOTO_FOR",
            GotoBak => "GOTO_BAK",
            SwiMap => "SWI_MAP",
            MvFoHe => "MV_FO_HE",
            MvFoNo => "MV_FO_NO",
            MvFoNp => "MV_FO_NP",
            MvCaMv => "MV_CA_MV",
            MvCaSt => "MV_CA_ST",
            MvCaXy => "MV_CA_XY",
            ChgObj => "CHG_OBJ",
            ChgNpc => "CHG_NPC",
            ChgtlXy => "CHGTL_XY",
            ChgtlVa => "CHGTL_VA",
            OpnBlak => "OPN_BLAK",
            OpenShp => "OPEN_SHP",
            HideNpc => "HIDE_NPC",
            ShowNpc => "SHOW_NPC",
            OpnRefi => "OPN_REFI",
            EmoHero => "EMO_HERO",
            EmoNpc => "EMO_NPC",
            EndEvnt => "END_EVNT",
        }
    }
}

/// One decoded event-script instruction: a fixed 3-byte row `[opcode][a][b]`.
///
/// `a` and `b` are the two operand bytes, kept **signed** (`i8`) because the VM
/// reads them as signed (movement deltas, face-side sign, jump offsets, etc.).
/// See the module-level opcode→operand-shape table for the per-opcode meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    /// The decoded opcode.
    pub op: OpCode,
    /// First operand byte (signed).
    pub a: i8,
    /// Second operand byte (signed).
    pub b: i8,
}

impl Instruction {
    /// The two operand bytes read as an unsigned big-endian 16-bit word
    /// (`a` high, `b` low) — the runtime interpretation the VM uses for the
    /// `MONEY`/`EXP`/`HP`/`SP` opcodes.
    pub fn word_be(self) -> u16 {
        ((self.a as u8 as u16) << 8) | (self.b as u8 as u16)
    }
}

/// One event script: a length-prefixed sequence of [`Instruction`] rows
/// (`f24b[i]`), executed by the EventScript VM starting at its first row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The decoded instruction rows, in file order.
    pub instructions: Vec<Instruction>,
}

/// A fully decoded `.evt` container for one map. Every section is captured so a
/// clean parse proves the whole blob was consumed to EOF; the [`scripts`] field
/// is the decoded EventScript bytecode.
///
/// [`scripts`]: Evt::scripts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evt {
    /// Map width in tiles (from the paired `.map`; = collision-grid columns).
    pub width: u8,
    /// Map height in tiles (from the paired `.map`; = collision-grid rows).
    pub height: u8,
    /// Section 1: the `width * height` collision/attribute grid (row-major).
    pub collision: Vec<u8>,
    /// Section 2: object tile-image ids referenced by the objects below.
    pub object_images: Vec<u8>,
    /// Section 2: object placements, each a raw 5-byte record
    /// `[tileX][tileY][b0][b1][imageIndex]`.
    pub objects: Vec<[u8; 5]>,
    /// Section 3: npc sprite ids loaded for this map.
    pub npc_ids: Vec<u8>,
    /// Section 3: npc placements, each a raw 3-byte record `[tileX][tileY][id]`.
    pub npcs: Vec<[u8; 3]>,
    /// Section 4: enemy type ids loaded for this map (at most 5).
    pub enemy_ids: Vec<u8>,
    /// Section 4: enemy spawns, each a raw 3-byte record.
    pub enemies: Vec<[u8; 3]>,
    /// Section 5: face image ids loaded for dialogue portraits.
    pub faces: Vec<u8>,
    /// Section 6: triggers (`f23a`). Outer index = trigger id; inner = its
    /// condition/entry rows (each a raw 7-byte record). An empty inner vec is an
    /// unused trigger slot.
    pub triggers: Vec<Vec<[u8; 7]>>,
    /// Section 6: the decoded EventScript bodies (`f24b`).
    pub scripts: Vec<Script>,
    /// Section 6: dialogue strings (`f25c`), each raw Latin-1 bytes.
    pub dialogue: Vec<Vec<u8>>,
    /// Section 7: initial-patch condition rows (each a raw 3-byte record).
    pub patch_conditions: Vec<[u8; 3]>,
    /// Section 7: initial-patch groups; each group is a list of raw 4-byte patch
    /// records (`[op 100–112][x][y][value]`).
    pub patch_groups: Vec<Vec<[u8; 4]>>,
}

/// Parse an `.evt` container. `width`/`height` are the paired `/m/NN.map`'s tile
/// dimensions (the collision grid's size, which is not stored in the `.evt`
/// itself). See [module docs](self) for the full layout.
///
/// Returns [`Err`] — never panics — on empty input, any section reading past the
/// end (truncation / operand underrun), an unknown script opcode, or trailing
/// bytes after the final section.
pub fn parse(input: &[u8], width: u8, height: u8) -> Result<Evt, FormatError> {
    if input.is_empty() {
        return Err(FormatError::Empty);
    }
    let mut r = Reader::new(input);

    // ---- Section 1: collision grid (width * height bytes) ----
    let area = (width as usize) * (height as usize);
    let collision = r.take(area, "evt collision grid")?.to_vec();

    // ---- Section 2: objects ----
    let object_images = read_u8_list(&mut r, "evt object image ids")?;
    let objects = read_fixed_records::<5>(&mut r, "evt object record")?;

    // ---- Section 3: npcs ----
    let npc_ids = read_u8_list(&mut r, "evt npc ids")?;
    let npcs = read_fixed_records::<3>(&mut r, "evt npc record")?;

    // ---- Section 4: enemies ----
    let enemy_ids = read_u8_list(&mut r, "evt enemy ids")?;
    let enemies = read_fixed_records::<3>(&mut r, "evt enemy record")?;

    // ---- Section 5: faces (signed count) ----
    let n_faces = signed_count(r.u8("evt face count")?);
    let faces = r.take(n_faces, "evt face ids")?.to_vec();

    // ---- Section 6: triggers, scripts, dialogue ----
    let n_trig = r.u8("evt trigger count")? as usize;
    let mut triggers = Vec::with_capacity(n_trig);
    for _ in 0..n_trig {
        triggers.push(read_fixed_records::<7>(&mut r, "evt trigger entry")?);
    }

    let n_script = r.u8("evt script count")? as usize;
    let mut scripts = Vec::with_capacity(n_script);
    for _ in 0..n_script {
        let n_instr = r.u8("evt script length")? as usize;
        let mut instructions = Vec::with_capacity(n_instr);
        for _ in 0..n_instr {
            let row = r.take(3, "evt instruction")?;
            let op = OpCode::from_u8(row[0]).ok_or(FormatError::BadField {
                what: "evt opcode (not in the game's opcode table)",
            })?;
            instructions.push(Instruction {
                op,
                a: row[1] as i8,
                b: row[2] as i8,
            });
        }
        scripts.push(Script { instructions });
    }

    let n_dialogue = r.u8("evt dialogue count")? as usize;
    let mut dialogue = Vec::with_capacity(n_dialogue);
    for _ in 0..n_dialogue {
        let len = r.u8("evt dialogue length")? as usize;
        dialogue.push(r.take(len, "evt dialogue text")?.to_vec());
    }

    // ---- Section 7: initial patches (signed counts) ----
    let patch_conditions = read_signed_records::<3>(&mut r, "evt patch condition")?;
    let n_groups = signed_count(r.u8("evt patch group count")?);
    let mut patch_groups = Vec::with_capacity(n_groups);
    for _ in 0..n_groups {
        patch_groups.push(read_signed_records::<4>(&mut r, "evt patch record")?);
    }

    // ---- Whole file must be consumed exactly ----
    if !r.is_empty() {
        return Err(FormatError::Inconsistent {
            what: "evt length: trailing bytes after the final section",
        });
    }

    Ok(Evt {
        width,
        height,
        collision,
        object_images,
        objects,
        npc_ids,
        npcs,
        enemy_ids,
        enemies,
        faces,
        triggers,
        scripts,
        dialogue,
        patch_conditions,
        patch_groups,
    })
}

/// Read a `[count: u8]` prefix followed by `count` raw bytes.
fn read_u8_list(r: &mut Reader<'_>, what: &'static str) -> Result<Vec<u8>, FormatError> {
    let n = r.u8(what)? as usize;
    Ok(r.take(n, what)?.to_vec())
}

/// Read a `[count: u8]` prefix followed by `count` fixed-size `N`-byte records.
fn read_fixed_records<const N: usize>(
    r: &mut Reader<'_>,
    what: &'static str,
) -> Result<Vec<[u8; N]>, FormatError> {
    let n = r.u8(what)? as usize;
    read_n_records::<N>(r, n, what)
}

/// Read a `[count: i8]` prefix (a signed loop bound; negative ⇒ 0) followed by
/// `count` fixed-size `N`-byte records.
fn read_signed_records<const N: usize>(
    r: &mut Reader<'_>,
    what: &'static str,
) -> Result<Vec<[u8; N]>, FormatError> {
    let n = signed_count(r.u8(what)?);
    read_n_records::<N>(r, n, what)
}

/// Read exactly `n` fixed-size `N`-byte records.
fn read_n_records<const N: usize>(
    r: &mut Reader<'_>,
    n: usize,
    what: &'static str,
) -> Result<Vec<[u8; N]>, FormatError> {
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let slice = r.take(N, what)?;
        let mut rec = [0u8; N];
        rec.copy_from_slice(slice);
        out.push(rec);
    }
    Ok(out)
}

/// Interpret a count byte the way the VM's signed loop bounds do: a value with
/// the high bit set (`>= 128`) is negative, so the loop runs zero times.
fn signed_count(b: u8) -> usize {
    let v = b as i8;
    if v < 0 {
        0
    } else {
        v as usize
    }
}
