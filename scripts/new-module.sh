#!/usr/bin/env bash
set -euo pipefail

template_root="$(cd "$(dirname "$0")/.." && pwd)"

# Scaffolds one Module: a folder under modules/ with both halves of the
# Generated Contract's authoring surface (ADR-0006). The name is also the
# Module's namespace prefix, so every table, GraphQL type, and root field it
# registers must carry it — module-host rejects the rest at generate time.
#
# The scaffold compiles and composes before its first migration: the Rust half
# exports a module_def() with no migrations and a pass-through register, and
# the ui half exports a moduleDef with a placeholder component. Feature code
# replaces both.

usage() {
  echo "usage: new-module.sh <name>" >&2
  echo "  <name> is the Module's snake_case name and namespace prefix," >&2
  echo "  for example: time_tracking" >&2
  exit 2
}

[ "$#" -eq 1 ] || usage
module_name="$1"

# snake_case only: the name is spliced into a crate name, a Rust module path,
# an npm package name, and every prefix comparison module-host performs.
if ! [[ "${module_name}" =~ ^[a-z][a-z0-9]*(_[a-z0-9]+)*$ ]]; then
  echo "ERROR: '${module_name}' is not snake_case." >&2
  usage
fi

module_directory="${template_root}/modules/${module_name}"
if [ -e "${module_directory}" ]; then
  echo "ERROR: modules/${module_name} already exists." >&2
  exit 1
fi

# Rust crates are <name>-module with lib <name>_module, npm packages are
# @tauri-graphql-template/<name>-module — exactly the names the generated
# registry and module index derive from the folder name.
lib_name="${module_name}_module"

mkdir -p "${module_directory}/rust/src/migrations"
mkdir -p "${module_directory}/ui/src"
mkdir -p "${module_directory}/ui/operations"

cat >"${module_directory}/rust/Cargo.toml" <<CARGO_TOML
[package]
name = "${module_name}-module"
version = "0.1.0"
description = "The ${module_name} Module: its migrations and generated registrations"
edition = "2021"
publish = false

[lib]
name = "${lib_name}"

[dependencies]
module-host = { path = "../../../crates/module-host" }
# Hand-authored Module code names none of the crates below. They are here
# because the generated entity directory's \`register_entity_modules!\`
# expansion resolves \`seaography\`, \`sea-orm\`, and \`tokio\` by crate name, and
# because the migration prelude's derives expand to \`sea_orm_migration\`
# paths. scripts/check-module-dependencies.mjs holds this list to the
# allowlist; scripts/check-module-imports.mjs proves the hand-authored half
# never names any of them.
sea-orm = { workspace = true }
sea-orm-migration = { workspace = true }
seaography = { workspace = true }
tokio = { workspace = true }
CARGO_TOML

cat >"${module_directory}/rust/src/lib.rs" <<LIB_RS
//! The ${module_name} Module, carried as one folder per ADR-0006.
//!
//! Everything this Module contributes is declared below. Hand-authored files
//! under \`src/\` name exactly two crate roots — \`crate\` and \`module_host\` —
//! which \`scripts/check-module-imports.mjs\` enforces.

pub mod migrations;

// Once the first migration exists and \`generate\` has produced \`src/entities/\`,
// declare \`pub mod entities;\` here and add
// \`entities: entities::register_entity_modules,\` below. Add
// \`custom: custom::register,\` when the Module needs something beyond generated
// CRUD; \`module_host::CustomOps\` is the complete list of what that can be.
module_host::module_def! {
    name: "${module_name}",
    migrations: migrations::Migrator,
}
LIB_RS

cat >"${module_directory}/rust/src/migrations/mod.rs" <<MIGRATIONS_RS
use module_host::migration::*;

// Author the Module's first reversible migration as an
// \`m<date>_<sequence>_<change>\` module here and add it to the vec below in
// declared order. Every table it creates must carry the \`${module_name}\`
// prefix, which module-host enforces at generate time.

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}
MIGRATIONS_RS

cat >"${module_directory}/ui/package.json" <<PACKAGE_JSON
{
  "name": "@tauri-graphql-template/${module_name}-module",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "exports": "./src/index.ts",
  "peerDependencies": {
    "@apollo/client": "4.2.10",
    "graphql": "16.14.2",
    "react": "19.2.8"
  }
}
PACKAGE_JSON

cat >"${module_directory}/ui/src/index.ts" <<INDEX_TS
import { type TypePolicies } from "@apollo/client";
import { createElement, type ComponentType } from "react";

// Replace this placeholder with the Module's real component, and grow
// typePolicies with the Cache Convergence rules its writes declare.
const Placeholder: ComponentType = () =>
  createElement("section", null, "${module_name} Module");

// Every Module's ui half exports one \`moduleDef\` of this shape; the
// generated src/generated/modules.ts imports it by convention, exactly as the
// generated registry imports each Rust half's \`module_def()\`.
export const moduleDef: {
  name: string;
  typePolicies: TypePolicies;
  Component: ComponentType;
} = {
  name: "${module_name}",
  typePolicies: {},
  Component: Placeholder,
};
INDEX_TS

# Caller Operations (.graphql) go here; generate skips the Module's document
# codegen until the first one exists.
touch "${module_directory}/ui/operations/.gitkeep"

# The workspace globs make the folder a build member, but cargo's --locked
# runs refuse a workspace member the lockfile has never seen. Resolving the
# workspace once records the new crate without touching pinned versions.
(cd "${template_root}" && cargo metadata --format-version 1 >/dev/null)

cat <<NEXT_STEPS
created modules/${module_name}/

Next steps:
  1. bun install and npm install, so the ui workspace package resolves and
     both frozen lockfiles record it — CI installs each independently.
  2. bun run generate (or npm run generate), which links the Module into the
     generated registry and module index; review the diff.
  3. Author the first reversible migration under
     modules/${module_name}/rust/src/migrations/ and re-run generate; then
     wire src/lib.rs to the generated entities as its comment describes.
  4. Author Caller Operations under modules/${module_name}/ui/operations/ and
     build the Module's UI in modules/${module_name}/ui/src/.
  5. bun run verify (or npm run verify).
NEXT_STEPS
