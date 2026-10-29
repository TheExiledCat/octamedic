#!/usr/bin/env bash
# Builds and runs octamedic with the Windows toolchain from WSL, for native audio.
# PACKAGE picks the program (octamed_cli by default, or octamedic_gui); arguments are passed on.
set -e

PACKAGE=${PACKAGE:-octamed_cli}
WIN_TARGET_DIR="$(cmd.exe /c echo %LOCALAPPDATA% 2>/dev/null | tr -d '\r')\\octamedic\\target"

cargo.exe build -p "$PACKAGE" --target-dir "$WIN_TARGET_DIR"

CMD="Start-Process -Wait -FilePath '$WIN_TARGET_DIR\\debug\\$PACKAGE.exe' -WorkingDirectory '$(wslpath -w "$PWD")'"
if [ $# -gt 0 ]; then
    CMD+=" -ArgumentList $(printf "'%s'," "$@" | sed 's/,$//')"
fi
powershell.exe -NoProfile -Command "$CMD"
