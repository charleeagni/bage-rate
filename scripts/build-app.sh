#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

repo_root="$(pwd -P)"
build_home="${HOME:-}"
cargo_home="${CARGO_HOME:-${build_home:+$build_home/.cargo}}"
rustup_home="${RUSTUP_HOME:-${build_home:+$build_home/.rustup}}"
remap_flags=()

# Rust release binaries can retain source paths in debug metadata. Keep
# developer-supplied flags and append remaps for every local Rust source root.
# rustc applies the last matching remap, so put broad paths before their
# nested paths and the repository-specific path last.
if [[ -n "$build_home" ]]; then
  remap_flags+=("--remap-path-prefix=$build_home=/__build_home__")
fi
if [[ -n "$cargo_home" ]]; then
  remap_flags+=("--remap-path-prefix=$cargo_home=/__build_cargo__")
fi
if [[ -n "$rustup_home" ]]; then
  remap_flags+=("--remap-path-prefix=$rustup_home=/__build_rustup__")
fi
remap_flags+=("--remap-path-prefix=$repo_root=.")

tauri_build() {
  local encoded_flags
  local flag
  if [[ -n "${CARGO_ENCODED_RUSTFLAGS+x}" ]]; then
    # Cargo gives this variable precedence over RUSTFLAGS, so retain that
    # behavior when callers have explicitly set it.
    encoded_flags="$CARGO_ENCODED_RUSTFLAGS"
  else
    # Cargo itself treats RUSTFLAGS as whitespace-separated words. Translate
    # those words without evaluating them, then use its space-safe encoding.
    local -a rustflags_words=()
    local whitespace_flags="${RUSTFLAGS:-}"
    whitespace_flags="${whitespace_flags//$'\n'/ }"
    local rustflag
    encoded_flags=""
    if [[ "$whitespace_flags" =~ [^[:space:]] ]]; then
      read -r -a rustflags_words <<< "$whitespace_flags"
      for rustflag in "${rustflags_words[@]}"; do
        encoded_flags+="${encoded_flags:+$'\x1f'}$rustflag"
      done
    fi
  fi

  for flag in "${remap_flags[@]}"; do
    encoded_flags+="${encoded_flags:+$'\x1f'}$flag"
  done
  CARGO_ENCODED_RUSTFLAGS="$encoded_flags" ./node_modules/.bin/tauri build "$@"
}

if [[ "$(uname -s)" != Darwin ]]; then
  tauri_build
  exit $?
fi

# Tauri signs, notarizes, and staples the app when the Apple notarization
# environment variables are present.
tauri_build --bundles app --ci
app_name="$(node -p "require('./src-tauri/tauri.conf.json').productName")"
app_version="$(node -p "require('./src-tauri/tauri.conf.json').version")"
app_bundle="target/release/bundle/macos/${app_name}.app"
codesign --verify --deep --strict --verbose=2 "$app_bundle"
app_executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_bundle/Contents/Info.plist")"
if [[ -z "$app_executable" || "$app_executable" == */* || "$app_executable" == *$'\n'* ]]; then
  echo "ERROR: Invalid CFBundleExecutable in $app_bundle/Contents/Info.plist" >&2
  exit 1
fi
./scripts/check-release-paths.sh "$app_bundle/Contents/MacOS/$app_executable"

case "$(uname -m)" in
  arm64) architecture=aarch64 ;;
  x86_64) architecture=x64 ;;
  *) echo 'Unsupported macOS architecture' >&2; exit 1 ;;
esac
mkdir -p target/release/bundle/dmg
image_path="$(pwd)/target/release/bundle/dmg/${app_name}_${app_version}_${architecture}.dmg"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/cute-dmg.XXXXXX")"
mounted=false
cleanup() {
  if [[ "$mounted" == true ]]; then hdiutil detach "$work_dir/mount" >/dev/null; fi
  rm -rf "$work_dir"
}
trap cleanup EXIT
mkdir "$work_dir/content" "$work_dir/mount"
ditto "$app_bundle" "$work_dir/content/${app_name}.app"
ln -s /Applications "$work_dir/content/Applications"
# A standard layout avoids requiring permission to automate Finder.
hdiutil create -ov -volname "$app_name" -srcfolder "$work_dir/content" -format UDZO "$image_path"
hdiutil verify "$image_path"
codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$image_path"
xcrun notarytool submit "$image_path" \
  --apple-id "$APPLE_ID" \
  --password "$APPLE_PASSWORD" \
  --team-id "$APPLE_TEAM_ID" \
  --wait
xcrun stapler staple "$image_path"
xcrun stapler validate "$image_path"
hdiutil attach -readonly -nobrowse -mountpoint "$work_dir/mount" "$image_path" >/dev/null
mounted=true
codesign --verify --deep --strict --verbose=2 "$work_dir/mount/${app_name}.app"
echo "Verified signed and notarized app and DMG: $image_path"
