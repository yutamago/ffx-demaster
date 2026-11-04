//! What mod/BattleLimitPatches.cs draws for the Overdrive mini-game windows (ui.menus = ps2), as
//! call lists: Tidus's bar (PS2 0x0021b360 in place of TOBtlDrawTidusLimitWindow), Auron's buttons
//! (PS2 0x0021ad88 in place of TOBtlDrawAuronLimitWindow), Lulu's Fury (PS2 0x0021b9d0 in place of
//! TOBtlDrawLuluLimitWindow) and the limit timer (PS2 0x0021a8f8 in place of 0x008955e0). The
//! game's state is read at the frame boundary; the lists hold the draws (the marker, the count and
//! the seconds as they were then). Addresses are ffx.sym VAs; evidence in docs/ui.md ("The
//! Overdrive mini-game windows").
use crate::hud::Layout;
use crate::menu::{f, Ctx, List};

const BOX_C2: u32 = 0x008f_3db0; // TODrawBoxXYWHC2
const BOX_FW: u32 = 0x008f_3eb0; // TODrawBoxXYWHC2FW(x, y, w, h, c0, c1, full)
const SHAPE_UV: u32 = 0x0090_3ee0; // TOMkpShapeXYWHUVC2 (600..: sheet 0x3dc0, HD-pixel UVs)
const FONT_1612: u32 = 0x0090_1cb0; // TOMkpBaseMesFont (expanded with the PS2 size words)
const ROM_BTL_TEXT: u32 = 0x0078_f8f0; // MsGetRomBtlText(id, 0)
const LOC_TEXT: u32 = 0x0064_12f0; // graphicGetLocTextWithLength(id, buf, len)
const CONFIRM: u32 = 0x008a_cf30; // TOGetSystemConfirmButtonAssign
const ICON_LAYOUT: u32 = 0x00c5_d578;
const GREY: u32 = 0x8080_8080;

pub const AURON: u32 = 0x0133_d6f4;
pub const LULU: u32 = 0x0133_d708;
pub const TIDUS: u32 = 0x0133_d734;
pub const TIMER: u32 = 0x0133_f77c;
const TEXT_Y: u32 = 0x0133_f778;
const LULU_MODE: u32 = 0x00c5_8ef0;

fn i16at(b: &[u8], o: usize) -> i32 {
    i32::from(i16::from_le_bytes([b[o], b[o + 1]]))
}
fn u32at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn shown(state: i32) -> bool {
    ((state - 3) as u32) < 2
}

/// mod BattleLimitPatches.prompt: the text centred at dum225 in the 16x12 font.
fn prompt(cx: &mut Ctx, l: &mut List, text: u32) -> ffx_harness::Result<()> {
    if text == 0 {
        return Ok(());
    }
    let w = cx.text_width(text)?;
    let y = cx.r16(TEXT_Y)?;
    l.c(FONT_1612, &[text, ((512 - w) / 2) as u32, y as u32, 0]);
    Ok(())
}

fn rom_prompt(cx: &mut Ctx, l: &mut List, id: u32) -> ffx_harness::Result<()> {
    let t = cx.call(ROM_BTL_TEXT, &[id, 0])?;
    prompt(cx, l, t)
}

/// mod BattleLimitPatches.gauge (PS2 0x00201b68).
fn gauge(t: &Layout, l: &mut List, x: i32, y: i32, w: i32) {
    t.shape(l, 0x1f4, x - 7, y - 4, 0, 0, GREY, GREY);
    t.shape(l, 0x1f6, x + w - 8, y - 4, 0, 0, GREY, GREY);
    t.shape(l, 0x1f5, x, y - 4, w, 0, GREY, GREY);
}

/// mod BattleLimitPatches.digits.
fn digits(t: &Layout, l: &mut List, s: &str, mut x: i32, y: i32, offset: u32, digit: i32, other: i32, c: u32) {
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            t.shape(l, ch as u32 + offset, x, y, 0, 0, c, c);
            x += digit;
            continue;
        }
        if ch == '.' {
            t.shape(l, 0x1df, x, y, 0, 0, c, c);
        }
        x += other;
    }
}

/// Tidus's window (mod BattleLimitPatches.tidus); empty outside states 3 and 4.
pub fn tidus(cx: &mut Ctx, scratch: u32, ps2btl: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let s = cx.game.read(TIDUS, 0x18)?;
    eprintln!("limit tidus: {}", s.iter().map(|b| format!("{b:02x}")).collect::<String>());
    if !shown(i16at(&s, 0)) {
        return Ok(l);
    }
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let (x, y, w) = (i16at(&s, 2), i16at(&s, 4), i16at(&s, 6));
    let c = u32at(&s, 0x14);
    l.set(slot, ps2btl);
    gauge(&t, &mut l, x, y, w);
    l.c(BOX_C2, &[i16at(&s, 8) as u32, y as u32, i16at(&s, 0xc) as u32, 9, c, c]);
    t.shape(&mut l, 0x1dd, i16at(&s, 0xe), y, 0, 0, GREY, GREY);
    l.set(slot, orig);
    rom_prompt(cx, &mut l, 0x3010)?;
    Ok(l)
}

/// An icon shape (mod BattleLimitPatches.ishape).
fn ishape(table: &[u8], l: &mut List, id: u32, x: i32, y: i32) {
    let rd16 = |o: usize| u16::from_le_bytes([table[o], table[o + 1]]);
    let e = u32at(table, 4 + 4 * (id as usize - 600)) as usize;
    for k in 0..u32at(table, e) as usize {
        let p = e + 4 + 12 * k;
        let (u, v) = (f32::from(table[p]), f32::from(table[p + 1]));
        let (dx, dy) = (i32::from(table[p + 2] as i8), i32::from(table[p + 3] as i8));
        let (pw, ph) = (f32::from(rd16(p + 4)), f32::from(rd16(p + 6)));
        let (mut u0, mut u1, mut v0, mut v1) = (4.0 * (u + 0.5), 4.0 * (u + pw - 0.5), 4.0 * (v + 0.5), 4.0 * (v + ph - 0.5));
        if rd16(p + 8) == 1 {
            std::mem::swap(&mut u0, &mut u1);
        }
        if rd16(p + 10) == 1 {
            std::mem::swap(&mut v0, &mut v1);
        }
        l.c(SHAPE_UV, &[id, f((x + dx) as f32), f((y + dy) as f32), f(pw), f(ph), f(u0), f(v0), f(u1), f(v1), GREY, GREY]);
    }
}

/// mod BattleLimitPatches.pad (PS2 0x001fc778).
fn pad(table: &[u8], l: &mut List, b: u32, x: i32, y: i32, japanese: bool) {
    if b & 0xf000 != 0 {
        for (bit, lit) in [(0x1000, 0x259), (0x2000, 0x25b), (0x4000, 0x25d), (0x8000, 0x25f)] {
            ishape(table, l, if b & bit != 0 { lit } else { lit - 1 }, x, y);
        }
    }
    let (b20, b40) = if japanese { (0x262, 0x261) } else { (0x261, 0x262) };
    for (bit, id) in [(0x10, 0x260), (0x20, b20), (0x40, b40), (0x80, 0x263), (4, 0x264), (8, 0x265), (1, 0x266), (2, 0x267), (0x800, 0x268), (0x100, 0x269)] {
        if b & bit != 0 {
            ishape(table, l, id, x, y);
        }
    }
}

/// Auron's window (mod BattleLimitPatches.auron).
pub fn auron(cx: &mut Ctx, scratch: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let s = cx.game.read(AURON, 0x14)?;
    eprintln!("limit auron: {}", s.iter().map(|b| format!("{b:02x}")).collect::<String>());
    if !shown(i16at(&s, 0)) {
        return Ok(l);
    }
    let (x, y) = (i16at(&s, 2), i16at(&s, 4));
    let (first, count, list) = (i16at(&s, 8), u32at(&s, 0xc) as i32, u32at(&s, 0x10));
    let table = cx.game.read(ICON_LAYOUT, 0x800)?;
    let japanese = cx.call(CONFIRM, &[])? == 1;
    if list != 0 && count > first {
        let b = cx.game.read_abs(list, 2 * count as usize)?;
        for i in first..count {
            let v = u32::from(u16::from_le_bytes([b[2 * i as usize], b[2 * i as usize + 1]]));
            pad(&table, &mut l, v, x + 30 * i, y, japanese);
        }
    }
    rom_prompt(cx, &mut l, 0x3011)?;
    Ok(l)
}

/// Lulu's window (mod BattleLimitPatches.lulu). `text` 0x100 bytes of scratch for the keyboard text.
pub fn lulu(cx: &mut Ctx, scratch: u32, ps2btl: u32, text: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let s = cx.game.read(LULU, 0x2c)?;
    eprintln!("limit lulu: {}", s.iter().map(|b| format!("{b:02x}")).collect::<String>());
    if !shown(i16at(&s, 0)) {
        return Ok(l);
    }
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let (x, y, w) = (i16at(&s, 2), i16at(&s, 4), i16at(&s, 6));
    l.set(slot, ps2btl);
    digits(&t, &mut l, &format!("{:02}", s[0x17]), i16at(&s, 0x10), i16at(&s, 0x12), 0x1ba, 16, 16, GREY);
    l.c(BOX_FW, &[x as u32, y as u32, i16at(&s, 0x14) as u32, 9, u32at(&s, 8), u32at(&s, 0xc), w as u32]);
    gauge(&t, &mut l, x, y, w);
    l.set(slot, orig);
    if cx.r32(LULU_MODE)? == 0 {
        cx.call(LOC_TEXT, &[0x2e8, text, 0x100])?;
        prompt(cx, &mut l, text)?;
    } else {
        rom_prompt(cx, &mut l, 0x3012)?;
    }
    Ok(l)
}

/// The limit timer (mod BattleLimitPatches.timer).
pub fn timer(cx: &mut Ctx, scratch: u32, ps2btl: u32) -> ffx_harness::Result<List> {
    let mut l = List::new(scratch);
    let s = cx.game.read(TIMER, 0x1c)?;
    eprintln!("limit timer: {}", s.iter().map(|b| format!("{b:02x}")).collect::<String>());
    if u32at(&s, 0) == 0 {
        return Ok(l);
    }
    let t = Layout::read(cx)?;
    let (slot, orig) = crate::menu::sheet_slot(cx, 0x3f00)?;
    let v = f32::from_bits(u32at(&s, 0x18));
    let c = if v < 1.0 { 0x8000_8080 } else { GREY };
    l.set(slot, ps2btl);
    digits(&t, &mut l, &format!("{v:.2}"), i16at(&s, 4), i16at(&s, 6), 0x1b0, 20, 20, c);
    l.set(slot, orig);
    Ok(l)
}
