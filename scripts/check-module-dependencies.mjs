// The seal, half two: what a Module's manifests are allowed to declare.
//
// The import scan says what a Module's source may name; this says what it may
// have. Without it, `cargo add` and `npm install` would quietly widen the seam
// and only review would notice. The Rust allowlist is larger than the import
// scan permits on purpose: those crates are there for the generated entity
// directory and the migration prelude's derives, both of which resolve them by
// crate name.

import { readFile } from "node:fs/promises";
import path from "node:path";

import { exists, modules, modulesDirectory, report } from "./module-tree.mjs";

const RUST_ALLOWED = new Set([
  // The seam.
  "module-host",
  // Named only by generated code and by macro expansions.
  "sea-orm",
  "sea-orm-migration",
  "seaography",
  "tokio",
]);

const UI_ALLOWED = new Set(["@apollo/client", "graphql", "react"]);

const DEPENDENCY_TABLE = /^\s*\[(?:target\.[^\]]+\.)?(dependencies|dev-dependencies|build-dependencies)\]\s*$/;
const OTHER_TABLE = /^\s*\[/;

/**
 * The crate names in a Cargo manifest's dependency tables. A hand-rolled read
 * rather than a TOML dependency, because the shape it has to understand is
 * `name = ...` and `name.workspace = ...` under a known heading, and adding a
 * parser to reach that would be its own kind of unsanctioned dependency.
 */
function cargoDependencies(manifest) {
  const found = [];
  let inside = false;
  for (const line of manifest.split("\n")) {
    if (DEPENDENCY_TABLE.test(line)) {
      inside = true;
      continue;
    }
    if (OTHER_TABLE.test(line)) {
      inside = false;
      continue;
    }
    if (!inside) continue;
    const [, name] = line.match(/^\s*([A-Za-z0-9_-]+)(?:\.[A-Za-z0-9_-]+)?\s*=/) ?? [];
    if (name) found.push(name);
  }
  return found;
}

const directory = modulesDirectory();
const failures = [];

for (const module of await modules(directory)) {
  const cargo = path.join(module.root, "rust", "Cargo.toml");
  if (await exists(cargo)) {
    for (const dependency of cargoDependencies(await readFile(cargo, "utf8"))) {
      if (!RUST_ALLOWED.has(dependency)) {
        failures.push(
          `${module.name}/rust/Cargo.toml: depends on "${dependency}", which is not a ` +
            `sanctioned Module dependency; a capability a Module needs is added to module-host`,
        );
      }
    }
  }

  const manifest = path.join(module.root, "ui", "package.json");
  if (await exists(manifest)) {
    const parsed = JSON.parse(await readFile(manifest, "utf8"));
    for (const table of ["dependencies", "devDependencies", "peerDependencies"]) {
      for (const dependency of Object.keys(parsed[table] ?? {})) {
        if (!UI_ALLOWED.has(dependency)) {
          failures.push(
            `${module.name}/ui/package.json: ${table} names "${dependency}", which is not a ` +
              `sanctioned Module dependency`,
          );
        }
      }
    }
    // Anything a Module's ui half uses is provided by the host application, so
    // it declares peers and pins nothing itself. A real dependency here would
    // let two Modules resolve two copies of React into one bundle.
    for (const table of ["dependencies", "devDependencies"]) {
      if (Object.keys(parsed[table] ?? {}).length > 0) {
        failures.push(
          `${module.name}/ui/package.json: declares ${table}; a Module's ui half declares ` +
            `peerDependencies so the host resolves one copy of each`,
        );
      }
    }
  }
}

report(failures, "module dependency allowlist holds");
