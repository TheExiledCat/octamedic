#!/usr/bin/env bash
# ==============================================================================
# build_mac.sh — Build OctaMEDIC for macOS
#
# Builds the GUI release binary for macOS and uses
# octamedic_gui/macos/bundle.sh to package OctaMEDIC.app into target/.
# ==============================================================================

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Error: build_mac.sh is only intended for macOS." >&2
    exit 1
fi

RUN_APP=false
APP_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --run)
            RUN_APP=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS] [MODULE_FILE]"
            echo ""
            echo "Options:"
            echo "  --run         Launch OctaMEDIC.app after building"
            echo "  --help, -h    Show this help message"
            exit 0
            ;;
        -*)
            echo "Error: Unrecognized option '$1'" >&2
            echo ""
            echo "Usage: $0 [OPTIONS] [MODULE_FILE]"
            echo "Try '$0 --help' for more information."
            exit 1
            ;;
        *)
            RUN_APP=true
            if [[ -e "$1" ]]; then
                APP_ARGS+=("$(realpath "$1")")
            else
                APP_ARGS+=("$1")
            fi
            shift
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TARGET="$(rustc -vV | sed -n 's/host: //p')"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' octamedic_gui/Cargo.toml | head -n 1)"
if [[ -z "$VERSION" ]]; then
    echo "Error: Failed to determine version from octamedic_gui/Cargo.toml" >&2
    exit 1
fi

echo "==> Building OctaMEDIC GUI (v$VERSION) for $TARGET..."
cargo build --release --locked --target "$TARGET" -p octamedic_gui

echo "==> Packaging OctaMEDIC.app into target/..."
octamedic_gui/macos/bundle.sh "$TARGET" "$VERSION" "$SCRIPT_DIR/target"

echo "==> Build complete! Output: $SCRIPT_DIR/target/OctaMEDIC.app"

if [ "$RUN_APP" = true ]; then
    echo "==> Launching OctaMEDIC.app..."
    if [ ${#APP_ARGS[@]} -gt 0 ]; then
        open "$SCRIPT_DIR/target/OctaMEDIC.app" --args "${APP_ARGS[@]}"
    else
        open "$SCRIPT_DIR/target/OctaMEDIC.app"
    fi
fi
