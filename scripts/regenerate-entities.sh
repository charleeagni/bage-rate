#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"
. "${template_root}/scripts/require-tools.sh"

if [ "$#" -gt 1 ]; then
  echo "usage: regenerate-entities.sh [output-root]" >&2
  exit 2
fi

output_root=""
if [ "$#" -eq 1 ]; then
  output_root="$1"
  if [ -e "${output_root}" ]; then
    echo "ERROR: generation output already exists: ${output_root}" >&2
    exit 1
  fi
  mkdir -p "${output_root}"
fi

scratch_directory="$(mktemp -d)"
trap 'rm -rf "${scratch_directory}"' EXIT

for module_directory in "${template_root}"/modules/*/rust; do
  [ -f "${module_directory}/Cargo.toml" ] || continue
  # A Module before its first migration (`m<date>_...` beside migrations/mod.rs)
  # owns no tables, so it has no entities to generate; the generator would only
  # emit an empty entity directory for its empty scratch Store.
  compgen -G "${module_directory}/src/migrations/m[0-9]*.rs" >/dev/null || continue
  module_name="$(basename "$(dirname "${module_directory}")")"

  # A scratch Store built from only this Module's migrations, so the generated
  # entities can only ever describe tables the Module owns.
  generation_database="${scratch_directory}/${module_name}.sqlite"
  (cd "${template_root}" && cargo run --locked --quiet --package app-schema --bin prepare_generation_db -- --module "${module_name}" "${generation_database}") >/dev/null

  if [ -n "${output_root}" ]; then
    generated_directory="${output_root}/${module_name}"
  else
    generated_directory="${scratch_directory}/${module_name}-entities"
  fi

  sea-orm-cli generate entity \
    --database-url "sqlite://${generation_database}" \
    --output-dir "${generated_directory}" \
    --entity-format dense \
    --seaography

  if [ -z "${output_root}" ]; then
    # This is the one deliberate replacement boundary: the directory is
    # entirely generator-owned and the exact target is fixed per Module.
    rm -rf "${module_directory}/src/entities"
    mv "${generated_directory}" "${module_directory}/src/entities"
    echo "generated modules/${module_name}/rust/src/entities"
  fi
done
