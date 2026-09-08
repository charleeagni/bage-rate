// The Web Target ships with `build:web` and is developed with `dev:web`. Every
// other mechanism it carries — the composed Server Process over real sockets,
// bundle separation, the Tauri-free dependency graph — is proven by something
// that runs it. These two scripts are only partly reachable that way: CI runs
// `build:web` end to end, but `dev-web.sh` starts a session and waits, so no
// check can run it to completion. `check-script-syntax.sh` proves both parse;
// these tests assert the decisions their text encodes, which parsing cannot see.

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const read = (relativePath) => readFile(path.join(repository, relativePath), "utf8");

/**
 * The single match of `pattern` in `source`, or a failure naming what went
 * missing. An unmatched regular expression would make the assertions that read
 * its captures vacuous, which is the failure mode these tests exist to prevent.
 */
const captureOnly = (source, pattern, what) => {
  const matches = [...source.matchAll(pattern)];
  assert.equal(matches.length, 1, `expected exactly one ${what}, found ${matches.length}`);
  return matches[0];
};

/** The Web Target output directory the bundler is configured to write. */
const bundlerWebOutputDirectory = async () =>
  captureOnly(
    await read("vite.config.ts"),
    /outDir:\s*target === "web" \? "([^"]+)" : "[^"]+"/g,
    "per-Target bundler output directory",
  )[1];

test("build:web builds the Web Target's UI assets, not the Desktop Target's", async () => {
  const script = await read("scripts/build-web.sh");
  const [bundlerCommand] = captureOnly(
    script,
    /^(.*vite\.js build)$/gm,
    "bundler invocation in scripts/build-web.sh",
  );
  // Without this the command would build the Desktop Target's bundle and then
  // announce it as the Web Target's, which no other check would catch: the
  // bundle separation check builds both Targets itself.
  assert.match(
    bundlerCommand,
    /^APP_TARGET=web\s/,
    "build:web no longer selects the Web Target, so it would bundle the Desktop Transport",
  );
});

test("build:web builds the Server Process from the lockfile in the release profile", async () => {
  const [, flags] = captureOnly(
    await read("scripts/build-web.sh"),
    /^cargo build ([^\n]*)$/gm,
    "cargo build invocation in scripts/build-web.sh",
  );
  assert.match(flags, /--package web-server/, "build:web no longer builds the Server Process package");
  // A shipped binary: resolved from the lockfile, optimised, and — because
  // `verify` owns the debug build — the only release build in the template.
  assert.match(flags, /--locked/, "build:web could resolve dependencies outside the lockfile");
  assert.match(flags, /--release/, "build:web would ship an unoptimised Server Process");
});

test("the paths build:web announces are the ones it produced", async () => {
  const script = await read("scripts/build-web.sh");
  const [, announcedAssets] = captureOnly(
    script,
    /^web_assets="([^"]+)"$/gm,
    "web_assets assignment in scripts/build-web.sh",
  );
  const [, announcedBinary] = captureOnly(
    script,
    /^server_process="([^"]+)"$/gm,
    "server_process assignment in scripts/build-web.sh",
  );
  const [, packageName] = captureOnly(
    script,
    /--package (\S+)/g,
    "--package argument in scripts/build-web.sh",
  );

  assert.equal(
    announcedAssets,
    await bundlerWebOutputDirectory(),
    "build:web announces a directory the bundler does not write",
  );
  assert.equal(
    announcedBinary,
    `target/release/${packageName}`,
    "build:web announces a binary path that its own cargo invocation does not produce",
  );

  // Comparing two strings inside one script only proves they agree. These
  // checks are what make the announced paths a claim about the filesystem, so
  // a moved output directory or a renamed package fails the build rather than
  // printing directions to files that are not there.
  assert.match(script, /\$\{web_assets\}\/index\.html/);
  assert.match(script, /! -x "\$\{server_process\}"/);
  // Cargo appends `.exe` on Windows, where the unsuffixed path never exists.
  assert.match(script, /\$\{server_process\}\.exe/);
});

test("dev:web supervises the Server Process it spawns", async () => {
  const script = await read("scripts/dev-web.sh");

  // Compiling first is what makes the bundler come up beside a Server Process
  // that is already listening rather than one that is still building.
  const buildAt = script.indexOf("cargo build");
  const runAt = script.indexOf("cargo run");
  assert.ok(buildAt >= 0 && runAt > buildAt, "dev:web no longer compiles the Server Process before spawning it");

  // Every way this command can end, including the bundler exiting normally: an
  // orphaned Server Process would hold the port and the next `dev:web` would
  // proxy to the previous session's Store.
  const [, signals] = captureOnly(script, /^trap (\S+ .*)$/gm, "trap in scripts/dev-web.sh");
  for (const signal of ["EXIT", "INT", "TERM"]) {
    assert.match(signals, new RegExp(`\\b${signal}\\b`), `dev:web could leave a Server Process behind on ${signal}`);
  }
  assert.match(script, /kill "\$\{server_process\}"/, "the trap no longer stops the Server Process");
  assert.match(script, /wait "\$\{server_process\}"/, "the trap does not wait for the Server Process to exit");

  // A Server Process that died — a stale one still holding the port, say —
  // must fail this command rather than leave the bundler proxying to nothing.
  const probeAt = script.search(/kill -0 "\$\{server_process\}"/);
  const bundlerAt = script.search(/^APP_TARGET=web .*vite\.js$/m);
  assert.ok(probeAt > runAt, "dev:web no longer checks the Server Process is alive");
  assert.ok(bundlerAt > probeAt, "the liveness check no longer gates starting the bundler");
  assert.match(script, /exit 1/, "a dead Server Process would not fail dev:web");

  // The bundler serves the Web Target here too; with the Desktop Target's
  // Transport it would reach for IPC that a browser does not have.
  assert.ok(bundlerAt >= 0, "dev:web no longer starts the bundler for the Web Target");
});

test("dev:web takes its Store from an override with a default", async () => {
  const [, variable, fallback] = captureOnly(
    await read("scripts/dev-web.sh"),
    /^store="\$\{(\w+):-([^}]+)\}"$/gm,
    "store assignment in scripts/dev-web.sh",
  );
  assert.equal(variable, "WEB_TARGET_DEV_STORE");
  assert.ok(fallback.length > 0, "dev:web has no Store default, so it would pass an empty --store");
  // An override nobody can discover is not an interface. The pairing between
  // this default and the binary's own is asserted in
  // scripts/cross-language-constants.test.mjs.
  assert.match(
    await read("docs/tauri-graphql-template.md"),
    new RegExp(`\`${variable}\``),
    `${variable} is undocumented`,
  );
});

test("both Web Target commands are reachable as package scripts", async () => {
  const { scripts } = JSON.parse(await read("package.json"));
  assert.equal(scripts["build:web"], "./scripts/build-web.sh");
  assert.equal(scripts["dev:web"], "./scripts/dev-web.sh");
});

test("continuous integration ships the Web Target on a real package-manager path", async () => {
  const ci = await read(".github/workflows/ci.yml");
  assert.match(
    ci,
    /run: npm run build:web/,
    "nothing runs build-web.sh, so its own text is unproven again",
  );
});

// `build:app` gates the Desktop Target's bundle behind `verify`; `build:web`
// does not gate the Web Target's. That difference is deliberate — CI runs
// `build:web` in the job that has just run `verify` — but a reader comparing the
// two commands must find it stated rather than infer an oversight.
test("the ungated build:web is documented as a decision", async () => {
  const { scripts } = JSON.parse(await read("package.json"));
  assert.match(scripts["build:app"], /verify\.sh &&/, "build:app no longer gates the Desktop bundle");
  const readme = await read("docs/tauri-graphql-template.md");
  assert.match(readme, /`build:web` does not run `verify`/);
});
