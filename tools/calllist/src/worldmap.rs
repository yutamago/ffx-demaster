//! The world map as mod/WorldMapPatches.cs draws it (ui.menus = ps2): PS2 0x00224700 in place of
//! MenuWorldMap_Draw_008e2b60 (the world map window's +0x10): the picture (the converter's PS2
//! worldmap), the marker scene, then the main menu's location plates and the place name centred.
//! The same arithmetic as the C#; evidence in docs/ui.md ("The world map").
use crate::menu::{Ctx, List};
use crate::screens::{Draw, Env};

const SCISSOR_XYWH: u32 = 0x008f_9230; // TODrawScissorXYWH
const PICTURE: u32 = 0x0090_2810; // TOMkpFont_00902810: the picture 0x2eac (PS2 0x002002d8)
const PLATE_RL: u32 = 0x008f_5e20; // (A, B, x, y, w, c0, c1) (PS2 0x001ff1e0)
const SHAPE_XY: u32 = 0x0090_3a10; // TOMkpShapeXY (meswin: expanded by menu::expand_shapes)
const FONT_1612: u32 = 0x0090_1cb0;
const SEQ_START: u32 = 0x008a_f0d0; // TOSeqStartScene(seq, name)
const SCENE_POS: u32 = 0x008a_f060; // TOSeqSetScenePosByName(seq, name, x, y)
const OPEN_PKT: u32 = 0x008b_ef30;
const KICK: u32 = 0x008b_ea20;
const VU1_SYNC: u32 = 0x0088_ec60;
const WINDOW: u32 = 0x0187_1698; // g_TkMenuWorldMapWindow

pub fn build(cx: &mut Ctx, _env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    let w = cx.r32(WINDOW)?;
    if w == 0 {
        eprintln!("worldmap: no world map window open");
        return Ok(vec![]);
    }
    let fade = u32::from_le_bytes(cx.game.read(w.wrapping_sub(cx.delta) + 0x64, 4)?[..4].try_into().unwrap());
    let mut l = List::new(scratch[0]);
    l.c(VU1_SYNC, &[]);
    l.c(OPEN_PKT, &[]);
    l.c(SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
    let c = (fade << 24) | 0x80_8080;
    l.c(PICTURE, &[0, 0, c, c]);
    if fade == 0x80 {
        let mx = cx.call(0x0087_3ae0, &[])? as i32;
        let my = cx.call(0x0087_3af0, &[])? as i32;
        eprintln!("worldmap: marker {mx},{my}");
        if mx != 0 || my != 0 {
            let s = l.string("scene30");
            l.c(SEQ_START, &[1, s]);
            l.c(SCENE_POS, &[1, s, (mx / 2) as u32, (my / 2 - 16) as u32]);
        }
        let r = |cx: &mut Ctx, a: u32| cx.r16(a);
        let (sx, sy, sw) = (r(cx, 0x025d_1608)?, r(cx, 0x025d_1606)?, r(cx, 0x025d_1604)?);
        let (px, py, pw) = (r(cx, 0x025d_15f8)?, r(cx, 0x025d_15f6)?, r(cx, 0x025d_15f4)?);
        let left = sx.min(px);
        let right = (sx + sw).max(px + pw);
        let shift = (512 - (right - left)) / 2 - left;
        l.c(PLATE_RL, &[0xd2, 0xd3, (sx + shift) as u32, sy as u32, sw as u32, cx.r32(0x025d_1600)?, cx.r32(0x025d_15fc)?]);
        l.c(PLATE_RL, &[0xd8, 0xd9, (px + shift) as u32, py as u32, pw as u32, cx.r32(0x025d_15f0)?, cx.r32(0x025d_15ec)?]);
        // points are stored mirrored: (y, x) from the lower address
        let shape = (r(cx, 0x025d_15ea)?, r(cx, 0x025d_15e8)?);
        let st = (r(cx, 0x025d_15e6)?, r(cx, 0x025d_15e4)?);
        l.c(SHAPE_XY, &[0xef, (px + shape.0 + shift) as u32, (py + shape.1) as u32]);
        let dic = cx.call(0x0086_c410, &[])?;
        let english = cx.call(0x0078_51d0, &[])?;
        let name = cx.call(0x0086_c430, &[dic, english])?;
        l.c(FONT_1612, &[name, (px + st.0 + shift) as u32, (py + st.1) as u32, 0]);
        eprintln!("worldmap: plates sub ({sx},{sy},{sw}) place ({px},{py},{pw}) shift {shift}");
    }
    l.c(KICK, &[]);
    l.c(VU1_SYNC, &[]);
    Ok(vec![Draw { target: (w + 0x10).wrapping_sub(cx.delta), list: l }])
}
