//! The Status screen as mod/StatusMenuPatches.cs draws it (ui.menus = ps2): not ported yet.
use crate::menu::Ctx;
use crate::screens::{Draw, Env};

pub fn build(_cx: &mut Ctx, _env: &Env, _scratch: &[u32]) -> ffx_harness::Result<Vec<Draw>> {
    Ok(vec![])
}
