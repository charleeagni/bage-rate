#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

# One command, both halves of the Web Target. The Server Process answers
# GraphQL over HTTP and graphql-ws; the bundler serves the UI with hot module
# replacement in a real browser, with its devtools, and proxies both endpoints
# to the Server Process. Development therefore keeps the deployment's
# same-origin model and needs no endpoint configuration.
#
# The Store defaults to the same file the Server Process uses when it is run
# by hand, so development data survives restarts of this command.
store="${WEB_TARGET_DEV_STORE:-server-store.db}"

# Compiled before it is spawned so that the bundler comes up beside a Server
# Process that is already listening rather than one that is still building.
cargo build --locked --quiet --package web-server
cargo run --locked --quiet --package web-server -- --store "${store}" &
server_process=$!

stop_server_process() {
  kill "${server_process}" 2>/dev/null || true
  wait "${server_process}" 2>/dev/null || true
}
trap stop_server_process EXIT INT TERM

# A Server Process that never bound — a stale one still holding the port, say —
# would otherwise leave the bundler proxying to nothing, and the developer
# would meet the failure as an unexplained error in the browser.
sleep 1
if ! kill -0 "${server_process}" 2>/dev/null; then
  echo "ERROR: the Server Process exited before the bundler started." >&2
  exit 1
fi

APP_TARGET=web ./scripts/run-js.sh node_modules/vite/bin/vite.js
