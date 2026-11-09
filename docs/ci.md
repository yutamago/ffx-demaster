# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow). One job, on
windows-latest:

1. reads `bundle.toml`: the ffx-mods revision and the bundle's mods;
2. checks out ffx-mods (github.com/yutamago/ffx-mods, public) at that
   revision into `ffx-mods/`;
3. builds there what the zip ships: `demaster.exe`
   (`x86_64-pc-windows-msvc`) and `demaster_live.dll`
   (`i686-pc-windows-msvc`), static CRT from ffx-mods' `.cargo/config.toml`,
   and every C# mod (`dotnet build -c Release ffx-mods.slnx`);
4. packages the bundle with ffx-mods' `tools/package.cs --bundle
   ffx-demaster --mods <bundle.toml's mods> --readme tools/package/README.txt`;
5. checks the zip: the shared library with `demaster.exe`, `install.cmd`,
   `install.ps1`, `README.txt`, `LICENSES.txt`; each mod of `bundle.toml`
   with its DLL, manifest, `settings.toml`, `README.txt`, `LICENSES.txt`
   and install script; `demaster_live.dll` for the music engine; exactly
   one `demaster.exe`; nothing else (no `efl/`, `build/`, `music/`,
   `tools/`, `.phyre`, `.iso`, images, sound, movies);
6. uploads the zip (artifact `bundle`, `ffx-demaster-ci.zip`).

The converter's and the mods' tests run in ffx-mods' own CI at every
revision; a revision goes into `bundle.toml` after it passed there and was
tested in the game.

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mods (`-p:Version`) to the tag without the
`v` (each mod compares its version with the converter's in its build
stamp), then creates a GitHub release named "FFX Demaster <tag>" with
`ffx-demaster-<version>.zip`, a short install note and GitHub's generated
notes. A tag with a `-` (`v0.2.0-alpha`) makes a prerelease. To release:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

## No secrets

ffx-mods and ffx-rust (the converter's crates, pinned in ffx-mods'
`Cargo.toml`) are public: the checkout and cargo fetch them without a
token, and pull requests from forks build the same way.

## Locally

`tools/bundle.sh <out dir> [version]` does the same on Linux/WSL with
cargo-xwin (`FFX_MODS=<checkout>` builds from a local ffx-mods instead of
the pinned revision).
