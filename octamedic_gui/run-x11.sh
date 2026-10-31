#!/usr/bin/env bash
# Runs octamedic_gui in WSL through X11, since windows exit straight away under WSLg's Wayland.
# Arguments are passed on.
set -e

WAYLAND_DISPLAY= cargo run --manifest-path "$(dirname "$0")/Cargo.toml" -- "$@"
