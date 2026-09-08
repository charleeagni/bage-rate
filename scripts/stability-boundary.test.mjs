// The Stability Boundary is only worth what its checks are wired into. These
// tests assert that both Targets stay inside the canonical verification command
// and inside continuous integration, so a later edit cannot quietly drop a
// Target's proof and leave the boundary documents claiming it.

import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const read = (relativePath) => readFile(path.join(repository, relativePath), "utf8");

test("verify drives every proof both Targets depend on", async () => {
  const verify = await read("scripts/verify.sh");
  const required = [
    // every script in scripts/, including the ones only a human invokes
    "./scripts/check-script-syntax.sh",
    // the seal on what a Module may name and may depend on, and the guard on
    // the one primitive that admits work no primitive expresses
    "scripts/check-module-imports.mjs",
    "scripts/check-module-dependencies.mjs",
    "scripts/check-escape-hatches.mjs",
    // the Generated Contract
    "./scripts/check-generated-drift.sh",
    // TypeScript, including the template configuration tests
    "typecheck",
    "./scripts/test-typescript.sh",
    // both Targets' bundles, and their separation
    "./scripts/check-target-bundles.sh",
    "cargo fmt --all -- --check",
    "cargo clippy --locked --workspace --all-targets -- -D warnings",
    // the composed Server Process over real sockets, and the Desktop Target
    "cargo test --locked --workspace",
    // the Server Process binary itself, and its freedom from Tauri
    "cargo build --locked --package web-server",
    "./scripts/check-server-process-dependencies.sh",
  ];
  for (const command of required) {
    assert.ok(verify.includes(command), `verify no longer runs ${command}`);
  }
});

// A development command that starts against stale generated Models, SDL,
// TauRPC bindings, or GraphQL documents runs a UI that no longer describes the
// Store it talks to. Both package managers honour `pre` hooks for arbitrary
// script names, so each Target's development command gets the same guard — and
// a Target added later cannot quietly arrive without one.
test("every development command is guarded by the Generated Contract drift check", async () => {
  const { scripts } = JSON.parse(await read("package.json"));
  const developmentCommands = Object.keys(scripts).filter(
    (name) => name === "dev" || name.startsWith("dev:"),
  );
  assert.deepEqual(
    developmentCommands.sort(),
    ["dev", "dev:web"],
    "a development command was added or renamed; give it a drift-check hook too",
  );
  for (const command of developmentCommands) {
    assert.equal(
      scripts[`pre${command}`],
      "./scripts/check-generated-drift.sh",
      `${command} can start against stale generated artifacts`,
    );
  }
});

test("the canonical TypeScript suite carries configuration and Transport tests", async () => {
  const suite = await read("scripts/test-typescript.sh");
  for (const testFile of [
    "scripts/configure-template.test.mjs",
    "scripts/cross-language-constants.test.mjs",
    "scripts/module-ownership.test.mjs",
    "scripts/module-seal.test.mjs",
    "scripts/stability-boundary.test.mjs",
    "scripts/web-target-commands.test.mjs",
    "modules/documents/ui/src/cache.test.ts",
    "modules/projects/ui/src/cache.test.ts",
    "src/graphql/transport/targetSelection.test.ts",
  ]) {
    assert.ok(suite.includes(testFile), `${testFile} left the verification path`);
  }
});

test("the bundle check proves separation in both directions", async () => {
  const check = await read("scripts/check-target-bundles.sh");
  // Desktop IPC, the TauRPC Transport, and the generated bindings must never
  // reach a browser; the network Transport must never reach the offline Target.
  for (const marker of [
    "__TAURI_INTERNALS__",
    "TauRPC",
    "graphql_execute",
    "graphql-transport-ws",
    "/graphql/ws",
  ]) {
    assert.ok(check.includes(marker), `${marker} is no longer checked for`);
  }
  // Each marker is also asserted present in the bundle that must carry it,
  // without which a renamed marker would make the check vacuously pass.
  assert.match(check, /proves nothing/);
});

test("the Server Process dependency check reads the graph, not the build", async () => {
  const check = await read("scripts/check-server-process-dependencies.sh");
  // Both dependency kinds a deployed binary carries, on every platform: a
  // host-only query would miss a webview stack that only resolves on Linux.
  assert.match(check, /--edges normal,build/);
  assert.match(check, /--target all/);
  // The Tauri crates and each platform's webview toolchain.
  for (const packageName of [
    "tauri",
    "wry",
    "webkit2gtk",
    "javascriptcore",
    "gtk",
    "webview2-com",
    "objc2-web-kit",
  ]) {
    assert.ok(check.includes(packageName), `${packageName} is no longer checked for`);
  }
  // As with the bundle check, the Desktop Target must still match the pattern,
  // without which renaming a crate would make this pass vacuously.
  assert.match(check, /proves nothing/);
});

test("continuous integration verifies under both supported package managers", async () => {
  const ci = await read(".github/workflows/ci.yml");
  assert.ok(ci.includes("npm run verify"), "npm no longer runs verify");
  assert.ok(ci.includes("bun run verify"), "Bun no longer runs verify");
  // The Web Target checks on the fast frontend path, the Desktop Target build,
  // and multi-platform Rust coverage all survive alongside them.
  assert.ok(ci.includes("npm run check:bundles"));
  assert.ok(ci.includes("npm run build:app:check"));
  assert.ok(ci.includes("cargo +1.95.0 test --locked --workspace"));
  assert.match(ci, /macos-15/);
  assert.match(ci, /windows-2025/);
});

test("one root lockfile keeps the dependency audit covering the whole workspace", async () => {
  const ignored = new Set(["node_modules", "target", ".git", "dist", "dist-web"]);
  const lockfiles = [];
  const walk = async (directory) => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      if (ignored.has(entry.name)) continue;
      const child = path.join(directory, entry.name);
      if (entry.isDirectory()) await walk(child);
      else if (entry.name === "Cargo.lock") lockfiles.push(path.relative(repository, child));
    }
  };
  await walk(repository);
  assert.deepEqual(
    lockfiles,
    ["Cargo.lock"],
    "a lockfile outside the workspace root would escape the Rust dependency audit",
  );

  const workspace = await read("Cargo.toml");
  for (const member of [
    "crates/app-schema",
    "crates/module-host",
    "crates/module-host-macros",
    "crates/seaolim",
    "crates/tauri-graphql-transport",
    "crates/web-server",
    "src-tauri",
  ]) {
    assert.match(workspace, new RegExp(`"${member}"`), `${member} left the audited workspace`);
  }
});

// A claim about the template's own working tree, not about a configured copy:
// configure never rewrites these documents, so a copy inherits whatever the
// template defines here.
test("the template defines the vocabulary and commands its documents use", async () => {
  const glossary = await read("CONTEXT.md");
  for (const term of ["Target", "Desktop Target", "Web Target", "Server Process"]) {
    assert.match(glossary, new RegExp(`\\*\\*${term}\\*\\*`), `${term} is undefined`);
  }
  const readme = await read("docs/tauri-graphql-template.md");
  for (const script of ["dev:web", "build:ui-assets", "build:web"]) {
    assert.match(readme, new RegExp(`\`${script}\``), `${script} is undocumented`);
  }
});

test("the boundary documents describe the Web Target they now include", async () => {
  const boundary = await read("docs/stability-boundary.md");
  const [included, unproven] = boundary.split("## Extension points");
  assert.match(included, /graphql-ws lifecycle/);
  assert.match(included, /loopback/);
  assert.match(included, /deep link/i);
  // The Tauri-free Server Process may only sit on the checked side while
  // check-server-process-dependencies.sh runs; the two move together.
  assert.match(included, /linking no Tauri code/);
  // Authorization stays on the unproven side, and says why it is not merely
  // unaudited: generated CRUD filters are unscoped.
  assert.match(unproven, /authorization/i);
  assert.match(unproven, /unscoped/);
  // The Web Target's response-header posture is a decision rather than a
  // silence: what the composed process sends is on the checked side, and the
  // deployment headers it does not send are on the unproven one.
  assert.match(included, /Content Security Policy/);
  assert.match(included, /X-Content-Type-Options/);
  assert.match(unproven, /Strict-Transport-Security/);

  // The authoring primitives sit on the checked side only while their proofs
  // run, and the two things the substrate cannot reach stay on the other.
  assert.match(included, /authoring primitives/);
  assert.match(included, /escape-hatch guard/);
  assert.match(unproven, /per-field hiding/);
  assert.match(unproven, /cross-module transactions/i);

  const architecture = await read("docs/architecture.md");
  assert.match(architecture, /Server Process/);
  assert.match(architecture, /CORS/);
  assert.match(architecture, /unscoped/);
  // The claim this ticket makes false must be gone.
  assert.doesNotMatch(architecture, /No HTTP server, open port/);

  // A reader must find the separate Stores and the deliberate absence of any
  // path between them where they first meet the two Targets.
  const readme = await read("docs/tauri-graphql-template.md");
  assert.match(readme, /no\s+sync, export, import, or migration path/);
  assert.match(readme, /unauthenticated/);
});
