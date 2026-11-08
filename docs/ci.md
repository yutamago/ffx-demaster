# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow):

| Job | Runner | What |
|---|---|---|
| Mod (C#) | ubuntu-latest | `dotnet build -c Release ffx-demaster.slnx`; uploads the mod folder (artifact `mod`). Needs no secret. |
| Converter tests (Linux) | ubuntu-latest | `cargo test --locked -p demaster -p demaster-live`. Tests that need the disc or the game skip themselves. |
| Windows build and package | windows-latest | `cargo build --locked --release --target x86_64-pc-windows-msvc -p demaster` (static CRT from `.cargo/config.toml`), runs `demaster.exe --version` and `jobs`, packages the zip with `tools/package.cs` from the `mod` artifact and checks it: the required files are there and nothing else (no `efl/`, `build/`, `tools/`, `.phyre`, `.iso`, images, sound, movies). Uploads `zip` (`yuta.demaster-ci.zip`). |

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mod (`-p:Version`) to the tag without the
`v` (the mod compares the two), then creates a GitHub release named
"FFX Demaster <tag>" with `yuta.demaster-<tag>.zip`, a short install note and GitHub's generated notes.
A tag with a `-` (`v0.2.0-alpha`) makes a prerelease. To release:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

## ffx-rust

The converter uses crates from ffx-rust (`github.com/yutamago/ffx-rust`,
public since 2026-11), pinned to a revision in `Cargo.toml`. cargo fetches
it like any git dependency: CI needs no secret, and forks build the same.
(Until it went public the Rust jobs read a token from the `FFX_RUST_TOKEN`
secret; that step is gone, and the secret can be deleted.)
