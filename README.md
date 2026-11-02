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
