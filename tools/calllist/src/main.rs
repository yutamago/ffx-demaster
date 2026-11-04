//! calllist <script> <trace windows path> [mod dir...]: shotrun with a few client-side commands
//! for injecting a call list (the ui.menus battle-window check, docs/ui.md):
//!   @base                 learn the image's load delta (FFX.exe is relocated)
//!   @alloc NAME SIZE      VirtualAlloc(RWX) through the IAT; NAME = absolute address
//!   @set NAME VALUE       NAME = VALUE (absolute)
//!   @thunk NAME           the call-list thunk (thunk.s) at NAME
//!   @ps2btl W T [centred] the PS2 battle window draw for window W through thunk T
//!   @ps2menu M C B NAME FNT  the PS2 main menu (menu.rs) through thunks M, C, B; NAME the atlas
//!                         name, FNT the PS2 fonts' texture name (ps2fnt)
//!   @ps2items L S A O NAME FNT [help H] [ability]  the PS2 item / ability list, sub-command bar and (ability)
//!                         party plate (items.rs) through thunks L, S, A; O scratch for out parameters
//!   @ps2screen SCREEN O N F T1 [T2 ...]  a main-menu sub-screen (screens.rs: equip, status, config,
//!                         overdrive, customize) through thunks T1..; O scratch, N / F the ps2win /
//!                         ps2fnt names
//!   @ps2help H N          the main menu's help bar (items::help) through thunk H
//!   @ps2limit T A L R B   the Overdrive mini-game windows (limit.rs): Tidus, Auron, Lulu, the timer
//!                         through thunks T, A, L, R (jmps at the PC draws); B the ps2btl name
//! Substitutions in any line: {NAME[+off]} -> the address in ffx.sym terms (what write/call/read
//! take), {&NAME[+off]} -> its absolute value as 4 LE bytes (hex), {&va:0x...} -> a VA's absolute
//! value as LE bytes, {=NAME} -> its absolute value as a number (a call argument), {f:1.5} -> f32 LE bytes, {i:-3} -> i32 LE bytes.
mod btl;
mod config;
mod customize;
mod equip;
mod hud;
mod items;
mod limit;
mod menu;
mod overdrive;
mod screens;
mod status;

/// thunk.s assembled (as --32, objcopy -O binary).
const THUNK: &str = "5589e5565753e8000000005e81c6f50000008b0685c0745d83f80175138b55088b4e0c8b0c0a8b5608890a83c610ebe283f802750d8b4e0c8b5608890a83c610ebd083f803750eff7508ff560883c40483c60cebbd8b5e0489d98d7c8e0485c97408ff3783ef0449ebf4ffd08d249c8d749e08eb9d5b5f5e5d31c0c3";
use std::collections::HashMap;

fn pe_word(exe: &[u8], va: u32) -> u32 {
    let pe = u32::from_le_bytes(exe[0x3c..0x40].try_into().unwrap()) as usize;
    let nsec = u16::from_le_bytes(exe[pe + 6..pe + 8].try_into().unwrap()) as usize;
    let opt = u16::from_le_bytes(exe[pe + 20..pe + 22].try_into().unwrap()) as usize;
    for i in 0..nsec {
        let o = pe + 24 + opt + 40 * i;
        let rd = |k: usize| u32::from_le_bytes(exe[o + k..o + k + 4].try_into().unwrap());
        let (vsize, vaddr, _rsize, raw) = (rd(8), rd(12) + 0x400000, rd(16), rd(20));
        if va >= vaddr && va < vaddr + vsize {
            let f = (raw + va - vaddr) as usize;
            return u32::from_le_bytes(exe[f..f + 4].try_into().unwrap());
        }
    }
    panic!("va {va:#x} not in a section");
}

fn le(v: u32) -> String {
    v.to_le_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let script = std::fs::read_to_string(args.next().expect("script"))?;
    let trace = args.next().expect("trace");
    let mut options = ffx_harness::Options { virtual_clock: true, ..ffx_harness::Options::default() };
    if let Some(s) = std::env::var("SHOTRUN_STARTUP").ok().and_then(|s| s.parse().ok()) {
        options.startup = std::time::Duration::from_secs(s);
    }
    let max = concat!(env!("CARGO_MANIFEST_DIR"), "/../sandbox/max.ini");
    options.settings = Some(std::env::var("SHOTRUN_SETTINGS").unwrap_or_else(|_| max.into()));
    options.mods = args.map(std::path::PathBuf::from).collect();
    let exe = std::fs::read(std::env::var("FFX_EXE").unwrap_or_else(|_| {
        format!("{}/steam-games/FFX-X2-HD/FFX.exe", std::env::var("HOME").unwrap())
    }))?;
    let mut game = ffx_harness::Game::launch(&options)?;
    eprintln!("slot {}, port {}", game.instance().unwrap_or(0), game.port());
    let mut vars: HashMap<String, u32> = HashMap::new();
    let mut delta: u32 = 0;
    let out = trace.rsplit_once(['\\', '/']).map_or(".", |(d, _)| d).to_string();
    let result: ffx_harness::Result<()> = (|| {
        for raw in script.lines() {
            let line = raw.split('#').next().unwrap_or("").trim().replace("$OUT", &out);
            if line.is_empty() {
                continue;
            }
            // substitutions
            let mut s = String::new();
            let mut rest = line.as_str();
            while let Some(i) = rest.find('{') {
                s.push_str(&rest[..i]);
                let j = rest[i..].find('}').expect("unclosed {") + i;
                let tok = &rest[i + 1..j];
                let val = if let Some(f) = tok.strip_prefix("f:") {
                    le(f.parse::<f32>().expect("float").to_bits())
                } else if let Some(n) = tok.strip_prefix("i:") {
                    le(n.parse::<i32>().expect("int") as u32)
                } else if let Some(v) = tok.strip_prefix("&va:") {
                    le(u32::from_str_radix(v.trim_start_matches("0x"), 16).unwrap().wrapping_add(delta))
                } else if let Some(n) = tok.strip_prefix('=') {
                    format!("{:#x}", vars[n])
                } else {
                    let (amp, t) = tok.strip_prefix('&').map_or((false, tok), |t| (true, t));
                    let (name, off) = t.split_once('+').map_or((t, 0), |(n, o)| {
                        (n, u32::from_str_radix(o.trim_start_matches("0x"), 16).unwrap())
                    });
                    let abs = vars[name].wrapping_add(off);
                    if amp { le(abs) } else { format!("{:#x}", abs.wrapping_sub(delta)) }
                };
                s.push_str(&val);
                rest = &rest[j + 1..];
            }
            s.push_str(rest);
            let words: Vec<&str> = s.split_whitespace().collect();
            match words[0] {
                "@base" => {
                    let va = 0x00c5d788;
                    let r = game.read(va, 4)?;
                    let now = u32::from_le_bytes(r[..4].try_into().unwrap());
                    delta = now.wrapping_sub(pe_word(&exe, va));
                    println!("@base -> delta {delta:#x}");
                }
                "@alloc" => {
                    let r = game.read(0x00b0c09c, 4)?;
                    let va_alloc = u32::from_le_bytes(r[..4].try_into().unwrap());
                    let size: u32 = words[2].parse().unwrap();
                    let reply = game.command(&format!("call {:#x} 0 {size} 0x3000 0x40", va_alloc.wrapping_sub(delta)))?;
                    let p = u32::from_str_radix(reply.trim().trim_start_matches("ok ").trim_start_matches("0x"), 16).unwrap();
                    vars.insert(words[1].to_string(), p);
                    println!("@alloc {} -> {p:#x}", words[1]);
                }
                "@ps2btl" => {
                    // @ps2btl <window index> <THUNK var> [centred] [ps2cursor NAME]
                    let t = vars[words[2]];
                    // a number, or k<kind>: the first open window of that kind
                    let i: u32 = if let Some(k) = words[1].strip_prefix('k') {
                        let k = u16::from_str_radix(k.trim_start_matches("0x"), 16).unwrap();
                        let all = game.read(0x0133c950, 0xf0 * 8)?;
                        (0..8u32)
                            .find(|&i| {
                                let o = (0xf0 * i) as usize;
                                all[o + 1] > 2 && u16::from_le_bytes([all[o + 6], all[o + 7]]) == k
                            })
                            .expect("no open window of that kind")
                    } else {
                        words[1].parse().unwrap()
                    };
                    let w = (0x0133c950 + 0xf0 * i).wrapping_add(delta);
                    let mut list = btl::ps2btl(&mut game, delta, w, t + 0x80, words.contains(&"centred"))?;
                    // ps2cursor NAME: the PS2 cursor (the mod's TOMakePktShapeCursor hook) instead of
                    // TOBtlSetCurPos, which the game would draw as the HD hand
                    if let Some(k) = words.iter().position(|w| *w == "ps2cursor") {
                        let (_, a) = list.pop().expect("cursor call");
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        list.extend(menu::ps2_cursor_calls(&mut cx, a[1] as i32, a[2] as i32, vars[words[k + 1]])?);
                    }
                    // odtab NAME: the Overdrive tab and battle words as BattleSeqPatches draws them
                    // (tab NAME: the tab alone)
                    if let Some(k) = words.iter().position(|w| *w == "odtab" || *w == "tab") {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        list.extend(menu::od_tab_calls(&mut cx, vars[words[k + 1]], words[k] == "odtab")?);
                    }
                    // footer NAME: the battle sequence scenes BattleSeqPatches starts and draws, at rest
                    // (hud::footer): layer 0 before the window's draw, layer 1 after it
                    if let Some(k) = words.iter().position(|w| *w == "footer") {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        let (a, b) = hud::footer(&mut cx, t + 0x800, vars[words[k + 1]])?;
                        let rebase = |l: &menu::List| -> Vec<(u32, Vec<u32>)> {
                            l.calls.iter().map(|(f, a)| (if *f > 3 { f.wrapping_add(delta) } else { *f }, a.clone())).collect()
                        };
                        let mut all = rebase(&a);
                        all.append(&mut list);
                        all.extend(rebase(&b));
                        list = all;
                    }
                    for (f, a) in &list {
                        println!("  call {:#x} {:x?}", f.wrapping_sub(delta), a);
                    }
                    let hex = btl::encode(&list);
                    println!("{}", game.command(&format!("write {:#x} {hex}", (t + 0x100).wrapping_sub(delta)))?);
                    println!("{}", game.command(&format!("write {:#x} {}", (w + 0x90).wrapping_sub(delta), le(t)))?);
                }
                "@thunk" => {
                    // @thunk NAME: the call-list thunk (thunk.s) at NAME
                    let t = vars[words[1]];
                    println!("{}", game.command(&format!("write {:#x} {THUNK}", t.wrapping_sub(delta)))?);
                }
                "@ps2menu" => {
                    // @ps2menu MAIN CMD BAR NAME: the main menu as MainMenuPatches draws it; MAIN, CMD,
                    // BAR hold thunks (0x1000 each), NAME the PS2 atlas's name string
                    let [m, c, b, name, fnt] = [1, 2, 3, 4, 5].map(|k| vars[words[k]]);
                    let built = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        menu::build(&mut cx, [m + 0x8000, c + 0x8000, b + 0x8000], name, fnt)?
                    };
                    for (t, l) in [(m, &built.main), (c, &built.cmd), (b, &built.help)] {
                        let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut game, delta }, &l.calls)?;
                        for (f, a) in &calls {
                            println!("  call {f:#x} {a:x?}");
                        }
                        for (a, s) in &l.strings {
                            let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                            game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                        }
                        game.command(&format!("write {:#x} {}", (t + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                    }
                    println!("{}", game.command(&format!("write {:#x} {}", 0x00c5b2b4u32, le(m)))?);
                    println!("{}", game.command(&format!("write {:#x} {}", built.cmd_window.wrapping_sub(delta), le(c)))?);
                    println!("{}", game.command(&format!("write {:#x} {}", built.help_window.wrapping_sub(delta), le(b)))?);
                }
                "@ps2items" => {
                    let [lt, st, at, out, name, fnt] = [1, 2, 3, 4, 5, 6].map(|k| vars[words[k]]);
                    let ability = words.contains(&"ability");
                    let built = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        items::build(&mut cx, [lt + 0x8000, st + 0x8000, at + 0x8000], out, name, fnt, ability)?
                    };
                    let put = |game: &mut ffx_harness::Game, t: u32, l: &menu::List| -> ffx_harness::Result<()> {
                        let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut *game, delta }, &l.calls)?;
                        for (f, a) in &calls {
                            println!("  call {f:#x} {a:x?}");
                        }
                        for (a, s) in &l.strings {
                            let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                            game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                        }
                        game.command(&format!("write {:#x} {}", (t + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                        Ok(())
                    };
                    if let Some((w, l)) = &built.list {
                        put(&mut game, lt, l)?;
                        println!("{}", game.command(&format!("write {:#x} {}", (w + 0x10).wrapping_sub(delta), le(lt)))?);
                    }
                    if let Some((w, l)) = &built.bar {
                        put(&mut game, st, l)?;
                        println!("{}", game.command(&format!("write {:#x} {}", (w + 0x10).wrapping_sub(delta), le(st)))?);
                    }
                    // help H: the main menu's help bar as MainMenuPatches draws it (menu.rs), through thunk H
                    if let Some(k) = words.iter().position(|w| *w == "help") {
                        let ht = vars[words[k + 1]];
                        let (h, hw) = {
                            let mut cx = menu::Ctx { game: &mut game, delta };
                            items::help(&mut cx, ht + 0x8000, name, ht)?
                        };
                        put(&mut game, ht, &h)?;
                        println!("{}", game.command(&format!("write {:#x} {}", hw.wrapping_sub(delta), le(ht)))?);
                    }
                    if let Some(l) = &built.ability {
                        put(&mut game, at, l)?;
                        // the ability menu record's draw (+4 of 0x00c5a320)
                        println!("{}", game.command(&format!("write {:#x} {}", 0x00c5a324u32, le(at)))?);
                    }
                }
                "@ps2help" => {
                    // @ps2help H N: only the main menu's help bar as MainMenuPatches draws it, through thunk H
                    let [ht, name] = [1, 2].map(|k| vars[words[k]]);
                    let (h, hw) = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        items::help(&mut cx, ht + 0x8000, name, ht)?
                    };
                    let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut game, delta }, &h.calls)?;
                    for (a, s) in &h.strings {
                        let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                        game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                    }
                    game.command(&format!("write {:#x} {}", (ht + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                    println!("{}", game.command(&format!("write {:#x} {}", hw.wrapping_sub(delta), le(ht)))?);
                }
                "@ps2screen" => {
                    let screen = words[1];
                    let [out, name, fnt] = [2, 3, 4].map(|k| vars[words[k]]);
                    let thunks: Vec<u32> = words[5..].iter().map(|w| vars[*w]).collect();
                    let scratch: Vec<u32> = thunks.iter().map(|t| t + 0x8000).collect();
                    let (draws, slot, orig) = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        let (slot, orig) = menu::atlas_slot(&mut cx)?;
                        let dum75 = cx.r32(0x025d_16ac)?;
                        let env = screens::Env { out, fo: menu::Fonts { slot, win: name, fnt }, dum75 };
                        (screens::build(&mut cx, screen, &env, &scratch)?, slot, orig)
                    };
                    assert!(draws.len() <= thunks.len(), "{screen}: {} draws, {} thunks", draws.len(), thunks.len());
                    for (d, &t) in draws.into_iter().zip(&thunks) {
                        let mut calls = vec![(2, vec![slot, name])];
                        calls.extend(d.list.calls.iter().cloned());
                        calls.push((2, vec![slot, orig]));
                        let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut game, delta }, &calls)?;
                        for (f, a) in &calls {
                            println!("  call {f:#x} {a:x?}");
                        }
                        for (a, s) in &d.list.strings {
                            let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                            game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                        }
                        game.command(&format!("write {:#x} {}", (t + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                        println!("{}", game.command(&format!("write {:#x} {}", d.target, le(t)))?);
                    }
                }
                "@protect" => {
                    // @protect S VA LEN: VirtualProtect(VA, LEN, RWX) through GetModuleHandleA and
                    // GetProcAddress of FFX.exe's import table; S is scratch memory
                    let s = vars[words[1]];
                    let va = u32::from_str_radix(words[2].trim_start_matches("0x"), 16).unwrap();
                    let len: u32 = words[3].parse().unwrap();
                    let hex = |t: &str| t.bytes().chain([0]).map(|b| format!("{b:02x}")).collect::<String>();
                    game.command(&format!("write {:#x} {}", s.wrapping_sub(delta), hex("kernel32.dll")))?;
                    game.command(&format!("write {:#x} {}", (s + 0x20).wrapping_sub(delta), hex("VirtualProtect")))?;
                    let iat = |game: &mut ffx_harness::Game, va: u32| -> ffx_harness::Result<u32> {
                        Ok(u32::from_le_bytes(game.read(va, 4)?[..4].try_into().unwrap()))
                    };
                    let gmh = iat(&mut game, 0x00b0c114)?;
                    let gpa = iat(&mut game, 0x00b0c118)?;
                    let k32 = game.command(&format!("call {:#x} {s:#x}", gmh.wrapping_sub(delta)))?;
                    let k32 = u32::from_str_radix(k32.trim().trim_start_matches("ok ").trim_start_matches("0x"), 16).unwrap();
                    let vp = game.command(&format!("call {:#x} {k32:#x} {:#x}", gpa.wrapping_sub(delta), s + 0x20))?;
                    let vp = u32::from_str_radix(vp.trim().trim_start_matches("ok ").trim_start_matches("0x"), 16).unwrap();
                    let r = game.command(&format!("call {:#x} {:#x} {len:#x} 0x40 {:#x}", vp.wrapping_sub(delta), va.wrapping_add(delta), s + 0x40))?;
                    println!("@protect {va:#x} {len} -> {r}");
                }
                "@ps2bg" => {
                    // @ps2bg G: the menu background as MainMenuPatches draws it: TODrawMenuBGC2 jumps
                    // to thunk G (the PS2 gradient), TODrawMenuBG returns at once (the mod skips it only
                    // inside TkMenuDrawAll; the menu screens draw it there)
                    let g = vars[words[1]];
                    let b = game.read(0x025d1678, 8)?;
                    let pack = |b: &[u8]| u32::from_le_bytes([b[3], b[1], b[2], b[0]]);
                    let list = vec![(0x008f3db0u32, vec![0, 0, 512, 416, pack(&b[4..8]), pack(&b[0..4])])];
                    game.command(&format!("write {:#x} {}", (g + 0x100).wrapping_sub(delta), menu::encode(&list, delta)))?;
                    let at = 0x008f5c60u32.wrapping_add(delta);
                    let rel = g.wrapping_sub(at + 5);
                    println!("{}", game.command(&format!("write {:#x} e9{}", 0x008f5c60u32, le(rel)))?);
                    println!("{}", game.command(&format!("write {:#x} c3", 0x008f5c10u32))?);
                }
                "@scenes" | "@stopscenes" => {
                    // @scenes S name...: TOSeqStartScene(1, name) for each, as mod Ps2Menu.scenes does at
                    // a ported screen's first draw (the PS2 at its open); @stopscenes: TOSeqStopScene(1,
                    // name), as Ps2Menu.scenes_frame once no draw asks for it; names in scratch S
                    let f = if words[0] == "@scenes" { 0x008a_f0d0u32 } else { 0x008a_f4f0 };
                    let s = vars[words[1]];
                    for (i, n) in words[2..].iter().enumerate() {
                        let at = s + 0x20 * i as u32;
                        let hex: String = n.bytes().chain([0]).map(|b| format!("{b:02x}")).collect();
                        game.command(&format!("write {:#x} {hex}", at.wrapping_sub(delta)))?;
                        let r = game.command(&format!("call {f:#x} 1 {at:#x}"))?;
                        println!("{} {n} -> {r}", words[0]);
                    }
                }
                "@detour" => {
                    // @detour SITE T HEX: the call at SITE (e8 rel32, an ffx.sym VA) goes to T, which
                    // runs HEX and jumps on to the original target (as a mod hook in front of the
                    // callee; the page must be writable: @protect)
                    let site = u32::from_str_radix(words[1].trim_start_matches("0x"), 16).unwrap();
                    let t = vars[words[2]];
                    let hex = words[3];
                    let r = game.read(site + 1, 4)?;
                    let rel = u32::from_le_bytes(r[..4].try_into().unwrap());
                    let site_abs = site.wrapping_add(delta);
                    let target = site_abs.wrapping_add(5).wrapping_add(rel);
                    let jmp_at = t + (hex.len() / 2) as u32;
                    let code = format!("{hex}e9{}", le(target.wrapping_sub(jmp_at + 5)));
                    game.command(&format!("write {:#x} {code}", t.wrapping_sub(delta)))?;
                    let call = le(t.wrapping_sub(site_abs + 5));
                    println!("{}", game.command(&format!("write {site:#x} e8{call}"))?);
                }
                "@ps2hud" => {
                    // @ps2hud H C P N B F: the battle HUD as mod BattleHudPatches draws it: the help bar
                    // (thunk H in place of TOBtlDrawHelpWin) and the CTB list (thunk C in place of
                    // TOBtlDrawCtbWindow) and the party list (thunk P in place of TOBtlDrawStatusWin): a
                    // jmp at each function's entry (pages made writable with
                    // @protect; needs the game's own code, FFX_HOOK_DISABLE=replacements). N / B the
                    // ps2win / ps2btl names, F ps2fnt's. Thunks of 0x10000: lists at +0x100, strings at +0x8000.
                    let [h, c, p, n, b, fnt] = [1, 2, 3, 4, 5, 6].map(|k| vars[words[k]]);
                    let built = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        let (help, on) = hud::help(&mut cx, h + 0x8000, n)?;
                        let ctb = hud::ctb(&mut cx, c + 0x8000, b)?;
                        let party = hud::status(&mut cx, p + 0x8000, n, b, fnt)?;
                        if on {
                            let s = h + 0xf000;
                            cx.game.command(&format!("write {:#x} 62686c696e6500", s.wrapping_sub(delta)))?;
                            let r = cx.call(0x008a_f1b0, &[2, s])?;
                            println!("@ps2hud bhline -> {r:#x}");
                        }
                        [(h, 0x008936d0u32, help), (c, 0x008a0590u32, ctb), (p, 0x00896450u32, party)]
                    };
                    for (t, fun, l) in built {
                        let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut game, delta }, &l.calls)?;
                        for (a, s) in &l.strings {
                            let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                            game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                        }
                        println!("@ps2hud {fun:#x}: {} calls", calls.len());
                        game.command(&format!("write {:#x} {}", (t + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                        game.command(&format!("write {:#x} {THUNK}", t.wrapping_sub(delta)))?;
                        let at = fun.wrapping_add(delta);
                        println!("{}", game.command(&format!("write {fun:#x} e9{}", le(t.wrapping_sub(at + 5))))?);
                    }
                }
                "@ps2limit" => {
                    // @ps2limit T A L R B: the Overdrive mini-game windows as mod BattleLimitPatches
                    // draws them: thunks T (Tidus, TOBtlDrawTidusLimitWindow), A (Auron), L (Lulu), R
                    // (the timer, 0x008955e0) behind a jmp at each PC draw's entry; TODrawLimitBG and
                    // Wakka's frame return at once (the PS2 draws neither). Pages made writable with
                    // @protect (0x00892000, 0x00895000, 0x00897000, 0x008a1000); the game's own code
                    // has to run (FFX_HOOK_DISABLE=replacements). Thunks of 0x10000: lists at +0x100,
                    // strings at +0x8000, Lulu's keyboard text at +0xf000. B the ps2btl name.
                    let [tt, at, lt, rt, b] = [1, 2, 3, 4, 5].map(|k| vars[words[k]]);
                    let built = {
                        let mut cx = menu::Ctx { game: &mut game, delta };
                        let ti = limit::tidus(&mut cx, tt + 0x8000, b)?;
                        let au = limit::auron(&mut cx, at + 0x8000)?;
                        let lu = limit::lulu(&mut cx, lt + 0x8000, b, lt + 0xf000)?;
                        let tm = limit::timer(&mut cx, rt + 0x8000, b)?;
                        [(tt, 0x0089_7240u32, ti), (at, 0x0089_2740, au), (lt, 0x0089_5660, lu), (rt, 0x0089_55e0, tm)]
                    };
                    for (t, fun, l) in built {
                        let calls = menu::expand_shapes(&mut menu::Ctx { game: &mut game, delta }, &l.calls)?;
                        for (a, s) in &l.strings {
                            let hex: String = s.iter().map(|x| format!("{x:02x}")).collect();
                            game.command(&format!("write {:#x} {hex}", a.wrapping_sub(delta)))?;
                        }
                        for (f, a) in &calls {
                            println!("  call {f:#x} {a:x?}");
                        }
                        println!("@ps2limit {fun:#x}: {} calls", calls.len());
                        game.command(&format!("write {:#x} {}", (t + 0x100).wrapping_sub(delta), menu::encode(&calls, delta)))?;
                        game.command(&format!("write {:#x} {THUNK}", t.wrapping_sub(delta)))?;
                        let at = fun.wrapping_add(delta);
                        println!("{}", game.command(&format!("write {fun:#x} e9{}", le(t.wrapping_sub(at + 5))))?);
                    }
                    println!("{}", game.command("write 0x8a1ce0 c3")?);
                    println!("{}", game.command("write 0x8a1e50 31c0c3")?);
                }
                "@deref" => {
                    // @deref VA LEN [OFF HEX]...: LEN bytes (hex) at the absolute pointer stored at VA
                    // (ffx.sym terms); then HEX written at pointer + OFF for each pair
                    let va = u32::from_str_radix(words[1].trim_start_matches("0x"), 16).unwrap();
                    let len: usize = words[2].parse().unwrap();
                    let r = game.read(va, 4)?;
                    let p = u32::from_le_bytes(r[..4].try_into().unwrap());
                    for pair in words[3..].chunks(2) {
                        let off = u32::from_str_radix(pair[0].trim_start_matches("0x"), 16).unwrap();
                        game.command(&format!("write {:#x} {}", (p + off).wrapping_sub(delta), pair[1]))?;
                    }
                    let b = game.read_abs(p, len)?;
                    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
                    println!("@deref {va:#x} -> {p:#x}: {hex}");
                }
                "@set" => {
                    let v = u32::from_str_radix(words[2].trim_start_matches("0x"), 16).unwrap();
                    vars.insert(words[1].to_string(), v);
                }
                _ => {
                    for l in ffx_harness::replay::play(&mut game, &s, &trace)? {
                        println!("{l}");
                    }
                }
            }
        }
        Ok(())
    })();
    let log = game.new_log();
    let quit = game.quit();
    for l in log.lines().filter(|l| l.contains("FAULT") || l.contains("PANIC")).take(20) {
        eprintln!("log: {l}");
    }
    result?;
    quit
}
