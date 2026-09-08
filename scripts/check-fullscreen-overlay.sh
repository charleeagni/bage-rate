#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'This check requires a macOS GUI session.' >&2
  exit 1
fi
if [[ $# -gt 1 || ( $# -eq 1 && "$1" != --baseline ) ]]; then
  echo 'Usage: scripts/check-fullscreen-overlay.sh [--baseline]' >&2
  exit 1
fi
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/cute-fullscreen.XXXXXX")"
trap 'rm -rf "$probe_dir"' EXIT
fixture_bundle="$probe_dir/FullscreenFixture.app"
mkdir -p "$fixture_bundle/Contents/MacOS"
cat > "$fixture_bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>fixture</string>
<key>CFBundleIdentifier</key><string>dev.cute.fullscreen-fixture</string>
<key>CFBundleName</key><string>Fullscreen Fixture</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
swiftc -module-cache-path "$probe_dir/module-cache" \
  scripts/fixtures/fullscreen-space.swift -o "$fixture_bundle/Contents/MacOS/fixture"
# This opens a temporary native app, enters fullscreen, and returns to the desktop.
# No application database or saved preferences are read or changed.
CUTE_FULLSCREEN_FIXTURE="$fixture_bundle/Contents/MacOS/fixture" \
  cargo run --locked -p tauri-graphql-app --example fullscreen_overlay_probe -- "$@"
