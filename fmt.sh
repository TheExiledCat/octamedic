#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --quiet

find octamed octamed_cli octamedic_core octamedic_gui -name '*.rs' -not -path '*/target/*' \
    -exec perl -pi -e 's/^(\s*)Self::new\(\)$/$1return Self::new();/' {} +

if rustup run nightly rustfmt --version >/dev/null 2>&1; then
    cargo +nightly fmt --all
else
    echo "nightly rustfmt not found, imports won't be grouped (rustup component add rustfmt --toolchain nightly)" >&2
    cargo fmt --all
fi

cargo clippy --workspace --all-targets --quiet
