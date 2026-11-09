# Changelog

Releases are built by `.github/workflows/release.yml` from a tag `v*`
(`docs/ci.md`).

## Unreleased

FFX Demaster becomes a bundle of the PS2 mods of
[ffx-mods](https://github.com/yutamago/ffx-mods), where its code moved:

- one zip, `ffx-demaster-<version>.zip`, with twelve mods (PS2 music engine,
  soundtrack, movies, characters, maps, effects, lighting, no HD
  post-effects, interface, aspect ratio, game feel, bug fixes) and the
  shared library;
- one install step, `mods\yuta.ffx-mods\install.cmd`: every mod into
  `mods\loadorder`, every mod's PS2 files from one pass over your own disc
  (an image or the disc in a drive);
- settings per mod (`mods\yuta.<mod>\settings.toml`) instead of one
  `demaster.toml`; one F8 window for all the mods;
- the old `mods\yuta.demaster` is taken out of `mods\loadorder` by the
  installer.

Built from ffx-mods at the revision in `bundle.toml`. No game data.
