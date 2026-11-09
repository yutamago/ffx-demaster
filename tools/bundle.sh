#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Builds the FFX Demaster zip on Linux/WSL: ffx-mods at the revision pinned in bundle.toml,
# its C# mods, demaster.exe and demaster_live.dll (cargo-xwin, through ffx-mods' own
# tools/build-windows.sh and tools/build-live.sh), then ffx-mods' tools/package.cs --bundle
# with this repo's tools/package/README.txt. CI does the same on Windows (docs/ci.md).
#
# usage: tools/bundle.sh <out dir> [version]       -> <out dir>/ffx-demaster-<version>.zip
#   FFX_MODS=<checkout>   use that ffx-mods checkout as it is (its HEAD, uncommitted changes
#                         included) instead of the pinned revision; says so when they differ
#   version               default 0.1.0-dev; also the version of the mods and the converter
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
out="$(mkdir -p "${1:?usage: tools/bundle.sh <out dir> [version]}" && cd "$1" && pwd)"
version="${2:-0.1.0-dev}"
version="${version#v}"

read -r url rev mods < <(python3 -I -c '
import sys, tomllib
b = tomllib.load(open(sys.argv[1], "rb"))["ffx-mods"]
print(b["repo"], b["rev"], ",".join(b["mods"]))' "$repo/bundle.toml")

if [ -n "${FFX_MODS:-}" ]; then
    src="$(cd "$FFX_MODS" && pwd)"
    head="$(git -C "$src" rev-parse HEAD)"
    [ "$head" = "$rev" ] || echo "note: $src is at $head, bundle.toml pins $rev" >&2
    [ -z "$(git -C "$src" status --porcelain)" ] || echo "note: $src has uncommitted changes" >&2
else
    src="${XDG_CACHE_HOME:-$HOME/.cache}/ffx-demaster/ffx-mods"
    [ -d "$src/.git" ] || git clone --quiet "$url" "$src"
    git -C "$src" fetch --quiet origin
    git -C "$src" checkout --quiet --detach "$rev"
fi
echo "ffx-mods: $src ($(git -C "$src" rev-parse --short HEAD)), mods: $mods, version $version"

cd "$src"
export DEMASTER_VERSION="$version"
tools/build-windows.sh
tools/build-live.sh
dotnet build -c Release -p:Version="$version" ffx-mods.slnx
dotnet run tools/package.cs -- "$out" "$version" --bundle ffx-demaster --mods "$mods" \
    --readme "$repo/tools/package/README.txt"
