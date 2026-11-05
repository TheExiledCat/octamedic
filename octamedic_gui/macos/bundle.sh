#!/usr/bin/env bash
# Makes OctaMEDIC.app out of a release build of the GUI. The release workflow uses it, and it works
# the same locally:
#   cargo build --release --target aarch64-apple-darwin -p octamedic_gui
#   octamedic_gui/macos/bundle.sh aarch64-apple-darwin 0.0.1 dist
set -euo pipefail

if [ $# -ne 3 ]; then
    echo "usage: $0 <target> <version> <output folder>" >&2
    exit 1
fi
target="$1"
version="$2"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
icon="$root/octamedic_gui/assets/icon.png"
app="$3/OctaMEDIC.app"

# the oldest macOS rustc builds the target for, printed as MACOSX_DEPLOYMENT_TARGET=11.0
minimum="$(rustc --print deployment-target --target "$target")"
minimum="${minimum#*=}"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# named after the app, since that's what the app menu calls it (About, Hide, Quit)
cp "$root/target/$target/release/octamedic_gui" "$app/Contents/MacOS/OctaMEDIC"
sed -e "s/@VERSION@/$version/g" -e "s/@MINIMUM_SYSTEM_VERSION@/$minimum/g" \
    "$here/Info.plist" > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist" > /dev/null

# every size an .icns has, each at 1x and 2x
iconset="$(mktemp -d)/OctaMEDIC.iconset"
mkdir "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$icon" --out "$iconset/icon_${size}x${size}.png" > /dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$icon" --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
done
iconutil --convert icns --output "$app/Contents/Resources/OctaMEDIC.icns" "$iconset"
rm -rf "$(dirname "$iconset")"

# ad-hoc signed, so the signature covers the whole bundle and Apple Silicon will run it. It isn't
# notarized, so macOS still asks before opening a downloaded copy
codesign --force --sign - "$app"
echo "$app"
