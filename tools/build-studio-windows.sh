#!/usr/bin/env bash
# Builds ffx-music-studio.exe (FFX Music Studio, docs/studio.md) for Windows
# the way tools/build-windows.sh builds demaster.exe (cargo-xwin, static CRT).
# Usage: tools/build-studio-windows.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
"$repo/tools/build-windows.sh" -p ffx-music-studio "$@"
echo "built: $repo/target/x86_64-pc-windows-msvc/release/ffx-music-studio.exe"
