# ffx-demaster

Restores the PS2 look in the FFX HD Remaster (Steam) as a Fahrenheit mod:
presentation from the PS2, logic from the PC. Every change is a setting.
Plan and status: `docs/plan.md`. What differs: `docs/differences.md`.

## Rules

- Never commit game data: no disc images, extracted files, `.phyre`, PNG/DDS
  output. The mod ships code only; the converter builds assets from the
  player's own PS2 disc image on their machine.
- Never add a `Co-Authored-By: Claude` footer (or any Claude attribution) to
  commit messages, PR descriptions or other messages (user, 2026-11-03).
- Pushing straight to `main` is fine (user, 2026-11-02). Agents working in a
  worktree commit on their branch; the orchestrator merges.
- ffx-rust (`../ffx-decomp`, private, github.com/yutamago/ffx-rust) is ours:
  use its crates and docs freely (git dependency, pinned rev in `Cargo.toml`).
  Its `docs/formats/*.md` and `symbols/ffx.sym` are the reference for FFX.exe.
- Decompilation findings go back into ffx-rust (user, 2026-11-02): every new
  FFX.exe name/address/struct/format fact (and PS2 ELF / IOP / disc format
  facts) is logged in `docs/ffx-rust-findings.md` (address, name, evidence,
  which of our docs explains it) as you find it. The orchestrator ports them to
  ffx-rust (`symbols/ffx.sym` sorted with evidence, `docs/formats/*.md`) on a
  branch + PR there; agents don't edit ffx-rust directly.
- Fahrenheit (`../fahrenheit`, alpha12) is a reference for facts; the C# mod
  uses its SDK (`Fahrenheit.Sdk/1.0.0-alpha12`), like `../ffx-debug-revival`.
- Check results by eye: write PNG previews and look at them, and test in the
  game (below) before calling a feature done. Compare against the PS2
  original, not against what seems plausible: the user compares with real
  PS2 screenshots.
- Python: `uv run --with <deps> python -I script.py`. Rust: if cargo stalls
  on `sccache`, build with `RUSTC_WRAPPER=`.

## Data (never in git)

| What | Where |
|---|---|
| PS2 disc (German PAL, SCES_504.92) | `~/ffx-ps2/Final Fantasy X (Germany).iso` |
| PS2 files extracted under PC names | `~/ffx-work/demaster/ps2/` (`index.tsv`); `jppc/...` = PC `ffx_ps2/ffx/master/jppc/...` |
| PC game | `~/steam-games/FFX-X2-HD` (`data/FFX_Data.vbf`); `FFX.exe` sha256 in ffx-rust `symbols/ffx.sym` |
| PC copies of `ffx_ps2/ffx/master` | `~/ffx-work/demaster/pcps2/` |
| Scratch, research output | `~/ffx-work/demaster/{scratch,research}` |
| ffx-rust CLI | `../ffx-decomp/target/release/ffx` (`vbf`, `texture export`, `model export/import`, `phyre`) |
| Ghidra dump of FFX.exe | `~/ffx-work/decomp` (grep first) |

## Converter (`converter/`, binary `demaster`)

- `disc.rs`: reads the ISO, names files with the PC's tables, unpacks
  (`docs/disc.md`).
- `phyretex.rs`: writes a `*.dds.phyre` (ARGB8, any size) from an HD texture
  as template. PS2 textures go in at PS2 size, uncompressed (user decision),
  with a full mip chain when the HD texture has one (`mip_chain`).
- `ps2chr.rs`, `bake.rs`, `chrjob.rs`: character textures. Each HD triangle is
  matched to its PS2 twin by position and painted with the PS2 texels.
- `chralpha.rs`: character materials and texture alpha. The PS2 blended every
  character polygon by texture alpha; materials (or segments) that sample
  see-through PS2 texels get the `UATFOPQ` (alpha to coverage) twin of their
  shader variant and a low `AlphaThreshold`
  (`docs/models.md`, "Alpha"; `demaster materials-debug`).
- `gsmem.rs`, `ps2map.rs`, `mapjob.rs`: field and battle map textures,
  cut out of an emulated PS2 GS memory by the HD texture names
  (`docs/maps.md`; `demaster maps <iso> <vbf> <out> all|<map|area>...`).
- `ps2fx.rs`, `fxjob.rs`: effect textures (spells, summons, map and
  monster particles) from the PS2 oef2 blocks, by HD name
  (`docs/effects.md`). `demaster fx <iso> <vbf> <out> all`.
- `uijob.rs`: menu atlases, portraits, title screen, loading screen, button
  glyphs (`docs/ui.md`; font plan in `docs/font.md`).
  `demaster ui <iso> <vbf> <out>`.
- `fontjob.rs`: the PS2 font for the European languages: glyph pages
  and the width table of `uspc/menu/base.ftc` (`docs/font.md`), run by
  `demaster ui` and the `ui.font` job.
- `ps2movie.rs`, `moviejob.rs`: the PS2 FMVs (IPU bitstream decoder,
  delta-coded sound) encoded to the PC's WebM through an external ffmpeg
  (`docs/movies.md`). `demaster movies <iso> <vbf> <mod> all|<name>...`,
  `demaster movie-debug ...`.
- `sqseq.rs`, `spu2.rs`, `fsb5.rs`, `musicjob.rs`: the PS2 music driver
  (port of `iopsound.irx`) on an SPU2 model, rendering the disc's
  `musicNNN.bgm` into the "Original" soundtrack bank (`docs/music.md`).
  `demaster music <iso> <vbf> <out> all|<id|name>`, `demaster music-bank`.
- `bgjob.rs`: 2D-room background layers (`docs/screen.md`).
  `demaster backgrounds <iso> <vbf> <out> all|<map>`.
- `shaderjob.rs`, `dxbc.rs`: the character pixel shaders with the PS2 light
  falloff (`render.lighting`; DXBC token walk and checksum, `docs/render.md`).
- Output layout: `<out>/efl/x/<archive path>` (Fahrenheit's file loader).
- `config.rs` (`demaster.toml`, defaults in `converter/demaster.toml`),
  `jobs.rs` (Job trait + registry: new jobs go in `jobs::registry()`),
  `build.rs` (`demaster build`: skips unchanged jobs, removes files of jobs
  that are off, writes `build/files.json` and `build/stamp.json`).
  Settings live in 4 places a test keeps in sync: `config.rs`,
  `converter/demaster.toml`, `mod/Settings.cs` (+ `mod/Patches.cs` for code
  patches), `docs/config.md`.
- A change that alters a job's output must bump that job's `Job::version`, or
  `demaster build` keeps the old files (the build hash covers the converter
  version, the job version, its settings and inputs, not the code).
- Jobs must be deterministic (same input, same bytes): no HashMap iteration
  order in results; parallel work writes in a fixed order.
- The converter is a library too (`src/lib.rs`, every module `pub`) for
  FFX Music Studio.

```bash
cargo build --release -p demaster
./target/release/demaster build --config my.toml --mod-dir <mod folder>   # [source] iso/game in my.toml
./target/release/demaster chr <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out> all   # + PNG previews
tools/build-windows.sh            # demaster.exe (cargo-xwin, static CRT)
dotnet build -c Release ffx-demaster.slnx && dotnet run tools/package.cs -- out.zip [demaster.exe]
```

The zip's players' README and licence header are `tools/package/*.txt`.
CI, tag releases and the `FFX_RUST_TOKEN` secret: `docs/ci.md`.

Paths with `(`/`)` (the ISO's name) upset the agent sandbox: symlink the ISO
to a plain path (`/tmp/dm-ffx.iso`). Windows processes see WSL symlinks
through `\\wsl.localhost` as missing: give them the real path.

## FFX Music Studio (`studio/`, binary `ffx-music-studio`)

Plays and edits (in memory) the disc's music on `sqseq` + `spu2`
(`docs/studio.md`): egui window, cpal output (Windows; Linux only with
`--features alsa`), `--render <track> <s> <out.wav> [edits]` through the
same path. `seq.rs` decodes/encodes BGMs (byte-exact) and normalizes tracks
for note edits; the driver takes the studio's `sqseq::Mods` (default = the
PS2 driver exactly). Tests: `cargo test -p ffx-music-studio`, with the disc
`FFX_STUDIO_CONFIG=<demaster.toml>`. Windows: `tools/build-studio-windows.sh`.
Capturing its window with wincap: end it with `taskkill.exe /PID` (a WSL
`timeout` leaves the Windows process running).

## Testing in the game

- Test games run with every graphics option at maximum (`tools/sandbox/max.ini`,
  shotrun's default; ffx-rust's own sandbox is VQ_LOW/TQ_LOW, which drops the
  top 2 mip levels and makes HD baselines blurry).
- `tools/shotrun` + `tools/replays/*.txt`: screenshots through the ffx-rust
  harness with our output as a mod (README, "Testing in the game"). Up to 3
  games run side by side (harness slots). Capture only the game window; never
  take full-desktop screenshots (they show the user's screen).
- Fahrenheit install for real tests: `C:\_workspaces\fh-demaster-test`
  (mod folder `mods/yuta.demaster`, game copy `~/ffx-work/demaster/game`
  without the ffx-rust `DINPUT8.dll`). The mod folder holds the installed
  `yuta.demaster.dll`, `demaster.exe` and a `demaster.toml` pointing at the
  ISO and game copy through `\\wsl.localhost`. Launch from WSL with
  `cd .../bin && timeout 45 ./fhstage0.exe '\\wsl.localhost\Ubuntu\home\angelo\ffx-work\demaster\game\FFX.exe'`
  (the sandbox refuses `cmd.exe`/`powershell.exe`; the game exits with
  fhstage0), then read `logs/<newest>/Yuta.Demaster.DemasterModule.log` and
  count "replaces file" in `Fahrenheit.Runtime.FhFileLoaderModule.log`.
  Other agents run their own FFX.exe instances: never kill FFX by name.
- PS2 debugger (`docs/debugger.md`): PCSX2 2.8.2 + DebugServer in
  `C:\_workspaces\pcsx2-debug` (TCP 21512, PINE 28011), MCP server `pcsx2`
  (`~/tools/pcsx2-mcp`, local scope): breakpoints, watchpoints, registers,
  disassembly, backtrace on the running PS2 game. Start it with
  `PCSX2=/mnt/c/_workspaces/pcsx2-debug tools/compare/pcsx2.sh start|stop`;
  one PCSX2 at a time.
