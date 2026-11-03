//! What mod/MainMenuPatches.cs draws for the main menu (ui.menus = ps2), as call lists for the
//! thunk: the main draw (PS2 0x0024a4a8 in place of TkMenuDrawMain), the option window (PS2
//! 0x0024b148 in place of MenuMain_DrawCmdWindow) and the help bar (PS2 0x00243888 in place of
//! 0x008a9350). The game's lookups are called at the frame boundary; the lists hold the draws.
//! Addresses are ffx.sym VAs (`abs` rebases them); evidence in docs/ui.md ("The main menu").
use ffx_harness::Game;

fn hexu(r: &str) -> u32 {
    u32::from_str_radix(r.trim().trim_start_matches("ok ").trim_start_matches("0x"), 16).unwrap_or(0)
}

pub struct Ctx<'a> {
    pub game: &'a mut Game,
    pub delta: u32,
}

impl Ctx<'_> {
    pub fn abs(&self, va: u32) -> u32 {
        va.wrapping_add(self.delta)
    }
    pub fn call(&mut self, va: u32, args: &[u32]) -> ffx_harness::Result<u32> {
        let mut s = format!("call {va:#x}");
        for a in args {
            s.push_str(&format!(" {a:#x}"));
        }
        Ok(hexu(&self.game.command(&s)?))
    }
    pub fn r32(&mut self, va: u32) -> ffx_harness::Result<u32> {
        Ok(u32::from_le_bytes(self.game.read(va, 4)?[..4].try_into().unwrap()))
    }
    pub fn r16(&mut self, va: u32) -> ffx_harness::Result<i32> {
        Ok(i32::from(i16::from_le_bytes(self.game.read(va, 2)?[..2].try_into().unwrap())))
    }
    fn write_abs(&mut self, abs: u32, bytes: &[u8]) -> ffx_harness::Result<()> {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        self.game.command(&format!("write {:#x} {hex}", abs.wrapping_sub(self.delta)))?;
        Ok(())
    }
    /// The words mod Ps2Menu.ps2_sized sets: (VA, the PS2 value, the value now).
    pub fn size_words(&mut self) -> ffx_harness::Result<Vec<(u32, u32, u32)>> {
        let lang = self.call(FFX_LANG, &[])? as i32;
        let mut out = vec![];
        for (va, v) in SIZE_WORDS.iter().zip(ps2_size_values(lang)) {
            out.push((*va, v.to_bits(), self.r32(*va)?));
        }
        Ok(out)
    }
    /// mod Ps2Menu.text_width: TkMn2StringWidth's low half with the PS2 size words set.
    pub fn text_width(&mut self, text: u32) -> ffx_harness::Result<i32> {
        let words = self.size_words()?;
        for &(va, v, _) in &words {
            self.write_abs(self.abs(va), &v.to_le_bytes())?;
        }
        let w = self.call(TEXT_WIDTH, &[text]);
        for &(va, _, old) in &words {
            self.write_abs(self.abs(va), &old.to_le_bytes())?;
        }
        Ok(i32::from(w? as u16 as i16))
    }
}

const TEXT_WIDTH: u32 = 0x008c_2cb0; // TkMn2StringWidth: TOGetMessageWH's width (low half)
const FFX_LANG: u32 = 0x0064_10e0; // TOGetFFXLang
/// mod Ps2Menu.ps2_sized's words: the glyph extent factors 0x00c5d778 / 0x00c5d77c,
/// g_TkMenuScissorScales (0x018716a8, 0x018716ac), the cached letter spaces, the shadow offset.
const SIZE_WORDS: [u32; 8] = [0x00c5_d778, 0x00c5_d77c, 0x0187_16a8, 0x0187_16ac, 0x0187_16b4, 0x0187_16b8, 0x0187_16e0, 0x0187_16e4];

/// mod Ps2Menu.ps2_size_values: the 16x12 font at the PS2's size (w x 18, shadow (+2, +2), advance w + 2).
fn ps2_size_values(lang: i32) -> [f32; 8] {
    let yoff = if lang == 10 { -1.0 } else if (1..=8).contains(&lang) { 1.0 } else { 0.0 };
    let (pc, ux, uy) = (0.78f32, 1920f32 / 512.0, 1080f32 / 416.0);
    [ux / uy / pc, (18.0 + pc * yoff) / (pc * (18.0 + yoff)), 1.0, 1.0 / pc, 2.0, 2.0, 2.0 - 2.0 / ux, 2.0 - 2.0 / uy]
}

/// Builds call lists; strings go to a scratch area (absolute addresses).
pub struct List {
    pub calls: Vec<(u32, Vec<u32>)>,
    scratch: u32,
    pub strings: Vec<(u32, Vec<u8>)>,
}

impl List {
    pub fn new(scratch: u32) -> Self {
        List { calls: vec![], scratch, strings: vec![] }
    }
    pub fn c(&mut self, f: u32, a: &[u32]) {
        self.calls.push((f, a.to_vec()));
    }
    pub fn set(&mut self, dst: u32, v: u32) {
        self.calls.push((2, vec![dst, v]));
    }
    pub fn string(&mut self, s: &str) -> u32 {
        let a = self.scratch;
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        self.scratch += (b.len() as u32 + 3) & !3;
        self.strings.push((a, b));
        a
    }
}

pub fn f(x: f32) -> u32 {
    x.to_bits()
}

// PC functions (ffx.sym)
const BOX_C2: u32 = 0x008f3db0; // TODrawBoxXYWHC2(x, y, w, h, c0, c1) ints
const BOX_RGBA: u32 = 0x008fb150; // TOMakePktBoxXYWHRGBA(x, y, w, h, r, g, b, a)
const PLATE_LR: u32 = 0x008f5e90; // (A, B, x, y, w, c0, c1): A stretched c0..c1, B fixed in c1 (PS2 0x001ff2b8)
const PLATE_RL: u32 = 0x008f5e20; // (A, B, x, y, w, c0, c1): A fixed in c0, B stretched c0..c1 (PS2 0x001ff1e0)
const SHAPE_XY: u32 = 0x00903a10; // TOMkpShapeXY(id, x, y)
const SHAPE_XYC2: u32 = 0x00903a50; // TOMkpShapeXYC2(id, x, y, c0, c1)
const SHAPE_XYWHC2: u32 = 0x00903b70; // TOMkpShapeXYWHC2(id, x, y, w, h, c0, c1)
const SHAPE_PART: u32 = 0x009034b0; // TOMkpShapePartIndexXYWHC2(id, part, f x, f y, f w, f h, c0, c1)
const FONT_1612: u32 = 0x00901cb0; // TOMkpBaseMesFont(text, x, y, colour) (PS2 0x001f3810)
const FONT_1612A: u32 = 0x00901ce0; // (text, x, y, colour, alpha) (PS2 0x001f3858)
const STR_S: u32 = 0x00901660; // TOMkpCrossEasyStrFontSClut(s, f x, f y, colour, f sx, sy)
const DIGIT_R: u32 = 0x009055c0; // ToMakeBtlEasyDigitRight(v, f x, f y, grey, f sx, f sy): right edge at x
const SX: f32 = 0.78; // the PC's battle digit size (BattlePatches)
const SY: f32 = 1.0;
const TIMER: u32 = 0x008e19f0; // (t, f x, f y, colour, colon colour)
const OPEN_PKT: u32 = 0x008bef30;
const KICK: u32 = 0x008bea20;
const VU1_SYNC: u32 = 0x0088ec60;
const SHAPE_NAME: u32 = 0x008ac8c0; // TOGetShapTextureName
const HELP: u32 = 0x008a_9350; // the PC's help bar draw (a main-menu window's +0x10)

const GREY: u32 = 0x8080_8080;

pub struct Built {
    pub main: List,
    pub cmd: List,
    pub help: List,
    pub cmd_window: u32,
    pub help_window: u32,
}

/// The PS2 cursor (0x001fe800, kind 0): meswin shape 45's parts, the ball (part 0) in dum75's
/// colour, the hand in grey.
fn cursor(l: &mut List, x: i32, y: i32, dum75: u32) {
    for (f, a) in cursor_parts(x, y, dum75) {
        l.c(f, &a);
    }
}

/// Shape 45's parts at their offsets with their sizes given: TOMkpShapePartIndexXYWHC2 then keeps
/// the UVs one texel inside the cell, as the PS2 sprite (u + w - 1) does; with w = h = 0 it would
/// sample the neighbouring cells' edge texels.
pub fn cursor_parts(x: i32, y: i32, dum75: u32) -> [(u32, Vec<u32>); 2] {
    let p = |dx: i32, dy: i32, w: f32, h: f32| [f((x + dx) as f32), f((y + dy) as f32), f(w), f(h)];
    let (a, b) = (p(-42, -11, 40.0, 40.0), p(-34, -3, 40.0, 24.0));
    [
        (SHAPE_PART, vec![45, 0, a[0], a[1], a[2], a[3], dum75, dum75]),
        (SHAPE_PART, vec![45, 1, b[0], b[1], b[2], b[3], GREY, GREY]),
    ]
}

/// The atlas swap: TOGetShapTextureName(0x3e00) answers from a table slot (0x00c59d58..); the
/// lists store the PS2 atlas's name there around their draws, as the mod's hook answers.
/// Returns the slot (absolute) and the meswin name it holds.
pub fn atlas_slot(cx: &mut Ctx) -> ffx_harness::Result<(u32, u32)> {
    sheet_slot(cx, 0x3e00)
}

/// [`atlas_slot`] for any sheet (0x3f00: the battle atlas).
pub fn sheet_slot(cx: &mut Ctx, sheet: u32) -> ffx_harness::Result<(u32, u32)> {
    let orig = cx.call(SHAPE_NAME, &[sheet])?;
    let table = cx.game.read(0x00c5_9d58, 0x168)?;
    let slot = (0..table.len() / 4)
        .find(|&k| u32::from_le_bytes(table[4 * k..4 * k + 4].try_into().unwrap()) == orig)
        .map(|k| cx.abs(0x00c5_9d58 + 4 * k as u32))
        .expect("meswin name slot");
    eprintln!("meswin name {orig:#x} slot {slot:#x}");
    Ok((slot, orig))
}

/// The PS2 cursor at (x, y) in place of a TOMakePktShapeCursor(0, x, y), as absolute call records
/// (the mod's TOMakePktShapeCursor hook).
pub fn ps2_cursor_calls(cx: &mut Ctx, x: i32, y: i32, name: u32) -> ffx_harness::Result<Vec<(u32, Vec<u32>)>> {
    let (slot, orig) = atlas_slot(cx)?;
    let ball = cx.r32(0x025d_16ac)?;
    // TODrawBtlWindow draws the cursors after TODrawScissorXYWH(0, 0, 512, 416)
    let mut v = vec![(cx.abs(0x008f_dee0), vec![0, 0, 512, 416]), (2, vec![slot, name])];
    v.extend(cursor_parts(x, y, ball).into_iter().map(|(f, a)| (cx.abs(f), a)));
    v.push((2, vec![slot, orig]));
    Ok(v)
}

/// What mod/BattleSeqPatches.cs draws, at fixed places: the Overdrive tab (battle shape 413's one
/// part, (0, 112) 160 x 16 at dx -32, from the PS2 battle atlas `name`) where the battle sequence
/// puts it at rest, (46, 318) in 0x80808080; and the four battle words (TOMakePktBtlIconA kinds
/// 0..3) from their PS2 places in `icon` at the PS2 size, at (200, 120 + 30 k).
pub fn od_tab_calls(cx: &mut Ctx, name: u32) -> ffx_harness::Result<Vec<(u32, Vec<u32>)>> {
    let (slot, orig) = sheet_slot(cx, 0x3f00)?;
    let c = 0x8080_8080;
    let mut v = vec![(cx.abs(0x008f_dee0), vec![0, 0, 512, 416]), (2, vec![slot, name])];
    v.push((cx.abs(SHAPE_PART), vec![413, 0, f(46.0 - 32.0), f(318.0), f(160.0), f(16.0), c, c]));
    v.push((2, vec![slot, orig]));
    const WORDS: [[u32; 4]; 4] = [[0, 110, 164, 127], [164, 96, 231, 118], [55, 96, 152, 110], [0, 96, 45, 106]];
    for (k, r) in WORDS.iter().enumerate() {
        let (x, y) = (200.0, 120.0 + 30.0 * k as f32);
        let uv = r.map(|t| f(4.0 * t as f32));
        v.push((cx.abs(0x0090_3ee0), vec![600, f(x), f(y), f((r[2] - r[0]) as f32), f((r[3] - r[1]) as f32), uv[0], uv[1], uv[2], uv[3], c, c]));
    }
    Ok(v)
}

pub fn build(cx: &mut Ctx, scratch: [u32; 3], ps2_name: u32, fnt_name: u32) -> ffx_harness::Result<Built> {
    let (slot, orig) = atlas_slot(cx)?;
    let fo = Fonts { slot, win: ps2_name, fnt: fnt_name };
    let dum75 = cx.r32(0x025d_16ac)?;

    // ---- main draw (PS2 0x0024a4a8)
    let mut m = List::new(scratch[0]);
    m.set(slot, ps2_name);
    m.c(VU1_SYNC, &[]);
    m.c(OPEN_PKT, &[]);
    let n = cx.call(0x008a_9b50, &[])? as i32; // TkMenuGetPlayerListMax2
    let party = cx.call(0x008a_9b70, &[])? as i32; // TkMenuGetPlayerPartyMask
    let (nx, ny, nw) = (cx.r16(0x025d_1676)?, cx.r16(0x025d_1674)?, cx.r16(0x025d_1672)?);
    let pitch = 44; // num_namepltspc, set by the draw
    let (c0, c1) = (cx.r32(0x025d_166c)?, cx.r32(0x025d_1668)?);
    let pt = |cx: &mut Ctx, a: u32| -> ffx_harness::Result<(i32, i32)> { Ok((cx.r16(a + 2)?, cx.r16(a)?)) };
    let namestr = pt(cx, 0x025d_1662)?;
    let hptex = pt(cx, 0x025d_165e)?;
    let hpstr = pt(cx, 0x025d_165a)?;
    let mptex = pt(cx, 0x025d_1656)?;
    let mpstr = pt(cx, 0x025d_1652)?;
    let slv = pt(cx, 0x025d_164e)?;
    let spstr = pt(cx, 0x025d_164a)?;
    eprintln!("plate: namestr {namestr:?} hptex {hptex:?} hpstr {hpstr:?} mptex {mptex:?} mpstr {mpstr:?} slvshape {slv:?} spstr {spstr:?}");
    let na_pos = (cx.r16(0x023c_c144)?, cx.r16(0x023c_c146)?);
    let na_wh = (cx.r16(0x023c_c13c)?, cx.r16(0x023c_c13e)?);
    let na_c = (cx.r32(0x023c_c148)?, cx.r32(0x023c_c140)?);
    let line = (cx.r16(0x0187_168c)?, cx.r16(0x0187_168e)?, cx.r16(0x0187_1694)?);
    let choice = cx.r32(0x0187_154c)? as i32;
    let (swap_a, swap_b) = (cx.r32(0x0187_155c)? as i32, cx.r32(0x0187_1560)? as i32);
    let timer = cx.call(0x008a_9c60, &[])? as i32;
    eprintln!(
        "main: n {n} party {party} plate {nx},{ny} w {nw} c {c0:08x} {c1:08x} noactive {na_pos:?} {na_wh:?} {:08x} {:08x} line {line:?} choice {choice} dum75 {dum75:08x}",
        na_c.0, na_c.1
    );
    let pick = |cx: &mut Ctx, i: i32| cx.call(0x008a_9b00, &[i as u32]).map(|v| v as i32);
    for i in 0..n {
        let s = cx.r16(0x0187_1678 + 2 * i as u32)?;
        if s == 0 {
            continue;
        }
        let cos = cx.call(0x0088_e750, &[s as u32])? & 0xffff;
        let dx = -((cos as i32) << 10) / 0x3000;
        let chr = pick(cx, i)?;
        let y = ny + i * pitch;
        let x = dx + nx;
        if i < party {
            m.c(BOX_C2, &[(x + na_pos.0) as u32, (y + na_pos.1) as u32, na_wh.0 as u32, na_wh.1 as u32, na_c.0, na_c.1]);
        }
        m.c(PLATE_LR, &[4, 5, x as u32, y as u32, nw as u32, c0, c1]);
        m.c(SHAPE_XYWHC2, &[0x16, (x + line.0) as u32, (y + line.1) as u32, line.2 as u32, 0, GREY, GREY]);
        let name = cx.call(0x008a_c850, &[chr as u32])?; // TOGetSaveChrName
        let colour = cx.call(0x008a_99f0, &[chr as u32])?; // MenuPlayer_DamageClass
        m.c(FONT_1612, &[name, (x + namestr.0) as u32, (y + namestr.1) as u32, colour]);
        m.c(SHAPE_XY, &[0x21, (x + hptex.0) as u32, (y + hptex.1) as u32]);
        let (hp, hpm) = (cx.call(0x008a_98c0, &[chr as u32])?, cx.call(0x008a_9990, &[chr as u32])?);
        // PS2 0x001f6500: font S, colour 0x25
        font_s(&mut m, &fo, &format!("{hp:5}/{hpm:5}"), x + hpstr.0, y + hpstr.1, 0x25);
        m.c(SHAPE_XY, &[0x22, (x + mptex.0) as u32, (y + mptex.1) as u32]);
        let (mp, mpm) = (cx.call(0x008a_9970, &[chr as u32])?, cx.call(0x008a_99b0, &[chr as u32])?);
        font_s(&mut m, &fo, &format!("{mp:4}/{mpm:4}"), x + mpstr.0, y + mpstr.1, 0x25);
        m.c(SHAPE_XY, &[0x3d, (x + slv.0) as u32, (y + slv.1) as u32]);
        let sp = cx.call(0x008a_9b80, &[chr as u32])?;
        // PS2 0x001f5e88(sp, x + spstr.x + slv.x, y + slv.y + spstr.y, 0): font M
        value_m(&mut m, &fo, i64::from(sp), x + spstr.0 + slv.0, y + slv.1 + spstr.1, 0);
        let (nxp, nyp) = (x + namestr.0, y + namestr.1);
        if choice == chr {
            cursor(&mut m, nxp, nyp, dum75);
        }
        if swap_a >= 0 && pick(cx, swap_a)? == chr {
            if swap_b < 0 {
                cursor(&mut m, nxp, nyp, dum75);
            } else if (timer / 2) & 1 != 0 {
                cursor(&mut m, nxp - 10, nyp - 2, dum75);
            }
        }
        if swap_b >= 0 && pick(cx, swap_b)? == chr {
            cursor(&mut m, nxp, nyp, dum75);
        }
    }
    // footer: TOSeqGetPoint(1, name, &x, &y) through a scratch area
    let sp_scratch = scratch[2] + 0x600;
    let point = |cx: &mut Ctx, name: &str| -> ffx_harness::Result<(i32, i32)> {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        cx.write_abs(sp_scratch + 0x20, &b)?;
        cx.write_abs(sp_scratch, &[0; 8])?;
        cx.call(0x008a_e450, &[1, sp_scratch + 0x20, sp_scratch, sp_scratch + 4])?;
        let r = cx.game.read_abs(sp_scratch, 8)?;
        Ok((
            i32::from(i16::from_le_bytes([r[0], r[1]])),
            i32::from(i16::from_le_bytes([r[4], r[5]])),
        ))
    };
    let gilstr = point(cx, "pos_gilstr")?;
    let timestr = point(cx, "pos_timestr")?;
    let gilshape = point(cx, "pos_gilshape")?;
    let gilshape2 = point(cx, "pos_gilshape2")?;
    eprintln!("footer: gil {gilstr:?} time {timestr:?} shapes {gilshape:?} {gilshape2:?}");
    let gil = cx.call(0x0078_4e80, &[])?;
    // PS2 0x001f63c0: font S, colour 0x25; the time PS2 0x0024ad28
    value_s(&mut m, &fo, i64::from(gil), gilstr.0, gilstr.1, 0x25);
    let t = cx.call(0x008a_9c50, &[])?;
    play_time(&mut m, &fo, t, timestr.0, timestr.1, 0x25, if t & 1 != 0 { 0x25 } else { 1 });
    m.c(SHAPE_XY, &[0xd5, gilshape.0 as u32, gilshape.1 as u32]);
    m.c(SHAPE_XY, &[0xd6, (gilshape.0 + gilshape2.0) as u32, (gilshape.1 + gilshape2.1) as u32]);
    let r = |cx: &mut Ctx, a: u32| cx.r16(a);
    let sub = (r(cx, 0x025d_1608)?, r(cx, 0x025d_1606)?, r(cx, 0x025d_1604)?);
    let place = (r(cx, 0x025d_15f8)?, r(cx, 0x025d_15f6)?, r(cx, 0x025d_15f4)?);
    let placeshape = pt(cx, 0x025d_15e8)?;
    let placestr = pt(cx, 0x025d_15e4)?;
    m.c(PLATE_RL, &[0xd2, 0xd3, sub.0 as u32, sub.1 as u32, sub.2 as u32, cx.r32(0x025d_1600)?, cx.r32(0x025d_15fc)?]);
    m.c(PLATE_RL, &[0xd8, 0xd9, place.0 as u32, place.1 as u32, place.2 as u32, cx.r32(0x025d_15f0)?, cx.r32(0x025d_15ec)?]);
    m.c(SHAPE_XY, &[0x27, (place.0 + placeshape.0) as u32, (place.1 + placeshape.1) as u32]);
    let dic = cx.call(0x0086_c410, &[])?;
    let eng = cx.call(0x0078_51d0, &[])?;
    let text = cx.call(0x0086_c430, &[dic, eng])?;
    m.c(FONT_1612, &[text, (place.0 + placestr.0) as u32, (place.1 + placestr.1) as u32, 0]);
    m.c(KICK, &[]);
    m.c(VU1_SYNC, &[]);
    m.set(slot, orig);

    // ---- option window (PS2 0x0024b148 / 0x0024af70)
    let mut c = List::new(scratch[1]);
    let mask = cx.call(0x008a_a460, &[])?;
    let slide = cx.r32(0x0187_1558)? as i32;
    let cur = cx.r32(0x0187_1548)? as i32;
    let count = cx.call(0x008e_2280, &[])? as i32;
    let rect = (r(cx, 0x025d_1648)?, r(cx, 0x025d_1646)?, r(cx, 0x025d_1644)?, r(cx, 0x025d_1642)?);
    let cpitch = r(cx, 0x025d_1640)?;
    let (k0, k1) = (cx.r32(0x025d_163c)?, cx.r32(0x025d_1638)?);
    eprintln!("options: mask {mask:#x} slide {slide:#x} cur {cur} count {count} rect {rect:?} pitch {cpitch} c {k0:08x} {k1:08x}");
    if mask & 0x80800 == 0 {
        c.set(slot, ps2_name);
        c.c(VU1_SYNC, &[]);
        c.c(OPEN_PKT, &[]);
        let mut order: Vec<i32> = if slide != 0 { (0..count).filter(|&i| i != cur).collect() } else { vec![] };
        order.push(cur);
        for i in order {
            let v = (rect.1 + i * cpitch) * slide + (0x1000 - slide) * 3;
            let row_y = v / 4096;
            let alpha = (slide / 32) as u32;
            let x0 = rect.0;
            c.c(BOX_RGBA, &[(x0 + 4) as u32, (row_y + 4) as u32, rect.2 as u32, rect.3 as u32, 0, 0, 0, alpha]);
            c.c(BOX_C2, &[x0 as u32, row_y as u32, rect.2 as u32, rect.3 as u32, (k0 & 0xff_ffff) | alpha << 24, (k1 & 0xff_ffff) | alpha << 24]);
            let id = cx.call(0x008e_2270, &[i as u32])?;
            let grey = u32::from(cx.call(0x008e_2290, &[i as u32])? as i16 == 0);
            let text = cx.call(0x008d_df80, &[id])?; // TkMenuMainMes
            c.c(FONT_1612A, &[text, (x0 + 15) as u32, (row_y + 2) as u32, grey, alpha]);
            if i == cur && slide == 0x1000 {
                cursor(&mut c, x0 + 15, row_y + 2, dum75);
            }
        }
        c.c(KICK, &[]);
        c.c(VU1_SYNC, &[]);
        c.set(slot, orig);
    }
    // the window whose draw is MenuMain_DrawCmdWindow (records of 0x98 from 0x01840900)
    let recs = cx.game.read(0x0184_0900, 0x98 * 32)?;
    let want = cx.abs(0x008e_20f0).to_le_bytes();
    let off = recs.windows(4).position(|w| w == want).expect("no window draws MenuMain_DrawCmdWindow") as u32;
    let cmd_window = cx.abs(0x0184_0900) + off;

    // ---- help bar (PS2 0x00243888 in place of the PC's 0x008a9350)
    let mut h = List::new(scratch[2]);
    if mask & 0x80800 == 0 {
        h.set(slot, ps2_name);
        h.c(VU1_SYNC, &[]);
        h.c(OPEN_PKT, &[]);
        let sub = (r(cx, 0x025d_1630)?, r(cx, 0x025d_162e)?, r(cx, 0x025d_162c)?);
        let help = (r(cx, 0x025d_161c)?, r(cx, 0x025d_161a)?, r(cx, 0x025d_1618)?);
        let helpstr = pt(cx, 0x025d_160a)?;
        h.c(PLATE_LR, &[0x45, 0x46, sub.0 as u32, sub.1 as u32, sub.2 as u32, cx.r32(0x025d_1628)?, cx.r32(0x025d_1624)?]);
        h.c(PLATE_LR, &[6, 7, help.0 as u32, help.1 as u32, help.2 as u32, cx.r32(0x025d_1614)?, cx.r32(0x025d_1610)?]);
        // shape 0x3e, all parts (the PC's TOAdpEasyShapeXYWHC2 draws only the last, the label)
        for (fun, a) in help_label(cx, sub.0 + 0x50, sub.1)? {
            h.c(fun, &a);
        }
        let msg = cx.r32(0x0184_1c00)?;
        eprintln!("help: sub {sub:?} help {help:?} str {helpstr:?} msg {msg:#x}");
        if msg != 0 {
            h.c(FONT_1612, &[msg, (help.0 + helpstr.0) as u32, (help.1 + helpstr.1) as u32, 0]);
        }
        h.c(KICK, &[]);
        h.c(VU1_SYNC, &[]);
        h.set(slot, orig);
    }
    let want = cx.abs(HELP).to_le_bytes();
    let off = recs.windows(4).position(|w| w == want).expect("no window draws the help bar") as u32;
    let help_window = cx.abs(0x0184_0900) + off;
    Ok(Built { main: m, cmd: c, help: h, cmd_window, help_window })
}

/// Shape 0x3e as the PS2 draws it (mod MainMenuPatches.help_label): a TOMkpShapeXY call, which
/// [`expand_shapes`] turns into every part (the PC's own draws only the last).
pub fn help_label(_cx: &mut Ctx, x: i32, y: i32) -> ffx_harness::Result<Vec<(u32, Vec<u32>)>> {
    Ok(vec![(SHAPE_XY, vec![0x3e, x as u32, y as u32])])
}

/// mod Ps2Menu.shape_c2: TOMkpShapeXY / TOMkpShapeXYC2 calls of meswin shapes (ids below 399, not
/// 199) become one TOMkpShapeXYWHUVC2 per layout part (FFX.exe 0x00c5b460), at (x + dx, y + dy),
/// w x h, UVs half a texel inside the cell of the 256 x 256 PS2 atlas.
pub fn expand_shapes(cx: &mut Ctx, calls: &[(u32, Vec<u32>)]) -> ffx_harness::Result<Vec<(u32, Vec<u32>)>> {
    let table = cx.game.read(0x00c5_b460, 0xe68)?;
    let rd32 = |o: usize| u32::from_le_bytes(table[o..o + 4].try_into().unwrap()) as usize;
    let rd16 = |o: usize| u16::from_le_bytes([table[o], table[o + 1]]);
    let words = cx.size_words()?;
    let mut out = Vec::with_capacity(calls.len());
    for (fun, a) in calls {
        // the 16x12 font at the PS2's size (mod Ps2Menu.font / font_a: ps2_sized)
        if *fun == FONT_1612 || *fun == FONT_1612A {
            for &(va, v, _) in &words {
                out.push((2, vec![cx.abs(va), v]));
            }
            out.push((*fun, a.clone()));
            for &(va, _, old) in &words {
                out.push((2, vec![cx.abs(va), old]));
            }
            continue;
        }
        let c2 = *fun == SHAPE_XYC2;
        let id = a.first().copied().unwrap_or(0);
        if (*fun != SHAPE_XY && !c2) || id >= 399 || id == 199 {
            out.push((*fun, a.clone()));
            continue;
        }
        let (x, y) = (a[1] as i32, a[2] as i32);
        let (c0, c1) = if c2 { (a[3], a[4]) } else { (GREY, GREY) };
        let e = rd32(4 + 4 * (id as usize % 200));
        for k in 0..rd32(e) {
            let p = e + 4 + 12 * k;
            let (u, v) = (f32::from(table[p]), f32::from(table[p + 1]));
            let (dx, dy) = (i32::from(table[p + 2] as i8), i32::from(table[p + 3] as i8));
            let (w, h) = (f32::from(rd16(p + 4)), f32::from(rd16(p + 6)));
            let (mut u0, mut u1, mut v0, mut v1) = ((u + 0.5) / 256.0, (u + w - 0.5) / 256.0, (v + 0.5) / 256.0, (v + h - 0.5) / 256.0);
            if rd16(p + 8) == 1 {
                std::mem::swap(&mut u0, &mut u1);
            }
            if rd16(p + 10) == 1 {
                std::mem::swap(&mut v0, &mut v1);
            }
            out.push((SHAPE_UV, vec![id, f((x + dx) as f32), f((y + dy) as f32), f(w), f(h), f(u0), f(v0), f(u1), f(v1), c0, c1]));
        }
    }
    Ok(out)
}

/// The list as the thunk reads it (records 3 are { 3, 1, f }); function VAs rebased by `delta`.
pub fn encode(list: &[(u32, Vec<u32>)], delta: u32) -> String {
    let mut s = String::new();
    let mut w = |v: u32| s.push_str(&v.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>());
    for (f, args) in list {
        w(if *f > 3 { f.wrapping_add(delta) } else { *f });
        if *f == 3 {
            w(1);
            w(args[0]);
            continue;
        }
        w(args.len() as u32);
        for a in args {
            w(*a);
        }
    }
    w(0);
    s
}

/// The PS2 fonts M and S as mod/Ps2Menu.cs draws them: glyph quads of meswin shape 0 with
/// explicit UVs while the meswin name slot holds the ps2fnt name (converter uijob::PS2_FONT:
/// 1024 x 1024, tiles of 256 x 256, faces 0..7 then shadows 0..7; glyph i of font M at (1, 1)
/// in the 16 x 16 cell (i % 16, i / 16), of font S in (i % 16, 6 + i / 16)).
#[derive(Clone, Copy)]
pub struct Fonts {
    /// the meswin name slot (absolute)
    pub slot: u32,
    /// what the slot holds around the font draws (the ps2win name inside ported draws)
    pub win: u32,
    /// the ps2fnt name (absolute)
    pub fnt: u32,
}

const SHAPE_UV: u32 = 0x0090_3ee0; // TOMkpShapeXYWHUVC2

fn glyphs(l: &mut List, fo: &Fonts, s: &str, mut x: i32, y: i32, colour: u32, m: bool) {
    l.set(fo.slot, fo.fnt);
    let (face, shade) = ((colour & 7) as i32, ((colour >> 4) & 7) as i32);
    let (w, h, adv) = if m { (10, 12, 12) } else { (8, 10, 10) };
    for ch in s.bytes() {
        let i = i32::from(ch) - 0x20;
        if i > 0 && i < 0x60 {
            let u = 16 * (i % 16) + 1;
            let v = 16 * (if m { 0 } else { 6 } + i / 16) + 1;
            for pass in 0..2 {
                let k = if pass == 0 { 8 + shade } else { face };
                let (tu, tv) = ((256 * (k % 4) + u) as f32, (256 * (k / 4) + v) as f32);
                let d = if pass == 0 { 1 } else { 0 };
                l.c(SHAPE_UV, &[
                    0,
                    f((x + d) as f32),
                    f((y + d) as f32),
                    f(w as f32),
                    f(h as f32),
                    f(tu / 1024.0),
                    f(tv / 1024.0),
                    f((tu + w as f32) / 1024.0),
                    f((tv + h as f32) / 1024.0),
                    GREY,
                    GREY,
                ]);
            }
        }
        x += adv;
    }
    l.set(fo.slot, fo.win);
}

/// Font M text (PS2 0x001f5ff8).
pub fn font_m(l: &mut List, fo: &Fonts, s: &str, x: i32, y: i32, colour: u32) {
    glyphs(l, fo, s, x, y, colour, true);
}

/// Font S text (PS2 0x001f6630).
pub fn font_s(l: &mut List, fo: &Fonts, s: &str, x: i32, y: i32, colour: u32) {
    glyphs(l, fo, s, x, y, colour, false);
}

/// "%d" in font M, the last cell ending at x + 2 (PS2 0x001f5f58).
pub fn value_m(l: &mut List, fo: &Fonts, v: i64, x: i32, y: i32, colour: u32) {
    let s = v.to_string();
    font_m(l, fo, &s, x + 2 - 12 * s.len() as i32, y, colour);
}

/// "%d" in font S, the last cell ending at x + 2 (PS2 0x001f6408).
pub fn value_s(l: &mut List, fo: &Fonts, v: i64, x: i32, y: i32, colour: u32) {
    let s = v.to_string();
    font_s(l, fo, &s, x + 2 - 10 * s.len() as i32, y, colour);
}

/// The play time (PS2 0x0024ad28, seconds): minutes at x - 18, ":" at x - 28, hours at x - 58.
pub fn play_time(l: &mut List, fo: &Fonts, t: u32, x: i32, y: i32, colour: u32, colon: u32) {
    let (hours, minutes) = ((t / 3600).min(999), t / 60 % 60);
    font_s(l, fo, &format!("{minutes:02}"), x - 18, y, colour);
    font_s(l, fo, ":", x - 28, y, colon);
    font_s(l, fo, &format!("{hours:3}"), x - 58, y, colour);
}
