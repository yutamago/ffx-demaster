//! The Overdrive screen as mod/OverdriveMenuPatches.cs draws it (ui.menus = ps2): the record's draw
//! (PS2 0x002312d0 for 0x008d09b0: the party plate and the Overdrive plate), the View / Set Mode
//! windows (PS2 0x00230708 for MenuLimit_WindowDraw_008d16f0), plus the sub-command bar and the
//! help bar as ItemMenuPatches / MainMenuPatches draw them (items.rs). The same arithmetic as the
//! C#; layout words by name through TkMn2SeqGet*. Evidence in docs/ui.md ("The Overdrive screen").
use crate::menu::{Ctx, List, cursor_parts, f, font_s, value_m, value_s};
use crate::screens::{Draw, Env};

const BOX_C2: u32 = 0x008f3db0;
const BOX_RGBA: u32 = 0x008fb150;
const PLATE_LR: u32 = 0x008f5e90;
const PLATE_RL: u32 = 0x008f5e20;
const SHAPE_XY: u32 = 0x00903a10;
const SHAPE_XYWH: u32 = 0x00903b70;
const FONT_1612: u32 = 0x00901cb0;
const ICON: u32 = 0x008e6af0;
const SCISSOR: u32 = 0x008fdee0;
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const LABEL: u32 = 0x008f8d50; // TOMkpShape2ECCCell_008f8d50(n, f x, f y, f w, f h, alpha)
const WIN_SCISSOR: u32 = 0x008c0f70; // TkMn2DrawMenuWindowScissor(y, h, part, ratio)
const GREY: u32 = 0x8080_8080;
const SHADOW: u32 = 0x8000_0000;
const ROW: i32 = 32;

const RECORD_DRAW_SLOT: u32 = 0x00c5a76c; // the Overdrive record (0x00c5a768) +4
const WINDOWS: [u32; 2] = [0x0186a714, 0x0186a718]; // the View and Set Mode windows (+0x10 MenuLimit_WindowDraw_008d16f0)

fn i16_at(b: &[u8], o: usize) -> i32 {
    i32::from(i16::from_le_bytes([b[o], b[o + 1]]))
}
fn u16_at(b: &[u8], o: usize) -> i32 {
    i32::from(u16::from_le_bytes([b[o], b[o + 1]]))
}

struct Cx<'a, 'b> {
    cx: &'a mut Ctx<'b>,
    env: &'a Env,
}

impl Cx<'_, '_> {
    fn name(&mut self, s: &str) -> ffx_harness::Result<u32> {
        let at = self.env.out + 0x200;
        let hex: String = s.bytes().chain([0]).map(|b| format!("{b:02x}")).collect();
        self.cx.game.command(&format!("write {:#x} {hex}", at.wrapping_sub(self.cx.delta)))?;
        Ok(at)
    }
    fn halves(&mut self, n: usize) -> ffx_harness::Result<Vec<i32>> {
        let b = self.cx.game.read(self.env.out.wrapping_sub(self.cx.delta), 2 * n)?;
        Ok((0..n).map(|k| i16_at(&b, 2 * k)).collect())
    }
    /// TkMn2SeqGetPos(name)
    fn pos(&mut self, s: &str) -> ffx_harness::Result<(i32, i32)> {
        let n = self.name(s)?;
        self.cx.call(0x008c_2b50, &[n, self.env.out])?;
        let v = self.halves(2)?;
        Ok((v[0], v[1]))
    }
    /// TkMn2SeqGetRect(name)
    fn rect(&mut self, s: &str) -> ffx_harness::Result<(i32, i32, i32, i32)> {
        let n = self.name(s)?;
        self.cx.call(0x008c_2b90, &[n, self.env.out])?;
        let v = self.halves(4)?;
        eprintln!("{s} {v:?}");
        Ok((v[0], v[1], v[2], v[3]))
    }
    /// TkMn2SeqGetColor(name)
    fn colour(&mut self, s: &str) -> ffx_harness::Result<u32> {
        let n = self.name(s)?;
        let c = self.cx.call(0x008c_2ad0, &[n])?;
        eprintln!("{s} {c:08x}");
        Ok(c)
    }
    fn call(&mut self, va: u32, a: u32) -> ffx_harness::Result<u32> {
        self.cx.call(va, &[a])
    }
    fn cursor(&self, l: &mut List, x: i32, y: i32) {
        for (fun, a) in cursor_parts(x, y, self.env.dum75) {
            l.c(fun, &a);
        }
    }
    /// A point of TOSeqInitParam's layout (mirrored: y, x).
    fn point(&mut self, va: u32) -> ffx_harness::Result<(i32, i32)> {
        Ok((self.cx.r16(va + 2)?, self.cx.r16(va)?))
    }
}

/// PS2 0x00225f38 -> 0x00200c60: the front-line band and the party plate.
fn status_plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: u32) -> ffx_harness::Result<()> {
    let fo = cx.env.fo;
    if (cx.cx.r32(0x0184_1c2c)? >> chr) & 1 != 0 {
        let (nx, ny, nh) = (cx.cx.r16(0x023c_c144)?, cx.cx.r16(0x023c_c146)?, cx.cx.r16(0x023c_c13e)?);
        let (c0, c1) = (cx.cx.r32(0x023c_c148)?, cx.cx.r32(0x023c_c140)?);
        l.c(BOX_C2, &[(x + nx) as u32, (y + ny) as u32, 512, nh as u32, c0, c1]);
    }
    if chr >= 8 {
        return Ok(());
    }
    let nw = cx.cx.r16(0x025d_1672)?;
    let (c0, c1) = (cx.cx.r32(0x025d_166c)?, cx.cx.r32(0x025d_1668)?);
    l.c(PLATE_LR, &[4, 5, x as u32, y as u32, nw as u32, c0, c1]);
    let namestr = cx.point(0x025d_1662)?;
    let name = cx.call(0x008a_c850, chr)?;
    let colour = cx.call(0x008a_99f0, chr)?;
    l.c(FONT_1612, &[name, (x + namestr.0) as u32, (y + namestr.1) as u32, colour]);
    let (hptex, hpstr, mptex, mpstr) = (cx.point(0x025d_165e)?, cx.point(0x025d_165a)?, cx.point(0x025d_1656)?, cx.point(0x025d_1652)?);
    let (hp, hpm) = (cx.call(0x008a_98c0, chr)?, cx.call(0x008a_9990, chr)?);
    let (mp, mpm) = (cx.call(0x008a_9970, chr)?, cx.call(0x008a_99b0, chr)?);
    l.c(SHAPE_XY, &[0x21, (x + hptex.0) as u32, (y + hptex.1) as u32]);
    font_s(l, &fo, &format!("{hp:5}/{hpm:5}"), x + hpstr.0, y + hpstr.1, 0x25);
    l.c(SHAPE_XY, &[0x22, (x + mptex.0) as u32, (y + mptex.1) as u32]);
    font_s(l, &fo, &format!("{mp:4}/{mpm:4}"), x + mpstr.0, y + mpstr.1, 0x25);
    l.c(SHAPE_XYWH, &[0xe7, 0, y as u32, 512, 0, GREY, GREY]);
    let (slv, spstr) = (cx.point(0x025d_164e)?, cx.point(0x025d_164a)?);
    l.c(SHAPE_XY, &[0x107, (x + slv.0 + 0x68) as u32, (y + slv.1) as u32]);
    let sp = cx.call(0x008a_9b80, chr)?;
    value_m(l, &fo, i64::from(sp), x + slv.0 + spstr.0, y + slv.1 + spstr.1, 0);
    let ap = cx.call(0x0078_4e90, chr)?.wrapping_sub(cx.call(0x0078_52b0, chr)?);
    value_s(l, &fo, i64::from(ap as i32), x + slv.0 + 0x8b, y + slv.1 - 13, 0x25);
    Ok(())
}

fn mix(a: u32, b: u32, f: i32) -> u32 {
    (0..4).map(|k| 8 * k).fold(0, |r, s| {
        let v = (((a >> s) & 0xff) as i32 * (0x1000 - f) + ((b >> s) & 0xff) as i32 * f) / 4096;
        r | (v as u32) << s
    })
}

/// PS2 0x00229d60: the Overdrive plate's pieces moved by (x, y) - rect_lgplt.
fn gauge_plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, chr: u32) -> ffx_harness::Result<()> {
    let lgplt = cx.rect("rect_lgplt")?;
    let (dx, dy) = (x - lgplt.0, y - lgplt.1);
    let ltb = cx.rect("rect_ltbplt")?;
    let cb = cx.colour("col_ltbplt")?;
    l.c(BOX_C2, &[(ltb.0 + dx) as u32, (ltb.1 + dy) as u32, ltb.2 as u32, ltb.3 as u32, cb, cb]);
    let lt = cx.rect("rect_ltplt")?;
    let (c0, c1) = (cx.colour("col_ltplt0")?, cx.colour("col_ltplt1")?);
    l.c(BOX_C2, &[(lt.0 + dx) as u32, (lt.1 + dy) as u32, lt.2 as u32, lt.3 as u32, c0, c1]);
    let (c0, c1) = (cx.colour("col_lgplt0")?, cx.colour("col_lgplt1")?);
    l.c(PLATE_RL, &[0xd8, 0xd9, x as u32, y as u32, lgplt.2 as u32, c0, c1]);
    let typ = cx.pos("pos_typshape")?;
    l.c(SHAPE_XY, &[0x109, (lt.0 + typ.0 + dx) as u32, (lt.1 + typ.1 + dy) as u32]);
    let lgs = cx.pos("pos_lgshape")?;
    l.c(SHAPE_XY, &[0x108, (x + lgs.0 + 16) as u32, (y + lgs.1) as u32]);
    let lg = cx.rect("rect_lg")?;
    let (g, max) = (cx.call(0x008a_9930, chr)? as i32, cx.call(0x008a_9950, chr)? as i32);
    eprintln!("overdrive gauge {g} / {max}");
    if max != 0 && g != 0 {
        let fr = if g >= max { 0x1000 } else { (g << 12) / max };
        let (c0, c1) = if fr == 0x1000 {
            let c = cx.colour("col_lmtgf0")?;
            (c, c)
        } else {
            let c0 = cx.colour("col_lmtg0")?;
            (c0, mix(c0, cx.colour("col_lmtg1")?, fr))
        };
        l.c(BOX_C2, &[(lg.0 + dx) as u32, (lg.1 + dy) as u32, (lg.2 * fr >> 12) as u32, lg.3 as u32, c0, c1]);
    }
    let st = cx.pos("pos_ltstr")?;
    let id = cx.call(0x008c_1bd0, chr)?;
    let name = cx.call(0x008d_dfc0, id + 0x1033)?;
    l.c(FONT_1612, &[name, (ltb.0 + st.0 + dx) as u32, (ltb.1 + st.1 + dy) as u32, 0]);
    Ok(())
}

fn record(cx: &mut Cx, l: &mut List) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let chr = cx.cx.call(0x008a_9860, &[])?;
    let p = cx.pos("pos_4nameplt")?;
    eprintln!("overdrive chr {chr} pos_4nameplt {p:?}");
    status_plate(cx, l, p.0, p.1, chr)?;
    let g = cx.rect("rect_2lgplt")?;
    gauge_plate(cx, l, g.0, g.1, chr)?;
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// PS2 0x00230328.
fn row(cx: &mut Cx, l: &mut List, w: &[u8], x: i32, y: i32, idx: i32, mode: i32) -> ffx_harness::Result<()> {
    let y = if mode == 0 { y - i16_at(w, 0x46) / 128 } else { y };
    if idx < 0 || idx >= u16_at(w, 0x30) {
        return Ok(());
    }
    let (c0, c1) = (cx.cx.r32(0x025d_1690)?, cx.cx.r32(0x025d_168c)?);
    l.c(SHAPE_XYWH, &[0xda, x as u32, y as u32, 0, 0, c0, c0]);
    l.c(SHAPE_XYWH, &[0xdb, (x + 16) as u32, y as u32, 209, 0, c0, c1]);
    let pm = cx.pos("pos_pmshape")?;
    l.c(SHAPE_XY, &[0xd7, (x + pm.0 + 0x58) as u32, (y + pm.1) as u32]);
    let e = cx.cx.game.read(0x0159_7770 + 4 * idx as u32, 4)?;
    if e[3] == 0 {
        return Ok(());
    }
    let id = u32::from(u16::from_le_bytes([e[0], e[1]]));
    match e[2] {
        1 => {
            let rec = cx.cx.call(0x0079_a4b0, &[id, cx.env.out])?;
            let strings = cx.cx.r32(cx.env.out.wrapping_sub(cx.cx.delta))?;
            let r = cx.cx.game.read(rec.wrapping_sub(cx.cx.delta), 0x16)?;
            let name = strings + u32::from(u16::from_le_bytes([r[0], r[1]]));
            l.c(ICON, &[f((x + 12) as f32), f((y + 1) as f32), f(15.0), f(18.0), u32::from(r[0x14]), 0x80, 0x80, 0x80, 0x80]);
            l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, 0]);
        }
        2 => {
            let name = cx.call(0x0087_3b50, id)?;
            l.c(FONT_1612, &[name, (x + 12) as u32, (y + 1) as u32, 0]);
        }
        3 => {
            let name = cx.call(0x008d_dfc0, id + 0x1033)?;
            l.c(FONT_1612, &[name, (x + 0x1d) as u32, (y + 1) as u32, 0]);
        }
        _ => {}
    }
    Ok(())
}

fn rows(cx: &mut Cx, l: &mut List, w: &[u8], mode: i32, top: i32, lp: (i32, i32), rp: (i32, i32), dy: i32) -> ffx_harness::Result<()> {
    let mut y = lp.1 + dy;
    for k in 0..9 {
        row(cx, l, w, lp.0, y, 2 * top - 2 + 2 * k, mode)?;
        row(cx, l, w, rp.0, y, 2 * top - 1 + 2 * k, mode)?;
        y += ROW;
    }
    Ok(())
}

/// PS2 0x00230708 (title or mode label) and 0x002305c0 (rows, scroll bar, cursor).
fn window(cx: &mut Cx, l: &mut List, w: &[u8]) -> ffx_harness::Result<()> {
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let chr = cx.cx.call(0x008a_9860, &[])?;
    let kind = u32::from_le_bytes(w[0x5c..0x60].try_into().unwrap());
    if kind != 0 {
        let r = cx.rect("rect_lwplt")?;
        let (c0, c1) = (cx.colour("col_lwplt0")?, cx.colour("col_lwplt1")?);
        l.c(BOX_C2, &[r.0 as u32, r.1 as u32, r.2 as u32, r.3 as u32, c0, c1]);
        let list = cx.cx.call(0x0079_0200, &[0, cx.env.out])?;
        let k = cx.cx.call(0x008d_2dc0, &[list, 5, chr, 0x48])?;
        let id = u32::from(u16::from_le_bytes(cx.cx.game.read((list + 2 + 4 * k).wrapping_sub(cx.cx.delta), 2)?[..2].try_into().unwrap()));
        let name = cx.call(0x008c_1a20, id)?;
        let tw = i32::from(cx.call(0x008c_2cb0, name)? as u16 as i16);
        eprintln!("overdrive title id {id:#x} width {tw}");
        l.c(FONT_1612, &[name, (r.0 + (r.2 - tw) / 2) as u32, (r.1 + 4) as u32, 0]);
    } else {
        let r = cx.rect("rect_lmojiplt")?;
        l.c(PLATE_RL, &[0xda, 0xdb, r.0 as u32, r.1 as u32, 0xd5, SHADOW, SHADOW]);
        l.c(LABEL, &[0, f((r.0 + 0x15) as f32), f((r.1 + 5) as f32), f(170.0), f(14.0), 0x80]);
    }
    let lp = cx.pos("pos_2llplt")?;
    let rp = cx.pos("pos_2lrplt")?;
    let (top, next, fr) = (i16_at(w, 0x32), i16_at(w, 0x34), i16_at(w, 0x46));
    let (count, shown) = (u16_at(w, 0x30), u16_at(w, 0x3a));
    eprintln!("overdrive window kind {kind} count {count} top {top} next {next} shown {shown} pos {lp:?} {rp:?}");
    if top == next {
        l.c(WIN_SCISSOR, &[rp.1 as u32, 0xe0, 0, fr as u32]);
        rows(cx, l, w, 0, top, lp, rp, -ROW)?;
    } else {
        l.c(WIN_SCISSOR, &[rp.1 as u32, 0xe0, 1, fr as u32]);
        rows(cx, l, w, 1, top, lp, rp, -ROW)?;
        l.c(WIN_SCISSOR, &[rp.1 as u32, 0xe0, 2, fr as u32]);
        rows(cx, l, w, 2, next, lp, rp, -(fr * 0xe0) / 4096 - ROW)?;
    }
    l.c(SCISSOR, &[0, 0, 512, 416]);
    let s = cx.rect("rect_lscroll")?;
    let total = (count + 1) >> 1;
    if shown < total {
        l.c(BOX_RGBA, &[s.0 as u32, s.1 as u32, s.2 as u32, s.3 as u32, 0x11, 0x11, 0x11, 0x80]);
        let t = if top == 0 { s.1 as f32 } else { s.1 as f32 + s.3 as f32 * top as f32 / total as f32 };
        let b = if top + shown >= total { (s.1 + s.3) as f32 } else { s.1 as f32 + s.3 as f32 * (top + shown) as f32 / total as f32 };
        l.c(BOX_RGBA, &[(s.0 + 1) as u32, t as i32 as u32, (s.2 - 2) as u32, (b as i32 - t as i32) as u32, 0xf0, 0xf0, 0xf0, 0x80]);
    }
    if (w[0x45] as i8) >= 0 {
        let c = i16_at(w, 0x48);
        let x = 12 + if c & 1 != 0 { rp.0 } else { lp.0 };
        cx.cursor(l, x, rp.1 + 1 + ROW * ((c >> 1) - top));
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

pub fn build(cx0: &mut Ctx, env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    let mut draws = vec![];
    let mut next = scratch.iter().copied();
    {
        let mut cx = Cx { cx: &mut *cx0, env };
        let mut l = List::new(next.next().expect("thunk"));
        record(&mut cx, &mut l)?;
        draws.push(Draw { target: RECORD_DRAW_SLOT, list: l });
        for wv in WINDOWS {
            let w = cx.cx.r32(wv)?;
            if w == 0 {
                continue;
            }
            let b = cx.cx.game.read(w.wrapping_sub(cx.cx.delta), 0x98)?;
            let draw = u32::from_le_bytes(b[0x10..0x14].try_into().unwrap());
            eprintln!("overdrive window {wv:#x} at {w:#x} draw {draw:#x}");
            let mut l = List::new(next.next().expect("thunk"));
            window(&mut cx, &mut l, &b)?;
            draws.push(Draw { target: (w + 0x10).wrapping_sub(cx.cx.delta), list: l });
        }
    }
    // the sub-command bar (View / Set Mode) as ItemMenuPatches draws it, and the help bar
    if let Some(s) = next.next() {
        let items = crate::items::build(cx0, [s, s + 0x2000, s + 0x4000], env.out, env.fo.win, env.fo.fnt, false)?;
        if let Some((w, l)) = items.bar {
            draws.push(Draw { target: (w + 0x10).wrapping_sub(cx0.delta), list: l });
        }
    }
    if let Some(s) = next.next() {
        let (h, hw) = crate::items::help(cx0, s, env.fo.win, s - 0x8000)?;
        draws.push(Draw { target: hw.wrapping_sub(cx0.delta), list: h });
    }
    Ok(draws)
}
