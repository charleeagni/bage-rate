import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { constants } from "node:fs";
import { access, cp, mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const identityFiles = [
  "package.json",
  "package-lock.json",
  "bun.lock",
  "index.html",
  "src/app-config.ts",
  "src-tauri/tauri.conf.json",
];

const copyIntoFixture = async (fixture, relativePath) => {
  await mkdir(path.dirname(path.join(fixture, relativePath)), { recursive: true });
  await cp(path.join(repository, relativePath), path.join(fixture, relativePath));
};

const createFixture = async () => {
  const fixture = await mkdtemp(path.join(tmpdir(), "tauri-graphql-configure-"));
  for (const relativePath of identityFiles) {
    await copyIntoFixture(fixture, relativePath);
  }
  return fixture;
};

const runConfigure = (fixture, overrides = {}) =>
  execFileSync(
    process.execPath,
    [
      path.join(repository, "scripts/configure-template.mjs"),
      "--name",
      overrides.name ?? "example-desktop",
      "--title",
      overrides.title ?? "Example Desktop",
      "--identifier",
      overrides.identifier ?? "com.example.desktop",
    ],
    { cwd: fixture, stdio: "pipe" },
  );

test("configure updates every app identity surface", async () => {
  const fixture = await createFixture();
  runConfigure(fixture);

  const manifest = JSON.parse(await readFile(path.join(fixture, "package.json"), "utf8"));
  const packageLock = JSON.parse(
    await readFile(path.join(fixture, "package-lock.json"), "utf8"),
  );
  const bunLock = await readFile(path.join(fixture, "bun.lock"), "utf8");
  const config = JSON.parse(
    await readFile(path.join(fixture, "src-tauri/tauri.conf.json"), "utf8"),
  );
  assert.equal(manifest.name, "example-desktop");
  assert.equal(packageLock.name, "example-desktop");
  assert.equal(packageLock.packages[""].name, "example-desktop");
  assert.match(bunLock, /"name": "example-desktop"/);
  assert.equal(config.productName, "Example Desktop");
  assert.equal(config.identifier, "com.example.desktop");
  assert.match(await readFile(path.join(fixture, "index.html"), "utf8"), /Example Desktop/);
  assert.match(
    await readFile(path.join(fixture, "src/app-config.ts"), "utf8"),
    /Example Desktop/,
  );
});

// A configured copy is the product this template ships, so the commands, the
// crates, and the vocabulary a copy is meant to inherit are asserted on the
// output of `configure` rather than on the template's own working tree.
test("configure leaves every Target's commands intact", async () => {
  const fixture = await createFixture();
  runConfigure(fixture);

  const { scripts } = JSON.parse(
    await readFile(path.join(fixture, "package.json"), "utf8"),
  );
  assert.deepEqual(
    {
      dev: scripts.dev,
      "dev:web": scripts["dev:web"],
      "build:app": scripts["build:app"],
      "build:ui-assets": scripts["build:ui-assets"],
      "build:web": scripts["build:web"],
    },
    {
      dev: "tauri dev",
      "dev:web": "./scripts/dev-web.sh",
      "build:app": "./scripts/verify.sh && ./scripts/build-app.sh",
      "build:ui-assets": "vite build",
      "build:web": "./scripts/build-web.sh",
    },
  );
  // The freed name must mean the Web Target, not the shared asset bundle.
  assert.notEqual(scripts["build:web"], scripts["build:ui-assets"]);
});

test("both Targets' commands run under either package manager", async () => {
  const fixture = await createFixture();
  runConfigure(fixture);

  const { scripts } = JSON.parse(
    await readFile(path.join(fixture, "package.json"), "utf8"),
  );
  const shellScripts = [...new Set(Object.values(scripts))]
    .flatMap((command) => command.match(/\.\/scripts\/[\w-]+\.sh/g) ?? []);
  assert.ok(shellScripts.includes("./scripts/dev-web.sh"));
  assert.ok(shellScripts.includes("./scripts/build-web.sh"));
  for (const shellScript of shellScripts) {
    await access(path.join(repository, shellScript), constants.X_OK);
  }

  // Nothing a Target's command reaches for may hard-code one package manager.
  for (const helper of ["run-js.sh", "run-package-script.sh"]) {
    const source = await readFile(path.join(repository, "scripts", helper), "utf8");
    assert.match(source, /bun/);
    assert.match(source, /npm|node/);
  }
});

// configure renames the application, never the reusable crates, so a copy keeps
// building the Server Process the Web Target commands invoke. Which crates the
// workspace must list is the template's own claim, asserted in
// scripts/stability-boundary.test.mjs; what a copy must inherit is that
// configure left that list exactly as it found it.
test("configure leaves the Rust workspace of a copy byte-identical", async () => {
  const fixture = await createFixture();
  await copyIntoFixture(fixture, "Cargo.toml");
  runConfigure(fixture);

  assert.equal(
    await readFile(path.join(fixture, "Cargo.toml"), "utf8"),
    await readFile(path.join(repository, "Cargo.toml"), "utf8"),
    "configure rewrote the Rust workspace a copy builds",
  );
});

test("configure rejects unsafe identifiers", async () => {
  const fixture = await createFixture();
  assert.throws(() => runConfigure(fixture, { identifier: "not valid" }));
});

test("configure preflights every identity surface before writing", async () => {
  const fixture = await createFixture();
  const indexPath = path.join(fixture, "index.html");
  await writeFile(indexPath, "<!doctype html><html><head></head><body></body></html>\n");

  const before = new Map(
    await Promise.all(
      identityFiles.map(async (relativePath) => [
        relativePath,
        await readFile(path.join(fixture, relativePath), "utf8"),
      ]),
    ),
  );

  assert.throws(() => runConfigure(fixture));
  for (const relativePath of identityFiles) {
    assert.equal(
      await readFile(path.join(fixture, relativePath), "utf8"),
      before.get(relativePath),
      `${relativePath} changed despite a failed preflight`,
    );
  }
});
