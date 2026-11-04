//! What mod/BattleHudPatches.cs draws for the battle HUD (ui.menus = ps2), as call lists: the help
//! bar (PS2 0x00218338 in place of TOBtlDrawHelpWin) and the CTB list (PS2 0x002138c8 in place of
//! TOBtlDrawCtbWindow). The game's state is read at the frame boundary; the lists hold the draws.
//! Addresses are ffx.sym VAs; evidence in docs/ui.md ("The battle HUD").
use crate::menu::{f, Ctx, List};

const SHAPE_UV: u32 = 0x0090_3ee0; // TOMkpShapeXYWHUVC2
const BOX_RGBA: u32 = 0x008f_b150; // TOMakePktBoxXYWHRGBA
const PLATE_LR: u32 = 0x008f_5e90;
const FONT_1612: u32 = 0x0090_1cb0;
const SCISSOR: u32 = 0x008f_dee0; // TOMakePktScissor(x0, y0, x1, y1)
const SCISSOR_XYWH: u32 = 0x008f_9230; // TODrawScissorXYWH
const BATTLE_LAYOUT: u32 = 0x00c5_c2c8;
const GREY: u32 = 0x8080_8080;

/// The battle layout table (mod BattleHudPatches.bshape reads it in place).
pub struct Layout(Vec<u8>);

impl Layout {
    pub fn read(cx: &mut Ctx) -> ffx_harness::Result<Self> {
        Ok(Layout(cx.game.read(BATTLE_LAYOUT, 0x1400)?))
    }
    fn r32(&self, o: usize) -> usize {
        u32::from_le_bytes(self.0[o..o + 4].try_into().unwrap()) as usize
    }
    fn r16(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.0[o], self.0[o + 1]])
    }
    /// mod BattleHudPatches.bshape: each part at (x + dx, y + dy), its own size unless w / h are
    /// given, UVs half a texel inside the cell.
    pub fn shape(&self, l: &mut List, id: u32, x: i32, y: i32, w: i32, h: i32, c0: u32, c1: u32) {
        let e = self.r32(4 + 4 * (id as usize - 400));
        for k in 0..self.r32(e) {
            let p = e + 4 + 12 * k;
            let (u, v) = (f32::from(self.0[p]), f32::from(self.0[p + 1]));
            let (dx, dy) = (i32::from(self.0[p + 2] as i8), i32::from(self.0[p + 3] as i8));
            let (pw, ph) = (f32::from(self.r16(p + 4)), f32::from(self.r16(p + 6)));
            let (mut u0, mut u1, mut v0, mut v1) = ((u + 0.5) / 256.0, (u + pw - 0.5) / 256.0, (v + 0.5) / 256.0, (v + ph - 0.5) / 256.0);
            if self.r16(p + 8) == 1 {
                std::mem::swap(&mut u0, &mut u1);
            }
            if self.r16(p + 10) == 1 {
                std::mem::swap(&mut v0, &mut v1);
            }
            let ww = if w != 0 { w as f32 } else { pw };
            let hh = if h != 0 { h as f32 } else { ph };
            l.c(SHAPE_UV, &[id, f((x + dx) as f32), f((y + dy) as f32), f(ww), f(hh), f(u0), f(v0), f(u1), f(v1), c0, c1]);
        }
    }
    fn grey(&self, l: &mut List, id: u32, x: i32, y: i32) {
        self.shape(l, id, x, y, 0, 0, GREY, GREY);
    }
}

fn boxx(l: &mut List, x: i32, y: i32, w: i32, h: i32, c: u32) {
    l.c(BOX_RGBA, &[x as u32, y as u32, w as u32, h as u32, c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff, 0x80]);
}

/// mod BattleHudPatches.frame: battle shapes 400..408 as nine pieces.
pub fn frame(t: &Layout, l: &mut List, x: i32, y: i32, w: i32, h: i32) {
    let g = GREY;
    t.shape(l, 0x194, x, y, w, h, g, g);
    t.shape(l, 0x190, x, y, 0, 0, g, g);
    t.shape(l, 0x191, x, y, 0, h, g, g);
    t.shape(l, 0x192, x, y + h, 0, 0, g, g);
    t.shape(l, 0x193, x, y, w, 0, g, g);
    t.shape(l, 0x195, x, y + h, w, 0, g, g);
    t.shape(l, 0x196, x + w, y, 0, 0, g, g);
    t.shape(l, 0x197, x + w, y, 0, h, g, g);
    t.shape(l, 0x198, x + w, y + h, 0, 0, g, g);
}

/// mod BattleHudPatches.scroll (PS2 0x001fbfc0).
pub fn scroll(l: &mut List, x: i32, y: i32, w: i32, h: i32, total: i32, shown: i32, top: i32) {
    if shown >= total {
        return;
    }
    let (ix0, iy0, ix1, iy1) = (x + 1, y + 1, x + w - 1, y + h - 1);
    let step = (iy1 - iy0) as f32 / total as f32;
    let mut t = iy0 as f32 + step * top as f32;
    let mut b = t + step * shown as f32;
    if top == 0 {
        t = iy0 as f32;
    }
    if top + shown >= total {
        b = iy1 as f32;
    }
    boxx(l, x, y, w, h, 0x70_7070);
    boxx(l, ix0, iy0, ix1 - ix0, iy1 - iy0, 0);
    boxx(l, ix0, t as i32, ix1 - ix0, b as i32 - t as i32, 0xc0_c0c0);
}

/// The help bar (mod BattleHudPatches.help), and whether the help setting is on (then the caller
/// starts "bhline" once, as the mod does when it turns on). Empty when the help window is closed.
pub fn help(cx: &mut Ctx, scratch: u32, ps2win: u32) -> ffx_harness::Result<(List, bool)> {
    let mut l = List::new(scratch);
    let (slot, orig) = crate::menu::atlas_slot(cx)?;
    let cfg_ptr = cx.call(0x0078_5290, &[])?;
    let cfg = u32::from_le_bytes(cx.game.read_abs(cfg_ptr, 4)?[..4].try_into().unwrap());
    let on = cfg & 0x400 != 0;
    let open = cx.game.read(0x0133_c93c, 1)?[0];
    eprintln!("hud help: config {cfg:#x} open {open}");
    if !on || open != 1 {
        return Ok((l, on));
    }
    let r = |cx: &mut Ctx, a: u32| cx.r16(a);
    let sub = (r(cx, 0x025d_1630)?, r(cx, 0x025d_162e)?, r(cx, 0x025d_162c)?);
    let bar = (r(cx, 0x025d_161c)?, r(cx, 0x025d_161a)?, r(cx, 0x025d_1618)?);
    l.set(slot, ps2win);
    l.c(SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
    l.c(PLATE_LR, &[0x45, 0x46, sub.0 as u32, sub.1 as u32, sub.2 as u32, cx.r32(0x025d_1628)?, cx.r32(0x025d_1624)?]);
    l.c(PLATE_LR, &[6, 7, bar.0 as u32, bar.1 as u32, bar.2 as u32, 0x4000_0000, 0x4000_0000]);
    for (fun, a) in crate::menu::help_label(cx, sub.0 + 0x50, sub.1)? {
        l.c(fun, &a);
    }
    l.c(FONT_1612, &[cx.abs(0x0133_f538), 40, 12, 0]);
    l.set(slot, orig);
    Ok((l, on))
}

/// The CTB list at dum19 (mod BattleHudPatches.ctb).
pub fn ctb(cx: &mut Ctx, scratch: u32, ps2btl: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let (x, y) = (cx.r16(0x025d_0a8a)?, cx.r16(0x025d_0a88)?);
    let row = cx.game.read(0x0134_0446, 1)?[0] as i8 as i32;
    let off = cx.r16(0x0134_0448)?;
    let slide = cx.abs(0x008a_2b10);
    l.set(slot, ps2btl);
    l.c(SCISSOR, &[0, 0, 0x200, 0x1a0]);
    frame(&t, &mut l, x, y, 0x3a, 0xae);
    if row > 0 {
        t.grey(&mut l, 0x1b6, x + 0x3a, y);
    }
    if row < 8 {
        t.grey(&mut l, 0x1b7, x + 0x3a, y + 0xae);
    }
    scroll(&mut l, x + 2, y, 4, 0xae, 16, 8, row);
    l.c(SCISSOR, &[x as u32, y as u32, (x + 0x3a) as u32, (y + 0xae) as u32]);
    let clip = (x, y, x + 0x3a, y + 0xae);
    let slots = cx.game.read(0x0134_0478, 32 * 4)?;
    let mut rows = vec![];
    for i in 0..32 {
        let e = u32::from_le_bytes(slots[4 * i..4 * i + 4].try_into().unwrap());
        if e == 0 {
            rows.push(None);
            continue;
        }
        let b = cx.game.read_abs(e, 0x18)?;
        let chr = b[1] as i8 as i32;
        let wait = i32::from_le_bytes(b[4..8].try_into().unwrap());
        let (ex, ey) = (i16::from_le_bytes([b[0x10], b[0x11]]) as i32, i16::from_le_bytes([b[0x12], b[0x13]]) as i32);
        let cb = u32::from_le_bytes(b[0x14..0x18].try_into().unwrap());
        rows.push(Some((chr, wait, ex, ey, cb)));
    }
    eprintln!("hud ctb at {x},{y} row {row} scroll {off}: {:?}", rows.iter().flatten().map(|r| (r.0, r.1, r.2, r.3)).collect::<Vec<_>>());
    for (i, r) in rows.iter().enumerate() {
        let Some((chr, wait, ex, ey, cb)) = *r else { continue };
        let (px, py) = ((ex + x + 8) as i16 as i32, (off + ey + y) as i16 as i32);
        if cb == slide {
            continue;
        }
        if i == 0 {
            t.grey(&mut l, 0x1d8, px, py);
        }
        ctb_row(cx, &t, &mut l, chr, wait, px, py, clip)?;
    }
    let depth = cx.game.read(0x023c_c092, 1)?[0] as i8 as u32;
    if cx.game.read(0x0133_c950 + 0xf0 * depth + 2, 1)?[0] == 1 {
        l.c(SCISSOR_XYWH, &[(x - 0x1e) as u32, y as u32, 0x58, 0xae]);
        let mask = cx.r32(0x0134_0470)?;
        for r in rows.iter().take(16).flatten() {
            let (chr, _, _, ey, _) = *r;
            if (mask >> (chr & 31)) & 1 == 0 {
                continue;
            }
            let py = (off + ey + y) as i16 as i32;
            let c = cx.r32(if chr < 0x14 { 0x025d_16a8 } else { 0x025d_16a4 })?;
            t.shape(&mut l, 0x1ad, x, py, 0, 0, c, c);
            t.grey(&mut l, 0x1ae, x, py);
        }
    }
    l.c(SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
    l.set(slot, orig);
    Ok(l)
}

/// mod BattleHudPatches.ctb_row (PS2 0x00213b98).
fn ctb_row(cx: &mut Ctx, t: &Layout, l: &mut List, chr: i32, wait: i32, x: i32, y: i32, clip: (i32, i32, i32, i32)) -> ffx_harness::Result<()> {
    let (dark, mid, light) = (0x7c_0f59, 0xc1_3995, 0xd6_75b5);
    // mod BattleHudPatches.clipped_box: the boxes cut to the window (the PC's scissor skips them)
    let mut bar = |x0: i32, x1: i32, c: u32| {
        let (a0, b0, a1, b1) = (x0.max(clip.0), y.max(clip.1), x1.min(clip.2), (y + 20).min(clip.3));
        if a1 > a0 && b1 > b0 {
            boxx(l, a0, b0, a1 - a0, b1 - b0, c);
        }
    };
    if wait >= 60 {
        bar(x, x + 40, dark);
    }
    if (40..60).contains(&wait) {
        bar(x, x - 40 + wait, dark);
        bar(x - 40 + wait, x + 40, mid);
    }
    if (20..40).contains(&wait) {
        bar(x, x - 20 + wait, mid);
        bar(x - 20 + wait, x + 40, light);
    }
    if wait < 20 {
        bar(x, x + wait, light);
    }
    let fx = x + wait.min(20);
    if chr < 8 {
        let young = chr == 6 && (cx.call(0x0086_c470, &[])? as i32) < 0x434;
        t.grey(l, if young { 0x1d4 } else { 0x1b8 + chr as u32 }, fx, y);
    }
    if (8..18).contains(&chr) {
        t.grey(l, 0x1c0, fx, y);
    }
    if chr >= 0x14 {
        let ty = cx.call(0x0079_ae30, &[chr as u32])? as i32;
        let id = match ty {
            0x14 => 0x1cb + cx.call(0x0079_ae10, &[chr as u32])?,
            0x15 => 0x1c2,
            0x16 => 0x1c3 + cx.call(0x0079_ae10, &[chr as u32])?,
            0x17 => 0x1d3,
            _ => 0x1c1,
        };
        t.grey(l, id, fx, y);
    }
    Ok(())
}

const SHAPE_XY: u32 = 0x0090_3a10; // TOMkpShapeXY (meswin ids: expanded by menu::expand_shapes)
const BOX_C2: u32 = 0x008f_3db0; // TODrawBoxXYWHC2
const BOX_FW: u32 = 0x008f_3eb0; // TODrawBoxXYWHC2FW(x, y, w, h, c0, c1, full)

/// The party list (mod BattleHudPatches.status). `fnt` the ps2fnt name for fonts M and S.
pub fn status(cx: &mut Ctx, scratch: u32, ps2win: u32, ps2btl: u32, fnt: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::atlas_slot(cx)?;
    let (bslot, borig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let fo = crate::menu::Fonts { slot, win: ps2win, fnt };
    l.c(SCISSOR, &[0, 0, 0x200, 0x1a0]);
    l.set(slot, ps2win);
    l.set(bslot, ps2btl);
    let mode = cx.game.read(0x0133_d782, 1)?[0] as i8 as i32;
    let pt = |cx: &mut Ctx, a: u32| -> ffx_harness::Result<(i32, i32)> { Ok((cx.r16(a + 2)?, cx.r16(a)?)) };
    let (x0, y0, w, pitch) = (cx.r16(0x025d_0aee)?, cx.r16(0x025d_0aec)?, cx.r16(0x025d_0aea)?, cx.r16(0x025d_0ae8)?);
    let n = pt(cx, 0x025d_0ad4)?;
    let d3 = pt(cx, 0x025d_0ad0)?;
    let [d4, d5, d6, d7, d8] = [0x025d_0ac4, 0x025d_0ac0, 0x025d_0abc, 0x025d_0ab8, 0x025d_0ab4].map(|a| pt(cx, a).unwrap());
    let (g_w, g_h) = (cx.r16(0x025d_0ab2)?, cx.r16(0x025d_0ab0)?);
    let (f_x, f_y, f_w, f_h) = (cx.r16(0x025d_0aa8)?, cx.r16(0x025d_0aa6)?, cx.r16(0x025d_0aa4)?, cx.r16(0x025d_0aa2)?);
    let targets = cx.r32(0x0133_d178)?;
    let active = cx.r16(0x023c_c088)?;
    let cols = [cx.r32(0x025d_0ae4)?, cx.r32(0x025d_0ae0)?, cx.r32(0x025d_0adc)?, cx.r32(0x025d_0ad8)?];
    eprintln!("hud status mode {mode} at {x0},{y0} w {w} pitch {pitch} name {n:?} d3 {d3:?} active {active} targets {targets:#x}");
    for i in 0..3 {
        let chr = u16::from_le_bytes(cx.game.read(0x0133_f7ac + 2 * i as u32, 2)?[..2].try_into().unwrap()) as i32;
        if chr == 0xff {
            continue;
        }
        let row = cx.game.read(0x0133_f7d8 + 0x90 * i as u32, 0x90)?;
        let i32at = |o: usize| i32::from_le_bytes(row[o..o + 4].try_into().unwrap());
        let (x, y) = (x0, y0 + i * pitch);
        let (bx, by) = (x + d3.0, y + d3.1);
        let at = |p: (i32, i32)| (bx + p.0, by + p.1);
        let (lp, vp, l2, v2, g) = (at(d4), at(d5), at(d6), at(d7), at(d8));
        let colour = u32::from(row[0x19]);
        let targeted = chr < 32 && (targets >> chr) & 1 != 0;
        let hl = if mode == 3 { (active == chr).then_some((cols[0], cols[1])) } else if (0..=2).contains(&mode) && targeted { Some((cols[2], cols[3])) } else { None };
        if let Some((c0, c1)) = hl {
            t.shape(&mut l, 0x199, x, y, 0, 0, c0, c0);
            t.shape(&mut l, 0x19a, x + 24, y, w - 24, 0, c0, c1);
            l.c(BOX_C2, &[x as u32, y as u32, 50, 1, 0x0080_8080, 0x80a0_a0a0]);
            l.c(BOX_C2, &[(x + 50) as u32, y as u32, w as u32, 1, 0x80a0_a0a0, 0x80a0_a0a0]);
            l.c(BOX_C2, &[x as u32, (y + 19) as u32, 50, 1, 0x0080_8080, 0x8000_0000]);
            l.c(BOX_C2, &[(x + 50) as u32, (y + 19) as u32, w as u32, 1, 0x8000_0000, 0x8000_0000]);
        }
        if !(0..=3).contains(&mode) {
            continue;
        }
        let name = cx.call(0x0079_af10, &[chr as u32])?;
        let nw = cx.text_width(name)?;
        l.c(FONT_1612, &[name, (x + n.0 - nw) as u32, (y + n.1) as u32, colour]);
        match mode {
            3 => {
                l.c(SHAPE_XY, &[0xe9, lp.0 as u32, lp.1 as u32]);
                crate::menu::value_m(&mut l, &fo, i64::from(i32at(0x10)), vp.0, vp.1, colour);
                l.c(SHAPE_XY, &[0xea, l2.0 as u32, l2.1 as u32]);
                crate::menu::value_m(&mut l, &fo, i64::from(i32at(0x28)), v2.0, v2.1, u32::from(row[0x31]));
                let gv = i32at(0x38);
                boxx(&mut l, g.0, g.1, g_w, g_h, 0);
                let (fx, fy) = (g.0 + f_x - 1, g.1 + f_y);
                if f_w != 0 && gv != 0 {
                    if gv >= f_w {
                        boxx(&mut l, fx, fy, f_w, f_h, 0x2e_79f7);
                    } else {
                        l.c(BOX_FW, &[fx as u32, fy as u32, gv as u32, f_h as u32, 0x808a_f8ff, 0x8017_77e3, f_w as u32]);
                    }
                }
            }
            0 | 1 => {
                l.c(SHAPE_XY, &[if mode == 0 { 0xe9 } else { 0xea }, lp.0 as u32, lp.1 as u32]);
                crate::menu::value_m(&mut l, &fo, i64::from(i32at(if mode == 0 { 0x10 } else { 0x28 })), vp.0, vp.1, colour);
                crate::menu::font_m(&mut l, &fo, "/", vp.0 + 8, vp.1, 0);
                let max = cx.call(if mode == 0 { 0x0079_adf0 } else { 0x0079_ae70 }, &[chr as u32])?;
                crate::menu::value_m(&mut l, &fo, i64::from(max as i32), v2.0, v2.1, 0x80);
            }
            _ => {
                let count = i16::from_le_bytes([row[0x40], row[0x41]]);
                if count != 0 {
                    let k = i16::from_le_bytes([row[0x3e], row[0x3f]]) as usize;
                    let id = u16::from_le_bytes([row[0x42 + 2 * k], row[0x43 + 2 * k]]);
                    let s = cx.call(0x0078_fcf0, &[1, u32::from(id), 1])?;
                    l.c(FONT_1612, &[s, lp.0 as u32, vp.1 as u32, 0]);
                }
            }
        }
    }
    l.set(slot, orig);
    l.set(bslot, borig);
    Ok(l)
}

impl Layout {
    /// mod BattleSeqPatches.prim_layout: a sequence primitive's shape at (x, y), scaled sx / sy %.
    pub fn scaled(&self, l: &mut List, id: u32, x: f32, y: f32, sx: f32, sy: f32, c: u32) {
        let e = self.r32(4 + 4 * (id as usize - 400));
        for k in 0..self.r32(e) {
            let p = e + 4 + 12 * k;
            let (u, v) = (f32::from(self.0[p]), f32::from(self.0[p + 1]));
            let (dx, dy) = (f32::from(self.0[p + 2] as i8), f32::from(self.0[p + 3] as i8));
            let (w, h) = (f32::from(self.r16(p + 4)), f32::from(self.r16(p + 6)));
            let (mut u0, mut u1, mut v0, mut v1) = ((u + 0.5) / 256.0, (u + w - 0.5) / 256.0, (v + 0.5) / 256.0, (v + h - 0.5) / 256.0);
            if self.r16(p + 8) == 1 {
                std::mem::swap(&mut u0, &mut u1);
            }
            if self.r16(p + 10) == 1 {
                std::mem::swap(&mut v0, &mut v1);
            }
            l.c(SHAPE_UV, &[id, f(x + dx * sx / 100.0), f(y + dy * sy / 100.0), f(w * sx / 100.0), f(h * sy / 100.0), f(u0), f(v0), f(u1), f(v1), c, c]);
        }
    }
}

/// mod BattleHudPatches.INFO_ROW (PS2 0x00570e48).
const INFO_ROW: [u32; 28] = [
    0x00ea_eaea, 0x6ad2_ac23, 0x6a9f_491f, 0x00ea_eaea,
    0x07ba_3f96, 0x73aa_72bd, 0x7349_1e40, 0x0783_4270,
    0x0773_73b8, 0x7367_67ba, 0x731e_1e51, 0x074d_4d7b,
    0x076d_89b7, 0x7346_88b3, 0x7324_2d5e, 0x074e_5580,
    0x0d60_2b32, 0x6660_2b32, 0x6660_2b32, 0x0d60_2b32,
    0x0d60_2b32, 0x6660_2b32, 0x6660_2b32, 0x0d60_2b32,
    0x0063_1c4e, 0x8063_1c4e, 0x8063_1c4e, 0x0063_1c4e,
];

/// mod BattleHudPatches.info_row (PS2 0x002198e0).
fn info_row(l: &mut List, x: i32, y: i32, w: i32, h: i32, kind: usize) {
    let c = &INFO_ROW[4 * kind..4 * kind + 4];
    l.c(BOX_C2, &[x as u32, y as u32, 40, h as u32, c[0], c[1]]);
    l.c(BOX_C2, &[(x + 40) as u32, y as u32, (w - 80) as u32, h as u32, c[1], c[2]]);
    l.c(BOX_C2, &[(x + w - 40) as u32, y as u32, 40, h as u32, c[2], c[3]]);
}

/// The target info window (mod BattleHudPatches.info, PS2 0x00218a80(chr, 0x14, 0x46)) for the
/// current target (0x0133d1f4); empty while g_BtlInfoStatusShown is 0. The status shown is the one
/// g_BtlInfoStatus points at now (the mod ticks it every 21 frames).
pub fn info(cx: &mut Ctx, scratch: u32, ps2win: u32, ps2btl: u32, fnt: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let shown = cx.game.read(0x0133_c93e, 1)?[0];
    let chr = cx.r16(0x0133_d1f4)? as u32 & 0xffff;
    eprintln!("hud info: shown {shown} chr {chr:#x}");
    if shown == 0 {
        return Ok(l);
    }
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::atlas_slot(cx)?;
    let (bslot, borig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let fo = crate::menu::Fonts { slot, win: ps2win, fnt };
    let mut f = [0u32; 4];
    for (k, v) in f.iter_mut().enumerate() {
        *v = cx.call(0x0089_7650, &[chr, k as u32])? & 0xff;
    }
    let european = cx.call(0x0064_10e0, &[])? != 0;
    let st = cx.game.read(0x0133_f5c0, 0x58)?;
    let s16 = |o: usize| i16::from_le_bytes([st[o], st[o + 1]]) as i32;
    let count = s16(6);
    let mut h = 0x16 + 0x10 * f.iter().filter(|v| **v != 0).count() as i32;
    if count > 0 {
        h += if european { 0x18 } else { 0x12 };
    }
    eprintln!("hud info: flags {f:x?} count {count} h {h}");
    let (mut x, mut y) = (0x14, 0x46);
    l.set(slot, ps2win);
    l.set(bslot, ps2btl);
    l.c(SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
    frame(&t, &mut l, x, y, 0xa9, h);
    t.grey(&mut l, 0x1aa, x + 0x24, y);
    x += 3;
    y += 3;
    l.c(SHAPE_XY, &[0xe9, (x + 2) as u32, (y + 2) as u32]);
    let hp = cx.call(0x0079_add0, &[chr])? as i32;
    let max = cx.call(0x0079_adf0, &[chr])? as i32;
    crate::menu::value_s(&mut l, &fo, i64::from(hp), x + 0x54, y + 2, 0);
    crate::menu::font_s(&mut l, &fo, "/", x + 0x5a, y + 2, 0);
    crate::menu::value_s(&mut l, &fo, i64::from(max), x + 0xa0, y + 2, 0);
    y += 0x10;
    for (k, &v) in f.iter().enumerate() {
        if v == 0 {
            continue;
        }
        info_row(&mut l, x, y, 0xa3, 0x10, k);
        info_row(&mut l, x, y, 0xa3, 1, 6);
        t.grey(&mut l, 0x1af + k as u32, x + 24, y);
        for (bit, id) in [(1, 0x1d9), (4, 0x1da), (8, 0x1db), (2, 0x1dc)] {
            if v & bit == 0 {
                t.grey(&mut l, id, x + 24, y);
            }
        }
        y += 0x10;
    }
    if count > 0 {
        y += 2;
        info_row(&mut l, x, y, 0xa3, if european { 0x16 } else { 0x10 }, 5);
        let k = s16(4).clamp(0, count - 1) as usize;
        let id = u16::from_le_bytes([st[8 + 2 * k], st[9 + 2 * k]]);
        let text = cx.call(0x0078_fcf0, &[1, u32::from(id), 1])?;
        let w = cx.text_width(text)?;
        l.c(FONT_1612, &[text, (x + (0xa9 - w) / 2) as u32, (y + 2) as u32, 0]);
    }
    l.set(slot, orig);
    l.set(bslot, borig);
    Ok(l)
}

/// The target's info line in the help bar (mod BattleHudPatches.set_info_mes / draw_info_mes, PS2
/// 0x00219550 / 0x002192a8), `scrolled` pixels into its scroll (the mod moves it 2 a frame; the
/// PS2 shot of compare-battle-info catches it 60 in). Empty while g_BtlInfoWinShown is 0.
pub fn info_mes(cx: &mut Ctx, scratch: u32, ps2win: u32, scrolled: i32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let shown = cx.game.read(0x0133_c93d, 1)?[0];
    let chr = cx.r16(0x0133_d1f4)? as u32 & 0xffff;
    if shown == 0 {
        return Ok(l);
    }
    let (slot, orig) = crate::menu::atlas_slot(cx)?;
    let x0 = cx.text_width(cx.abs(0x0133_f538))? + 0x50;
    let text = cx.call(0x0078_f7c0, &[chr, 1])?;
    let w = cx.text_width(text)?;
    let x = if w < 0x1ae - x0 { x0 } else { x0 - scrolled };
    eprintln!("hud info line: box x0 {x0} text w {w} at x {x}");
    l.set(slot, ps2win);
    l.c(SCISSOR, &[x0 as u32, 12, 0x1ae, 0x24]);
    l.c(FONT_1612, &[text, x as u32, 12, 0]);
    l.c(SCISSOR, &[0, 0, 0x1ff, 0x19f]);
    l.set(slot, orig);
    Ok(l)
}

/// A scene colour as the scene dump prints it (rrggbbaa) -> the record's word (bytes r g b a).
fn rgba(v: u32) -> u32 {
    v.swap_bytes()
}

/// mod BattleSeqPatches.line: a horizontal scene line as a one-pixel gradient box.
fn hline(l: &mut List, x0: i32, x1: i32, y: i32, c0: u32, c1: u32) {
    l.c(BOX_C2, &[x0 as u32, y as u32, (x1 - x0) as u32, 1, rgba(c0), rgba(c1)]);
}

/// The battle sequence scenes the mod starts and draws (BattleSeqPatches: open_main, prim_layout,
/// line) for MsCheckLeftWindow 2, at rest (the harness cannot run the hooks, so their resting
/// frames are drawn as fixed calls): `bnormall` (layer 0, before the windows: lines, the purple
/// bar, the row bands 418, the orange glow) and `bnormalllay`'s footer (layer 1, after them: the
/// line at y 407, the ring 417, the BEFEHL brush 415, the ">>>" plate 411). The Overdrive tab
/// (413) is menu::od_tab_calls. Returns (layer 0, layer 1); the battle sheet is swapped to `btl`
/// around the shapes.
pub fn footer(cx: &mut Ctx, scratch: u32, btl: u32) -> ffx_harness::Result<(List, List)> {
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let mut a = List::new(scratch);
    hline(&mut a, 0, 109, 329, 0x0000_0050, 0x0000_0050);
    hline(&mut a, 109, 150, 329, 0x0000_0050, 0x0000_0000);
    a.c(BOX_C2, &[0, 330, 137, 9, rgba(0x4e0f_5e64), rgba(0x4e0f_5e00)]);
    hline(&mut a, 0, 161, 340, 0x0000_003c, 0x0000_003c);
    a.set(slot, btl);
    for y in [341.0, 361.0, 381.0] {
        t.scaled(&mut a, 418, 80.0, y, 90.0, 100.0, rgba(0x8080_8046));
    }
    a.set(slot, orig);
    hline(&mut a, 0, 148, 336, 0x0000_0050, 0x0000_0050);
    hline(&mut a, 148, 183, 336, 0x0000_0050, 0x0000_0000);
    hline(&mut a, 0, 110, 333, 0xffff_ff50, 0xffff_ff50);
    hline(&mut a, 110, 147, 333, 0xffff_ff50, 0xffff_ff00);
    a.c(BOX_C2, &[0, 339, 100, 62, rgba(0xff64_0064), rgba(0xff46_0000)]);
    let mut b = List::new(scratch + 0x400);
    b.c(SCISSOR, &[0, 0, 0x200, 0x1a0]);
    hline(&mut b, 0, 168, 407, 0x0000_0050, 0x0000_0050);
    hline(&mut b, 168, 208, 407, 0x0000_0050, 0x0000_0000);
    b.set(slot, btl);
    t.scaled(&mut b, 417, 26.0, 407.0, 100.0, 100.0, rgba(0x8080_8050));
    t.scaled(&mut b, 415, -2.0, 392.0, 100.0, 100.0, rgba(0x8080_8080));
    t.scaled(&mut b, 411, 90.0, 402.0, 100.0, 100.0, rgba(0x8080_8080));
    b.set(slot, orig);
    Ok((a, b))
}
