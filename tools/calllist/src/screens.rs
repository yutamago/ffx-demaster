//! `@ps2screen SCREEN O N F T1 [T2 ...]`: the main menu's sub-screens as their mod/*MenuPatches.cs
//! draw them (ui.menus = ps2), one module per screen. O is scratch memory for out parameters, N the
//! PS2 atlas name (ps2win), F the PS2 fonts' name (ps2fnt), T1.. thunks (64 KB each: the list at
//! +0x100, strings at +0x8000). A screen's `build` returns one [`Draw`] per draw it replaces; the
//! i-th goes through thunk Ti, and Ti is written to the draw's `target` (a window's +0x10, a menu
//! record's +4, ...). main.rs wraps each list in the meswin slot swap (N in, the original out).
use crate::menu::{Ctx, Fonts, List};

/// A replaced draw: `target` is the ffx.sym VA of the pointer that gets the thunk (main.rs writes
/// it with `write`), `list` the calls the C# hook makes there.
pub struct Draw {
    pub target: u32,
    pub list: List,
}

/// What every screen gets: `out` scratch for out parameters (absolute), the fonts (slot, ps2win,
/// ps2fnt names), the PS2 cursor ball colour (dum75).
pub struct Env {
    pub out: u32,
    pub fo: Fonts,
    pub dum75: u32,
}

pub fn build(cx: &mut Ctx, screen: &str, env: &Env, scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    match screen {
        "equip" => crate::equip::build(cx, env, scratch),
        "status" => crate::status::build(cx, env, scratch),
        "config" => crate::config::build(cx, env, scratch),
        "overdrive" => crate::overdrive::build(cx, env, scratch),
        "customize" => crate::customize::build(cx, env, scratch),
        _ => panic!("unknown screen {screen}"),
    }
}
