# FFX Demaster

A [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) mod that makes
the Final Fantasy X HD Remaster (Steam) look like the original PS2 game:
presentation from the PS2, logic from the PC.

**Status: early.** The PS2 textures of the monsters, NPCs, aeons, weapons
and objects whose HD model kept the PS2 mesh work. Maps, effects, the party
models, the interface and the rendering changes are still to come
([`docs/plan.md`](docs/plan.md)).

The mod ships no game assets. Its converter, `demaster.exe`, reads your own
PS2 disc image and writes the PS2 textures, models and movies as loose files
that Fahrenheit's file loader puts in place of the HD ones. Every change is a
setting, so any part of the HD Remaster can be kept.

## Install

You need:
- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows);
- [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit/releases)
  1.0.0-alpha12 or later, installed as in its
  [installation guide](https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide);
- an image (`.iso`) of your own Final Fantasy X PS2 disc, any region.

Then:

1. Unpack the release zip into your Fahrenheit folder, so the mod ends up in
   `mods\yuta.demaster\` (next to Fahrenheit's `bin` folder).
2. Open `mods\yuta.demaster\demaster.toml` in a text editor and set `iso`
   under `[source]` to your disc image, in single quotes:

   ```toml
   [source]
   iso = 'D:\PS2\Final Fantasy X.iso'
   ```

   If the game isn't in a usual Steam folder, also set `game` to the folder
   with `FFX.exe`.
3. Run `mods\yuta.demaster\demaster.exe` (double-click, or `demaster.exe
   build` in a terminal). It writes the PS2 files into the mod folder; the
   first run takes a few minutes.
4. Add a line `yuta.demaster` to `mods\loadorder` (create the file if it
   isn't there).
5. Start the game through Fahrenheit (`bin\fhstage0.exe <path to FFX.exe>`).

In the game, **F8** opens a small window that says whether the PS2 files
match `demaster.toml`, and how to rebuild them if not. It opens by itself
when they don't.

## Settings

Every difference between the versions is a setting in `demaster.toml`; the
defaults give the PS2 look with the PC's game logic. [`docs/config.md`](docs/config.md)
describes them all. **After changing a setting, run `demaster.exe` again and
restart the game**: Fahrenheit picks up the mod's files only at startup. A
rebuild only redoes what changed and removes the files of whatever you
switched back to HD.

To uninstall, delete `mods\yuta.demaster` and its line in `mods\loadorder`.

## Documents

- [`docs/config.md`](docs/config.md): every setting.
- [`docs/differences.md`](docs/differences.md): every difference between
  the two versions, how the mod restores it, its setting and the effort.
- [`docs/disc.md`](docs/disc.md): how the PS2 disc stores its files and how
  they map to the PC's names.
- [`docs/research/`](docs/research): the evidence. Version differences with
  sources, the measured HD-vs-PS2 asset comparison, the PS2 texture formats,
  Fahrenheit's file loading.

## Build

The repository has two parts:

- `converter/`: `demaster`, the converter (Rust). It needs the ffx-rust
  crates (private, `github.com/yutamago/ffx-rust`, pinned in `Cargo.toml`).
- `mod/`: `yuta.demaster`, the Fahrenheit mod (C#, .NET 10,
  `Fahrenheit.Sdk/1.0.0-alpha12`).
- `studio/`: `ffx-music-studio`, FFX Music Studio, a player and editor for
  the PS2 music of your disc on the converter's sound driver
  (`docs/studio.md`).

```bash
cargo build --release -p demaster              # Linux build: target/release/demaster
tools/build-windows.sh                         # demaster.exe via cargo-xwin (Linux/WSL)
tools/build-studio-windows.sh                  # ffx-music-studio.exe the same way
dotnet build -c Release ffx-demaster.slnx      # mod/artifacts/deploy/rel/mods/yuta.demaster/
dotnet run tools/package.cs -- yuta.demaster.zip   # the release zip
cargo test -p demaster                         # config tests, incl. that mod/ knows every setting
```

On Windows, `cargo build --release -p demaster --target
x86_64-pc-windows-msvc` builds `demaster.exe` directly. CI
(`.github/workflows/build.yml`) does that on every push and uploads the zip
as an artifact; pushing a tag `v*` publishes a release
(`.github/workflows/release.yml`). Both need the repository secret
`FFX_RUST_TOKEN` with read access to ffx-rust.

While developing, run the converter against any folder:

```bash
target/release/demaster build --config my.toml --mod-dir <Fahrenheit>/mods/yuta.demaster
```

### Adding a converter job

A job turns part of the disc into files for the mod folder and is switched by
settings. Implement `jobs::Job` (`name`, `settings`, `enabled`, `run`) in a
module and add its jobs to `jobs::registry()` in `converter/src/jobs.rs`.
`run` gets the disc (`ctx.disc(|d| ...)`), `FFX_Data.vbf` (`ctx.pc`) and the
config, and writes files with `out.put(archive_path, bytes)`. The build
handles the rest: skipping unchanged jobs, removing the files of jobs that
are off, the manifest and the stamp. Every setting already exists in
`converter/src/config.rs`; a new one goes there, in `converter/demaster.toml`,
`mod/Settings.cs` and `docs/config.md` (a test checks they agree). Settings
applied by code go in `mod/Patches.cs`.

## Tools

- [`tools/ps2x`](tools/ps2x): extracts every file of a PS2 disc image under
  the PC's names and unpacks the compressed ones.

  ```bash
  cargo run --release --manifest-path tools/ps2x/Cargo.toml -- \
    <disc.iso> <cdrom.fnd> <sizetbl.bin> <out dir> <jp cdrom.fid>
  ```

  `cdrom.fnd` and the `jp` `cdrom.fid` come from the PC archive
  (`ffx_ps2/ffx/proj/battle/jp/cddata/`), `sizetbl.bin` from the disc's region
  folder (`ffx_ps2/ffx/proj/prog/cdidx/<region>/`).
- [`tools/py`](tools/py): research scripts. `ps2tex.py` decodes PS2
  character and map textures to PNG; the others compare PS2 and HD models and
  maps. Run them with `uv run --with pillow --with numpy python -I …`. Some
  still have the research machine's paths in them.

Never commit disc images, extracted files or anything from the game's
archives.

## Testing in the game

`tools/shotrun` plays a replay script (`tools/replays/*.txt`) through the
ffx-rust test harness, with the converter's output as a loose-file mod, and
saves screenshots. It needs an ffx-rust checkout next to this repo (built hook:
`tools/build-hook.sh` there) and runs from that checkout:

```bash
cd ../ffx-decomp
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release --manifest-path ../ffx-demaster/tools/shotrun/Cargo.toml -- \
  ../ffx-demaster/tools/replays/battle-djose.txt 'C:\shots\ps2\djose.trace' <out>/efl/x
```

Run it once without the mod folder for the HD baseline. `tools/calllist` is
the same with a few extra commands for checking C# draw hooks the harness
can't run: it makes the game itself call a list of FFX.exe functions from a
draw callback (`tools/replays/ui-battle-*.txt`, `docs/ui.md`). `FFX_HOOK_LOG_OPENS=1`
(passed through `WSLENV`) logs every file the game opens; files served by the
mod show as `mods:`. Don't set `SHOTRUN_ORIGINAL`: it turns off the hook that
serves the mod files.

### PS2 vs Demaster (`tools/compare`)

`tools/compare` puts the PS2 original, running in PCSX2, next to the
Demaster for the same scene ([`docs/compare.md`](docs/compare.md)): per
scene an exact view (PCSX2's software renderer at 512x416 against the
Demaster scaled down) and a detail view (the hardware renderer upscaled
from a GS dump of the same frame against the Demaster at full size), each
PS2 | Demaster | difference, with numbers per screen region. PCSX2
(portable, `C:\_workspaces\pcsx2`) is driven by `pcsx2ctl.exe`
(`tools/build-pcsx2ctl.sh`): PINE for memory and save states, keys posted
to its window for the pad and the hotkeys, PCSX2's own screenshots and GS
dumps. The PC saves the replays use go on a PCSX2 folder memory card as PS2
saves (the formats are the same).

```bash
tools/build-pcsx2ctl.sh                                         # once
uv run python -I tools/compare/setup.py /mnt/c/_workspaces/pcsx2 <PC save folder> 1 5   # once
tools/compare/pcsx2.sh start; tools/compare/pcsx2.sh ctl run '<Windows path of tools\compare\states.txt>'; tools/compare/pcsx2.sh stop   # once: save states
target/release/demaster ui /tmp/dm-ffx.iso ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf ~/ffx-work/demaster/scratch/compare-ui
uv run --with pillow --with numpy python -I tools/compare/compare.py all [replay...]
```

`compare.py ps2`, `hw`, `pc` and `diff` run the steps alone (one game at a
time); the PNGs and `report.tsv` land in `/mnt/c/shots/compare/`.
`tools/compare/pcsx2.sh start|stop|ctl` runs PCSX2 by hand (stop ends it by
PID).

A real Fahrenheit install for manual tests: `C:\_workspaces\fh-demaster-test`
(alpha12, mod `yuta.demaster`, game copy without the ffx-rust hook in
`~/ffx-work/demaster/game`). Its `run.ps1 -Seconds N` starts the game through
`fhstage0.exe` and stops it after N seconds; the results are in
`logs\<session>\` (`Yuta.Demaster.DemasterModule.log`: build status and
patches; `Fahrenheit.Runtime.FhFileLoaderModule.log`: one "replaces file"
line per file of the mod).
