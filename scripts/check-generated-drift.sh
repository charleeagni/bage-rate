#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${template_root}"
. scripts/require-tools.sh

scratch_directory="$(mktemp -d)"
trap 'rm -rf "${scratch_directory}"' EXIT
failures=0

echo "==> module registry drift"
./scripts/generate-module-registry.sh "${scratch_directory}/module-registry" >/dev/null
if ! diff -ru crates/module-registry "${scratch_directory}/module-registry"; then
  echo "ERROR: the generated module registry is stale." >&2
  failures=$((failures + 1))
fi

echo "==> entity drift"
./scripts/regenerate-entities.sh "${scratch_directory}/entities" >/dev/null
for module_directory in modules/*/rust; do
  [ -d "${module_directory}/src/entities" ] || continue
  module_name="$(basename "$(dirname "${module_directory}")")"
  if ! diff -ru "${module_directory}/src/entities" "${scratch_directory}/entities/${module_name}"; then
    echo "ERROR: generated SeaORM entities for module ${module_name} are stale." >&2
    failures=$((failures + 1))
  fi
done

echo "==> schema drift"
cargo run --locked --quiet --package app-schema --bin export_schema -- "${scratch_directory}/schema.graphql" >/dev/null
if ! diff -u schema.graphql "${scratch_directory}/schema.graphql"; then
  echo "ERROR: the committed GraphQL SDL is stale." >&2
  failures=$((failures + 1))
fi

echo "==> TauRPC binding drift"
cargo run --locked --quiet --package tauri-graphql-transport --bin export_bindings -- "${scratch_directory}/taurpc.ts" >/dev/null
if ! diff -u src/generated/taurpc.ts "${scratch_directory}/taurpc.ts"; then
  echo "ERROR: the generated TauRPC bindings are stale." >&2
  failures=$((failures + 1))
fi

echo "==> module index drift"
./scripts/generate-module-index.sh "${scratch_directory}/modules.ts" >/dev/null
if ! diff -u src/generated/modules.ts "${scratch_directory}/modules.ts"; then
  echo "ERROR: the generated frontend module index is stale." >&2
  failures=$((failures + 1))
fi

echo "==> per-module GraphQL document drift"
for module_directory in modules/*/ui; do
  compgen -G "${module_directory}/operations/*.graphql" >/dev/null || continue
  module_name="$(basename "$(dirname "${module_directory}")")"
  cargo run --locked --quiet --package app-schema --bin export_schema -- \
    --module "${module_name}" "${scratch_directory}/${module_name}-mini.graphql" >/dev/null
  CODEGEN_SCHEMA="${scratch_directory}/${module_name}-mini.graphql" \
    CODEGEN_DOCUMENTS="./${module_directory}/operations/**/*.graphql" \
    CODEGEN_OUTPUT="${scratch_directory}/${module_name}-graphql.ts" \
    ./scripts/run-js.sh node_modules/@graphql-codegen/cli/esm/bin.js --config codegen.ts >/dev/null
  if ! diff -u "${module_directory}/src/generated/graphql.ts" "${scratch_directory}/${module_name}-graphql.ts"; then
    echo "ERROR: the generated GraphQL documents for module ${module_name} are stale." >&2
    failures=$((failures + 1))
  fi
done

if [ "${failures}" -ne 0 ]; then
  echo "generated-artifact drift check failed (${failures} stale artifact groups)" >&2
  exit 1
fi

echo "generated-artifact drift check passed"
