# ffx-demaster

FFX Demaster restores the PS2 look in the FFX HD Remaster (Steam): presentation
from the PS2, logic from the PC. The mod is in ffx-mods (`../ffx-mods`,
github.com/yutamago/ffx-mods): the PS2 mods there, bundled as the mod
`mods/ps2-bundle` (`yuta.ps2-bundle`, zip `yuta.ps2-bundle-<version>.zip`,
built and released by ffx-mods' CI from the same revision). The converter
(`demaster`), the live music DLL, the shared C# library, every mod's code,
the installer and the research and design docs are there (read its
`CLAUDE.md` before working on them). This repository is the research and
test-tool home of the PS2 mods; nothing here is built or released, and
there is no CI:

| Path | What |
|---|---|
| `tools/shotrun`, `tools/calllist`, `tools/replays/` | screenshots and injected game calls through the ffx-rust harness, with the mods' `efl/x` folders as loose-file mods |
| `tools/compare`, `tools/pcsx2ctl`, `tools/loopcap`, `tools/sandbox`, `tools/py` | PS2 (PCSX2) against the PC game, sound capture, the harness's graphics options, research scripts (`docs/compare.md`, `docs/debugger.md`) |

A change to a mod, the bundle, the converter or the installer is made in
ffx-mods (its rules: settings in sync, Job versions, no mod reads another
mod's settings except through `mods.rs` `Foreign`; a new PS2 mod also goes
into `mods/ps2-bundle`). The replays' comments name the old paths
(`mod/*.cs`, `docs/*.md`): `tools/replays/README.md` maps them.

## Rules

- Never commit game data: no disc images, extracted files, `.phyre`, PNG/DDS
  output, built mod folders, zips.
- Never add a `Co-Authored-By: Claude` footer (or any Claude attribution) to
  commit messages, PR descriptions or other messages (user, 2026-11-03).
- Pushing straight to `main` is fine (user, 2026-11-02). Agents working in a
  worktree commit on their branch; the orchestrator merges. Leave
  `wip/ui-last-items` alone (unrun replay edits for later ps2-interface
  work: sphere grid, world map, Overdrive).
- Decompilation findings go into ffx-mods' `docs/ffx-rust-findings.md`
  (the orchestrator ports them to ffx-rust, `../ffx-decomp`).
- Check results by eye and in the game before calling a feature done;
  compare against the PS2 original (the user compares with real PS2
  screenshots).
- Python: `uv run --with <deps> python -I script.py`. Rust: if cargo stalls
  on `sccache`, build with `RUSTC_WRAPPER=`.

## Data (never in git)

| What | Where |
|---|---|
| PS2 disc (German PAL, SCES_504.92) | `~/ffx-ps2/Final Fantasy X (Germany).iso`; symlink it to `/tmp/dm-ffx.iso` for tools (its `(`/`)` upset the agent sandbox) |
| PS2 files extracted under PC names | `~/ffx-work/demaster/ps2/` (`index.tsv`) |
| PC game | `~/steam-games/FFX-X2-HD` (`data/FFX_Data.vbf`) |
| Game copy for Fahrenheit tests | `~/ffx-work/demaster/game` (no ffx-rust `DINPUT8.dll`) |
| Scratch, research output | `~/ffx-work/demaster/{scratch,research}` |
| ffx-rust CLI | `../ffx-decomp/target/release/ffx` |
| Ghidra dump of FFX.exe | `~/ffx-work/decomp` (grep first) |

Windows processes see WSL symlinks through `\\wsl.localhost` as missing:
give them the real path.

## The mods' files for the tools

An installed PS2 bundle (`C:\_workspaces\fh-bundle-test\mods\yuta.*\efl\x`),
or ffx-mods' converter output without installing:
`target/release/demaster chr|maps|fx|ui|backgrounds|movies ...` (its
`CLAUDE.md`). The bundle zip: `dotnet run tools/package.cs -- <out> <version>`
in ffx-mods (its `CLAUDE.md`, "Testing in the game").

## Testing in the game

- Test games run with every graphics option at maximum (`tools/sandbox/max.ini`,
  shotrun's default; ffx-rust's own sandbox is VQ_LOW/TQ_LOW, which drops the
  top 2 mip levels and makes HD baselines blurry).
- `tools/shotrun` + `tools/replays/*.txt`: screenshots through the ffx-rust
  harness with the mods' `efl/x` folders (README, "Testing in the game").
  The harness serves files only; the mods' code patches don't run there.
  Up to 3 games run side by side (harness slots). Capture only the game
  window; never take full-desktop screenshots (they show the user's screen).
- Fahrenheit installs: `C:\_workspaces\fh-bundle-test` (agents' tests of
  the PS2 bundle), `C:\_workspaces\fh-demaster-test` (the user's: the PS2
  bundle, `yuta.ps2-bundle` since 2026-11-09; touch it only when asked),
  `C:\_workspaces\fh-mods-test` (ffx-mods' single-mod tests). Install as a
  player: unpack the zip, run
  `mods\yuta.ps2-bundle\install.cmd -Iso '<real Windows path>' -Game <game> -NoPause`
  (through `powershell.exe -File ...\install.ps1` from WSL).
- Every game run under `flock /home/angelo/ffx-work/demaster/fh-test.lock`,
  one game at a time; wait until no other FFX.exe runs (other sessions run
  games without the lock, and a second boot fails). Launch from WSL with
  `cd .../bin && ./fhstage0.exe '\\wsl.localhost\Ubuntu\home\angelo\ffx-work\demaster\game\FFX.exe' --extdbg`
  (`--extdbg` skips the ~45 s Sophos freeze; dumps at `SophosED.dll+0x7d193`
  in `crash/` are that: delete them). The WSL `timeout` does NOT end the
  Windows game: end the fhstage0.exe you started with
  `taskkill.exe /PID <pid> /F` (its FFX.exe closes with it); never kill FFX
  by name, never pick "the newest FFX.exe": other agents start games too (a
  forgotten game ran for 8 hours on 2026-11-03). Then read
  `logs/<newest>/Yuta.FfxMods.*.log` and count "replaces file" in
  `Fahrenheit.Runtime.FhFileLoaderModule.log`.
- The boot intro (logo, staff credits, the campfire with the party, then
  the title) needs no input: it shows characters, lighting, 4:3, credits,
  the movies' first frames and the title screen.
- PS2 debugger (`docs/debugger.md`): PCSX2 2.8.2 + DebugServer in
  `C:\_workspaces\pcsx2-debug` (TCP 21512, PINE 28011), MCP server `pcsx2`
  (`~/tools/pcsx2-mcp`, local scope). Start it with
  `PCSX2=/mnt/c/_workspaces/pcsx2-debug tools/compare/pcsx2.sh start|stop`;
  one PCSX2 at a time.
- Disk is tight on C: (full builds are ~7 GB): delete test outputs and
  screenshots when done.

## FFX Music Studio

`../ffx-music-studio` (github.com/yutamago/ffx-music-studio) depends on the
`demaster` crate of ffx-mods (git, pinned rev).
