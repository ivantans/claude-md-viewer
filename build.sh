#!/bin/bash
# Builds "Claude MD Viewer.app" in this folder. Requires the Xcode command line tools.
set -euo pipefail
cd "$(dirname "$0")"
APP="Claude MD Viewer.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
swiftc -O -swift-version 5 -target "$(uname -m)-apple-macos13.0" -o "$APP/Contents/MacOS/ClaudeMDViewer" Sources/main.swift
cp Info.plist "$APP/Contents/Info.plist"
cp Resources/* "$APP/Contents/Resources/"
codesign --force --sign - "$APP" >/dev/null 2>&1
echo "Built: $PWD/$APP"
