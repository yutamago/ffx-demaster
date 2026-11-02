# ffx-demaster

Restores the PS2 look in the FFX HD Remaster (Steam) as a Fahrenheit mod:
presentation from the PS2, logic from the PC. Every change is a setting.
Plan and status: `docs/plan.md`. What differs: `docs/differences.md`.

## Rules

- Never commit game data: no disc images, extracted files, `.phyre`, PNG/DDS
  output. The mod ships code only; the converter builds assets from the
  player's own PS2 disc image on their machine.
- Pushing straight to `main` is fine (user, 2026-11-02). Agents working in a
  worktree commit on their branch; the orchestrator merges.
- ffx-rust (`../ffx-decomp`, private, github.com/yutamago/ffx-rust) is ours:
  use its crates and docs freely (git dependency, pinned rev in `Cargo.toml`).
  Its `docs/formats/*.md` and `symbols/ffx.sym` are the reference for FFX.exe.
  Do not edit that repo from here; note needed changes in `docs/plan.md`.
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
  as template. PS2 textures go in at PS2 size, uncompressed (user decision).
- `ps2chr.rs`, `bake.rs`, `chrjob.rs`: character textures. Each HD triangle is
  matched to its PS2 twin by position and painted with the PS2 texels.
- `gsmem.rs`, `ps2map.rs`, `mapjob.rs`: field and battle map textures,
  cut out of an emulated PS2 GS memory by the HD texture names
  (`docs/maps.md`; `demaster maps <iso> <vbf> <out> all|<map|area>...`).
- `ps2fx.rs`, `fxjob.rs`: effect textures (spells, summons, map and
  monster particles) from the PS2 oef2 blocks, by HD name
  (`docs/effects.md`). `demaster fx <iso> <vbf> <out> all`.
- Output layout: `<out>/efl/x/<archive path>` (Fahrenheit's file loader).
- `config.rs` (`demaster.toml`, defaults in `converter/demaster.toml`),
  `jobs.rs` (Job trait + registry: new jobs go in `jobs::registry()`),
  `build.rs` (`demaster build`: skips unchanged jobs, removes files of jobs
  that are off, writes `build/files.json` and `build/stamp.json`).
  Settings live in 4 places a test keeps in sync: `config.rs`,
  `converter/demaster.toml`, `mod/Settings.cs` (+ `mod/Patches.cs` for code
  patches), `docs/config.md`.
- Jobs must be deterministic (same input, same bytes): no HashMap iteration
  order in results; parallel work writes in a fixed order.

```bash
cargo build --release -p demaster
./target/release/demaster build --config my.toml --mod-dir <mod folder>   # [source] iso/game in my.toml
./target/release/demaster chr <iso> ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf <out> all   # + PNG previews
tools/build-windows.sh            # demaster.exe (cargo-xwin, static CRT)
dotnet build -c Release ffx-demaster.slnx && dotnet run tools/package.cs -- out.zip
```

Paths with `(`/`)` (the ISO's name) upset the agent sandbox: symlink the ISO
to a plain path (`/tmp/dm-ffx.iso`). Windows processes see WSL symlinks
through `\\wsl.localhost` as missing: give them the real path.

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
