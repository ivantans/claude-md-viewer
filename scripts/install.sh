#!/bin/sh
# Installs the latest Claude MD Viewer release on macOS:
#
#   curl -fsSL https://raw.githubusercontent.com/ivantans/claude-md-viewer/main/scripts/install.sh | sh
#
# The release builds are not signed. A file fetched with curl is not quarantined, so an app
# installed this way opens without the "Apple could not verify" dialog. Set INSTALL_DIR to
# install somewhere other than /Applications.
set -eu

app="Claude MD Viewer.app"
releases="https://github.com/ivantans/claude-md-viewer/releases/latest/download"
destination="${INSTALL_DIR:-/Applications}"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "This installer is for macOS. See the README for Windows and Linux downloads." >&2
  exit 1
fi
# hw.optional.arm64 is 1 on Apple Silicon even when this shell runs under Rosetta.
if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = "1" ]; then
  asset="Claude-MD-Viewer-macOS-Apple-Silicon.dmg"
else
  asset="Claude-MD-Viewer-macOS-Intel.dmg"
fi

work="$(mktemp -d)"
cleanup() {
  hdiutil detach -quiet "$work/volume" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

echo "Downloading $asset..."
curl -fL --proto '=https' --tlsv1.2 --progress-bar -o "$work/$asset" "$releases/$asset"
hdiutil attach -quiet -nobrowse -readonly -mountpoint "$work/volume" "$work/$asset"

echo "Installing to $destination/$app..."
mkdir -p "$destination"
rm -rf "$destination/$app"
ditto "$work/volume/$app" "$destination/$app"

echo "Done. Open it with: open \"$destination/$app\""
