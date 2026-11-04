# Changelog

Releases are built by `.github/workflows/release.yml` from a tag `v*`
(`docs/ci.md`). [`docs/plan.md`](docs/plan.md) has the status of every part.

## Unreleased

First release. `demaster.exe` builds from your own PS2 disc image:

- character textures, and PS2 meshes for 165 remodelled characters, with
  their texture animation (eyes, mouths) and cloth;
- field, battle map and effect textures;
- menus, portraits, title, loading screen, button glyphs and the PS2 font
  (partly; see `docs/plan.md`);
- the PS2 movies (4:3, PS2 audio; ffmpeg is downloaded on first use);
- the PS2 music as the "Original" soundtrack.

The zip has the mod, `demaster.exe`, the default `demaster.toml`, a
`README.txt` for players and `LICENSES.txt`. No game data.
FFX Music Studio (`ffx-music-studio.exe`) is attached to the release on its
own.
