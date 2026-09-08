#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"

# The canonical proof for both Targets. The Module seal holds each Module's
# hand-authored source and manifests to what the authoring surface sanctions,
# and the valve guard holds declared escape hatches to a cap and a written
# exception; the TypeScript suite carries the template configuration tests,
# the Transport selection tests, and the seal's own behavioural tests; the bundle
# check builds both Targets' UI assets and proves neither carries the other's
# Transport; the workspace tests drive the composed Server Process over real
# sockets, including the full graphql-ws lifecycle.
#
# The syntax sweep comes first because it is the cheapest, and because it is the
# only thing that reaches `dev-web.sh`, which starts a development session and
# so can never be run to completion by a check.
./scripts/check-script-syntax.sh
# The seal, before anything expensive: both halves are file reads, and a Module
# that reached below the seam has nothing to gain from a green test run.
./scripts/run-js.sh scripts/check-module-imports.mjs
./scripts/run-js.sh scripts/check-module-dependencies.mjs
./scripts/check-generated-drift.sh
# The valve guard reads the declarations out of the composed registry, so it
# runs after the drift check has already built app-schema.
./scripts/run-js.sh scripts/check-escape-hatches.mjs
./scripts/run-package-script.sh typecheck
./scripts/test-typescript.sh
./scripts/check-target-bundles.sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
# The other half of the Web Target: the Server Process links and produces a
# binary. Debug rather than release, because `build:web` owns the release build
# and repeating it here would double the slowest step in this script for no
# additional proof.
cargo build --locked --package web-server
# That build runs where the Tauri prerequisites are already installed, so it
# would stay green if the Server Process grew a Tauri dependency. This checks
# the resolved graph instead.
./scripts/check-server-process-dependencies.sh
