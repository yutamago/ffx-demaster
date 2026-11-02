//! What the C# PS2 battle window draw does for window `w` (absolute address), as a call list
//! (the same algorithm as mod/BattlePatches.cs), with the game's lookups called at the boundary.
use ffx_harness::Game;

fn hexu(r: &str) -> u32 {
    u32::from_str_radix(r.trim().trim_start_matches("ok ").trim_start_matches("0x"), 16).unwrap_or(0)
}

pub fn ps2btl(game: &mut Game, delta: u32, w: u32, scratch: u32, centred: bool) -> ffx_harness::Result<Vec<(u32, Vec<u32>)>> {
    let abs = |va: u32| va.wrapping_add(delta);
    let fr = |a: u32| a.wrapping_sub(delta);
    let wb = game.read_abs(w, 0xf0)?;
    let i16_ = |o: usize| i16::from_le_bytes([wb[o], wb[o + 1]]) as i32;
    let u32_ = |o: usize| u32::from_le_bytes(wb[o..o + 4].try_into().unwrap());
    let kind = i16_(6);
    let chr = i16_(8);
    let list = u32_(0x20);
    let count = u32_(0x24) as i32;
    let nparts = i16_(0x28);
    let showzero = i16_(0x2a) == 1;
    let parts: Vec<u32> = (0..nparts.clamp(0, 8) as usize).map(|k| i16_(0x2c + 2 * k) as u32 & 0xffff).collect();
    let cols = i16_(0x38).max(1);
    let rows = i16_(0x3a);
    let first = i16_(0x40);
    let cur = i16_(0x42);
    let lb = game.read_abs(list, (count.max(0) as usize) * 2 + 2)?;
    let f = |x: f32| x.to_bits();
    let mut names = Vec::new();
    let mut namew = 0f32;
    for e in 0..count {
        let cmd = u16::from_le_bytes([lb[2 * e as usize], lb[2 * e as usize + 1]]) as i16 as i32;
        if cmd == 0xff {
            names.push(None);
            continue;
        }
        let rec = hexu(&game.command(&format!("call 0x0079a4b0 {:#x} {:#x}", cmd as u32, scratch))?);
        let strings = u32::from_le_bytes(game.read_abs(scratch, 4)?[..4].try_into().unwrap());
        let off = u16::from_le_bytes(game.read_abs(rec, 2)?[..2].try_into().unwrap()) as u32;
        let name = strings + off;
        // the scratch is an out-pointer: pass it in sym terms? No: args are raw values, so absolute
        game.command(&format!("call 0x00905290 {name:#x} {scratch:#x} 0 {:#x} {:#x}", f(0.78), f(1.0)))?;
        let wdt = f32::from_bits(u32::from_le_bytes(game.read_abs(scratch, 4)?[..4].try_into().unwrap()));
        let use_ = hexu(&game.command(&format!("call 0x0089aca0 {:#x} {:#x}", chr as u32, cmd as u32))?) as i32;
        if use_ != -4 && wdt > namew {
            namew = wdt;
        }
        let icon = game.read_abs(rec + 0x14, 1)?[0] as i8 as i32 as u32;
        let num = hexu(&game.command(&format!("call 0x007904b0 {:#x}", cmd as u32))?) as i32;
        names.push(Some((name, wdt, use_, icon, num)));
    }
    let mut colw = namew;
    for p in &parts {
        let (k, d) = ((p >> 8) as i32, (p & 0xff) as f32);
        colw += match k {
            1 | 7 => 20.0,
            2 | 4 => 12.0 * d + 5.0,
            3 => 12.0,
            5 | 6 | 0x12 => 12.0 * d,
            _ => 0.0,
        };
    }
    let colw = colw.ceil() as i32;
    let g = game.read(0x025d0af0, 12)?;
    let c1 = u32::from_le_bytes(g[0..4].try_into().unwrap());
    let c0 = u32::from_le_bytes(g[4..8].try_into().unwrap());
    let py = i16::from_le_bytes([g[8], g[9]]) as i32;
    let px = i16::from_le_bytes([g[10], g[11]]) as i32;
    let minx = i16::from_le_bytes(game.read(0x023cc090, 2)?[..2].try_into().unwrap()) as i32;
    let wtot = colw * cols + (cols - 1) * 16;
    let x1 = (px + wtot + 32).max(minx);
    let (y0, y1) = (py, py + rows * 20 + 2);
    let (tx0, ty0) = (px + 16, py + 1);
    let (tx1, ty1) = (tx0 + wtot, ty0 + rows * 20);
    let lines = (count - 1 + cols) / cols;
    eprintln!(
        "ps2btl kind {kind} chr {chr} count {count} parts {parts:x?} cols {cols} rows {rows} namew {namew} colw {colw} rect 0,{y0}..{x1},{y1} c0 {c0:08x} c1 {c1:08x} min {minx} first {first} cur {cur}"
    );
    let mut out = vec![
        (abs(0x008f3db0), vec![0, y0 as u32, x1 as u32, (y1 - y0) as u32, c0, c1]),
        (
            abs(0x008e6cc0),
            vec![f((x1 - 10) as f32), f(ty0 as f32), f(4.0), f((ty1 - ty0) as f32), (first / cols) as u32, rows as u32, lines as u32],
        ),
        (abs(0x008fdee0), vec![tx0 as u32, ty0 as u32, tx1 as u32, ty1 as u32]),
    ];
    let mut curx = tx0;
    let last = count.min(first + rows * cols);
    for e in first.max(0)..last {
        let Some((name, wdt, use_, icon, num)) = names[e as usize] else { continue };
        if use_ == -4 {
            continue;
        }
        let grey = (use_ < 0) as u32;
        let (col, row) = (e % cols, (e - first) / cols);
        let x0 = (tx0 + col * (colw + 16)) as f32;
        let mut x = x0;
        let y = (ty0 + row * 20) as f32;
        if centred {
            x += ((colw as f32 - wdt) / 2.0).floor();
            out.push((abs(0x00905930), vec![name, f(x), f(y), grey, f(0.78), f(1.0)]));
            if e == cur {
                curx = x as i32;
            }
        } else {
            for p in &parts {
                let k = (p >> 8) as i32;
                if k == 0x10 {
                    out.push((abs(0x00905930), vec![name, f(x), f(y), grey, f(0.78), f(1.0)]));
                    x += namew;
                } else if k == 1 && !matches!(kind, 1 | 2 | 3 | 4 | 0xe) {
                    out.push((abs(0x008e6af0), vec![f(x), f(y + 2.0), f(12.0), f(12.0), icon, 0x80, 0x80, 0x80, 0x80]));
                    x += 20.0;
                } else if k == 2 {
                    let d = (p & 0xff) as f32;
                    x += 5.0;
                    if num > 0 || showzero {
                        out.push((abs(0x009055c0), vec![num as u32, f(x + 12.0 * d - 2.0), f(y), grey, f(0.78), f(1.0)]));
                    }
                    x += 12.0 * d;
                }
            }
            if e == cur {
                curx = x0 as i32;
            }
        }
    }
    let crow = (cur - first) / cols;
    out.push((abs(0x008999f0), vec![0, curx as u32, (ty0 + crow * 20) as u32]));
    Ok(out)
}

/// The call list as the thunk reads it: { fn, n, args[n] }..., 0.
pub fn encode(list: &[(u32, Vec<u32>)]) -> String {
    let mut s = String::new();
    let mut w = |v: u32| s.push_str(&v.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>());
    for (f, args) in list {
        w(*f);
        w(args.len() as u32);
        for a in args {
            w(*a);
        }
    }
    w(0);
    s
}
