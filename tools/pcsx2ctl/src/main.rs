//! pcsx2ctl: drives a running PCSX2 (2.x, Qt) for the PS2 side of the
//! PS2-vs-Demaster comparison (`docs/compare.md`).
//!
//! - PINE (PCSX2's IPC, TCP `127.0.0.1:<slot>` on Windows): EE memory,
//!   save states, status.
//! - Keys: `WM_KEYDOWN`/`WM_KEYUP` posted to PCSX2's display window, so the
//!   pad and the hotkeys work without focus and without touching the
//!   user's keyboard. The pad bindings are the ones `tools/compare` writes
//!   into `PCSX2.ini` (`[Pad1]`, see `BUTTONS`).
//! - Screenshots: PCSX2's own screenshot hotkey (the GS output at the
//!   internal resolution, written to its `snaps` folder), or a capture of
//!   the display window only (`PrintWindow`), never the desktop.
//!
//! Commands: `status`, `windows`, `run <script> [NAME=value]...`, and every
//! script line as a command (`pcsx2ctl savestate 3`).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

type Result<T> = std::result::Result<T, String>;

/// The pad buttons and the keys they are bound to in `[Pad1]` of the
/// PCSX2.ini `tools/compare/pcsx2.sh` sets up (key names as PCSX2 writes
/// them, `Keyboard/<name>`).
const BUTTONS: &[(&str, &str)] = &[
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("cross", "K"),
    ("circle", "L"),
    ("square", "J"),
    ("triangle", "I"),
    ("start", "Return"),
    ("select", "Backspace"),
    ("l1", "Q"),
    ("r1", "E"),
    ("l2", "1"),
    ("r2", "3"),
];

/// Milliseconds per frame for `wait`: PAL, 50 Hz.
const FRAME_MS: u64 = 20;

// ---------------------------------------------------------------- PINE

struct Pine {
    s: TcpStream,
}

const OP_READ8: u8 = 0;
const OP_READ32: u8 = 2;
const OP_WRITE8: u8 = 4;
const OP_WRITE32: u8 = 6;
const OP_VERSION: u8 = 8;
const OP_SAVESTATE: u8 = 9;
const OP_LOADSTATE: u8 = 0xa;
const OP_TITLE: u8 = 0xb;
const OP_ID: u8 = 0xc;
const OP_UUID: u8 = 0xd;
const OP_GAMEVERSION: u8 = 0xe;
const OP_STATUS: u8 = 0xf;

impl Pine {
    fn connect(slot: u16) -> Result<Pine> {
        let s = TcpStream::connect(("127.0.0.1", slot)).map_err(|e| {
            format!(
                "PINE: cannot connect to 127.0.0.1:{slot} ({e}); is PCSX2 running with PINE on?"
            )
        })?;
        s.set_read_timeout(Some(Duration::from_secs(30))).ok();
        Ok(Pine { s })
    }

    /// Sends one message holding `ops` (each an opcode and its arguments)
    /// and returns the reply's data after the result byte.
    fn batch(&mut self, ops: &[u8]) -> Result<Vec<u8>> {
        let mut msg = Vec::with_capacity(ops.len() + 4);
        msg.extend(u32::try_from(ops.len() + 4).unwrap().to_le_bytes());
        msg.extend(ops);
        self.s
            .write_all(&msg)
            .map_err(|e| format!("PINE write: {e}"))?;
        let mut head = [0u8; 4];
        self.s
            .read_exact(&mut head)
            .map_err(|e| format!("PINE read: {e}"))?;
        let size = u32::from_le_bytes(head) as usize;
        let mut rest = vec![0u8; size.saturating_sub(4)];
        self.s
            .read_exact(&mut rest)
            .map_err(|e| format!("PINE read: {e}"))?;
        if rest.first() != Some(&0) {
            return Err("PINE: the emulator refused the request (no game running?)".into());
        }
        Ok(rest[1..].to_vec())
    }

    fn string(&mut self, op: u8) -> Result<String> {
        let r = self.batch(&[op])?;
        if r.len() < 4 {
            return Ok(String::new());
        }
        let n = u32::from_le_bytes(r[0..4].try_into().unwrap()) as usize;
        let s = &r[4..(4 + n).min(r.len())];
        Ok(String::from_utf8_lossy(s).trim_end_matches('\0').to_owned())
    }

    fn status(&mut self) -> Result<u32> {
        let r = self.batch(&[OP_STATUS])?;
        Ok(u32::from_le_bytes(r[0..4].try_into().unwrap()))
    }

    fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        let mut a = addr;
        let end = addr + len as u32;
        while a < end {
            let mut ops = Vec::new();
            let mut kinds = Vec::new();
            while a < end && ops.len() < 40000 {
                if a % 4 == 0 && end - a >= 4 {
                    ops.push(OP_READ32);
                    ops.extend(a.to_le_bytes());
                    kinds.push(4);
                    a += 4;
                } else {
                    ops.push(OP_READ8);
                    ops.extend(a.to_le_bytes());
                    kinds.push(1);
                    a += 1;
                }
            }
            let r = self.batch(&ops)?;
            out.extend(&r[..kinds.iter().sum::<usize>().min(r.len())]);
        }
        Ok(out)
    }

    fn write(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        let mut i = 0usize;
        while i < data.len() {
            let mut ops = Vec::new();
            while i < data.len() && ops.len() < 40000 {
                let a = addr + i as u32;
                if a % 4 == 0 && data.len() - i >= 4 {
                    ops.push(OP_WRITE32);
                    ops.extend(a.to_le_bytes());
                    ops.extend(&data[i..i + 4]);
                    i += 4;
                } else {
                    ops.push(OP_WRITE8);
                    ops.extend(a.to_le_bytes());
                    ops.push(data[i]);
                    i += 1;
                }
            }
            self.batch(&ops)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- windows

#[cfg(windows)]
mod win {
    use super::Result;
    use std::path::Path;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VK_TO_VSC, MapVirtualKeyW};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    pub struct Win {
        pub hwnd: HWND,
        pub parent: HWND,
        pub class: String,
        pub title: String,
        pub rect: RECT,
        pub visible: bool,
    }

    fn text(h: HWND, class: bool) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe {
            if class {
                GetClassNameW(h, buf.as_mut_ptr(), 256)
            } else {
                GetWindowTextW(h, buf.as_mut_ptr(), 256)
            }
        };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn info(h: HWND, parent: HWND) -> Win {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe { GetClientRect(h, &mut rect) };
        Win {
            hwnd: h,
            parent,
            class: text(h, true),
            title: text(h, false),
            rect,
            visible: unsafe { IsWindowVisible(h) } != 0,
        }
    }

    struct Ctx {
        pid: u32,
        out: Vec<Win>,
    }

    unsafe extern "system" fn top(h: HWND, l: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(l as *mut Ctx) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        if pid == ctx.pid {
            ctx.out.push(info(h, std::ptr::null_mut()));
            unsafe { EnumChildWindows(h, Some(child), l) };
        }
        1
    }

    unsafe extern "system" fn child(h: HWND, l: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(l as *mut Ctx) };
        ctx.out.push(info(h, unsafe { GetParent(h) }));
        1
    }

    /// Every window of process `pid`, top-level ones followed by their
    /// children.
    pub fn windows(pid: u32) -> Vec<Win> {
        let mut ctx = Ctx {
            pid,
            out: Vec::new(),
        };
        unsafe { EnumWindows(Some(top), &mut ctx as *mut Ctx as LPARAM) };
        ctx.out
    }

    /// PCSX2's display: the largest visible window of the process whose
    /// class is not a tooltip or menu; with `-nogui` that is the top-level
    /// display window, in the full UI the display widget's child window.
    pub fn display(pid: u32) -> Result<HWND> {
        let ws = windows(pid);
        ws.iter()
            .filter(|w| w.visible && w.rect.right > 200 && w.rect.bottom > 150)
            .filter(|w| !w.class.contains("Tool") && !w.class.contains("Popup"))
            .filter(|w| {
                w.title.contains("Display")
                    || w.title.contains("FINAL")
                    || w.title.contains("Final")
                    || !w.parent.is_null()
                    || ws.len() == 1
            })
            .max_by_key(|w| (!w.parent.is_null() as i32, w.rect.right * w.rect.bottom))
            .map(|w| w.hwnd)
            .or_else(|| {
                ws.iter()
                    .filter(|w| w.visible && w.rect.right > 200)
                    .max_by_key(|w| w.rect.right * w.rect.bottom)
                    .map(|w| w.hwnd)
            })
            .ok_or_else(|| format!("no visible window of PID {pid}"))
    }

    pub fn vk(name: &str) -> Result<u16> {
        let n = name.to_ascii_lowercase();
        let v = match n.as_str() {
            "up" => 0x26,
            "down" => 0x28,
            "left" => 0x25,
            "right" => 0x27,
            "return" | "enter" => 0x0d,
            "backspace" => 0x08,
            "escape" => 0x1b,
            "space" => 0x20,
            "tab" => 0x09,
            _ if n.len() == 1 && n.as_bytes()[0].is_ascii_alphanumeric() => {
                n.to_ascii_uppercase().as_bytes()[0] as u16
            }
            _ if n.starts_with('f')
                && n[1..].parse::<u16>().is_ok_and(|k| (1..=12).contains(&k)) =>
            {
                0x70 + n[1..].parse::<u16>().unwrap() - 1
            }
            _ => return Err(format!("unknown key {name}")),
        };
        Ok(v)
    }

    pub fn key(h: HWND, vk: u16, down: bool) {
        let sc = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) };
        let ext = matches!(vk, 0x25..=0x28);
        let mut l: isize = 1 | ((sc as isize) << 16) | if ext { 1 << 24 } else { 0 };
        if !down {
            l |= (1 << 30) | (1 << 31);
        }
        unsafe {
            PostMessageW(h, if down { WM_KEYDOWN } else { WM_KEYUP }, vk as usize, l);
        }
    }

    /// The window's client area through `PrintWindow` (DWM's copy, so it
    /// works for a D3D/Vulkan swap chain and a covered window), as RGB rows.
    pub fn capture(h: HWND) -> Result<(u32, u32, Vec<u8>)> {
        unsafe {
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(h, &mut r);
            let (w, hgt) = (r.right - r.left, r.bottom - r.top);
            if w <= 0 || hgt <= 0 {
                return Err("window has no client area".into());
            }
            let wdc = GetDC(h);
            let mdc = CreateCompatibleDC(wdc);
            let bmp = CreateCompatibleBitmap(wdc, w, hgt);
            let old = SelectObject(mdc, bmp);
            // PW_CLIENTONLY | PW_RENDERFULLCONTENT
            let ok = PrintWindow(h, mdc, 1 | 2);
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -hgt;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            let mut px = vec![0u8; (w * hgt * 4) as usize];
            GetDIBits(
                mdc,
                bmp,
                0,
                hgt as u32,
                px.as_mut_ptr().cast(),
                &mut bi,
                DIB_RGB_COLORS,
            );
            SelectObject(mdc, old);
            DeleteObject(bmp);
            DeleteDC(mdc);
            ReleaseDC(h, wdc);
            if ok == 0 {
                return Err("PrintWindow failed".into());
            }
            let rgb = px.chunks(4).flat_map(|p| [p[2], p[1], p[0]]).collect();
            Ok((w as u32, hgt as u32, rgb))
        }
    }

    pub fn save_png(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<()> {
        super::write_png(path, w, h, rgb)
    }
}

fn write_png(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgb).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------- driver

struct Ctl {
    slot: u16,
    pid: Option<u32>,
    snaps: PathBuf,
    sstates: PathBuf,
    pine: Option<Pine>,
}

/// The save state slot `state` and `keep` go through.
const WORK_SLOT: u8 = 10;

fn parse_u32(s: &str) -> Result<u32> {
    let r = if let Some(h) = s.strip_prefix("0x") {
        u32::from_str_radix(h, 16)
    } else {
        s.parse()
    };
    r.map_err(|_| format!("bad number {s}"))
}

fn parse_hex(s: &str) -> Result<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err(format!("odd hex string {s}"));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| format!("bad hex {s}")))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sleep_frames(n: u64) {
    std::thread::sleep(Duration::from_millis(n * FRAME_MS));
}

impl Ctl {
    fn pine(&mut self) -> Result<&mut Pine> {
        if self.pine.is_none() {
            self.pine = Some(Pine::connect(self.slot)?);
        }
        Ok(self.pine.as_mut().unwrap())
    }

    /// PCSX2's file for save state slot `slot` of the running disc:
    /// `<serial> (<disc CRC>).<slot>.p2s` in the save state folder.
    fn slot_file(&mut self, slot: u8) -> Result<PathBuf> {
        let p = self.pine()?;
        let id = p.string(OP_ID)?;
        let crc = p.string(OP_UUID)?.to_ascii_uppercase();
        Ok(self.sstates.join(format!("{id} ({crc}).{slot:02}.p2s")))
    }

    /// Saves to slot `slot` and waits for PCSX2 to finish the file.
    fn save_slot(&mut self, slot: u8) -> Result<PathBuf> {
        let f = self.slot_file(slot)?;
        let before = std::fs::metadata(&f).and_then(|m| m.modified()).ok();
        self.pine()?.batch(&[OP_SAVESTATE, slot])?;
        let t0 = Instant::now();
        let mut last = (None, 0u64);
        loop {
            std::thread::sleep(Duration::from_millis(250));
            if let Ok(m) = std::fs::metadata(&f) {
                let t = m.modified().ok();
                if t.is_some() && t != before {
                    if last == (t, m.len()) {
                        return Ok(f);
                    }
                    last = (t, m.len());
                }
            }
            if t0.elapsed() > Duration::from_secs(30) {
                return Err(format!("save state {} did not appear", f.display()));
            }
        }
    }

    fn pid(&self) -> Result<u32> {
        self.pid
            .ok_or_else(|| "needs --pid <PCSX2 PID> (tasklist.exe)".to_string())
    }

    #[cfg(windows)]
    fn key(&mut self, name: &str, hold: u64, after: u64) -> Result<()> {
        let h = win::display(self.pid()?)?;
        let vk = win::vk(name)?;
        win::key(h, vk, true);
        sleep_frames(hold);
        win::key(h, vk, false);
        sleep_frames(after);
        Ok(())
    }

    #[cfg(not(windows))]
    fn key(&mut self, _: &str, _: u64, _: u64) -> Result<()> {
        Err("keys need the Windows build".into())
    }

    fn newest_snap(&self, kind: &str) -> Option<(PathBuf, SystemTime)> {
        let mut best: Option<(PathBuf, SystemTime)> = None;
        let mut stack = vec![self.snaps.clone()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().contains(kind))
                {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if best.as_ref().is_none_or(|b| t > b.1) {
                            best = Some((p, t));
                        }
                    }
                }
            }
        }
        best
    }

    /// Presses a PCSX2 hotkey that writes a file into `snaps` (whose name
    /// contains `kind`), waits for the new file and moves it to `out`.
    fn grab(&mut self, hotkey: &str, kind: &str, out: &Path) -> Result<()> {
        let before = self.newest_snap(kind).map(|b| b.1);
        self.key(hotkey, 2, 0)?;
        let t0 = Instant::now();
        loop {
            if let Some((p, t)) = self.newest_snap(kind) {
                if before.is_none_or(|b| t > b) {
                    if let Some(d) = out.parent() {
                        std::fs::create_dir_all(d).ok();
                    }
                    // PCSX2 writes the PNG on a worker thread: wait until it
                    // can be opened exclusively and its size holds
                    let mut last = u64::MAX;
                    loop {
                        std::thread::sleep(Duration::from_millis(300));
                        let len = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                        if len > 0
                            && len == last
                            && std::fs::OpenOptions::new().write(true).open(&p).is_ok()
                        {
                            break;
                        }
                        last = len;
                        if t0.elapsed() > Duration::from_secs(20) {
                            return Err(format!("{} stays busy", p.display()));
                        }
                    }
                    std::fs::copy(&p, out).map_err(|e| format!("{}: {e}", out.display()))?;
                    std::fs::remove_file(&p).ok();
                    return Ok(());
                }
            }
            if t0.elapsed() > Duration::from_secs(20) {
                return Err(format!(
                    "no new {kind} file appeared in {}",
                    self.snaps.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// PCSX2's screenshot hotkey (F12 in our ini): the GS output at the
    /// internal resolution, as PNG.
    fn shot(&mut self, out: &Path) -> Result<()> {
        self.grab("f12", ".png", out)?;
        println!("shot {}", out.display());
        Ok(())
    }

    fn exec(&mut self, line: &str) -> Result<()> {
        let a: Vec<&str> = line.split_whitespace().collect();
        if a.is_empty() || a[0].starts_with('#') {
            return Ok(());
        }
        let num = |i: usize, d: u64| -> Result<u64> {
            a.get(i).map_or(Ok(d), |s| parse_u32(s).map(u64::from))
        };
        match a[0] {
            "wait" => sleep_frames(num(1, 1)?),
            "press" => {
                let b = a.get(1).ok_or("press <button>")?;
                let k = BUTTONS
                    .iter()
                    .find(|x| x.0 == *b)
                    .ok_or_else(|| format!("unknown button {b}"))?
                    .1;
                self.key(k, num(2, 4)?, num(3, 20)?)?;
            }
            "key" => {
                let k = a.get(1).ok_or("key <name>")?.to_string();
                self.key(&k, num(2, 4)?, num(3, 20)?)?;
            }
            "savestate" => {
                let s = num(1, 1)? as u8;
                self.pine()?.batch(&[OP_SAVESTATE, s])?;
                // the save runs on the emulator thread; give it time
                std::thread::sleep(Duration::from_secs(3));
                println!("saved state {s}");
            }
            "loadstate" => {
                let s = num(1, 1)? as u8;
                self.pine()?.batch(&[OP_LOADSTATE, s])?;
                std::thread::sleep(Duration::from_secs(3));
                println!("loaded state {s}");
            }
            "keep" => {
                // keep <name>: save the state as sstates\compare\<name>.p2s
                let name = a.get(1).ok_or("keep <name>")?;
                let f = self.save_slot(WORK_SLOT)?;
                let dir = self.sstates.join("compare");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let to = dir.join(format!("{name}.p2s"));
                std::fs::copy(&f, &to).map_err(|e| format!("{}: {e}", to.display()))?;
                println!("kept {}", to.display());
            }
            "state" => {
                // state <name>: load sstates\compare\<name>.p2s (through the work slot)
                let name = a.get(1).ok_or("state <name>")?;
                let from = self.sstates.join("compare").join(format!("{name}.p2s"));
                let f = self.slot_file(WORK_SLOT)?;
                std::fs::copy(&from, &f).map_err(|e| format!("{}: {e}", from.display()))?;
                self.pine()?.batch(&[OP_LOADSTATE, WORK_SLOT])?;
                std::thread::sleep(Duration::from_millis(1500));
                println!("state {name}");
            }
            "read" => {
                let addr = parse_u32(a.get(1).ok_or("read <addr> <len>")?)?;
                let len = num(2, 4)? as usize;
                let d = self.pine()?.read(addr, len)?;
                for (i, c) in d.chunks(16).enumerate() {
                    println!("{:08x}: {}", addr as usize + i * 16, hex(c));
                }
            }
            "dump" => {
                let addr = parse_u32(a.get(1).ok_or("dump <addr> <len> <file>")?)?;
                let len = num(2, 4)? as usize;
                let d = self.pine()?.read(addr, len)?;
                let f = a.get(3).ok_or("dump <addr> <len> <file>")?;
                std::fs::write(f, d).map_err(|e| format!("{f}: {e}"))?;
                println!("dumped {len:#x} bytes to {f}");
            }
            "write" => {
                let addr = parse_u32(a.get(1).ok_or("write <addr> <hex>")?)?;
                let d = parse_hex(&a[2..].concat())?;
                self.pine()?.write(addr, &d)?;
            }
            "writefile" => {
                let addr = parse_u32(a.get(1).ok_or("writefile <addr> <file> [offset] [len]")?)?;
                let f = a.get(2).ok_or("writefile <addr> <file>")?;
                let d = std::fs::read(f).map_err(|e| format!("{f}: {e}"))?;
                let off = num(3, 0)? as usize;
                let len = num(4, (d.len() - off) as u64)? as usize;
                self.pine()?.write(addr, &d[off..off + len])?;
            }
            "waitfor" => {
                // waitfor <addr> <hex> [timeout frames]: until memory equals
                let addr = parse_u32(a.get(1).ok_or("waitfor <addr> <hex>")?)?;
                let want = parse_hex(a.get(2).ok_or("waitfor <addr> <hex>")?)?;
                let limit = num(3, 3000)?;
                let t0 = Instant::now();
                loop {
                    let got = self.pine()?.read(addr, want.len())?;
                    if got == want {
                        break;
                    }
                    if t0.elapsed() > Duration::from_millis(limit * FRAME_MS) {
                        return Err(format!(
                            "waitfor {addr:#x}: still {} after {limit} frames",
                            hex(&got)
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(FRAME_MS));
                }
            }
            "pressuntil" => {
                // pressuntil <button> <addr> <hex> [tries]: press, wait 100
                // frames, until memory equals (a skip that must land once)
                let usage = "pressuntil <button> <addr> <hex> [tries]";
                let b = *a.get(1).ok_or(usage)?;
                let k = BUTTONS
                    .iter()
                    .find(|x| x.0 == b)
                    .ok_or_else(|| format!("unknown button {b}"))?
                    .1;
                let addr = parse_u32(a.get(2).ok_or(usage)?)?;
                let want = parse_hex(a.get(3).ok_or(usage)?)?;
                let tries = num(4, 30)?;
                let mut done = false;
                for _ in 0..tries {
                    self.key(k, 4, 100)?;
                    if self.pine()?.read(addr, want.len())? == want {
                        done = true;
                        break;
                    }
                }
                if !done {
                    return Err(format!("pressuntil: {addr:#x} never became {}", hex(&want)));
                }
            }
            "gsdump" => {
                // gsdump <file.gs.zst>: PCSX2's single-frame GS dump (F11 in
                // our ini), to replay with another renderer or scale
                let out = Path::new(a.get(1).ok_or("gsdump <file>")?);
                self.grab("f11", ".gs", out)?;
                println!("gsdump {}", out.display());
            }
            "shot" => self.shot(Path::new(a.get(1).ok_or("shot <png>")?))?,
            "wincap" => {
                #[cfg(windows)]
                {
                    let h = win::display(self.pid()?)?;
                    let (w, hh, rgb) = win::capture(h)?;
                    let p = Path::new(a.get(1).ok_or("wincap <png>")?);
                    win::save_png(p, w, hh, &rgb)?;
                    println!("wincap {} {w}x{hh}", p.display());
                }
                #[cfg(not(windows))]
                return Err("wincap needs the Windows build".into());
            }
            "status" => {
                let p = self.pine()?;
                let st = p.status()?;
                let ver = p.string(OP_VERSION)?;
                let (title, id, gv) = if st == 2 {
                    Default::default()
                } else {
                    (
                        p.string(OP_TITLE)?,
                        p.string(OP_ID)?,
                        p.string(OP_GAMEVERSION)?,
                    )
                };
                let st = ["running", "paused", "shutdown"]
                    .get(st as usize)
                    .copied()
                    .unwrap_or("?");
                println!("{ver}: {st}; {id} {gv} {title}");
            }
            "echo" => println!("{}", a[1..].join(" ")),
            "windows" =>
            {
                #[cfg(windows)]
                for w in win::windows(self.pid()?) {
                    println!(
                        "{:?} parent {:?} {}x{} visible {} class {:?} title {:?}",
                        w.hwnd, w.parent, w.rect.right, w.rect.bottom, w.visible, w.class, w.title
                    );
                }
            }
            c => return Err(format!("unknown command {c}")),
        }
        Ok(())
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut ctl = Ctl {
        slot: 28011,
        pid: None,
        snaps: PathBuf::from(r"C:\_workspaces\pcsx2\snaps"),
        sstates: PathBuf::from(r"C:\_workspaces\pcsx2\sstates"),
        pine: None,
    };
    while args.first().is_some_and(|a| a.starts_with("--")) {
        let flag = args.remove(0);
        let val = if args.is_empty() {
            String::new()
        } else {
            args.remove(0)
        };
        match flag.as_str() {
            "--slot" => ctl.slot = val.parse().expect("--slot"),
            "--pid" => ctl.pid = Some(val.parse().expect("--pid")),
            "--snaps" => ctl.snaps = PathBuf::from(val),
            "--sstates" => ctl.sstates = PathBuf::from(val),
            _ => {
                eprintln!("unknown flag {flag}");
                std::process::exit(2);
            }
        }
    }
    if args.is_empty() {
        eprintln!(
            "usage: pcsx2ctl [--pid PID] [--slot 28011] [--snaps DIR] <command>\n  \
             run <script> [NAME=value]...   script lines: wait, press, key, savestate, loadstate,\n  \
             read, dump, write, writefile, waitfor, pressuntil, shot, gsdump, wincap,\n  \
             keep, state, status, echo\n  \
             <script line>                  one command, e.g. status, windows, savestate 3"
        );
        std::process::exit(2);
    }
    let res = if args[0] == "run" {
        let script = std::fs::read_to_string(&args[1]).unwrap_or_else(|e| {
            eprintln!("{}: {e}", args[1]);
            std::process::exit(2)
        });
        let vars: Vec<(String, String)> = args[2..]
            .iter()
            .filter_map(|a| {
                a.split_once('=')
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
            .collect();
        let mut r = Ok(());
        for (n, raw) in script.lines().enumerate() {
            let mut line = raw.trim().to_string();
            for (k, v) in &vars {
                line = line.replace(&format!("${k}"), v);
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            println!("> {line}");
            if let Err(e) = ctl.exec(&line) {
                r = Err(format!("{}:{}: {e}", args[1], n + 1));
                break;
            }
        }
        r
    } else {
        ctl.exec(&args.join(" "))
    };
    if let Err(e) = res {
        eprintln!("pcsx2ctl: {e}");
        std::process::exit(1);
    }
}
