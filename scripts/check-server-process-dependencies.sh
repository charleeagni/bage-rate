#!/usr/bin/env bash
set -euo pipefail

# Proves the Server Process links no Tauri code and needs no webview toolchain.
#
# `cargo build --package web-server` cannot prove this: every developer machine
# and CI runner that builds this workspace already has the Tauri prerequisites
# installed, so adding `tauri.workspace = true` to crates/web-server/Cargo.toml
# would keep that build green while silently breaking the headless deployment
# story. The property is one of the resolved dependency graph, so it is checked
# there.
#
# `--target all` rather than the host target: the webview toolchain differs per
# platform, and a Linux-only dependency must fail this check when it runs on
# macOS too. `-e normal,build` covers both what the binary links and what its
# build scripts need; dev-dependencies are excluded because they never reach a
# deployed binary.

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

# The Tauri crates themselves, plus the native webview stacks they pull in on
# Linux (webkit2gtk and its GTK/GLib/libsoup surround), macOS, and Windows.
forbidden='^(tauri|taurpc|tao|wry|webkit2gtk|javascriptcore|soup[0-9]*-sys|glib|gtk|gdk|webview2-com|muda|objc2-web-kit)'

dependency_names() {
  cargo tree --locked --package "$1" --edges normal,build --target all --prefix none |
    awk 'NF { print $1 }' | sort -u
}

# Resolved before grepping so that a cargo failure aborts the script rather
# than being read as an empty, and therefore clean, dependency list.
server_dependencies="$(dependency_names web-server)"
desktop_dependencies="$(dependency_names tauri-graphql-app)"

server_matches="$(printf '%s\n' "${server_dependencies}" | grep -E "${forbidden}" || true)"
if [[ -n "${server_matches}" ]]; then
  echo "ERROR: the Server Process depends on Tauri or webview packages:" >&2
  printf '  %s\n' ${server_matches} >&2
  echo "The Web Target must build and run in a container with no webview toolchain." >&2
  echo "See docs/stability-boundary.md; move the claim to the unproven side or drop the dependency." >&2
  exit 1
fi

# Without this half the check would pass vacuously if the Tauri crates were
# renamed or the pattern above broke: the Desktop Target must still match it.
if ! printf '%s\n' "${desktop_dependencies}" | grep -qE "${forbidden}"; then
  echo "ERROR: the Desktop Target matches none of the Tauri or webview package patterns," >&2
  echo "so checking the Server Process against them proves nothing. Update the pattern in $0." >&2
  exit 1
fi

echo "The Server Process depends on no Tauri or webview package."
