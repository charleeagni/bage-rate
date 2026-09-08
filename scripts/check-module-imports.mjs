// The seal, half one: what a Module's hand-authored source is allowed to name.
//
// A Module registers through `module_host` and nothing else. That is a rule a
// compiler cannot enforce here, because the generated entity directory needs
// `seaography`, `sea-orm`, and `tokio` in the manifest and a manifest is
// crate-wide. So the rule is checked instead — over the files a Module
// actually writes, with `entities/` and `generated/` excluded because a
// Module does not write those.
//
// The Rust half allows exactly two crate roots, `crate` and `module_host`,
// plus the language's own. The ui half allows React, Apollo's cache types,
// GraphQL, and the Module's own files.

import { readFile } from "node:fs/promises";
import path from "node:path";

import {
  handAuthoredFiles,
  modules,
  modulesDirectory,
  report,
} from "./module-tree.mjs";

const RUST_ROOTS = new Set(["crate", "self", "super", "Self", "std", "core", "module_host"]);

// The crates a Module's manifest carries for its generated half. Naming one in
// hand-authored code is the seal breaking, so they are rejected by name rather
// than left to the `use`-root scan, which a bare `seaography::Builder` call
// would slip past.
const RUST_FORBIDDEN = [
  "async_graphql",
  "futures_util",
  "sea_orm",
  "sea_orm_migration",
  "seaography",
  "seaolim",
];

const UI_PACKAGES = new Set(["@apollo/client", "graphql", "react", "react-dom"]);

// A Module's own tests run under the repository's test runner, so a test file
// may reach for it. Nothing else in the ui half may.
const isTest = (file) => /\.test\.tsx?$/.test(file);

const stripped = (source) =>
  source
    // Comments quote the crates they warn about, and a doc comment naming
    // `seaography` is the opposite of a violation.
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
    .replace(/^[ \t]*#!.*$/gm, "");

/**
 * The modules a Module's crate declares. Rust's uniform paths let `use` name a
 * sibling module directly, so those names are roots too — and they are the
 * Module's own code by construction.
 */
function declaredModules(sources) {
  const declared = new Set();
  for (const source of sources) {
    for (const [, name] of stripped(source).matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_]\w*)/gm)) {
      declared.add(name);
    }
  }
  return declared;
}

function rustViolations(source, file, localModules) {
  const found = [];
  const code = stripped(source);

  for (const [, root] of code.matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+(?:::)?([A-Za-z_]\w*)/gm)) {
    if (!RUST_ROOTS.has(root) && !localModules.has(root)) {
      found.push(`${file}: imports from "${root}"; a Module imports from crate or module_host`);
    }
  }
  for (const crate of RUST_FORBIDDEN) {
    if (new RegExp(`\\b${crate}\\b`).test(code)) {
      found.push(
        `${file}: names "${crate}", which lives below the seam; reach it through module_host`,
      );
    }
  }
  if (/\b__private\b/.test(code)) {
    found.push(`${file}: names module_host::__private, which exists only for the seam's macros`);
  }
  if (/^\s*extern\s+crate\b/m.test(code)) {
    found.push(`${file}: declares an extern crate`);
  }
  return found;
}

function uiViolations(source, file, moduleRoot) {
  const found = [];
  const code = stripped(source);
  const specifiers = [
    ...code.matchAll(/^\s*(?:import|export)[\s\S]*?from\s+["']([^"']+)["']/gm),
    ...code.matchAll(/\bimport\s*\(\s*["']([^"']+)["']\s*\)/g),
  ].map(([, specifier]) => specifier);

  for (const specifier of specifiers) {
    if (specifier.startsWith("node:") && isTest(file)) continue;
    if (specifier.startsWith(".")) {
      const resolved = path.resolve(path.dirname(file), specifier);
      if (!resolved.startsWith(`${moduleRoot}${path.sep}`)) {
        found.push(`${file}: imports "${specifier}", which leaves the Module's own folder`);
      }
      continue;
    }
    const packageName = specifier.startsWith("@")
      ? specifier.split("/").slice(0, 2).join("/")
      : specifier.split("/")[0];
    if (!UI_PACKAGES.has(packageName)) {
      found.push(
        `${file}: imports "${specifier}"; a Module's ui half sees React, @apollo/client, and graphql`,
      );
    }
  }
  return found;
}

const directory = modulesDirectory();
const failures = [];

for (const module of await modules(directory)) {
  const rustRoot = path.join(module.root, "rust", "src");
  const rustFiles = await handAuthoredFiles(rustRoot, [".rs"]);
  const rustSources = await Promise.all(rustFiles.map((file) => readFile(file, "utf8")));
  const localModules = declaredModules(rustSources);
  for (const [index, file] of rustFiles.entries()) {
    failures.push(
      ...rustViolations(rustSources[index], path.relative(directory, file), localModules),
    );
  }

  const uiRoot = path.join(module.root, "ui");
  for (const file of await handAuthoredFiles(uiRoot, [".ts", ".tsx"])) {
    failures.push(
      ...uiViolations(await readFile(file, "utf8"), file, uiRoot).map((violation) =>
        violation.replaceAll(`${directory}${path.sep}`, ""),
      ),
    );
  }
}

report(failures, "module import seal holds");
