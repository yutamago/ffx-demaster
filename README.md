# FFX Demaster

The research and test tools of the PS2 mods for the Final Fantasy X HD
Remaster (Steam): PS2 models, textures, maps, effects, movies, music,
lighting, font and menus, the 4:3 picture, with the PC version's game
logic underneath.

**The mod itself is in [ffx-mods](https://github.com/yutamago/ffx-mods):**
the [PS2 bundle](https://github.com/yutamago/ffx-mods#ps2-bundle)
(`yuta.ps2-bundle`, formerly FFX Demaster) is the twelve PS2 mods in one
zip with one install step, released there as
`yuta.ps2-bundle-<version>.zip` with the per-mod zips. The converter
(`demaster`), every mod's code, the installer and the research and design
documents are there too.

This repository keeps what is used to find out how the PS2 did something
and to check the mods against it:

| Path | What |
|---|---|
| `tools/shotrun`, `tools/replays/` | screenshots through the ffx-rust test harness, with the mods' `efl/x` folders as loose-file mods |
| `tools/calllist` | the harness with injected FFX.exe calls, for checking C# draw hooks the harness can't run |
| `tools/compare`, `tools/pcsx2ctl` | the PS2 original (PCSX2) next to the PC game with the mods, scene by scene ([`docs/compare.md`](docs/compare.md)) |
| `tools/loopcap` | one process's sound with packet times, for lip-sync measurements |
| `tools/sandbox/max.ini` | the game's graphics options at maximum, for the harness runs |
| `tools/py` | research scripts (PS2 textures, models, maps against the HD's) |
| [`docs/debugger.md`](docs/debugger.md) | the PS2 debugger: PCSX2 with a debug server and the `pcsx2` MCP server |

Nothing here is built or released; there is no CI. The tools need a local
ffx-rust checkout (`../ffx-decomp`) and, for the mods' files, an installed
PS2 bundle or ffx-mods' converter output.

## Testing in the game

`tools/shotrun` plays a replay script (`tools/replays/*.txt`) through the
ffx-rust test harness, with the mods' files as loose-file mods (one
argument per `efl/x` folder), and saves screenshots. It needs an ffx-rust
checkout next to this repo (built hook: `tools/build-hook.sh` there) and
runs from that checkout:

```bash
M=/mnt/c/_workspaces/fh-bundle-test/mods      # an installed PS2 bundle (or ffx-mods' converter output)
cd ../ffx-decomp
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release --manifest-path ../ffx-demaster/tools/shotrun/Cargo.toml -- \
  ../ffx-demaster/tools/replays/battle-djose.txt 'C:\shots\ps2\djose.trace' \
  $M/yuta.ps2-characters/efl/x $M/yuta.ps2-maps/efl/x $M/yuta.ps2-interface/efl/x
```

The harness serves files only: the mods' code patches don't run in it (the
replays stand in for some of them with jumps and writes). Run it once
without mod folders for the HD baseline. A replay with a `realtime` line
runs at the game's own rate with FMOD on the sound card (no fast mode, no
virtual clock); `tools/loopcap` (`tools/build-loopcap.sh`) records one
process's sound with the time of each packet, for lip-sync measurements
(ffx-mods' `docs/timing.md`, "Voice start"). `tools/calllist` is the same
with a few extra commands for checking C# draw hooks the harness can't run:
it makes the game itself call a list of FFX.exe functions from a draw
callback (`tools/replays/ui-battle-*.txt`, ffx-mods' `docs/ui.md`).
`FFX_HOOK_LOG_OPENS=1` (passed through `WSLENV`) logs every file the game
opens; files served by a mod show as `mods:`. Don't set
`SHOTRUN_ORIGINAL`: it turns off the hook that serves the mod files. The
replays' comments name files by their FFX Demaster paths before the move
([`tools/replays/README.md`](tools/replays/README.md) maps them).

Converter output for these tools, without installing: ffx-mods'
`target/release/demaster chr|maps|fx|ui|backgrounds ... <out>` writes
`<out>/efl/x` (ffx-mods' `CLAUDE.md`).

### PS2 vs PC (`tools/compare`)

`tools/compare` puts the PS2 original, running in PCSX2, next to the PC
game with the mods for the same scene ([`docs/compare.md`](docs/compare.md)):
per scene an exact view (PCSX2's software renderer at 512x416 against the
PC scaled down) and a detail view (the hardware renderer upscaled from a GS
dump of the same frame against the PC at full size), each PS2 | PC |
difference, with numbers per screen region. PCSX2 (portable,
`C:\_workspaces\pcsx2`) is driven by `pcsx2ctl.exe`
(`tools/build-pcsx2ctl.sh`): PINE for memory and save states, keys posted
to its window for the pad and the hotkeys, PCSX2's own screenshots and GS
dumps. The PC saves the replays use go on a PCSX2 folder memory card as PS2
saves (the formats are the same).

```bash
tools/build-pcsx2ctl.sh                                         # once
uv run python -I tools/compare/setup.py /mnt/c/_workspaces/pcsx2 <PC save folder> 1 5   # once
tools/compare/pcsx2.sh start; tools/compare/pcsx2.sh ctl run '<Windows path of tools\compare\states.txt>'; tools/compare/pcsx2.sh stop   # once: save states
../ffx-mods/target/release/demaster ui /tmp/dm-ffx.iso ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf ~/ffx-work/demaster/scratch/compare-ui
uv run --with pillow --with numpy python -I tools/compare/compare.py all [--mod <efl/x>]... [replay...]
```

`compare.py ps2`, `hw`, `pc` and `diff` run the steps alone (one game at a
time); the PNGs and `report.tsv` land in `/mnt/c/shots/compare/`.
`tools/compare/pcsx2.sh start|stop|ctl` runs PCSX2 by hand (stop ends it by
PID). The PS2 debugger (PCSX2 with a debug server and the `pcsx2` MCP
server): [`docs/debugger.md`](docs/debugger.md).

A real Fahrenheit install for tests of the PS2 bundle (from ffx-mods):
`C:\_workspaces\fh-bundle-test` (alpha12, the bundle's mods, game copy
without the ffx-rust hook in `~/ffx-work/demaster/game`); the logs are in
`logs\<session>\` (`Yuta.FfxMods.*.log`: build status and patches of each
mod; `Fahrenheit.Runtime.FhFileLoaderModule.log`: one "replaces file" line
per file of the mods).

## Tools

- [`tools/py`](tools/py): research scripts. `ps2tex.py` decodes PS2
  character and map textures to PNG; the others compare PS2 and HD models and
  maps. Run them with `uv run --with pillow --with numpy python -I …`. Some
  still have the research machine's paths in them.
- `tools/sandbox/max.ini`: the game's graphics options at maximum, for the
  harness runs.
- ffx-mods has the disc extractor (`tools/ps2x`) and the live music test
  (`tools/live-test.sh`).

Never commit disc images, extracted files or anything from the game's
archives.
