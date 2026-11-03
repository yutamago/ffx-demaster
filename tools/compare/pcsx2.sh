#!/usr/bin/env bash
# Starts and stops the portable PCSX2 of tools/compare (docs/compare.md).
#
#   tools/compare/pcsx2.sh start [state]   boot the disc (optionally into save state N);
#                                          prints the PID, writes it to $PCSX2/compare.pid
#   tools/compare/pcsx2.sh stop            taskkill /PID of that PID (never by name)
#   tools/compare/pcsx2.sh ctl <args>      pcsx2ctl.exe --pid <PID> <args>
#
# PCSX2 dir: $PCSX2 (default /mnt/c/_workspaces/pcsx2), disc: $PCSX2_ISO
# (default C:\_workspaces\pcsx2\iso\ffx-de.iso). A WSL timeout does not end
# the Windows process; stop does.
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
PCSX2="${PCSX2:-/mnt/c/_workspaces/pcsx2}"
ISO="${PCSX2_ISO:-C:\\_workspaces\\pcsx2\\iso\\ffx-de.iso}"
CTL="${PCSX2CTL:-$repo/tools/pcsx2ctl/target/x86_64-pc-windows-msvc/release/pcsx2ctl.exe}"
pidfile="$PCSX2/compare.pid"

pids() {
    tasklist.exe /FI "IMAGENAME eq pcsx2-qt.exe" /FO CSV /NH 2>/dev/null | tr -d '\r' |
        awk -F'","' '/pcsx2-qt/ {print $2}' | sort
}

case "${1:-}" in
start)
    before="$(pids)"
    if [ -n "$before" ]; then
        echo "a PCSX2 is already running (PID $before); stop it first (one game at a time)" >&2
        exit 1
    fi
    cd "$PCSX2"
    args=(-batch)
    if [ -n "${2:-}" ]; then args+=(-state "$2"); fi
    nohup ./pcsx2-qt.exe "${args[@]}" -- "$ISO" >/dev/null 2>&1 &
    pid=""
    for _ in $(seq 1 50); do
        sleep 0.2
        pid="$(pids | head -n1)"
        [ -n "$pid" ] && break
    done
    [ -n "$pid" ] || { echo "PCSX2 did not start" >&2; exit 1; }
    echo "$pid" >"$pidfile"
    # wait for PINE and a running game
    for _ in $(seq 1 100); do
        if "$CTL" --pid "$pid" status 2>/dev/null | grep -q "running"; then break; fi
        sleep 0.3
    done
    # the first key PCSX2's window gets after the start is lost: spend it on
    # an unbound key
    "$CTL" --pid "$pid" key z 2 10 >/dev/null 2>&1 || true
    echo "$pid"
    ;;
stop)
    [ -f "$pidfile" ] || { echo "no $pidfile" >&2; exit 1; }
    pid="$(cat "$pidfile")"
    taskkill.exe /PID "$pid" /F >/dev/null 2>&1 || true
    for _ in $(seq 1 50); do
        pids | grep -qx "$pid" || break
        sleep 0.2
    done
    rm -f "$pidfile"
    echo "stopped $pid"
    ;;
ctl)
    shift
    exec "$CTL" --pid "$(cat "$pidfile")" "$@"
    ;;
*)
    sed -n '2,12p' "$0"
    exit 2
    ;;
esac
