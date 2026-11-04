#!/usr/bin/env bash
# Builds tools/loopcap as loopcap.exe (WASAPI process loopback capture) with cargo-xwin.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86_64
export RUSTC_WRAPPER=
target=x86_64-pc-windows-msvc
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
eval "$(flock "$cache/ffx-xwin.lock" cargo xwin env --target "$target" 2>/dev/null)"
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
cd "$repo/tools/loopcap"
cargo build -j 4 --release --target "$target" "$@"
echo "built: $repo/tools/loopcap/target/$target/release/loopcap.exe"
