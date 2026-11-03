//! What mod/ItemMenuPatches.cs draws (ui.menus = ps2), as call lists for the thunk: the item /
//! ability list window (PS2 0x0024f0e8 for MenuItem_ListWindowDraw_008c7750), the sub-command bar
//! (PS2 0x00236e68 for MenuSubCmd_DrawRow_008e3cf0) and the ability screen's party plate (PS2
//! 0x00250978 for 0x008c64a0). The same arithmetic as the C#; the game's lookups are called at the
//! frame boundary. Addresses are ffx.sym VAs; evidence in docs/ui.md ("The Items and Abilities
//! screens").
use crate::menu::{Ctx, List, cursor_parts, f};

const BOX_C2: u32 = 0x008f3db0;
const BOX_RGBA: u32 = 0x008fb150;
const PLATE_LR: u32 = 0x008f5e90;
const PLATE_RL: u32 = 0x008f5e20;
const SHAPE_XY: u32 = 0x00903a10;
const SHAPE_XYC2: u32 = 0x00903a50;
const SHAPE_XYWH: u32 = 0x00903b70;
const FONT_1612: u32 = 0x00901cb0;
const STR_S: u32 = 0x00901660;
const DIGIT_R: u32 = 0x009055c0;
const ICON: u32 = 0x008e6af0;
const SCISSOR: u32 = 0x008fdee0;
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const GREY: u32 = 0x8080_8080;
const SHADOW: u32 = 0x8000_0000;
const ROW: i32 = 32;

pub struct Items {
    pub list: Option<(u32, List)>,
    pub bar: Option<(u32, List)>,
    pub ability: Option<List>,
}

fn i16_at(b: &[u8], o: usize) -> i32 {
    i32::from(i16::from_le_bytes([b[o], b[o + 1]]))
}
fn u16_at(b: &[u8], o: usize) -> i32 {
    i32::from(u16::from_le_bytes([b[o], b[o + 1]]))
}
fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

struct Cx<'a, 'b> {
    cx: &'a mut Ctx<'b>,
    out: u32, // scratch for out parameters (absolute)
    dum75: u32,
}

impl Cx<'_, '_> {
    fn window(&mut self, ptr_va: u32) -> ffx_harness::Result<Option<(u32, Vec<u8>)>> {
        let w = self.cx.r32(ptr_va)?;
        if w == 0 {
            return Ok(None);
        }
        let b = self.cx.game.read(w.wrapping_sub(self.cx.delta), 0x98)?;
        Ok(Some((w, b)))
    }
    fn com(&mut self, id: u32, data: u32) -> ffx_harness::Result<(u32, u32)> {
        let rec = self.cx.call(data, &[id, self.out])?;
        let strings = self.cx.r32(self.out.wrapping_sub(self.cx.delta))?;
        let name_off = u32::from(u16::from_le_bytes(self.cx.game.read(rec.wrapping_sub(self.cx.delta), 2)?[..2].try_into().unwrap()));
        Ok((rec, strings + name_off))
    }
    fn byte(&mut self, abs: u32) -> ffx_harness::Result<u32> {
        Ok(u32::from(self.cx.game.read(abs.wrapping_sub(self.cx.delta), 1)?[0]))
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
    l.c(SHAPE_XYWH, &[0xdb, (x + 16) as u32, y as u32, 209, 0, c0, c1]);
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

fn icon_name(l: &mut List, icon: u32, name: u32, x: i32, y: i32, colour: u32) {
    l.c(ICON, &[f((x + 12) as f32), f((y + 1) as f32), f(15.0), f(18.0), icon, 0x80, 0x80, 0x80, 0x80]);
    l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, colour]);
}

fn value(l: &mut List, v: u32, x: i32, y: i32, grey: u32) {
    l.c(DIGIT_R, &[v, f((x + 2) as f32), f(y as f32), grey, f(0.78), f(1.0)]);
}

fn row(cx: &mut Cx, l: &mut List, w: &[u8], x: i32, y: i32, idx: i32) -> ffx_harness::Result<()> {
    let idx = idx + 2 * i16_at(w, 0x32);
    let y = y - i16_at(w, 0x46) / 128;
    if idx < 0 {
        return Ok(());
    }
    let kind = w[0x43];
    if kind == 1 {
        if idx >= cx.cx.call(0x008c_8a20, &[])? as i32 {
            return Ok(());
        }
        let id = u32::from(u16::from_le_bytes(cx.cx.game.read(0x0159_6f70 + 2 * idx as u32, 2)?[..2].try_into().unwrap()));
        if id == 0 || cx.cx.call(0x008a_b2c0, &[id])? == 0 {
            return Ok(());
        }
        plate(cx, l, x, y, 1)?;
        let (rec, name) = cx.com(id, 0x0079_08a0)?;
        let icon = cx.byte(rec + 0x12)?;
        icon_name(l, icon, name, x, y, 0);
        return Ok(());
    }
    let max = cx.cx.call(if kind == 0 { 0x008c_8c30 } else { 0x008c_8a00 }, &[])? as i32;
    if idx >= max {
        return Ok(());
    }
    let e = cx.cx.game.read(0x0159_7770 + 4 * idx as u32, 4)?;
    let id = u32::from(u16::from_le_bytes([e[0], e[1]]));
    let dim = u32::from(e[3] == 0);
    if kind == 0 {
        plate(cx, l, x, y, 0xa)?;
        let n = if id == 0 { 0 } else { cx.cx.call(0x0079_04b0, &[id])? };
        if n != 0 {
            let (rec, name) = cx.com(id, 0x0079_a4b0)?;
            let icon = cx.byte(rec + 0x14)?;
            icon_name(l, icon, name, x, y, dim);
            value(l, n, x + 211, y + 6, dim);
        }
    } else if e[2] == 1 {
        plate(cx, l, x, y, 0x12)?;
        let (rec, name) = cx.com(id, 0x0079_a4b0)?;
        let icon = cx.byte(rec + 0x14)?;
        icon_name(l, icon, name, x, y, dim);
        let ply = cx.cx.call(0x0079_0a90, &[id])?;
        let mp = cx.byte(ply + 0x25)?;
        if mp != 0 {
            value(l, mp, x + 211, y + 6, dim);
        }
    } else if e[2] == 2 {
        plate(cx, l, x, y, 2)?;
    }
    Ok(())
}

/// The list window at rest (PS2 0x0024f0e8; the scrolling passes are not mirrored).
fn list(cx: &mut Cx, l: &mut List, w: &[u8]) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let kind = w[0x43];
    let (count, top, shown) = (u16_at(w, 0x30), i16_at(w, 0x32), u16_at(w, 0x3a));
    let r = if cx.cx.r32(0x0186_a408)? == 1 { 0x0186_a410 } else { 0x0186_a418 };
    let (rx, ry, rw, rh) = (cx.cx.r16(r)?, cx.cx.r16(r + 2)?, cx.cx.r16(r + 4)?, cx.cx.r16(r + 6)?);
    eprintln!("list kind {kind} count {count} top {top} shown {shown} scroll rect {rx} {ry} {rw} {rh}");
    if shown < count {
        l.c(BOX_RGBA, &[rx as u32, ry as u32, rw as u32, rh as u32, 0x11, 0x11, 0x11, 0x80]);
        let t = if top == 0 { ry as f32 } else { ry as f32 + rh as f32 * top as f32 / count as f32 };
        let b = if top + shown >= count { (ry + rh) as f32 } else { ry as f32 + rh as f32 * (top + shown) as f32 / count as f32 };
        l.c(BOX_RGBA, &[(rx + 1) as u32, t as i32 as u32, (rw - 2) as u32, (b as i32 - t as i32) as u32, 0xf0, 0xf0, 0xf0, 0x80]);
    }
    let y0 = if kind == 2 { 119 } else { 98 };
    if kind == 2 {
        l.c(SCISSOR, &[0, 114, 511, 114 + 288]);
    } else {
        l.c(SCISSOR, &[0, 93, 511, 415]);
    }
    for k in -1..=shown {
        row(cx, l, w, 30, y0 + ROW * k, 2 * k)?;
        row(cx, l, w, 279, y0 + ROW * k, 2 * k + 1)?;
    }
    l.c(SCISSOR, &[0, 0, 512, 416]);
    if w[0x94] != 0 {
        // the cursor (PS2 0x0024efd0)
        let (n, v) = (i32::from(w[0x42]), i32::from(w[0x45] as i8));
        let p = if v < n { v } else { n - 1 };
        let show = (v < n || n == 2) && v >= 0;
        let mode = cx.cx.r32(0x0186_a40c)? as i32;
        if show || mode != 0 {
            let mut x = 30;
            if i32_at(w, 0x58 + 4 * p.max(0) as usize) != 0 {
                x += 255;
            }
            let y = y0 + ROW * (i16_at(w, 0x48) - top);
            cx.cursor(l, x + 12, y + 1);
        }
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// The first sub-command bar (PS2 0x00236e68), PS2 geometry (0x002373e0, minimum width 381).
fn bar(cx: &mut Cx, l: &mut List, w: &[u8]) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let n = u16_at(w, 0x30).max(1);
    let texts: Vec<u32> = (0..n).map(|i| u32::from_le_bytes(w[0x58 + 12 * i as usize..0x5c + 12 * i as usize].try_into().unwrap())).collect();
    let widths: Vec<i32> = texts.iter().map(|&t| cx.cx.call(0x008c_2cb0, &[t]).map(|v| i32::from(v as u16 as i16))).collect::<Result<_, _>>()?;
    let sum: i32 = widths.iter().sum();
    let a = sum + 32 * (n - 1) + 96;
    let width = if a <= 381 { 381 } else { a.min(0x1f0) };
    let spacing = if n < 2 { 0 } else { ((width - sum - 96) / (n - 1)).clamp(32, 128) };
    let (x, y) = ((512 - width) / 2, 37);
    let (fade, k) = (i16_at(w, 0x46), i16_at(w, 0x48));
    let wk = widths[k.clamp(0, n - 1) as usize];
    let cw = (width * fade + (wk + 24) * (0x1000 - fade)) >> 12;
    let before: i32 = (0..k.min(n)).map(|i| widths[i as usize] + spacing).sum();
    let mut tx = ((x * fade + (x - before - 36) * (0x1000 - fade)) >> 12) + 48;
    eprintln!("bar n {n} widths {widths:?} width {width} spacing {spacing} fade {fade:#x} k {k}");
    l.c(SCISSOR, &[0, 0, 512, 416]);
    if w[0x94] != 1 {
        l.c(PLATE_RL, &[0xd8, 0xd9, x as u32, y as u32, width as u32, 0x8010_1010, 0x8010_1010]);
    }
    for i in 0..n {
        let tw = widths[i as usize];
        l.c(SCISSOR, &[(x + 12) as u32, 0, (x + 12 + cw - 24) as u32, 415]);
        l.c(FONT_1612, &[texts[i as usize], tx as u32, (y + 2) as u32, 0]);
        l.c(SCISSOR, &[0, 0, 512, 416]);
        if i == k {
            if (w[0x45] as i8) <= 0 {
                cx.cursor(l, tx, y + 2);
            } else {
                l.c(SHAPE_XY, &[0xe2, (tx - 5) as u32, (y + 2) as u32]);
                l.c(SHAPE_XYWH, &[0xe3, (tx + 11) as u32, (y + 2) as u32, (tw - 11) as u32, 0, GREY, GREY]);
                l.c(SHAPE_XY, &[0xe4, (tx + tw) as u32, (y + 2) as u32]);
            }
        }
        tx += tw + spacing;
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// The ability screen's party plate (PS2 0x00250978 / 0x002014e8).
fn ability(cx: &mut Cx, l: &mut List) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let chr = cx.cx.call(0x008a_9860, &[])?;
    let party = cx.cx.r32(0x0184_1c2c)?;
    if party & (1 << chr) != 0 {
        let (nx, ny, nh) = (cx.cx.r16(0x023c_c144)?, cx.cx.r16(0x023c_c146)?, cx.cx.r16(0x023c_c13e)?);
        let (c0, c1) = (cx.cx.r32(0x023c_c148)?, cx.cx.r32(0x023c_c140)?);
        l.c(BOX_C2, &[nx as u32, (ny + 49) as u32, 512, nh as u32, c0, c1]);
    }
    let (x, y) = (0i32, 49i32);
    let pt = |cx: &mut Cx, a: u32| -> ffx_harness::Result<(i32, i32)> { Ok((cx.cx.r16(a + 2)?, cx.cx.r16(a)?)) };
    let nw = cx.cx.r16(0x025d_1672)?;
    let (c0, c1) = (cx.cx.r32(0x025d_166c)?, cx.cx.r32(0x025d_1668)?);
    l.c(PLATE_LR, &[4, 5, x as u32, y as u32, nw as u32, c0, c1]);
    let namestr = pt(cx, 0x025d_1662)?;
    let name = cx.cx.call(0x008a_c850, &[chr])?;
    let colour = cx.cx.call(0x008a_99f0, &[chr])?;
    l.c(FONT_1612, &[name, (x + namestr.0) as u32, (y + namestr.1) as u32, colour]);
    let (hptex, hpstr, mptex, mpstr) = (pt(cx, 0x025d_165e)?, pt(cx, 0x025d_165a)?, pt(cx, 0x025d_1656)?, pt(cx, 0x025d_1652)?);
    let (hp, hpm) = (cx.cx.call(0x008a_98c0, &[chr])?, cx.cx.call(0x008a_9990, &[chr])?);
    let (mp, mpm) = (cx.cx.call(0x008a_9970, &[chr])?, cx.cx.call(0x008a_99b0, &[chr])?);
    l.c(SHAPE_XY, &[0x21, (x + hptex.0) as u32, (y + hptex.1) as u32]);
    let s1 = l.string(&format!("{hp:5}/{hpm:5}"));
    l.c(STR_S, &[s1, f((x + hpstr.0) as f32), f((y + hpstr.1) as f32), 0x25, f(1.0), 0]);
    l.c(SHAPE_XY, &[0x22, (x + mptex.0) as u32, (y + mptex.1) as u32]);
    let s2 = l.string(&format!("{mp:4}/{mpm:4}"));
    l.c(STR_S, &[s2, f((x + mpstr.0) as f32), f((y + mpstr.1) as f32), 0x25, f(1.0), 0]);
    l.c(BOX_RGBA, &[0, (y + 33) as u32, 512, 3, 0, 0, 0, 0x80]);
    let (slv, spstr) = (pt(cx, 0x025d_164e)?, pt(cx, 0x025d_164a)?);
    l.c(SHAPE_XY, &[0x107, (x + slv.0 + 0x68) as u32, (y + slv.1) as u32]);
    let sp = cx.cx.call(0x008a_9b80, &[chr])?;
    value(l, sp, x + slv.0 + spstr.0, y + slv.1 + spstr.1, 0);
    let ap = cx.cx.call(0x0078_4e90, &[chr])?.wrapping_sub(cx.cx.call(0x0078_52b0, &[chr])?);
    value(l, ap, x + slv.0 + 139, y + slv.1 - 13, 0);
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// Builds the lists; `scratch` = string areas for the three lists, `out` scratch for out
/// parameters, `name` the PS2 atlas name (written into the meswin slot around each list).
pub fn build(cx0: &mut Ctx, scratch: [u32; 3], out: u32, name: u32, ability_screen: bool) -> ffx_harness::Result<Items> {
    let (slot, orig) = crate::menu::atlas_slot(cx0)?;
    let dum75 = cx0.r32(0x025d_16ac)?;
    let mut cx = Cx { cx: cx0, out, dum75 };
    let wrap = |l: &mut List, inner: &mut dyn FnMut(&mut List) -> ffx_harness::Result<()>| -> ffx_harness::Result<()> {
        l.set(slot, name);
        inner(l)?;
        l.set(slot, orig);
        Ok(())
    };
    let mut items = Items { list: None, bar: None, ability: None };
    if let Some((w, b)) = cx.window(0x0186_a394)? {
        let mut l = List::new(scratch[0]);
        wrap(&mut l, &mut |l| list(&mut cx, l, &b))?;
        items.list = Some((w, l));
    }
    if let Some((w, b)) = cx.window(0x023c_c128)? {
        if b[0x40] != 0 {
            let mut l = List::new(scratch[1]);
            wrap(&mut l, &mut |l| bar(&mut cx, l, &b))?;
            items.bar = Some((w, l));
        }
    }
    if ability_screen {
        let mut l = List::new(scratch[2]);
        wrap(&mut l, &mut |l| ability(&mut cx, l))?;
        items.ability = Some(l);
    }
    Ok(items)
}

/// The main menu's help bar as MainMenuPatches draws it (PS2 0x00243888; the same calls as
/// menu.rs builds), and the window whose draw (+0x10) is the PC's 0x008a9350.
pub fn help(cx: &mut Ctx, scratch: u32, name: u32, thunk: u32) -> ffx_harness::Result<(List, u32)> {
    let (slot, orig) = crate::menu::atlas_slot(cx)?;
    let mut h = List::new(scratch);
    let r = |cx: &mut Ctx, a: u32| cx.r16(a);
    h.set(slot, name);
    h.c(VU1_SYNC, &[]);
    h.c(OPEN_PKT, &[]);
    let sub = (r(cx, 0x025d_1630)?, r(cx, 0x025d_162e)?, r(cx, 0x025d_162c)?);
    let bar = (r(cx, 0x025d_161c)?, r(cx, 0x025d_161a)?, r(cx, 0x025d_1618)?);
    let helpstr = (r(cx, 0x025d_160c)?, r(cx, 0x025d_160a)?);
    h.c(PLATE_LR, &[0x45, 0x46, sub.0 as u32, sub.1 as u32, sub.2 as u32, cx.r32(0x025d_1628)?, cx.r32(0x025d_1624)?]);
    h.c(PLATE_LR, &[6, 7, bar.0 as u32, bar.1 as u32, bar.2 as u32, cx.r32(0x025d_1614)?, cx.r32(0x025d_1610)?]);
    h.c(SHAPE_XY, &[0x3e, (sub.0 + 0x50) as u32, sub.1 as u32]);
    let msg = cx.r32(0x0184_1c00)?;
    if msg != 0 {
        h.c(FONT_1612, &[msg, (bar.0 + helpstr.0) as u32, (bar.1 + helpstr.1) as u32, 0]);
    }
    h.c(KICK, &[]);
    h.c(VU1_SYNC, &[]);
    h.set(slot, orig);
    let recs = cx.game.read(0x0184_0900, 32 * 0x98)?;
    // the PC's draw, or the thunk a previous @ps2items put there
    let (want, ours) = (cx.abs(0x008a_9350).to_le_bytes(), thunk.to_le_bytes());
    let off = recs.windows(4).position(|w| w == want || w == ours).expect("no window draws the help bar") as u32;
    Ok((h, cx.abs(0x0184_0900) + off))
}
