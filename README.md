# FFX Demaster

The Final Fantasy X HD Remaster (Steam) with the look and sound of the
original PS2 game: presentation from the PS2, logic from the PC. PS2
models, textures, maps, effects, movies, music, lighting, font and menus,
the 4:3 picture, with the PC version's game logic underneath.

FFX Demaster is a **bundle**: the PS2 mods of
[ffx-mods](https://github.com/yutamago/ffx-mods), in one zip, installed and
built in one step. The code of every mod, the converter and their
documents are in ffx-mods; this repository pins the ffx-mods revision the
bundle is built from (`bundle.toml`), builds the zip, and holds the test
and comparison tools (PCSX2 against the PC game, below).

The zip ships no game data. Its converter, `demaster.exe`, reads your own
PS2 disc (an image or the disc in a drive) and writes the PS2 files as loose
files that Fahrenheit's file loader puts in place of the HD ones. Every
change is a setting, so any part of the HD Remaster can be kept.

## The mods

| Mod | What it changes | Size |
|---|---|---|
| [PS2 music engine](https://github.com/yutamago/ffx-mods#ps2-music-engine) | the PS2's own sound driver plays the original soundtrack live in the game | ~60 MB |
| [PS2 soundtrack](https://github.com/yutamago/ffx-mods#ps2-soundtrack) | the "Original" soundtrack rendered from the PS2 music; new games start with it | ~890 MB |
| [PS2 movies](https://github.com/yutamago/ffx-mods#ps2-movies) | the PS2 movies (FMVs), 4:3, with the PS2 sound | ~1.5 GB |
| [PS2 characters](https://github.com/yutamago/ffx-mods#ps2-characters) | PS2 models and textures: party, NPCs, monsters, aeons, weapons, objects | ~860 MB |
| [PS2 maps](https://github.com/yutamago/ffx-mods#ps2-maps) | PS2 field and battle map textures, PS2 painted backgrounds | ~3 GB |
| [PS2 effects](https://github.com/yutamago/ffx-mods#ps2-effects) | PS2 spell, summon and effect textures | ~500 MB |
| [PS2 lighting](https://github.com/yutamago/ffx-mods#ps2-lighting) | characters lit the PS2 way | ~80 MB |
| [No HD post-effects](https://github.com/yutamago/ffx-mods#no-hd-post-effects) | no HD glow, no shadow maps (the PS2's ground shadows) | code |
| [PS2 interface](https://github.com/yutamago/ffx-mods#ps2-interface) | PS2 menus, windows, font, portraits, title screen, button glyphs; a clean start-up | ~180 MB |
| [PS2 aspect ratio](https://github.com/yutamago/ffx-mods#ps2-aspect-ratio) | the 4:3 picture with the PS2 camera | code |
| [PS2 game feel](https://github.com/yutamago/ffx-mods#ps2-game-feel) | PS2 run speed, battle menu without input delay, battle subtitles | code |
| [Bug fixes](https://github.com/yutamago/ffx-mods#bug-fixes) | NPC mouths, the Farplane video, Lulu's Fury spins | code |

Sizes: the files the installer builds with the default settings (German PAL
disc). The other mods of ffx-mods (Modern Minimap, FPS unlock) are not PS2
parts and not in the bundle; their own zips install next to it.

## Install

You need:
- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows);
- [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit/releases)
  1.0.0-alpha12 or later, installed as in its
  [installation guide](https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide);
- your own Final Fantasy X PS2 disc, any region: an image (`.iso`), or the
  disc in a disc drive;
- about 7 GB of disk space.

1. Download `ffx-demaster-<version>.zip` from the
   [releases](https://github.com/yutamago/ffx-demaster/releases) and unpack
   it into your Fahrenheit folder (the one with Fahrenheit's `bin` folder).
   It adds `mods\yuta.ffx-mods\` (the shared library, the converter and the
   installer) and one folder per mod, `mods\yuta.<mod>\`.
2. Double-click `mods\yuta.ffx-mods\install.cmd`. It adds every mod to
   `mods\loadorder`, asks where your disc is (the disc drives with the disc
   in them are listed; or pick an image file), finds the game in your Steam
   folders (or asks for its folder) and builds every mod's PS2 files from
   one pass over the disc, each into its own folder. The first build takes
   about 45 minutes, most of it the movies (ffmpeg, if none is found, is
   downloaded: an LGPL build, 168 MB, checked by its SHA-256). From a
   terminal: `install.cmd -Iso 'D:\PS2\FFX.iso'` or `-Drive E`, `-Game
   <game folder>`, `-Force` (build everything again).
3. Start the game through Fahrenheit (`bin\fhstage0.exe <path to FFX.exe>`).

In the game, **F8** opens one window with a section per mod: whether its
files match its settings (and the command to rebuild them) and which code
patches are on. It opens by itself when something is wrong.

**Upgrading from the old all-in-one FFX Demaster** (`mods\yuta.demaster`):
the installer takes `yuta.demaster` out of `mods\loadorder` (the new mods
replace it) and offers the disc image the old one used. Copy the settings
you had changed in `mods\yuta.demaster\demaster.toml` into the mods'
`settings.toml` files (same keys, [`docs/config.md`](https://github.com/yutamago/ffx-mods/blob/main/docs/config.md)
says which mod has which; the old `audio.music_engine = "ps2"` is
`audio.music = "ps2"` in PS2 soundtrack now), run the installer again, then
delete `mods\yuta.demaster\`.

## Settings

**Per mod**: each mod has its own `mods\yuta.<mod>\settings.toml` with
only its settings and a comment above each; the defaults are the PS2's.
There is no single settings file for the bundle. The installer writes the
disc and the game folder into every mod's `[source]`.
[`docs/config.md`](https://github.com/yutamago/ffx-mods/blob/main/docs/config.md)
in ffx-mods describes every setting and which mod has it.

After changing a setting, run `mods\yuta.ffx-mods\install.cmd` again (it
rebuilds only what changed, in every mod; a mod's own `install.cmd` does
that mod alone) and restart the game. Code-only settings (aspect ratio,
game feel, bug fixes, post-effects) need only the restart.

## Switching single mods off

- **Part of a mod**: set its setting to `"hd"` (`false` for a fix) in that
  mod's `settings.toml` and run the installer again; the files of that part
  are removed.
- **A whole mod**: delete its line in `mods\loadorder` (Fahrenheit then
  doesn't load it), and its folder to get the disk space back. The
  installer in `mods\yuta.ffx-mods\` adds every mod whose folder is there
  back to `mods\loadorder`.

The mods work alone and in any combination. PS2 movies and PS2 maps (the
painted backgrounds) are built for the picture the game shows: 4:3 with PS2
aspect ratio, 16:9 without; after adding, removing or switching PS2 aspect
ratio, F8 shows them as "rebuild needed" until the installer runs again.

**Uninstall**: delete the mod folders (`mods\yuta.ps2-*`,
`mods\yuta.no-hd-post-effects`, `mods\yuta.bug-fixes`,
`mods\yuta.ffx-mods`) and their lines in `mods\loadorder`.

## Build

```bash
tools/bundle.sh dist [version]     # dist/ffx-demaster-<version>.zip
FFX_MODS=../ffx-mods tools/bundle.sh dist 0.1.0-dev   # from a local ffx-mods checkout
```

`tools/bundle.sh` checks out ffx-mods at the revision in `bundle.toml`
(into `~/.cache/ffx-demaster/ffx-mods`), builds its mods (`dotnet`),
`demaster.exe` and `demaster_live.dll` (cargo-xwin) and runs its
`tools/package.cs --bundle` with the mods of `bundle.toml` and this
repository's players' `tools/package/README.txt`. The zip holds
`mods/yuta.ffx-mods/` (the shared library, the one `demaster.exe`,
`install.cmd`/`install.ps1` for all the mods, `README.txt`,
`LICENSES.txt`) and each mod's folder (DLL, `settings.toml`, `README.txt`,
its own `install.cmd`, `LICENSES.txt`; `demaster_live.dll` for the music
engine). Nothing from the game.

To move the bundle to a newer ffx-mods: set `rev` in `bundle.toml`, build,
install the zip into a Fahrenheit test install and play the boot intro.

CI (`.github/workflows/build.yml`, [`docs/ci.md`](docs/ci.md)) builds the
same zip on Windows on every push and pull request and checks its contents;
a tag `v*` publishes a release with it (`release.yml`). No secrets: ffx-mods
and ffx-rust are public.

## Testing in the game

`tools/shotrun` plays a replay script (`tools/replays/*.txt`) through the
ffx-rust test harness, with the mods' files as loose-file mods (one
argument per `efl/x` folder), and saves screenshots. It needs an ffx-rust
checkout next to this repo (built hook: `tools/build-hook.sh` there) and
runs from that checkout:

```bash
M=/mnt/c/_workspaces/fh-bundle-test/mods      # an installed bundle (or ffx-mods' converter output)
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

A real Fahrenheit install for tests of the bundle:
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
