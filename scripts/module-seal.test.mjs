// The seal and the valve guard, proven by running them.
//
// Each test builds a fixture Module tree, points the real check at it, and
// asserts the exit code. A check that only ever runs against this repository's
// own Modules would pass forever after someone deleted its rules, so every
// rejection below has a matching acceptance: the fixture that must fail and
// the fixture that must pass differ in exactly the thing under test.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const run = (script, args) =>
  spawnSync("./scripts/run-js.sh", [`scripts/${script}`, ...args], {
    cwd: repository,
    encoding: "utf8",
  });

const SEALED_LIB = `pub mod entities;
pub mod migrations;

module_host::module_def! {
    name: "widgets",
    migrations: migrations::Migrator,
    entities: entities::register_entity_modules,
}
`;

const SEALED_CARGO = `[package]
name = "widgets-module"
version = "0.1.0"
edition = "2021"

[dependencies]
module-host = { path = "../../../crates/module-host" }
sea-orm = { workspace = true }
sea-orm-migration = { workspace = true }
seaography = { workspace = true }
tokio = { workspace = true }
`;

const SEALED_PACKAGE = JSON.stringify(
  {
    name: "@tauri-graphql-template/widgets-module",
    private: true,
    version: "0.1.0",
    type: "module",
    exports: "./src/index.ts",
    peerDependencies: { "@apollo/client": "4.2.10", graphql: "16.14.2", react: "19.2.8" },
  },
  null,
  2,
);

const SEALED_INDEX = `import { type TypePolicies } from "@apollo/client";
import { type ComponentType } from "react";

import { WidgetsPanel } from "./WidgetsPanel.tsx";

export const moduleDef: { name: string; typePolicies: TypePolicies; Component: ComponentType } = {
  name: "widgets",
  typePolicies: {},
  Component: WidgetsPanel,
};
`;

/**
 * A Module tree that passes both seal checks, with `overrides` replacing named
 * files. Every rejection test overrides exactly one.
 */
async function fixture(overrides = {}) {
  const root = await mkdtemp(path.join(tmpdir(), "module-seal-"));
  const files = {
    "widgets/rust/Cargo.toml": SEALED_CARGO,
    "widgets/rust/src/lib.rs": SEALED_LIB,
    "widgets/rust/src/migrations/mod.rs":
      "use module_host::migration::*;\n\npub struct Migrator;\n",
    // Generated, and excluded from the scan: it names the crates the seal
    // forbids by hand, which is the whole reason the seal is a check.
    "widgets/rust/src/entities/mod.rs": "seaography::register_entity_modules!([widgets,]);\n",
    "widgets/ui/package.json": SEALED_PACKAGE,
    "widgets/ui/src/index.ts": SEALED_INDEX,
    "widgets/ui/src/WidgetsPanel.tsx": 'import { useQuery } from "@apollo/client/react";\n',
    ...overrides,
  };
  for (const [relative, contents] of Object.entries(files)) {
    const file = path.join(root, relative);
    await mkdir(path.dirname(file), { recursive: true });
    await writeFile(file, contents);
  }
  return root;
}

async function withFixture(overrides, assertions) {
  const root = await fixture(overrides);
  try {
    await assertions(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("the import seal accepts a Module that names only crate and module_host", async () => {
  await withFixture({}, (root) => {
    const result = run("check-module-imports.mjs", ["--modules", root]);
    assert.equal(result.status, 0, result.stderr);
  });
});

test("the import seal rejects hand-authored Rust that reaches below the seam", async () => {
  for (const [what, source] of [
    ["a use of the builder", "use seaography::Builder;\n"],
    ["a bare path to the builder", "pub fn r(b: seaography::Builder) -> seaography::Builder { b }\n"],
    ["a direct SeaORM import", "use sea_orm::DatabaseConnection;\n"],
    ["the seam's macro-only module", "use module_host::__private::module_ctx;\n"],
    ["an extern crate", "extern crate tokio;\n"],
  ]) {
    await withFixture({ "widgets/rust/src/save.rs": source }, (root) => {
      const result = run("check-module-imports.mjs", ["--modules", root]);
      assert.notEqual(result.status, 0, `the seal accepted ${what}`);
    });
  }
});

test("the import seal ignores the generated entity directory", async () => {
  // `entities/mod.rs` in every fixture names `seaography` outright. If the
  // scan covered it, no Module could ever pass.
  await withFixture({}, (root) => {
    assert.equal(run("check-module-imports.mjs", ["--modules", root]).status, 0);
  });
});

test("the import seal rejects a ui half reaching outside its sanctioned packages", async () => {
  for (const [what, source] of [
    ["a Tauri API import", 'import { invoke } from "@tauri-apps/api/core";\n'],
    [
      "another Module's package",
      'import { moduleDef } from "@tauri-graphql-template/projects-module";\n',
    ],
    ["a path out of the Module", 'import { apolloClient } from "../../../../src/graphql/client.ts";\n'],
  ]) {
    await withFixture({ "widgets/ui/src/WidgetsPanel.tsx": source }, (root) => {
      const result = run("check-module-imports.mjs", ["--modules", root]);
      assert.notEqual(result.status, 0, `the seal accepted ${what}`);
    });
  }
});

test("the dependency allowlist accepts the sanctioned manifests", async () => {
  await withFixture({}, (root) => {
    const result = run("check-module-dependencies.mjs", ["--modules", root]);
    assert.equal(result.status, 0, result.stderr);
  });
});

test("the dependency allowlist rejects an unsanctioned crate", async () => {
  await withFixture(
    { "widgets/rust/Cargo.toml": `${SEALED_CARGO}reqwest = "0.13"\n` },
    (root) => {
      const result = run("check-module-dependencies.mjs", ["--modules", root]);
      assert.notEqual(result.status, 0, "the allowlist accepted an unsanctioned crate");
      assert.match(result.stderr, /reqwest/);
    },
  );
});

test("the dependency allowlist rejects an unsanctioned npm package", async () => {
  const manifest = JSON.parse(SEALED_PACKAGE);
  manifest.peerDependencies["date-fns"] = "4.0.0";
  await withFixture(
    { "widgets/ui/package.json": JSON.stringify(manifest, null, 2) },
    (root) => {
      const result = run("check-module-dependencies.mjs", ["--modules", root]);
      assert.notEqual(result.status, 0, "the allowlist accepted an unsanctioned package");
      assert.match(result.stderr, /date-fns/);
    },
  );
});

test("the dependency allowlist rejects a ui half that pins its own copy of React", async () => {
  const manifest = JSON.parse(SEALED_PACKAGE);
  manifest.dependencies = { react: "19.2.8" };
  await withFixture(
    { "widgets/ui/package.json": JSON.stringify(manifest, null, 2) },
    (root) => {
      const result = run("check-module-dependencies.mjs", ["--modules", root]);
      assert.notEqual(result.status, 0, "the allowlist accepted a second copy of React");
    },
  );
});

const EXCEPTION = `# widgets: widgetsReconcile

## Which primitive was insufficient

A custom mutation.

## Why

The write's shape is not known until the reads are done.

## The smallest seam taken

One declared escape hatch over this Module's own Models.

## Its drift-prevention test

\`widgets_reconcile_stays_inside_one_transaction\`.
`;

async function guard(declared, exceptions) {
  const root = await mkdtemp(path.join(tmpdir(), "escape-hatch-"));
  const declarations = path.join(root, "declared.json");
  await writeFile(declarations, JSON.stringify(declared));
  const records = path.join(root, "exceptions");
  await mkdir(records, { recursive: true });
  for (const [name, contents] of Object.entries(exceptions)) {
    await writeFile(path.join(records, name), contents);
  }
  const result = run("check-escape-hatches.mjs", [
    "--declarations",
    declarations,
    "--exceptions",
    records,
  ]);
  await rm(root, { recursive: true, force: true });
  return result;
}

test("the valve guard passes when no Module declares a hatch", async () => {
  const result = await guard({ widgets: [], gadgets: [] }, {});
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /no Module declares an escape hatch/);
});

test("the valve guard fails a declared hatch with no written exception", async () => {
  const result = await guard({ widgets: ["widgetsReconcile"] }, {});
  assert.notEqual(result.status, 0, "the guard accepted an undocumented escape hatch");
  assert.match(result.stderr, /no written exception/);
});

test("the valve guard fails an exception missing the discipline's sections", async () => {
  const result = await guard(
    { widgets: ["widgetsReconcile"] },
    { "widgets--widgetsReconcile.md": "# widgets: widgetsReconcile\n\nBecause.\n" },
  );
  assert.notEqual(result.status, 0, "the guard accepted an exception that explains nothing");
  assert.match(result.stderr, /Which primitive was insufficient/);
});

test("the valve guard passes a declared hatch with a complete exception", async () => {
  const result = await guard(
    { widgets: ["widgetsReconcile"] },
    { "widgets--widgetsReconcile.md": EXCEPTION },
  );
  assert.equal(result.status, 0, result.stderr);
});

test("the valve guard fails a Module that declares more hatches than the cap", async () => {
  const hatches = ["widgetsOne", "widgetsTwo", "widgetsThree"];
  const exceptions = Object.fromEntries(
    hatches.map((hatch) => [`widgets--${hatch}.md`, EXCEPTION]),
  );
  const result = await guard({ widgets: hatches }, exceptions);
  assert.notEqual(result.status, 0, "the guard let the hatch become the normal way to write");
  assert.match(result.stderr, /above the cap/);
});
