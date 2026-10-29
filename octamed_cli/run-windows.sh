#!/usr/bin/env bash
# Builds and runs octamed_cli with the Windows toolchain from WSL, for native audio.
# Arguments are passed on, and it runs in the current directory.
set -e

WIN_TARGET_DIR="$(cmd.exe /c echo %LOCALAPPDATA% 2>/dev/null | tr -d '\r')\\octamedic\\target"

(cd "$(dirname "$0")" && cargo.exe build -p octamed_cli --target-dir "$WIN_TARGET_DIR")

CMD="Start-Process -Wait -FilePath '$WIN_TARGET_DIR\\debug\\octamed_cli.exe' -WorkingDirectory '$(wslpath -w "$PWD")'"
if [ $# -gt 0 ]; then
    CMD+=" -ArgumentList $(printf "'%s'," "$@" | sed 's/,$//')"
fi
powershell.exe -NoProfile -Command "$CMD"
