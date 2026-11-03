//! The Customize screen as mod/CustomizeMenuPatches.cs draws it (ui.menus = ps2): the equipment
//! list (PS2 0x00232188 for MenuKaizou_Draw_008d6860, window 0x0186aa30) and the abilities to add
//! (PS2 0x00232730 for DrawGearCustomizationMenu, window 0x0186aa34). The same calls as the C#; the
//! game's lookups (the menu sequence's layout by name, weapons, abilities) are made at the frame
//! boundary. Addresses are ffx.sym VAs; evidence in docs/ui.md ("The Customize screen").
use crate::menu::{Ctx, Fonts, List, cursor_parts, f, font_s, value_m, value_s};
use crate::screens::{Draw, Env};

const BOX_C2: u32 = 0x008f3db0;
const BOX_RGBA: u32 = 0x008fb150;
const PLATE_LR: u32 = 0x008f5e90;
const PLATE_RL: u32 = 0x008f5e20;
const SHAPE_XY: u32 = 0x00903a10;
const SHAPE_XYC2: u32 = 0x00903a50;
const SHAPE_XYWH: u32 = 0x00903b70;
const SHAPE_UV: u32 = 0x00903ee0; // TOMkpShapeXYWHUVC2(id, f x, y, w, h, f u0, v0, u1, v1, c0, c1)
const MESWIN_LAYOUT: u32 = 0x00c5_b460;
const FONT_1612: u32 = 0x00901cb0;
const ICON: u32 = 0x008e6af0;
const SCISSOR: u32 = 0x008fdee0;
const WIN_SCISSOR: u32 = 0x008c0f70; // TkMn2DrawMenuWindowScissor(y, h, part, ratio)
const FRAME: u32 = 0x008f9b60; // TOMkpWindowFrame_008f9b60(x, y, w, h, c0, c1)
const SHEET_CELL: u32 = 0x008f8bb0; // TODrawSheetCell_008f8bb0(cell, f x, f y, f w, f h): strtex
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const GREY: u32 = 0x8080_8080;
const SHADOW: u32 = 0x8000_0000;
const ROW: i32 = 32;
const LIST_H: i32 = 0x120;

const WEAPONS: u32 = 0x0186_aa38;
const ITEM_LIST: u32 = 0x0159_7770;
const SAVE_WEAPON: u32 = 0x007a_bbd0;
const WEAPON_NAME: u32 = 0x007a_bdf0;
const ROM_ABILITY: u32 = 0x0079_0970;
const ROM_ITEM: u32 = 0x0079_09f0;

fn i16_at(b: &[u8], o: usize) -> i32 {
    i32::from(i16::from_le_bytes([b[o], b[o + 1]]))
}
fn u16_at(b: &[u8], o: usize) -> i32 {
    i32::from(u16::from_le_bytes([b[o], b[o + 1]]))
}

struct Cx<'a, 'b> {
    cx: &'a mut Ctx<'b>,
    out: u32, // scratch: out parameters at +0, the name at +0x100 (absolute)
    fo: Fonts,
    dum75: u32,
}

impl Cx<'_, '_> {
    fn read(&mut self, abs: u32, n: usize) -> ffx_harness::Result<Vec<u8>> {
        self.cx.game.read(abs.wrapping_sub(self.cx.delta), n)
    }
    fn write(&mut self, abs: u32, b: &[u8]) -> ffx_harness::Result<()> {
        let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
        self.cx.game.command(&format!("write {:#x} {hex}", abs.wrapping_sub(self.cx.delta)))?;
        Ok(())
    }
    fn name(&mut self, s: &str) -> ffx_harness::Result<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        self.write(self.out + 0x100, &b)?;
        Ok(self.out + 0x100)
    }
    /// TOSeqGetPoint(1, name): i16 x, y.
    fn pos(&mut self, s: &str) -> ffx_harness::Result<(i32, i32)> {
        let n = self.name(s)?;
        self.write(self.out, &[0; 8])?;
        self.cx.call(0x008a_e450, &[1, n, self.out, self.out + 4])?;
        let r = self.read(self.out, 8)?;
        Ok((i16_at(&r, 0), i16_at(&r, 4)))
    }
    /// TOSeqGetRect(1, name): i16 x, y, w, h.
    fn rect(&mut self, s: &str) -> ffx_harness::Result<(i32, i32, i32, i32)> {
        let n = self.name(s)?;
        self.write(self.out, &[0; 16])?;
        self.cx.call(0x008a_e4b0, &[1, n, self.out, self.out + 4, self.out + 8, self.out + 12])?;
        let r = self.read(self.out, 16)?;
        Ok((i16_at(&r, 0), i16_at(&r, 4), i16_at(&r, 8), i16_at(&r, 12)))
    }
    /// TkMn2SeqGetColor(name).
    fn colour(&mut self, s: &str) -> ffx_harness::Result<u32> {
        let n = self.name(s)?;
        self.cx.call(0x008c_2ad0, &[n])
    }
    /// A record getter (id, &strings) -> (record, strings base).
    fn rec(&mut self, data: u32, id: u32) -> ffx_harness::Result<(u32, u32)> {
        self.write(self.out, &[0; 4])?;
        let rec = self.cx.call(data, &[id, self.out])?;
        let strings = u32::from_le_bytes(self.read(self.out, 4)?[..4].try_into().unwrap());
        Ok((rec, strings))
    }
    fn weapon_id(&mut self, i: i32) -> ffx_harness::Result<u32> {
        let p = self.cx.r32(WEAPONS)?;
        Ok(u32::from(u16::from_le_bytes(self.read(p + 2 * i as u32, 2)?[..2].try_into().unwrap())))
    }
    /// MsGetSaveWeapon(id): the record's first 0x16 bytes.
    fn weapon(&mut self, id: u32) -> ffx_harness::Result<Vec<u8>> {
        let (rec, _) = self.rec(SAVE_WEAPON, id)?;
        self.read(rec, 0x16)
    }
    fn cursor(&self, l: &mut List, x: i32, y: i32) {
        for (fun, a) in cursor_parts(x, y, self.dum75) {
            l.c(fun, &a);
        }
    }
}

fn plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, kind: i32) -> ffx_harness::Result<()> {
    let at = 0x025d_16a0 - 8 * ((kind >> 3) & 3) as u32;
    let (c0, c1) = (cx.cx.r32(at)?, cx.cx.r32(at - 4)?);
    l.c(SHAPE_XYWH, &[0xda, x as u32, y as u32, 0, 0, c0, c0]);
    l.c(SHAPE_XYWH, &[0xdb, (x + 16) as u32, y as u32, 209 - 16, 0, c0, c1]); // PS2 0x001f4820: 209 in all
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

fn row_plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, kind: i32, pm: (i32, i32)) -> ffx_harness::Result<()> {
    plate(cx, l, x, y, kind)?;
    parts(cx, l, 0xd7, x + pm.0 + 0x58, y + pm.1)
}

/// A meswin shape in grey part by part, each part's texels inset by half a texel
/// (mod CustomizeMenuPatches.parts): FFX.exe's copy of the PS2 layout table 0x00c5b460, ids 200..
/// at id - 200; TOMkpShapeXYWHUVC2 on the 256 x 256 ps2win.
fn parts(cx: &mut Cx, l: &mut List, id: u32, x: i32, y: i32) -> ffx_harness::Result<()> {
    let i = if id >= 200 { id - 200 } else { id };
    let e = MESWIN_LAYOUT + cx.cx.r32(MESWIN_LAYOUT + 4 + 4 * i)?;
    let n = cx.cx.r32(e)?;
    let b = cx.cx.game.read(e + 4, 12 * n as usize)?;
    for k in 0..n as usize {
        let p = &b[12 * k..12 * k + 12];
        let (u, v) = (f32::from(p[0]), f32::from(p[1]));
        let (dx, dy) = (i32::from(p[2] as i8), i32::from(p[3] as i8));
        let (w, h) = (f32::from(u16::from_le_bytes([p[4], p[5]])), f32::from(u16::from_le_bytes([p[6], p[7]])));
        l.c(SHAPE_UV, &[
            0,
            f((x + dx) as f32),
            f((y + dy) as f32),
            f(w),
            f(h),
            f((u + 0.5) / 256.0),
            f((v + 0.5) / 256.0),
            f((u + w - 0.5) / 256.0),
            f((v + h - 0.5) / 256.0),
            GREY,
            GREY,
        ]);
    }
    Ok(())
}

fn label(l: &mut List, x: i32, y: i32, k: u32) {
    l.c(PLATE_RL, &[0xda, 0xdb, x as u32, y as u32, 0xd5, SHADOW, SHADOW]);
    l.c(SHEET_CELL, &[k, f((x + 21) as f32), f((y + 5) as f32), f(170.0), f(14.0)]);
}

fn icon(l: &mut List, x: i32, y: i32, id: u32) {
    l.c(ICON, &[f(x as f32), f(y as f32), f(15.0), f(18.0), id, 0x80, 0x80, 0x80, 0x80]);
}

fn icon_name(cx: &mut Cx, l: &mut List, data: u32, id: u32, icon_at: u32, x: i32, y: i32, colour: u32) -> ffx_harness::Result<()> {
    let (rec, strings) = cx.rec(data, id)?;
    let b = cx.read(rec, icon_at as usize + 1)?;
    icon(l, x + 12, y + 1, u32::from(b[icon_at as usize]));
    let name = strings + u32::from(u16::from_le_bytes([b[0], b[1]]));
    l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, colour]);
    Ok(())
}

fn weapon_name(cx: &mut Cx, l: &mut List, id: u32, x: i32, y: i32, colour: u32) -> ffx_harness::Result<()> {
    let w = cx.weapon(id)?;
    icon(l, x + 12, y + 1, (u32::from(w[5]) + 2 * u32::from(w[4]) + 1) & 0xff);
    let name = cx.cx.call(WEAPON_NAME, &[id])?;
    l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, colour]);
    if w[4] == w[6] {
        icon(l, x + 12, y + 1, 0x31);
    }
    Ok(())
}

fn scroll_bar(l: &mut List, r: (i32, i32, i32, i32), top: i32, shown: i32, count: i32) {
    if shown >= count {
        return;
    }
    let (rx, ry, rw, rh) = r;
    l.c(BOX_RGBA, &[rx as u32, ry as u32, rw as u32, rh as u32, 0x11, 0x11, 0x11, 0x80]);
    let t = if top == 0 { ry as f32 } else { ry as f32 + rh as f32 * top as f32 / count as f32 };
    let b = if top + shown >= count { (ry + rh) as f32 } else { ry as f32 + rh as f32 * (top + shown) as f32 / count as f32 };
    l.c(BOX_RGBA, &[(rx + 1) as u32, t as i32 as u32, (rw - 2) as u32, (b as i32 - t as i32) as u32, 0xf0, 0xf0, 0xf0, 0x80]);
}

type Row<'r> = dyn FnMut(&mut Cx, &mut List, i32, i32, i32) -> ffx_harness::Result<()> + 'r;

/// Rows -1 .. 9 from pos_ksobiplt in the window scissor (PS2 0x00231f90 / 0x00232690, 0x002274a8).
fn rows(cx: &mut Cx, l: &mut List, w: &[u8], at: (i32, i32), row: &mut Row) -> ffx_harness::Result<()> {
    let (top, next, scroll) = (i16_at(w, 0x32), i16_at(w, 0x34), i16_at(w, 0x46));
    if top == next {
        l.c(WIN_SCISSOR, &[at.1 as u32, LIST_H as u32, 0, scroll as u32]);
        for i in -1..10 {
            row(cx, l, at.0, at.1 + ROW * i - scroll / 128, top + i)?;
        }
    } else {
        l.c(WIN_SCISSOR, &[at.1 as u32, LIST_H as u32, 1, scroll as u32]);
        for i in -1..10 {
            row(cx, l, at.0, at.1 + ROW * i, top + i)?;
        }
        l.c(WIN_SCISSOR, &[at.1 as u32, LIST_H as u32, 2, scroll as u32]);
        let dy = -(scroll * LIST_H) / 4096;
        for i in -1..10 {
            row(cx, l, at.0, at.1 + dy + ROW * i, next + i)?;
        }
    }
    l.c(SCISSOR, &[0, 0, 512, 416]);
    Ok(())
}

/// The equipment list (PS2 0x00232188).
fn list(cx: &mut Cx, l: &mut List, w: &[u8], wabs: u32) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let at = cx.pos("pos_ksobiplt")?;
    let pm = cx.pos("pos_pmshape")?;
    let count = u16_at(w, 0x30);
    eprintln!("customize list: count {count} top {} next {} row {} scroll {} at {at:?} pm {pm:?}", i16_at(w, 0x32), i16_at(w, 0x34), i16_at(w, 0x48), i16_at(w, 0x46));
    rows(cx, l, w, at, &mut |cx, l, x, y, i| {
        if i < 0 || i >= count {
            return Ok(());
        }
        row_plate(cx, l, x, y, 0, pm)?;
        let id = cx.weapon_id(i)?;
        let rec = cx.weapon(id)?;
        if rec[2] == 0 || rec[3] & 2 != 0 {
            return Ok(());
        }
        let grey = u32::from(cx.cx.call(0x008d_5750, &[id, 0])? == 0);
        weapon_name(cx, l, id, x, y, grey)
    })?;
    cx.cursor(l, at.0 + 12, at.1 + 1 + ROW * (i16_at(w, 0x48) - i16_at(w, 0x32)));
    let sc = cx.rect("r_kscrol")?;
    scroll_bar(l, sc, i16_at(w, 0x32), u16_at(w, 0x3a), count);
    let m2 = cx.rect("r_kmojiplt2")?;
    label(l, m2.0, m2.1, 7);
    let m1 = cx.rect("r_kmojiplt1")?;
    label(l, m1.0, m1.1, 6);
    let zone = cx.pos("pos_kzonmplt")?;
    // the fade band (PS2 0x00231e18)
    let c = cx.cx.call(0x008d_4910, &[])? as i32;
    let a = ((c >> 5) as u32) << 24;
    let kab = cx.pos("pos_kabplt")?;
    eprintln!("customize: scroll rect {sc:?} moji1 {m1:?} moji2 {m2:?} zone {zone:?} kabplt {kab:?} status bg {c:#x}");
    if a != 0 {
        let (b0, b1) = (cx.colour("col_menubg0")?, cx.colour("col_menubg1")?);
        l.c(BOX_C2, &[0, zone.1 as u32, 512, (416 - zone.1) as u32, (b0 & 0xff_ffff) | a, (b1 & 0xff_ffff) | a]);
    }
    // the picked weapon's slots and abilities (PS2 0x00232030)
    let id = cx.weapon_id(i16_at(w, 0x48))?;
    let rec = cx.weapon(id)?;
    if rec[2] != 0 {
        for i in 0..i32::from(rec[0xb]) {
            row_plate(cx, l, kab.0, kab.1 + ROW * i, 0x10, pm)?;
        }
        for i in 0..4 {
            let ab = u32::from(u16::from_le_bytes([rec[0xe + 2 * i], rec[0xf + 2 * i]]));
            if ab != 0 && ab != 0xff {
                icon_name(cx, l, ROM_ABILITY, ab, 0x68, kab.0, kab.1 + ROW * i as i32, 0)?;
            }
        }
    }
    // the owner's party plate (PS2 0x00225f38 -> 0x00200c60)
    let chr = cx.cx.call(0x008d_56f0, &[wabs])? as i32;
    member(cx, l, zone.0, zone.1, chr)?;
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

fn member(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: i32) -> ffx_harness::Result<()> {
    let fo = cx.fo;
    let pt = |cx: &mut Cx, a: u32| -> ffx_harness::Result<(i32, i32)> { Ok((cx.cx.r16(a + 2)?, cx.cx.r16(a)?)) };
    let party = cx.cx.r32(0x0184_1c2c)?;
    if (0..32).contains(&chr) && party & (1 << chr) != 0 {
        let (nx, ny, nh) = (cx.cx.r16(0x023c_c144)?, cx.cx.r16(0x023c_c146)?, cx.cx.r16(0x023c_c13e)?);
        let (c0, c1) = (cx.cx.r32(0x023c_c148)?, cx.cx.r32(0x023c_c140)?);
        l.c(BOX_C2, &[(x + nx) as u32, (y + ny) as u32, 512, nh as u32, c0, c1]);
    }
    let (hptex, mptex, slv) = (pt(cx, 0x025d_165e)?, pt(cx, 0x025d_1656)?, pt(cx, 0x025d_164e)?);
    let nw = cx.cx.r16(0x025d_1672)?;
    let (c0, c1) = (cx.cx.r32(0x025d_166c)?, cx.cx.r32(0x025d_1668)?);
    eprintln!("customize member {chr} at {x},{y} party {party:#x}");
    if chr as u32 >= 8 {
        l.c(PLATE_LR, &[0xcc, 0xcd, x as u32, y as u32, nw as u32, c0, c1]);
        l.c(SHAPE_XY, &[0xe9, (x + hptex.0) as u32, (y + hptex.1) as u32]);
        l.c(SHAPE_XY, &[0xea, (x + mptex.0) as u32, (y + mptex.1) as u32]);
        l.c(SHAPE_XYWH, &[0xe7, 0, y as u32, 512, 0, GREY, GREY]);
        parts(cx, l, 0x107, x + slv.0 + 0x68, y + slv.1)?;
        return Ok(());
    }
    let chr = chr as u32;
    l.c(PLATE_LR, &[4, 5, x as u32, y as u32, nw as u32, c0, c1]);
    let namestr = pt(cx, 0x025d_1662)?;
    let name = cx.cx.call(0x008a_c850, &[chr])?;
    let colour = cx.cx.call(0x008a_99f0, &[chr])?;
    l.c(FONT_1612, &[name, (x + namestr.0) as u32, (y + namestr.1) as u32, colour]);
    let (hpstr, mpstr) = (pt(cx, 0x025d_165a)?, pt(cx, 0x025d_1652)?);
    let (hp, hpm) = (cx.cx.call(0x008a_98c0, &[chr])?, cx.cx.call(0x008a_9990, &[chr])?);
    let (mp, mpm) = (cx.cx.call(0x008a_9970, &[chr])?, cx.cx.call(0x008a_99b0, &[chr])?);
    l.c(SHAPE_XY, &[0x21, (x + hptex.0) as u32, (y + hptex.1) as u32]);
    font_s(l, &fo, &format!("{hp:5}/{hpm:5}"), x + hpstr.0, y + hpstr.1, 0x25);
    l.c(SHAPE_XY, &[0x22, (x + mptex.0) as u32, (y + mptex.1) as u32]);
    font_s(l, &fo, &format!("{mp:4}/{mpm:4}"), x + mpstr.0, y + mpstr.1, 0x25);
    l.c(SHAPE_XYWH, &[0xe7, 0, y as u32, 512, 0, GREY, GREY]);
    let spstr = pt(cx, 0x025d_164a)?;
    parts(cx, l, 0x107, x + slv.0 + 0x68, y + slv.1)?;
    let sp = cx.cx.call(0x008a_9b80, &[chr])?;
    value_m(l, &fo, i64::from(sp), x + slv.0 + spstr.0, y + slv.1 + spstr.1, 0);
    let ap = cx.cx.call(0x0078_4e90, &[chr])?.wrapping_sub(cx.cx.call(0x0078_52b0, &[chr])?);
    value_s(l, &fo, i64::from(ap as i32), x + slv.0 + 139, y + slv.1 - 13, 0x25);
    Ok(())
}

/// The abilities to add (PS2 0x00232730).
fn add(cx: &mut Cx, l: &mut List, w: &[u8]) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let pm = cx.pos("pos_pmshape")?;
    let e = cx.read(cx.cx.abs(ITEM_LIST) + 4 * i16_at(w, 0x48) as u32, 4)?;
    if e[3] != 0xff {
        let k = cx.cx.call(0x008c_1b70, &[])? + 8 * u32::from(e[3]);
        let kb = cx.read(k, 8)?;
        let back = cx.rect("r_kbackplt")?;
        require(cx, l, back.0, back.1, i32::from(u16::from_le_bytes([kb[4], kb[5]])), i32::from(kb[6]), pm)?;
        let m3 = cx.rect("r_kmojiplt3")?;
        label(l, m3.0, m3.1, 0x10);
    }
    let m1 = cx.rect("r_kmojiplt1")?;
    label(l, m1.0, m1.1, 7);
    let at = cx.pos("pos_ksobiplt")?;
    let count = u16_at(w, 0x30);
    eprintln!("customize add: count {count} top {} row {} entry {e:?}", i16_at(w, 0x32), i16_at(w, 0x48));
    rows(cx, l, w, at, &mut |cx, l, x, y, i| {
        if i < 0 || i >= count {
            return Ok(());
        }
        let r = cx.read(cx.cx.abs(ITEM_LIST) + 4 * i as u32, 4)?;
        row_plate(cx, l, x, y, 0x10, pm)?;
        if r[3] != 0xff {
            icon_name(cx, l, ROM_ABILITY, u32::from(u16::from_le_bytes([r[0], r[1]])), 0x68, x, y, u32::from(r[2] != 0xb))?;
        }
        Ok(())
    })?;
    cx.cursor(l, at.0 + 12, at.1 + 1 + ROW * (i16_at(w, 0x48) - i16_at(w, 0x32)));
    let sc = cx.rect("r_kscrol")?;
    scroll_bar(l, sc, i16_at(w, 0x32), u16_at(w, 0x3a), count);
    // the weapon of the equipment list's row (PS2 0x002324d8)
    let list = cx.cx.r32(0x0186_aa30)?;
    if list != 0 {
        let lw = cx.read(list, 0x98)?;
        let t = cx.pos("pos_ksobiplt2")?;
        row_plate(cx, l, t.0, t.1, 0, pm)?;
        let id = cx.weapon_id(i16_at(&lw, 0x48))?;
        let name = cx.cx.call(WEAPON_NAME, &[id])?;
        l.c(FONT_1612, &[name, (t.0 + 0x1d) as u32, (t.1 + 1) as u32, 0]);
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// The item cost window (PS2 0x00227800).
fn require(cx: &mut Cx, l: &mut List, x: i32, y: i32, item: i32, need: i32, pm: (i32, i32)) -> ffx_harness::Result<()> {
    let fo = cx.fo;
    let r = cx.rect("r_shohiplt")?;
    let (c0, c1) = (cx.colour("col_shohiplt0")?, cx.colour("col_shohiplt1")?);
    l.c(FRAME, &[x as u32, y as u32, r.2 as u32, r.3 as u32, c0, c1]);
    let sh = cx.pos("pos_shiplt")?;
    let shape = cx.pos("pos_shshape")?;
    let (sx, sy) = (x + sh.0, y + sh.1);
    row_plate(cx, l, sx, sy, 8, pm)?;
    parts(cx, l, 0xff, x + shape.0 + 0x10, y + shape.1)?;
    let (lp, rp) = (cx.pos("pos_lshohi")?, cx.pos("pos_rshohi")?);
    eprintln!("customize require: at {x},{y} item {item:#x} need {need} r_shohiplt {r:?} c {c0:08x} {c1:08x} shiplt {sh:?} shshape {shape:?} l {lp:?} r {rp:?}");
    if item < 0 {
        return Ok(());
    }
    let held = cx.cx.call(0x0079_04b0, &[item as u32])? as i32;
    let few = u32::from(held < need);
    icon_name(cx, l, ROM_ITEM, item as u32, 0x14, sx, sy, few)?;
    value_m(l, &fo, i64::from(held), x + shape.0 + lp.0, y + shape.1 + lp.1, few);
    let (rx, ry) = (x + shape.0 + rp.0, y + shape.1 + rp.1);
    if few == 1 {
        let text = cx.cx.call(0x008d_df80, &[0x15])?;
        let tw = cx.cx.text_width(text)?;
        l.c(FONT_1612, &[text, (rx - tw) as u32, ry as u32, 1]);
    } else {
        value_m(l, &fo, i64::from(held - need), rx, ry, 0);
    }
    Ok(())
}

pub fn build(cx0: &mut Ctx, env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    let mut cx = Cx { cx: cx0, out: env.out, fo: env.fo, dum75: env.dum75 };
    let mut draws = vec![];
    let a = cx.cx.r32(0x0186_aa30)?;
    if a != 0 {
        let w = cx.read(a, 0x98)?;
        let mut l = List::new(scratch[0]);
        list(&mut cx, &mut l, &w, a)?;
        draws.push(Draw { target: (a + 0x10).wrapping_sub(cx.cx.delta), list: l });
    }
    let b = cx.cx.r32(0x0186_aa34)?;
    if b != 0 {
        let w = cx.read(b, 0x98)?;
        let mut l = List::new(scratch[draws.len()]);
        add(&mut cx, &mut l, &w)?;
        draws.push(Draw { target: (b + 0x10).wrapping_sub(cx.cx.delta), list: l });
    }
    Ok(draws)
}
