//! calllist <script> <trace windows path> [mod dir...]: shotrun with a few client-side commands
//! for injecting a call list (the ui.menus battle-window check, docs/ui.md):
//!   @base                 learn the image's load delta (FFX.exe is relocated)
//!   @alloc NAME SIZE      VirtualAlloc(RWX) through the IAT; NAME = absolute address
//!   @set NAME VALUE       NAME = VALUE (absolute)
//! Substitutions in any line: {NAME[+off]} -> the address in ffx.sym terms (what write/call/read
//! take), {&NAME[+off]} -> its absolute value as 4 LE bytes (hex), {&va:0x...} -> a VA's absolute
//! value as LE bytes, {f:1.5} -> f32 LE bytes, {i:-3} -> i32 LE bytes.
mod btl;
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
                    // @ps2btl <window index> <THUNK var> [centred]
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
                    let list = btl::ps2btl(&mut game, delta, w, t + 0x80, words.get(3) == Some(&"centred"))?;
                    for (f, a) in &list {
                        println!("  call {:#x} {:x?}", f.wrapping_sub(delta), a);
                    }
                    let hex = btl::encode(&list);
                    println!("{}", game.command(&format!("write {:#x} {hex}", (t + 0x100).wrapping_sub(delta)))?);
                    println!("{}", game.command(&format!("write {:#x} {}", (w + 0x90).wrapping_sub(delta), le(t)))?);
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
