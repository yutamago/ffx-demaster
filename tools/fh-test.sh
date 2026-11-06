#!/usr/bin/env bash
# A Fahrenheit game that the ffx-rust harness can drive: Fahrenheit loads the
# mod (code patches, hooks, files), ffx-rust's DINPUT8.dll beside FFX.exe
# serves the harness (loadsave, read, write, press, shot, trace) on a port,
# in background mode (the user's keyboard, mouse and focus untouched).
#
#   tools/fh-test.sh start <fh install> <game copy> <port> <home> [ENV=value...]
#   tools/fh-test.sh stop  <fh install>
#
# <fh install>: a copy of the Fahrenheit install (WSL path under /mnt/c), its
#   mod in mods/yuta.demaster. <game copy>: a hard-linked copy of the game
#   folder (`cp -al`; the DLL is put into it, never into the shared copy).
# <home>: the Documents folder the game sees (FFX_HARNESS_HOME): a folder with
#   "Square Enix/FINAL FANTASY X&X-2 HD Remaster/" (GameSetting.ini and the
#   saves "FINAL FANTASY X/ffx_NNN"), e.g. a copy of a harness slot's home.
# Extra ENV=value pairs reach the game (DEMASTER_LIVE_WAV=..., DEMASTER_DSP_BUFFER=...).
# Then play a replay on it: SHOTRUN_CONNECT=<port> shotrun <replay> <trace> (the
# replay's `realtime` line is ignored: the game runs at its own rate anyway).
# The harness's replacements stay off (FFX_HOOK_DISABLE=replacements): the
# original code runs, with the mod's patches. stop ends the fhstage0.exe this
# started, by its PID (its FFX.exe with it), never a game by name.
set -euo pipefail
cmd="$1"; fh="$2"
pidfile="$fh/fh-test.pid"
pids() { tasklist.exe /FI "IMAGENAME eq $1" /FO CSV /NH 2>/dev/null | tr -d '\r' | awk -F'","' -v n="$1" '$1 == "\"" n {print $2}' | sort; }
case "$cmd" in
start)
    game="$3"; port="$4"; home="$5"; shift 5
    ffx_rust="$(realpath "${FFX_RUST:-$(dirname "$0")/../../ffx-decomp}")"
    dll="$ffx_rust/target/i686-pc-windows-msvc/release/dinput8.dll"
    [[ -f "$dll" ]] || { echo "no $dll (ffx-rust: tools/build-hook.sh)" >&2; exit 1; }
    rm -f "$game/DINPUT8.dll"
    install -m 755 "$dll" "$game/DINPUT8.dll"
    chmod +x "$game"/*.exe "$game"/*.dll
    export FFX_HARNESS="$port" FFX_HARNESS_BACKGROUND=1 FFX_HOOK_DISABLE=replacements
    export FFX_HARNESS_HOME="$(wslpath -w "$home")"
    WSLENV="${WSLENV:+$WSLENV:}FFX_HARNESS:FFX_HARNESS_BACKGROUND:FFX_HOOK_DISABLE:FFX_HARNESS_HOME"
    for kv in "$@"; do
        export "${kv%%=*}=${kv#*=}"
        WSLENV="$WSLENV:${kv%%=*}"
    done
    export WSLENV
    before="$(pids fhstage0.exe)"
    wgame="$(wslpath -w "$game/FFX.exe")"
    (cd "$fh/bin" && exec ./fhstage0.exe "$wgame" > /dev/null 2>&1) &
    pid=""
    for _ in $(seq 60); do
        new="$(comm -13 <(echo "$before") <(pids fhstage0.exe))"
        if [[ -n "$new" ]]; then
            [[ "$(wc -l <<< "$new")" -eq 1 ]] || { echo "several new fhstage0.exe: $new; not tracking" >&2; exit 1; }
            pid="$new"; break
        fi
        sleep 1
    done
    [[ -n "$pid" ]] || { echo "fhstage0.exe did not start" >&2; exit 1; }
    echo "$pid" > "$pidfile"
    echo "fhstage0 PID $pid; harness port $port"
    # ready once the harness answers
    for _ in $(seq 120); do
        if python3 -c "import socket,sys; s=socket.create_connection(('127.0.0.1',$port),1); s.sendall(b'ping\n'); sys.exit(0 if s.recv(64) else 1)" 2>/dev/null; then
            echo "harness up"; exit 0
        fi
        sleep 1
    done
    echo "the harness did not answer on $port" >&2; exit 1 ;;
stop)
    [[ -f "$pidfile" ]] || { echo "no $pidfile" >&2; exit 1; }
    pid="$(cat "$pidfile")"
    taskkill.exe /PID "$pid" /T /F || true
    rm -f "$pidfile" ;;
*) echo "start|stop" >&2; exit 2 ;;
esac
