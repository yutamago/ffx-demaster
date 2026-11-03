#!/usr/bin/env bash
# Builds tools/pcsx2ctl as pcsx2ctl.exe (x86_64-pc-windows-msvc) with cargo-xwin,
# like tools/build-windows.sh. Usage: tools/build-pcsx2ctl.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86_64
export RUSTC_WRAPPER=
target=x86_64-pc-windows-msvc
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
mkdir -p "$cache"
eval "$(flock "$cache/ffx-xwin.lock" cargo xwin env --target "$target" 2>/dev/null)"
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
cd "$repo/tools/pcsx2ctl"
cargo build -j 4 --release --target "$target" "$@"
echo "built: $repo/tools/pcsx2ctl/target/$target/release/pcsx2ctl.exe"
