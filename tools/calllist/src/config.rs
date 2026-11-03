//! The Config screen as mod/ConfigMenuPatches.cs draws it (ui.menus = ps2): the PS2's page
//! (SLPM_675.13 0x00235a68 / 0x00235a00, rows 0x00235720) in place of MenuConfig_Draw
//! (0x008cb320, the config menu record 0x00c5a5e4's +4 draw). The same calls as the C#, with the
//! game's lookups made at the frame boundary; the brushes at rest (the C# sweeps them in over
//! w / 14 + 2 frames when the value moves). Addresses are ffx.sym VAs; evidence in docs/ui.md
//! ("The Config screen").
use crate::menu::{Ctx, List, cursor_parts};
use crate::screens::{Draw, Env};

const DRAW_KICK: u32 = 0x008d_ce50; // TkMenuDrawKick
const CONFIG_MES: u32 = 0x008d_c9b0; // TkMenuConfigMes(id)
const CONFIG_HELP: u32 = 0x008d_c970; // TkMenuConfigHelp(id)
const SET_HELP: u32 = 0x008a_aeb0; // TkMenuSetHelpMessage
const GET_LOC: u32 = 0x0064_12d0; // SaveDataGetLoc(id, out)
const WIN_LAYOUT: u32 = 0x008f_a4f0; // TOGetWindowLayout(id)
const PLATE_LR: u32 = 0x008f_5e90;
const SHAPE_XY: u32 = 0x0090_3a10;
const SHAPE_XYWH: u32 = 0x0090_3b70;
const FONT_1612: u32 = 0x0090_1cb0;
const OPEN_PKT: u32 = 0x008b_ef30;
const KICK: u32 = 0x008b_ea20;
const VU1_SYNC: u32 = 0x0088_ec60;
const GREY: u32 = 0x8080_8080;

const LOC_ROW: u32 = 0x0186_a438;
const CURSOR: u32 = 0x0186_a444;
const ROWS: u32 = 0x0186_a448;
const ROW_LIST: u32 = 0x0186_a44c;
const HELP_TEXT: u32 = 0x0186_a4a0;
const PLATE: u32 = 0x0186_a450; // r_cnameplt: rect, c0 +8, c1 +0xc, shapes +0x10, +0x14
const RECT_CCHOPLT: u32 = 0x0186_a468;
const POS_CCHOPLT: u32 = 0x0186_a470;
const POS_CNAME: u32 = 0x0186_a480;
const NUM_CNAMEPLT: u32 = 0x0186_a48c;
const COL_HELPPLT0: u32 = 0x0186_a498;
const COL_HELPPLT1: u32 = 0x0186_a49c;
/// MenuConfig_Draw's slot: the config menu record (control 0x008cb0e0) +4.
const CONFIG_RECORD_DRAW: u32 = 0x00c5_a5e8;

fn rd(cx: &mut Ctx, abs: u32, n: usize) -> ffx_harness::Result<Vec<u8>> {
    cx.game.read(abs.wrapping_sub(cx.delta), n)
}
fn u32_of(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// A text the C# gets from SaveDataGetLoc into a buffer: the list makes the same call into its own
/// 256-byte buffer; the width is measured now from a copy in `out`.
fn loc_text(cx: &mut Ctx, l: &mut List, env: &Env, id: u32) -> ffx_harness::Result<(u32, i32)> {
    let buf = l.string(&" ".repeat(255));
    l.c(GET_LOC, &[id, buf]);
    cx.call(GET_LOC, &[id, env.out])?;
    let w = cx.text_width(env.out)?;
    Ok((buf, w))
}

fn row(cx: &mut Ctx, l: &mut List, env: &Env, r: i32) -> ffx_harness::Result<()> {
    let list = cx.r32(ROW_LIST)?;
    let rec = u32_of(&rd(cx, list + 4 * r as u32, 4)?, 0);
    let b = rd(cx, rec, 0x30)?;
    let pc = r == cx.r32(LOC_ROW)? as i32;
    let cur = i32::from(cx.game.read(CURSOR, 1)?[0] as i8);
    let (px, py0, pw) = (cx.r16(PLATE)?, cx.r16(PLATE + 2)?, cx.r16(PLATE + 4)?);
    let py = py0 + r * (cx.r32(NUM_CNAMEPLT)? as i32 - 5);
    let (c0, c1, a, bb) = (cx.r32(PLATE + 8)?, cx.r32(PLATE + 0xc)?, cx.r32(PLATE + 0x10)?, cx.r32(PLATE + 0x14)?);
    l.c(PLATE_LR, &[a, bb, px as u32, py as u32, pw as u32, c0, c1]);
    let (nx, ny) = (px + cx.r16(POS_CNAME)?, py + cx.r16(POS_CNAME + 2)?);
    let title = if pc { loc_text(cx, l, env, 0x1cd)?.0 } else { cx.call(CONFIG_MES, &[u32::from(u16::from_le_bytes([b[0x18], b[0x19]]))])? };
    l.c(FONT_1612, &[title, nx as u32, ny as u32, 0]);
    if r == cur {
        for (fun, a) in cursor_parts(nx, ny, env.dum75) {
            l.c(fun, &a);
        }
    }
    let (wx, wy) = (px + cx.r16(POS_CCHOPLT)?, py + cx.r16(POS_CCHOPLT + 2)?);
    let (ww, wh) = (cx.r16(RECT_CCHOPLT + 4)?, cx.r16(RECT_CCHOPLT + 6)?);
    let (h0, h1) = (cx.r32(COL_HELPPLT0)?, cx.r32(COL_HELPPLT1)?);
    let lay = cx.call(WIN_LAYOUT, &[200])?;
    let lb = rd(cx, lay, 12)?;
    let (cw, ch) = (i32::from(u16::from_le_bytes([lb[8], lb[9]])), i32::from(u16::from_le_bytes([lb[10], lb[11]])));
    l.c(SHAPE_XYWH, &[200, wx as u32, wy as u32, 0, 0, h0, h0]);
    l.c(SHAPE_XYWH, &[201, (wx + cw) as u32, wy as u32, (ww - cw) as u32, 0, h0, h1]);
    l.c(SHAPE_XYWH, &[202, wx as u32, (wy + ch) as u32, 0, (wh - ch) as u32, h0, h0]);
    l.c(SHAPE_XYWH, &[203, (wx + cw) as u32, (wy + ch) as u32, (ww - cw) as u32, (wh - ch) as u32, h0, h1]);
    let (n, value, on) = (u32_of(&b, 4) as i32, u32_of(&b, 8) as i32, u32_of(&b, 0xc) != 0);
    let ty = wy + cx.r16(POS_CCHOPLT + 2)?;
    eprintln!(
        "config row {r} rec {rec:#x} pc {pc} plate ({px}, {py}) w {pw} name ({nx}, {ny}) window ({wx}, {wy}) {ww} x {wh} corner {cw} x {ch} n {n} value {value} on {on}"
    );
    for i in 0..n {
        let id = u32::from(u16::from_le_bytes([b[0x1c + 4 * i as usize], b[0x1d + 4 * i as usize]]));
        let (text, tw) = if pc {
            loc_text(cx, l, env, (0x1d0 - 2 * i) as u32)?
        } else {
            let t = cx.call(CONFIG_MES, &[id])?;
            (t, cx.text_width(t)?)
        };
        let tx = wx + (i + 1) * ww / (n + 1) - tw / 2;
        l.c(FONT_1612, &[text, tx as u32, ty as u32, u32::from(!on)]);
        if i == value && on {
            // the brush at rest (PS2 0x00201f58 with p = steps)
            l.c(SHAPE_XY, &[0xe2, (tx - 5) as u32, ty as u32]);
            if tw > 11 {
                l.c(SHAPE_XYWH, &[0xe3, (tx + 11) as u32, ty as u32, (tw - 11) as u32, 0, GREY, GREY]);
            }
            l.c(SHAPE_XY, &[0xe4, (tx + tw) as u32, ty as u32]);
        }
        if r == cur && i == value {
            if pc {
                let help = cx.abs(HELP_TEXT);
                l.c(GET_LOC, &[(0x1d1 - 2 * i) as u32, help]);
                l.c(SET_HELP, &[help]);
            } else {
                let h = cx.call(CONFIG_HELP, &[id])?;
                l.c(SET_HELP, &[h]);
            }
        }
    }
    Ok(())
}

pub fn build(cx: &mut Ctx, env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    let mut l = List::new(scratch[0]);
    l.c(DRAW_KICK, &[]);
    l.c(DRAW_KICK, &[]);
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let rows = cx.r32(ROWS)? as i32;
    eprintln!("config rows {rows} loc row {} cursor {}", cx.r32(LOC_ROW)? as i32, cx.game.read(CURSOR, 1)?[0] as i8);
    for r in 0..rows {
        row(cx, &mut l, env, r)?;
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(vec![Draw { target: CONFIG_RECORD_DRAW, list: l }])
}
