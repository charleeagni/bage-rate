#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"
. scripts/require-tools.sh

# The registry links Modules into the host, and every cargo step below needs
# the workspace — registry included — to compile, so it regenerates first.
# It is pure file emission (no cargo), which is what makes that order possible.
./scripts/generate-module-registry.sh
# Adding or deleting a Module changes the registry's dependency list, and the
# --locked runs below refuse a lockfile that has not recorded that. Resolving
# the workspace once reconciles Cargo.lock without touching pinned versions.
cargo metadata --format-version 1 >/dev/null
./scripts/generate-module-index.sh
./scripts/regenerate-entities.sh
cargo run --locked --quiet --package app-schema --bin export_schema -- schema.graphql
cargo run --locked --quiet --package tauri-graphql-transport --bin export_bindings -- src/generated/taurpc.ts

# Per-module codegen against each Module's own mini-schema: an operation that
# names another Module's types fails here, which is the ownership enforcement.
scratch_directory="$(mktemp -d)"
trap 'rm -rf "${scratch_directory}"' EXIT
for module_directory in modules/*/ui; do
  compgen -G "${module_directory}/operations/*.graphql" >/dev/null || continue
  module_name="$(basename "$(dirname "${module_directory}")")"
  cargo run --locked --quiet --package app-schema --bin export_schema -- \
    --module "${module_name}" "${scratch_directory}/${module_name}.graphql"
  CODEGEN_SCHEMA="${scratch_directory}/${module_name}.graphql" \
    CODEGEN_DOCUMENTS="./${module_directory}/operations/**/*.graphql" \
    CODEGEN_OUTPUT="./${module_directory}/src/generated/graphql.ts" \
    ./scripts/run-js.sh node_modules/@graphql-codegen/cli/esm/bin.js --config codegen.ts
done

echo "generated module registry, module index, Models, SDL, TauRPC bindings, and per-module GraphQL documents"
