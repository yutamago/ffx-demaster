# FFX Demaster

A [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) mod that makes
the Final Fantasy X HD Remaster (Steam) look like the original PS2 game:
presentation from the PS2, logic from the PC.

**Status: research.** Nothing is playable yet.

The mod ships no game assets. A converter reads the player's own PS2 disc
image and writes the PS2 textures, models and movies as loose files that
Fahrenheit's file loader puts in place of the HD ones. Every change is a
setting, so any part of the HD Remaster can be kept.

## Documents

- [`docs/differences.md`](docs/differences.md): every difference between
  the two versions, how the mod restores it, its setting and the effort.
- [`docs/disc.md`](docs/disc.md): how the PS2 disc stores its files and how
  they map to the PC's names.
- [`docs/research/`](docs/research): the evidence. Version differences with
  sources, the measured HD-vs-PS2 asset comparison, the PS2 texture formats,
  Fahrenheit's file loading.

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

Run it once without the mod folder for the HD baseline. `FFX_HOOK_LOG_OPENS=1`
(passed through `WSLENV`) logs every file the game opens; files served by the
mod show as `mods:`. Don't set `SHOTRUN_ORIGINAL`: it turns off the hook that
serves the mod files.

A real Fahrenheit install for manual tests: `C:\_workspaces\fh-demaster-test`
(alpha12, mod `yuta.demaster`, game copy without the ffx-rust hook in
`~/ffx-work/demaster/game`).
