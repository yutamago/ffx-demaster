# Replays

Scripts for `tools/shotrun` and `tools/calllist` (the ffx-rust harness):
they play the PC game from a save and take screenshots, with the mods'
`efl/x` folders as loose-file mods (README, "Testing in the game";
[`docs/compare.md`](../../docs/compare.md)).

Their comments name the files as they were in FFX Demaster before its code
moved to [ffx-mods](https://github.com/yutamago/ffx-mods): `docs/<name>.md`
is ffx-mods' `docs/<name>.md`; `mod/<Name>Patches.cs` is in ffx-mods'
`mods/<mod>/` (the menu and battle window patches in `mods/ps2-interface/`;
`GamePatches.cs` is split over `ps2-game-feel`, `bug-fixes`, `ps2-interface`
(`LoadingPatches.cs`) and `ps2-soundtrack`; `RenderPatches.cs` over
`ps2-lighting`, `ps2-aspect-ratio`, `ps2-characters` (`TexturePatches.cs`)
and `no-hd-post-effects`); `live-test.sh` (the `music-live*` replays) is
ffx-mods' `tools/live-test.sh`.
