//! The Equip screen as mod/EquipMenuPatches.cs draws it (ui.menus = ps2), as call lists for the
//! thunk: the screen draw (PS2 0x0022cc28 in place of MenuEquip_Draw_008cf670, the equip menu
//! record's +4 at 0x00c5a748) and the weapon list window (PS2 0x0022bdc0 in place of
//! MenuWeapon_DrawList_008cf9f0, the list window's +0x10). The same arithmetic as the C#; the
//! game's lookups are called at the frame boundary. Addresses are ffx.sym VAs; evidence in
//! docs/ui.md ("The Equip screen").
use crate::menu::{Ctx, List, cursor_parts, f};
use crate::screens::{Draw, Env};

const KICK_MENU: u32 = 0x008dce50; // TkMenuDrawKick
const VU1_THUNK: u32 = 0x008c1790; // TkVU1SyncPath_thunk_008c1790
const INIT_PKT: u32 = 0x008c0be0; // TkMn2DrawInitPacket
const STATUS_PAGE: u32 = 0x008d4ee0; // MenuStatus_DrawPage (PS2 0x0022a770)
const PLATE_RL: u32 = 0x008f5e20; // PS2 0x001ff1e0 / 0x001ff028
const SHAPE_UV: u32 = 0x00903ee0; // TOMkpShapeXYWHUVC2: id 0 = sheet 0x3e00 (ps2win here), UVs 0..1
const MESWIN_LAYOUT: u32 = 0x00c5_b460; // the PS2 meswin layout table (= SLPM_675.13 0x00321328)
const GREY: u32 = 0x8080_8080;
const SHAPE_XYC2: u32 = 0x00903a50;
const SHAPE_XYWH: u32 = 0x00903b70;
const SHEET_CELL: u32 = 0x008f8bb0; // TODrawSheetCell_008f8bb0(cell, f x, f y, f w, f h): strtex (PS2 0x00201768)
const FONT_1612: u32 = 0x00901cb0;
const ICON: u32 = 0x008e6af0;
const SCISSOR: u32 = 0x008fdee0;
const WIN_SCISSOR: u32 = 0x008c0f70; // TkMn2DrawMenuWindowScissor(y, h, part, ratio) (PS2 0x002274a8)
const BOX_RGBA: u32 = 0x008fb150;
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const SHADOW: u32 = 0x8000_0000;
const ROW: i32 = 32;

// game state and layout (PS2 -> PC)
const FADE: u32 = 0x0186_a614; // g_TkMenuEquipFade (PS2 0x00693aa0)
const ALT: u32 = 0x0186_a624; // g_TkMenuEquipAltMode (PS2 0x00693ab4)
const LIST_OPEN: u32 = 0x0186_a62c; // g_TkMenuEquipListOpen (PS2 0x00693abc)
const ITEM: u32 = 0x0186_a630; // g_TkMenuEquipItem (PS2 0x00693ac0)
const LIST_WINDOW: u32 = 0x0186_a61c; // the weapon list window (PS2 0x00693aa8)
const R_SOMOJIPLT: u32 = 0x0186_a634; // i16 x, y, w, h (PS2 0x00693ac8)
const POS_SOBIPLT: u32 = 0x0186_a63c; // i16 x, y (PS2 0x00693ad0)
const R_SOMOJIPLT2: u32 = 0x0186_a640; // (PS2 0x00693ad8)
const POS_SOABPLT: u32 = 0x0186_a648; // (PS2 0x00693ae0)
const R_SOSCROL: u32 = 0x0186_a64c; // (PS2 0x00693ae8)
const POS_PMSHAPE: u32 = 0x023c_c420; // pos_pmshape (PS2 0x00325878)
const WPNLPLT: u32 = 0x0186_a900; // rect_wpnlplt's plate record, the armour row's at +0x48 (PS2 0x00693960 / 0x006939a8)
const POS_WPNMOJI: u32 = 0x0186_a86c; // pos_wpnmoji (PS2 0x006938b8)
const COL_BSOPLT0: u32 = 0x025d_16a0;
const ITEM_LIST: u32 = 0x0159_7770; // g_TkMenuItemList (PS2 0x01d57300)

struct Cx<'a, 'b> {
    cx: &'a mut Ctx<'b>,
    dum75: u32,
    out: u32, // scratch for out parameters (absolute)
    fnt: u32, // the ps2fnt name
}

impl Cx<'_, '_> {
    fn pt(&mut self, va: u32) -> ffx_harness::Result<(i32, i32)> {
        Ok((self.cx.r16(va)?, self.cx.r16(va + 2)?))
    }
    fn bytes(&mut self, abs: u32, n: usize) -> ffx_harness::Result<Vec<u8>> {
        self.cx.game.read(abs.wrapping_sub(self.cx.delta), n)
    }
    fn cursor(&self, l: &mut List, x: i32, y: i32) {
        for (fun, a) in cursor_parts(x, y, self.dum75) {
            l.c(fun, &a);
        }
    }
}

/// A row plate by kind (PS2 TOAdpShapeByKind 0x001f4c60), as items.rs.
fn plate(cx: &mut Cx, l: &mut List, x: i32, y: i32, kind: i32) -> ffx_harness::Result<()> {
    let at = COL_BSOPLT0 - 8 * ((kind >> 3) & 3) as u32;
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

/// A meswin shape (id 200..398) part by part (PS2 0x00226e18 -> 0x001fe178: grey, own size) as
/// TOMkpShapeXYWHUVC2 quads with the UVs half a texel inside each cell. Shape 0xd7 is 25 tiles of
/// one 8 x 16 cell; drawn whole, or by TOMkpShapePartIndexXYWHC2 with the sizes given, every tile
/// picks up a light line of the white cell left of it.
fn tiled(cx: &mut Cx, l: &mut List, id: u32, x: i32, y: i32) -> ffx_harness::Result<()> {
    let off = cx.cx.r32(MESWIN_LAYOUT + 4 + 4 * (id - 200))?;
    let n = cx.cx.r32(MESWIN_LAYOUT + off)?;
    let parts = cx.cx.game.read(MESWIN_LAYOUT + off + 4, 12 * n as usize)?;
    for k in 0..n as usize {
        let p = &parts[12 * k..12 * k + 12];
        let (dx, dy) = (i32::from(p[2] as i8), i32::from(p[3] as i8));
        let (w, h) = (u16::from_le_bytes([p[4], p[5]]), u16::from_le_bytes([p[6], p[7]]));
        let (u, v, w, h) = (f32::from(p[0]), f32::from(p[1]), f32::from(w), f32::from(h));
        let uv = [(u + 0.5) / 256.0, (v + 0.5) / 256.0, (u + w - 0.5) / 256.0, (v + h - 0.5) / 256.0].map(f);
        l.c(SHAPE_UV, &[0, f((x + dx) as f32), f((y + dy) as f32), f(w), f(h), uv[0], uv[1], uv[2], uv[3], GREY, GREY]);
    }
    Ok(())
}

/// PS2 0x00226218(x, y, k) -> 0x00201a70: a black plate (0xda, 0xdb over 213) and strtex label k.
fn label_plate(l: &mut List, x: i32, y: i32, k: u32) {
    l.c(PLATE_RL, &[0xda, 0xdb, x as u32, y as u32, 213, SHADOW, SHADOW]);
    l.c(SHEET_CELL, &[k, f((x + 21) as f32), f((y + 5) as f32), f(170.0), f(14.0)]);
}

/// PS2 0x001f5298(id, x, y, 0): the weapon's icon and name, the equipped mark (icon 0x31) when its
/// wearer is its owner.
fn weapon_name(cx: &mut Cx, l: &mut List, id: u32, x: i32, y: i32) -> ffx_harness::Result<()> {
    let rec = cx.cx.call(0x007a_bbd0, &[id, 0])?; // MsGetSaveWeapon
    let name = cx.cx.call(0x007a_bdf0, &[id])?; // MsGetSaveWeaponName
    let r = cx.bytes(rec, 8)?;
    let icon = (u32::from(r[4]) * 2 + u32::from(r[5]) + 1) & 0xff;
    l.c(ICON, &[f((x + 12) as f32), f((y + 1) as f32), f(15.0), f(18.0), icon, 0x80, 0x80, 0x80, 0x80]);
    l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, 0]);
    if r[4] == r[6] {
        l.c(ICON, &[f((x + 12) as f32), f((y + 1) as f32), f(15.0), f(18.0), 0x31, 0x80, 0x80, 0x80, 0x80]);
    }
    Ok(())
}

/// The screen (PS2 0x0022cc28): the status page (the PC's MenuStatus_DrawPage, HD unless a port
/// hooks it), the two label plates, the weapon / armour cursor, the picked gear's abilities.
fn screen(cx: &mut Cx, l: &mut List, slot: u32, win: u32, orig: u32) -> ffx_harness::Result<()> {
    l.c(KICK_MENU, &[]);
    l.c(KICK_MENU, &[]);
    l.c(VU1_THUNK, &[]);
    l.c(INIT_PKT, &[]);
    let chr = cx.cx.call(0x008a_9860, &[])?; // TkMenuGetCurrentPlayer
    let fade = cx.cx.r32(FADE)?;
    let a = |va: u32| cx.cx.abs(va);
    let args = [0, 0, chr, 0, a(0x023c_c3c8), a(0x023c_c3c4), a(0x023c_c3c0), a(0x023c_c3bc), fade];
    let _ = (args, orig);
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    // the status page: the Status port's page 0 (mod EquipMenuPatches calls StatusMenuPatches.page0)
    let fo = crate::menu::Fonts { slot, win, fnt: cx.fnt };
    crate::status::page_calls(cx.cx, fo, cx.out, l, chr, fade as i32)?;
    let r1 = cx.pt(R_SOMOJIPLT)?;
    let r2 = cx.pt(R_SOMOJIPLT2)?;
    label_plate(l, r1.0, r1.1, 6);
    label_plate(l, r2.0, r2.1, 7);
    // the cursor (PS2 0x0022bbd0): the weapon (armour) caption's place (PS2 0x00228b98 =
    // MenuStatus_GetPos), blinking while the list is open
    let alt = cx.cx.r32(ALT)?;
    let open = cx.cx.r32(LIST_OPEN)?;
    let timer = cx.cx.call(0x008a_9c60, &[])? as i32;
    let rec = cx.pt(if alt != 0 { WPNLPLT + 0x48 } else { WPNLPLT })?;
    let moji = cx.pt(POS_WPNMOJI)?;
    let item = cx.cx.r32(ITEM)?;
    eprintln!(
        "equip: chr {chr} fade {fade:#x} alt {alt} open {open} item {item:#x} mojiplt {r1:?} {r2:?} wpnlplt {rec:?} wpnmoji {moji:?} soab {:?} pmshape {:?}",
        cx.pt(POS_SOABPLT)?,
        cx.pt(POS_PMSHAPE)?
    );
    if fade as i32 >= 0x1000 && !(open != 0 && (timer / 2) & 1 != 0) {
        cx.cursor(l, rec.0 + moji.0, rec.1 + moji.1);
    }
    // the abilities (PS2 0x0022baa8): a purple plate (kind 0x10) and shape 0xd7 per slot, the names
    if item != 0xff {
        let w = cx.cx.call(0x007a_bbd0, &[item, 0])?;
        let r = cx.bytes(w, 0x16)?;
        let (x, y0) = cx.pt(POS_SOABPLT)?;
        let pm = cx.pt(POS_PMSHAPE)?;
        for i in 0..i32::from(r[0xb]) {
            let y = y0 + ROW * i;
            plate(cx, l, x, y, 0x10)?;
            tiled(cx, l, 0xd7, x + pm.0 + 0x58, y + pm.1)?;
        }
        for k in 0..4 {
            let id = u32::from(u16::from_le_bytes([r[0xe + 2 * k], r[0xf + 2 * k]]));
            if id != 0 && id != 0xff {
                let y = y0 + ROW * k as i32;
                let rec = cx.cx.call(0x0079_0970, &[id, cx.out])?; // MsGetRomAbility(id, &strings)
                let strings = cx.cx.r32(cx.out.wrapping_sub(cx.cx.delta))?;
                let b = cx.bytes(rec, 0x69)?;
                let name = strings + u32::from(u16::from_le_bytes([b[0], b[1]]));
                l.c(ICON, &[f((x + 12) as f32), f((y + 1) as f32), f(15.0), f(18.0), u32::from(b[0x68]), 0x80, 0x80, 0x80, 0x80]);
                l.c(FONT_1612, &[name, (x + 29) as u32, (y + 1) as u32, 0]);
            }
        }
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

/// One row (PS2 0x0022bc20): mode 0 follows the scroll (+0x46 / 128).
fn row(cx: &mut Cx, l: &mut List, w: &[u8], x: i32, y: i32, idx: i32, mode: i32) -> ffx_harness::Result<()> {
    let y = if mode == 0 { y - i32::from(i16::from_le_bytes([w[0x46], w[0x47]])) / 128 } else { y };
    let count = i32::from(u16::from_le_bytes([w[0x30], w[0x31]]));
    if idx < 0 || idx >= count {
        return Ok(());
    }
    plate(cx, l, x, y, 0)?;
    let pm = cx.pt(POS_PMSHAPE)?;
    tiled(cx, l, 0xd7, x + pm.0 + 0x58, y + pm.1)?;
    let e = cx.cx.game.read(ITEM_LIST + 4 * idx as u32, 2)?;
    let id = u32::from(u16::from_le_bytes([e[0], e[1]]));
    if id != 0 {
        weapon_name(cx, l, id, x, y)?;
    }
    Ok(())
}

/// The weapon list window (PS2 0x0022bdc0), at rest and while scrolling.
fn list(cx: &mut Cx, l: &mut List, w: &[u8]) -> ffx_harness::Result<()> {
    let h16 = |o: usize| i32::from(i16::from_le_bytes([w[o], w[o + 1]]));
    let u16_ = |o: usize| i32::from(u16::from_le_bytes([w[o], w[o + 1]]));
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    let (px, py) = cx.pt(POS_SOBIPLT)?;
    let (top, next, scroll) = (h16(0x32), h16(0x34), h16(0x46));
    let rows = |cx: &mut Cx, l: &mut List, mode: i32, first: i32, dy: i32| -> ffx_harness::Result<()> {
        let mut y = py + dy;
        for k in -1..6 {
            row(cx, l, w, px, y, first + k, mode)?;
            y += ROW;
        }
        Ok(())
    };
    if top == next {
        l.c(WIN_SCISSOR, &[py as u32, 160, 0, scroll as u32]);
        rows(cx, l, 0, top, -32)?;
    } else {
        l.c(WIN_SCISSOR, &[py as u32, 160, 1, scroll as u32]);
        rows(cx, l, 1, top, -32)?;
        l.c(WIN_SCISSOR, &[py as u32, 160, 2, scroll as u32]);
        rows(cx, l, 2, next, -160 * scroll / 4096 - 32)?;
    }
    l.c(SCISSOR, &[0, 0, 511, 415]);
    cx.cursor(l, px + 12, py + 1 + ROW * (h16(0x48) - top));
    let (count, shown) = (u16_(0x30), u16_(0x3a));
    let (rx, ry) = cx.pt(R_SOSCROL)?;
    let (rw, rh) = cx.pt(R_SOSCROL + 4)?;
    eprintln!("equip list: count {count} top {top} next {next} shown {shown} cur {} pos {px},{py} scroll rect {rx} {ry} {rw} {rh}", h16(0x48));
    if shown < count {
        l.c(BOX_RGBA, &[rx as u32, ry as u32, rw as u32, rh as u32, 0x11, 0x11, 0x11, 0x80]);
        let t = if top == 0 { ry as f32 } else { ry as f32 + rh as f32 * top as f32 / count as f32 };
        let b = if top + shown >= count { (ry + rh) as f32 } else { ry as f32 + rh as f32 * (top + shown) as f32 / count as f32 };
        l.c(BOX_RGBA, &[(rx + 1) as u32, t as i32 as u32, (rw - 2) as u32, (b as i32 - t as i32) as u32, 0xf0, 0xf0, 0xf0, 0x80]);
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(())
}

pub fn build(cx0: &mut Ctx, env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    let (slot, orig) = crate::menu::atlas_slot(cx0)?;
    let mut cx = Cx { cx: cx0, dum75: env.dum75, out: env.out, fnt: env.fo.fnt };
    let mut draws = vec![];
    let mut l = List::new(scratch[0]);
    screen(&mut cx, &mut l, slot, env.fo.win, orig)?;
    draws.push(Draw { target: 0x00c5_a748, list: l });
    let w = cx.cx.r32(LIST_WINDOW)?;
    if w != 0 {
        let b = cx.cx.game.read(w.wrapping_sub(cx.cx.delta), 0x98)?;
        let mut l = List::new(scratch[1]);
        list(&mut cx, &mut l, &b)?;
        draws.push(Draw { target: w.wrapping_sub(cx.cx.delta) + 0x10, list: l });
    }
    Ok(draws)
}
