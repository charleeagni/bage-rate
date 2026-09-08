#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

# The two halves of the Web Target are built independently: the bundler writes
# the Web Target UI assets, and Cargo builds the Server Process that reads them
# from a directory at runtime. Neither build consumes the other's output, so
# the Rust build needs no bundler run and a stale embedded bundle cannot
# happen.
APP_TARGET=web ./scripts/run-js.sh node_modules/vite/bin/vite.js build
cargo build --locked --release --package web-server

web_assets="dist-web"
server_process="target/release/web-server"
# Cargo appends the platform's executable suffix, which the announced path must
# not assume away on Windows.
if [[ ! -f "${server_process}" && -f "${server_process}.exe" ]]; then
  server_process="${server_process}.exe"
fi

# The paths announced below are the deliverables of this command, so they are
# checked rather than asserted in prose: a bundler output directory that moved,
# or a renamed package whose binary lands elsewhere, would otherwise print
# directions to files that do not exist.
if [[ ! -f "${web_assets}/index.html" ]]; then
  echo "ERROR: ${web_assets} holds no entry document; the bundler wrote the Web Target elsewhere." >&2
  exit 1
fi
if [[ ! -x "${server_process}" ]]; then
  echo "ERROR: ${server_process} is not an executable; the Server Process build wrote its binary elsewhere." >&2
  exit 1
fi

echo "Web Target UI assets: ${web_assets}"
echo "Server Process: ${server_process}"
echo "Serve both with: ${server_process}"
