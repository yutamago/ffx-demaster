//! The Status screen as mod/StatusMenuPatches.cs draws it (ui.menus = ps2): the main window
//! (PS2 0x0022ad00 for MenuStatus_DrawMainWindow 0x008d2710: face, equipment, Overdrive gauge,
//! party plate, stats, the "show abilities" bar, the status background) and the ability pages
//! (PS2 0x0022b5a0 for MenuStatus_WindowDraw_008d2790). The same arithmetic as the C#; the game's
//! lookups are called at the frame boundary. Evidence in docs/ui.md ("The Status screen").
use crate::menu::{Ctx, Fonts, List, f, font_s, value_m, value_s};
use crate::screens::{Draw, Env};

const BOX_C2: u32 = 0x008f3db0;
const PLATE_LR: u32 = 0x008f5e90;
const SHAPE_PART: u32 = 0x009034b0;
const SHAPE_UV: u32 = 0x00903ee0;
const SHAPE_XYC2: u32 = 0x00903a50;
const SHAPE_XYWH: u32 = 0x00903b70;
const FONT_1612: u32 = 0x00901cb0;
const ICON: u32 = 0x008e6af0;
const SCISSOR: u32 = 0x008fdee0;
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const GREY: u32 = 0x8080_8080;
const SHADOW: u32 = 0x8000_0000;

const CURRENT: u32 = 0x008a9860;
const CURRENT_AEON: u32 = 0x008a9880;
const EASE: u32 = 0x008d30c0;
const STATUS_BG: u32 = 0x008d4910;
const TRANS_FACE: u32 = 0x008c2ce0;
const FACE_SLIDE: u32 = 0x008d32b0;
const STATUS_MES: u32 = 0x008bee70;
const MENU1_MES: u32 = 0x008ddfc0;
const OD_ID: u32 = 0x008c1bd0;
const LIMIT: u32 = 0x008a9930;
const LIMIT_MAX: u32 = 0x008a9950;
const WEAPON: u32 = 0x008a9c70;
const ARMOR: u32 = 0x008a9820;
const SAVE_WEAPON: u32 = 0x007abbd0;
const WEAPON_NAME: u32 = 0x007abdf0;
const PLAYER_PTR: u32 = 0x00785330;
const DISP_COMMAND: u32 = 0x008d2e10;
const ROM_ABILITY: u32 = 0x00790970;
const COM_DATA: u32 = 0x0079a4b0;
const COMMAND_NAME: u32 = 0x008c1a20;
const CHR_NAME: u32 = 0x008ac850;
const DAMAGE_CLASS: u32 = 0x008a99f0;
const STRTEX_CELL: u32 = 0x008f8d50;

const RECT_3NAMEPLT: u32 = 0x0186a83c;
const POS_WPNMOJI: u32 = 0x0186a86c;
const POS_WPNICON: u32 = 0x0186a870;
const POS_WPNNM: u32 = 0x0186a874;
const POS_LTSTR: u32 = 0x0186a878;
const POS_LPMPLT: u32 = 0x0186a87c;
const POS_RPMPLT: u32 = 0x0186a880;
const NUM_PMPLT: u32 = 0x0186a884;
const PL_LGPLT: u32 = 0x0186a888;
const PL_3LGPLT: u32 = 0x0186a8a0;
const PL_LTPLT: u32 = 0x0186a8b8;
const PL_LTBPLT: u32 = 0x0186a8d0;
const PL_WPNBPLT: u32 = 0x0186a8e8;
const PL_WPNLPLT: u32 = 0x0186a900;
const PL_WPNRPLT: u32 = 0x0186a918;
const PL_WPNBPLT2: u32 = 0x0186a930;
const PL_WPNLPLT2: u32 = 0x0186a948;
const PL_WPNRPLT2: u32 = 0x0186a960;
const PL_WPNBLINE: u32 = 0x0186a978;
const PL_WPNBLINE2: u32 = 0x0186a990;
const POS_TYPSHAPE: u32 = 0x0186a9a8;
const POS_LGSHAPE: u32 = 0x0186a9b0;
const POS_3LGSHAPE: u32 = 0x0186a9b4;
const COL_LMTGF0: u32 = 0x0186a9bc;
const RECT_LG: u32 = 0x0186a9c0;
const COL_LMTG0: u32 = 0x0186a9cc;
const COL_LMTG1: u32 = 0x0186a9d0;
const RECT_SMOJIPLT: u32 = 0x0186a9d8;
const POS_LSTAPLT: u32 = 0x0186a9e0;
const POS_RSTAPLT: u32 = 0x0186a9e4;
const NUM_STAPLT: u32 = 0x0186a9e8;
const NUM_FACE1: u32 = 0x0186a9ec;
const NUM_FACE2: u32 = 0x0186a9f0;
const POS_FACE: u32 = 0x0186a9f4;
const PL_STSMW: u32 = 0x023cc400;
const POS_EXTEX: u32 = 0x023cc3e8;
const POS_EXMSSG: u32 = 0x023cc3e4;
const COL_MENUBG1: u32 = 0x023cc418;
const COL_MENUBG0: u32 = 0x023cc41c;
const POS_PMSHAPE: u32 = 0x023cc420;
const FACE_T: u32 = 0x023cc3c8;
const FACE_CUR: u32 = 0x023cc3c4;
const FACE_NEXT: u32 = 0x023cc3c0;
const STATS: u32 = 0x00c5a790;
const MESWIN_LAYOUT: u32 = 0x00c5b460;
const NAMEPLT_W: u32 = 0x025d1672;
const COL_NAMEPLT0: u32 = 0x025d166c;
const COL_NAMEPLT1: u32 = 0x025d1668;
const POS_NAMESTR: u32 = 0x025d1662;
const POS_HPTEX: u32 = 0x025d165e;
const POS_HPSTR: u32 = 0x025d165a;
const POS_MPTEX: u32 = 0x025d1656;
const POS_MPSTR: u32 = 0x025d1652;
const POS_SLVSHAPE: u32 = 0x025d164e;
const POS_SPSTR: u32 = 0x025d164a;
const NOACTIVE_H: u32 = 0x023cc13e;
const POS_NOACTIVE: u32 = 0x023cc144;
const COL_NOACTIVE0: u32 = 0x023cc148;
const COL_NOACTIVE1: u32 = 0x023cc140;
const PARTY_MASK: u32 = 0x01841c2c;
const COL_BSOPLT0: u32 = 0x025d16a0;

const MAIN_WINDOW: u32 = 0x0186a838; // MenuStatus_OpenMainWindow's window
const SUB_WINDOWS: u32 = 0x0186a820; // g_TkMenuStatusWindows[0..6]

type R<T> = ffx_harness::Result<T>;

#[derive(Clone, Copy)]
struct Plate {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    c0: u32,
    c1: u32,
    a: u32,
    b: u32,
}

struct Cx<'a, 'b> {
    cx: &'a mut Ctx<'b>,
    out: u32,
    fo: Fonts,
}

fn q12(v: i32) -> i32 {
    v / 4096
}

/// MenuHelp_Ratio (PS2 0x00224958).
fn ratio(a: i32, b: i32) -> i32 {
    2 * a * (b + 2) / 4
}

impl Cx<'_, '_> {
    fn call(&mut self, va: u32, args: &[u32]) -> R<u32> {
        self.cx.call(va, args)
    }
    fn i16(&mut self, va: u32) -> R<i32> {
        self.cx.r16(va)
    }
    fn u32(&mut self, va: u32) -> R<u32> {
        self.cx.r32(va)
    }
    fn pos(&mut self, va: u32) -> R<(i32, i32)> {
        Ok((self.cx.r16(va)?, self.cx.r16(va + 2)?))
    }
    /// TOSeqInitParam's mirrored point (y, x).
    fn point(&mut self, va: u32) -> R<(i32, i32)> {
        Ok((self.cx.r16(va + 2)?, self.cx.r16(va)?))
    }
    fn plate(&mut self, va: u32, dx: i32, dy: i32) -> R<Plate> {
        let b = self.cx.game.read(va, 24)?;
        let h = |o: usize| i32::from(i16::from_le_bytes([b[o], b[o + 1]]));
        let w = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        Ok(Plate { x: h(0) + dx, y: h(2) + dy, w: h(4), h: h(6), c0: w(8), c1: w(12), a: w(16), b: w(20) })
    }
    fn abs_bytes(&mut self, abs: u32, n: usize) -> R<Vec<u8>> {
        self.cx.game.read(abs.wrapping_sub(self.cx.delta), n)
    }
    fn dims(&mut self, id: u32) -> R<(i32, i32)> {
        let i = if id >= 200 { id - 200 } else { id };
        let e = MESWIN_LAYOUT + self.u32(MESWIN_LAYOUT + 4 + 4 * i)?;
        Ok((self.i16(e + 8)? & 0xffff, self.i16(e + 10)? & 0xffff))
    }
    /// TOMkpShapeXY as the C# draws it: part by part with TOMkpShapePartIndexXYWHC2, a run of one
    /// cell tiled side by side as one quad over the cell's inner columns.
    /// TOMkpShapeXY (grey): main.rs's expand_shapes draws it part by part, as mod Ps2Menu.shape.
    fn shape(&mut self, l: &mut List, id: u32, x: i32, y: i32) -> R<()> {
        l.c(0x0090_3a10, &[id, x as u32, y as u32]);
        Ok(())
    }
    /// (record, name pointer) of MsGetRomAbility / MsGetComData style getters (id, &strings).
    fn named(&mut self, getter: u32, id: u32) -> R<(Vec<u8>, u32)> {
        let rec = self.call(getter, &[id, self.out])?;
        let strings = u32::from_le_bytes(self.abs_bytes(self.out, 4)?[..4].try_into().unwrap());
        let r = self.abs_bytes(rec, 0x70)?;
        let name = strings + u32::from(u16::from_le_bytes([r[0], r[1]]));
        Ok((r, name))
    }
}

fn shape_wh(l: &mut List, id: u32, x: i32, y: i32, w: i32, h: i32, c0: u32, c1: u32) {
    l.c(SHAPE_XYWH, &[id, x as u32, y as u32, w as u32, h as u32, c0, c1]);
}
fn font(l: &mut List, text: u32, x: i32, y: i32, colour: u32) {
    l.c(FONT_1612, &[text, x as u32, y as u32, colour]);
}
fn icon(l: &mut List, x: i32, y: i32, id: u32) {
    l.c(ICON, &[f(x as f32), f(y as f32), f(15.0), f(18.0), id, 0x80, 0x80, 0x80, 0x80]);
}
fn bx(l: &mut List, x: i32, y: i32, w: i32, h: i32, c0: u32, c1: u32) {
    l.c(BOX_C2, &[x as u32, y as u32, w as u32, h as u32, c0, c1]);
}

fn plate_l(cx: &mut Cx, l: &mut List, a: u32, b: u32, x: i32, y: i32, w: i32, c0: u32, c1: u32) -> R<()> {
    let wa = cx.dims(a)?.0;
    shape_wh(l, a, x, y, 0, 0, c0, c0);
    shape_wh(l, b, x + wa, y, w - wa, 0, c0, c1);
    Ok(())
}
fn plate_lp(cx: &mut Cx, l: &mut List, p: Plate) -> R<()> {
    plate_l(cx, l, p.a, p.b, p.x, p.y, p.w, p.c0, p.c1)
}
fn plate_r(cx: &mut Cx, l: &mut List, p: Plate) -> R<()> {
    let wb = cx.dims(p.b)?.0;
    shape_wh(l, p.a, p.x, p.y, p.w - wb, 0, p.c0, p.c1);
    shape_wh(l, p.b, p.x + p.w - wb, p.y, 0, 0, p.c1, p.c1);
    Ok(())
}
fn window(cx: &mut Cx, l: &mut List, p: Plate) -> R<()> {
    let (wa, ha) = cx.dims(0xc8)?;
    shape_wh(l, 0xc8, p.x, p.y, 0, 0, p.c0, p.c0);
    shape_wh(l, 0xc9, p.x + wa, p.y, p.w - wa, 0, p.c0, p.c1);
    shape_wh(l, 0xca, p.x, p.y + ha, 0, p.h - ha, p.c0, p.c0);
    shape_wh(l, 0xcb, p.x + wa, p.y + ha, p.w - wa, p.h - ha, p.c0, p.c1);
    Ok(())
}
fn boxp(l: &mut List, p: Plate) {
    bx(l, p.x, p.y, p.w, p.h, p.c0, p.c1);
}
fn kind_plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, kind: i32) -> R<()> {
    let at = COL_BSOPLT0 - 8 * ((kind >> 3) & 3) as u32;
    let (c0, c1) = (cx.u32(at)?, cx.u32(at - 4)?);
    shape_wh(l, 0xda, x, y, 0, 0, c0, c0);
    shape_wh(l, 0xdb, x + 16, y, 209, 0, c0, c1);
    let extra = match kind & 3 {
        1 => 0xd7,
        2 => 0x10b,
        3 => 0x10a,
        _ => 0,
    };
    if extra != 0 {
        l.c(SHAPE_XYC2, &[extra, (x + 97) as u32, y as u32, SHADOW, SHADOW]);
    }
    Ok(())
}

fn gear(cx: &mut Cx, l: &mut List, id: u32, x: i32, y: i32) -> R<()> {
    let rec = cx.call(SAVE_WEAPON, &[id, cx.out])?;
    let r = cx.abs_bytes(rec, 8)?;
    let name = cx.call(WEAPON_NAME, &[id])?;
    icon(l, x + 12, y + 1, (u32::from(r[4]) * 2 + 1 + u32::from(r[5])) & 0xff);
    font(l, name, x + 29, y + 1, 0);
    if r[4] == r[6] {
        icon(l, x + 12, y + 1, 0x31);
    }
    eprintln!("gear {id:#x}: owner {} armour {} byte6 {}", r[4], r[5], r[6]);
    Ok(())
}

fn equipment(cx: &mut Cx, l: &mut List, chr: u32, dx: i32) -> R<()> {
    let p = cx.plate(PL_WPNBPLT, dx, 0)?;
    window(cx, l, p)?;
    let l1 = cx.plate(PL_WPNLPLT, dx, 0)?;
    plate_lp(cx, l, l1)?;
    let r1 = cx.plate(PL_WPNRPLT, dx, 0)?;
    boxp(l, r1);
    let p = cx.plate(PL_WPNBPLT2, dx, 0)?;
    window(cx, l, p)?;
    let l2 = cx.plate(PL_WPNLPLT2, dx, 0)?;
    plate_lp(cx, l, l2)?;
    let r2 = cx.plate(PL_WPNRPLT2, dx, 0)?;
    boxp(l, r2);
    let p = cx.plate(PL_WPNBLINE, dx, 0)?;
    plate_r(cx, l, p)?;
    let p = cx.plate(PL_WPNBLINE2, dx, 0)?;
    plate_r(cx, l, p)?;
    cx.shape(l, 0xd7, r1.x + 0x58, r1.y)?;
    cx.shape(l, 0xd7, r2.x + 0x58, r2.y)?;
    let moji = cx.pos(POS_WPNMOJI)?;
    let t = cx.call(STATUS_MES, &[5])?;
    font(l, t, l1.x + moji.0, l1.y + moji.1, 0);
    let t = cx.call(STATUS_MES, &[6])?;
    font(l, t, l2.x + moji.0, l2.y + moji.1, 0);
    let ic = cx.pos(POS_WPNICON)?;
    let wpn = cx.call(WEAPON, &[chr])?;
    gear(cx, l, wpn, r1.x + ic.0, r1.y + ic.1)?;
    let armor = cx.call(ARMOR, &[chr])?;
    if armor != 0xff {
        gear(cx, l, armor, r2.x + ic.0, r2.y + ic.1)?;
    } else {
        let nm = cx.pos(POS_WPNNM)?;
        let t = cx.call(STATUS_MES, &[0x17])?;
        font(l, t, r2.x + nm.0, r2.y + nm.1, 0);
    }
    eprintln!(
        "wpn plates b {:?} l {:?} r {:?} moji {moji:?} icon {ic:?}",
        (cx.plate(PL_WPNBPLT, 0, 0)?.x, cx.plate(PL_WPNBPLT, 0, 0)?.y),
        (l1.x, l1.y, l1.w, l1.a, l1.b),
        (r1.x, r1.y, r1.w, r1.h)
    );
    Ok(())
}

fn blend(a: u32, b: u32, r: i32) -> u32 {
    let mut c = 0;
    for k in (0..32).step_by(8) {
        let (ca, cb) = (((a >> k) & 0xff) as i32, ((b >> k) & 0xff) as i32);
        c |= ((q12(ca * (0x1000 - r) + cb * r) & 0xff) as u32) << k;
    }
    c
}

fn gauge(cx: &mut Cx, l: &mut List, x: i32, y: i32, v: i32, max: i32) -> R<()> {
    if max == 0 || v == 0 {
        return Ok(());
    }
    let r = if v >= max { 0x1000 } else { (v << 12) / max };
    let (w, h) = (q12(cx.i16(RECT_LG + 4)? * r), cx.i16(RECT_LG + 6)?);
    let (c0, c1) = if r == 0x1000 {
        let c = cx.u32(COL_LMTGF0)?;
        (c, c)
    } else {
        let c0 = cx.u32(COL_LMTG0)?;
        (c0, blend(c0, cx.u32(COL_LMTG1)?, r))
    };
    bx(l, x, y, w, h, c0, c1);
    Ok(())
}

fn overdrive(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: u32) -> R<()> {
    let lg = cx.plate(PL_LGPLT, 0, 0)?;
    let (dx, dy) = (x - lg.x, y - lg.y);
    let mut fp = 0;
    if chr < 8 {
        let p = cx.plate(PL_LTBPLT, dx, dy)?;
        boxp(l, p);
        let p = cx.plate(PL_LTPLT, dx, dy)?;
        boxp(l, p);
        let p = cx.plate(PL_LGPLT, dx, dy)?;
        plate_lp(cx, l, p)?;
        let lt = cx.plate(PL_LTPLT, 0, 0)?;
        let typ = cx.pos(POS_TYPSHAPE)?;
        cx.shape(l, 0x109, lt.x + typ.0 + dx, lt.y + typ.1 + dy)?;
        let sh = cx.pos(POS_LGSHAPE)?;
        cx.shape(l, 0x108, lg.x + sh.0 + dx + 0x10, lg.y + sh.1 + dy)?;
    } else {
        fp = 0xf;
        let big = cx.plate(PL_3LGPLT, 0, 0)?;
        let p = cx.plate(PL_LGPLT, dx, dy)?;
        plate_l(cx, l, p.a, p.b, p.x, p.y, big.w, big.c1, p.c1)?;
        let sh = cx.pos(POS_3LGSHAPE)?;
        cx.shape(l, 0x108, lg.x + sh.0 + dx + 0x10, lg.y + sh.1 + dy)?;
    }
    let rl = cx.pos(RECT_LG)?;
    let v = cx.call(LIMIT, &[chr])? as i32;
    let max = cx.call(LIMIT_MAX, &[chr])? as i32;
    eprintln!("overdrive at {x} {y}: lgplt {:?} rect_lg {rl:?} limit {v}/{max}", (lg.x, lg.y, lg.w, lg.a, lg.b));
    gauge(cx, l, rl.0 + dx + fp, rl.1 + dy, v, max)?;
    if chr < 8 {
        let ltb = cx.plate(PL_LTBPLT, 0, 0)?;
        let s = cx.pos(POS_LTSTR)?;
        let id = cx.call(OD_ID, &[chr])?;
        let t = cx.call(MENU1_MES, &[id + 0x1033])?;
        font(l, t, ltb.x + s.0 + dx, ltb.y + s.1 + dy, 0);
    }
    Ok(())
}

fn party_row(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: u32) -> R<()> {
    let fo = cx.fo;
    if cx.u32(PARTY_MASK)? & (1 << chr) != 0 {
        let (nx, ny, nh) = (cx.i16(POS_NOACTIVE)?, cx.i16(POS_NOACTIVE + 2)?, cx.i16(NOACTIVE_H)?);
        let (c0, c1) = (cx.u32(COL_NOACTIVE0)?, cx.u32(COL_NOACTIVE1)?);
        bx(l, x + nx, y + ny, 512, nh, c0, c1);
    }
    let hptex = cx.point(POS_HPTEX)?;
    let mptex = cx.point(POS_MPTEX)?;
    let slv = cx.point(POS_SLVSHAPE)?;
    let nw = cx.i16(NAMEPLT_W)?;
    let (c0, c1) = (cx.u32(COL_NAMEPLT0)?, cx.u32(COL_NAMEPLT1)?);
    if chr < 8 {
        l.c(PLATE_LR, &[4, 5, x as u32, y as u32, nw as u32, c0, c1]);
        let namestr = cx.point(POS_NAMESTR)?;
        let name = cx.call(CHR_NAME, &[chr])?;
        let colour = cx.call(DAMAGE_CLASS, &[chr])?;
        font(l, name, x + namestr.0, y + namestr.1, colour);
        let hpstr = cx.point(POS_HPSTR)?;
        let mpstr = cx.point(POS_MPSTR)?;
        let (hp, hpm) = (cx.call(0x008a_98c0, &[chr])?, cx.call(0x008a_9990, &[chr])?);
        let (mp, mpm) = (cx.call(0x008a_9970, &[chr])?, cx.call(0x008a_99b0, &[chr])?);
        cx.shape(l, 0x21, x + hptex.0, y + hptex.1)?;
        font_s(l, &fo, &format!("{hp:5}/{hpm:5}"), x + hpstr.0, y + hpstr.1, 0x25);
        cx.shape(l, 0x22, x + mptex.0, y + mptex.1)?;
        font_s(l, &fo, &format!("{mp:4}/{mpm:4}"), x + mpstr.0, y + mpstr.1, 0x25);
        shape_wh(l, 0xe7, 0, y, 512, 0, GREY, GREY);
        cx.shape(l, 0x107, x + slv.0 + 0x68, y + slv.1)?;
        let spstr = cx.point(POS_SPSTR)?;
        let sp = cx.call(0x008a_9b80, &[chr])?;
        value_m(l, &fo, i64::from(sp), x + slv.0 + spstr.0, y + slv.1 + spstr.1, 0);
        let ap = cx.call(0x0078_4e90, &[chr])?.wrapping_sub(cx.call(0x0078_52b0, &[chr])?);
        value_s(l, &fo, i64::from(ap as i32), x + slv.0 + 0x8b, y + slv.1 - 13, 0x25);
    } else {
        l.c(PLATE_LR, &[0xcc, 0xcd, x as u32, y as u32, nw as u32, c0, c1]);
        cx.shape(l, 0xe9, x + hptex.0, y + hptex.1)?;
        cx.shape(l, 0xea, x + mptex.0, y + mptex.1)?;
        shape_wh(l, 0xe7, 0, y, 512, 0, GREY, GREY);
        cx.shape(l, 0x107, x + slv.0 + 0x68, y + slv.1)?;
    }
    Ok(())
}

fn page0(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: u32, t: i32) -> R<()> {
    let s = q12(cx.call(EASE, &[t as u32, 1])? as i32 * 0x155);
    l.c(TRANS_FACE, &[0]);
    let face = cx.pos(POS_FACE)?;
    let (fx, fy) = (x + face.0, y + face.1);
    let (fw, fh) = (cx.u32(NUM_FACE1)? as i32, cx.u32(NUM_FACE2)? as i32);
    let (ft, fc, fnx) = (cx.u32(FACE_T)?, cx.u32(FACE_CUR)?, cx.u32(FACE_NEXT)?);
    eprintln!("face at {fx} {fy} scissor {fw} x {fh}, t {ft} cur {fc} next {fnx}; slide {s}");
    l.c(SCISSOR, &[fx as u32, fy as u32, (fx + fw) as u32, (fy + fh) as u32]);
    l.c(FACE_SLIDE, &[f(fx as f32), f(fy as f32), f(91.0), f(98.0), ft, fc, fnx]);
    l.c(SCISSOR, &[0, 0, 512, 416]);
    equipment(cx, l, chr, s)?;
    let lg = cx.plate(PL_LGPLT, 0, 0)?;
    overdrive(cx, l, s + lg.x, lg.y, chr)?;
    let np = cx.pos(RECT_3NAMEPLT)?;
    eprintln!("rect_3nameplt {np:?}");
    party_row(cx, l, np.0, np.1, chr)
}

fn stats(cx: &mut Cx, l: &mut List, y: i32, chr: u32, t: i32) -> R<()> {
    let fo = cx.fo;
    let slide = q12(cx.call(EASE, &[t as u32, 1])? as i32 * 0xd0);
    let (mut sx, mut sy) = cx.pos(POS_LPMPLT)?;
    let pm = cx.pos(POS_PMSHAPE)?;
    let p = cx.call(PLAYER_PTR, &[chr])?;
    let rec = cx.abs_bytes(p, 0x94)?;
    let step = cx.u32(NUM_PMPLT)? as i32;
    eprintln!("stats from {sx} {sy} step {step} pmshape {pm:?} slide {slide}");
    let mut i = 0;
    loop {
        let (mes, field) = (cx.u32(STATS + 8 * i)? as i32, cx.u32(STATS + 8 * i + 4)?);
        if mes < 0 {
            break;
        }
        if i == 4 {
            (sx, sy) = cx.pos(POS_RPMPLT)?;
        }
        let py = y + sy + ratio(slide, (i % 4) as i32);
        kind_plate(cx, l, sx, py, 0x10)?;
        cx.shape(l, 0x10a, sx + pm.0 + 0x58, py + pm.1)?;
        let t = cx.call(STATUS_MES, &[mes as u32])?;
        font(l, t, sx + 0x1d, py + 1, 0);
        eprintln!("stat {i}: mes {mes:#x} field {field:#x} = {}", rec[field as usize]);
        value_m(l, &fo, i64::from(rec[field as usize]), sx + 0xd3, py + 6, 0);
        sy += step;
        i += 1;
    }
    if !(0xe..0x12).contains(&chr) {
        let mw = cx.plate(PL_STSMW, 0, y)?;
        window(cx, l, mw)?;
        let ex = cx.pos(POS_EXTEX)?;
        cx.shape(l, 0xf3, mw.x + ex.0, mw.y + ex.1)?;
        let em = cx.pos(POS_EXMSSG)?;
        let t = cx.call(STATUS_MES, &[0x1a])?;
        font(l, t, mw.x + em.0, mw.y + em.1, 0);
        eprintln!("stsmw {:?} extex {ex:?} exmssg {em:?}", (mw.x, mw.y, mw.w, mw.h));
    }
    Ok(())
}

fn status_bg(cx: &mut Cx, l: &mut List, bg: i32) -> R<()> {
    let a = ((bg >> 5) as u32) << 24;
    if a == 0 {
        return Ok(());
    }
    let c0 = (cx.u32(COL_MENUBG0)? & 0xffffff) | a;
    let c1 = (cx.u32(COL_MENUBG1)? & 0xffffff) | a;
    let y = cx.i16(POS_FACE + 2)?;
    bx(l, 0, y, 0x200, 0x1a0 - y, c0, c1);
    Ok(())
}

fn main_window(cx: &mut Cx, l: &mut List, w: &[u8]) -> R<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let chr = cx.call(CURRENT, &[])?;
    let t = i32::from(i16::from_le_bytes([w[0x52], w[0x53]]));
    page0(cx, l, 0, 0, chr, t)?;
    stats(cx, l, 0, chr, t)?;
    let bg = cx.call(STATUS_BG, &[])? as i32;
    eprintln!("main window: chr {chr} t {t:#x} status bg {bg:#x}");
    status_bg(cx, l, bg)?;
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

fn sub_window(cx: &mut Cx, l: &mut List, w: &[u8]) -> R<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let wi = |o: usize| u32::from_le_bytes(w[o..o + 4].try_into().unwrap());
    let wh = |o: usize| i32::from(i16::from_le_bytes([w[o], w[o + 1]]));
    let (sub, c, kind, aeon) = (wi(0x58), wi(0x5c), wi(0x60), wi(0x64) != 0);
    let chr = cx.call(if aeon { CURRENT_AEON } else { CURRENT }, &[])?;
    let mut ty = cx.i16(RECT_SMOJIPLT + 2)?;
    let mut ry = cx.i16(POS_RSTAPLT + 2)?;
    let mut step = cx.u32(NUM_STAPLT)? as i32;
    if aeon {
        ty += 0xb;
        ry += 0xb;
        step -= 1;
    }
    let n = cx.call(DISP_COMMAND, &[kind, sub, c, chr, cx.out])? as i32;
    let lb = cx.abs_bytes(cx.out, 88)?;
    let list: Vec<u32> = (0..22).map(|k| u32::from_le_bytes(lb[4 * k..4 * k + 4].try_into().unwrap())).collect();
    let e = cx.call(EASE, &[wh(0x52) as u32, wh(0x54) as u32])? as i32;
    let (mut dx, mut dy) = (0, 0);
    if wh(0x54) > 0 {
        dy = q12(e * 0xd0);
    } else {
        dx = e / 16;
    }
    let alpha = wh(0x52) >> 5;
    let ca = (alpha as u32) << 24;
    let tx = cx.i16(RECT_SMOJIPLT)?;
    eprintln!("sub window kind {kind} sub {sub:#x} aeon {aeon}: {n} entries {:?}, title {:#x} at {tx} {ty} alpha {alpha:#x}, rows from {ry} step {step}", &list[..n.max(0) as usize], wi(0x68));
    plate_l(cx, l, 0xda, 0xdb, tx, ty + dy, 0xd5, ca, ca)?;
    l.c(STRTEX_CELL, &[wi(0x68), f((tx + 0x15) as f32), f((ty + dy + 5) as f32), f(170.0), f(14.0), alpha as u32]);
    let pm = cx.pos(POS_PMSHAPE)?;
    let (lx, rx) = (cx.i16(POS_LSTAPLT)?, cx.i16(POS_RSTAPLT)?);
    for k in 0..n {
        let row = k / 2;
        let y = step * row + ry + ratio(dy, row);
        let x = if k & 1 != 0 { rx + dx * row / 32 } else { lx - dx * row / 32 };
        if y > 0x1a0 || !(-0x200..=0x200).contains(&x) {
            continue;
        }
        kind_plate(cx, l, x, y, 0x10)?;
        cx.shape(l, 0xd7, x + pm.0 + 0x58, y + pm.1)?;
        let id = list[k as usize];
        if id == 0 {
            continue;
        }
        if sub == 1 {
            if id == 100 {
                let t = cx.call(COMMAND_NAME, &[0x30ff])?;
                font(l, t, x + 12, y + 1, 0);
            } else {
                icon(l, x + 15, y + 2, 0x13);
                let t = cx.call(CHR_NAME, &[id])?;
                font(l, t, x + 34, y + 1, 0);
            }
        } else if kind == 0 {
            let (r, name) = cx.named(ROM_ABILITY, id)?;
            icon(l, x + 12, y + 1, u32::from(r[0x68]));
            font(l, name, x + 29, y + 1, 0);
        } else {
            let (r, name) = cx.named(COM_DATA, id)?;
            icon(l, x + 12, y + 1, u32::from(r[0x14]));
            font(l, name, x + 29, y + 1, 0);
        }
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// The main window (if open) and every open ability page; `scratch` = string areas per thunk.
/// Page 0 (PS2 0x0022a770) for another screen's list (Equip draws the same page).
pub fn page_calls(cx0: &mut Ctx, fo: Fonts, out: u32, l: &mut List, chr: u32, t: i32) -> R<()> {
    let mut cx = Cx { cx: cx0, out, fo };
    page0(&mut cx, l, 0, 0, chr, t)
}

pub fn build(cx0: &mut Ctx, env: &Env, scratch: &[u32]) -> R<Vec<Draw>> {
    let delta = cx0.delta;
    let mut cx = Cx { cx: cx0, out: env.out, fo: env.fo };
    let mut draws = vec![];
    let mut windows = vec![(MAIN_WINDOW, true)];
    windows.extend((0..6).map(|i| (SUB_WINDOWS + 4 * i, false)));
    for (slot, main) in windows {
        let w = cx.u32(slot)?;
        if w == 0 {
            continue;
        }
        let b = cx.abs_bytes(w, 0x98)?;
        let draw = u32::from_le_bytes(b[0x10..0x14].try_into().unwrap()).wrapping_sub(delta);
        // the PC's draw (0x008d2710 / 0x008d2790), or a thunk an earlier @ps2screen put there
        eprintln!("window {slot:#x} = {w:#x}: draw {draw:#x}");
        let mut l = List::new(scratch[draws.len()]);
        if main {
            main_window(&mut cx, &mut l, &b)?;
        } else {
            sub_window(&mut cx, &mut l, &b)?;
        }
        draws.push(Draw { target: w.wrapping_sub(delta) + 0x10, list: l });
    }
    Ok(draws)
}
