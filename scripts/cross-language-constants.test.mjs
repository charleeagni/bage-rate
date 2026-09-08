// A handful of constants are necessarily declared twice, once in TypeScript and
// once in Rust, because no build step and no crate is shared across that seam.
// Prose comments name each pairing, but prose does not fail a build: the port
// could move in Rust and every existing test would still pass, because the Rust
// tests bind ephemeral ports and nothing else reads the TypeScript constant.
// These tests are the enforcement those pairings otherwise lack.

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const read = (relativePath) => readFile(path.join(repository, relativePath), "utf8");

/**
 * The single match of `pattern` in `source`, or a failure naming what went
 * missing. A silently unmatched regular expression would make every assertion
 * below vacuous, which is the failure mode these tests exist to prevent.
 */
const captureOnly = (source, pattern, what) => {
  const matches = [...source.matchAll(pattern)];
  assert.equal(matches.length, 1, `expected exactly one declaration of ${what}, found ${matches.length}`);
  return matches[0];
};

/** A Rust integer literal expression built only from products of decimals. */
const rustIntegerProduct = (expression, what) => {
  const factors = expression.replaceAll("_", "").split("*").map((factor) => factor.trim());
  return factors.reduce((product, factor) => {
    assert.match(factor, /^\d+$/, `${what} is no longer a plain product of integers: ${expression}`);
    return product * Number(factor);
  }, 1);
};

test("the development proxy origin names the port the Server Process binds by default", async () => {
  const [, origin] = captureOnly(
    await read("src/graphql/transport/webEndpoints.ts"),
    /DEVELOPMENT_SERVER_PROCESS_ORIGIN\s*=\s*"([^"]+)"/g,
    "DEVELOPMENT_SERVER_PROCESS_ORIGIN",
  );
  const [, host, port] = captureOnly(
    await read("crates/web-server/src/config.rs"),
    /DEFAULT_BIND[^=]*=\s*SocketAddr::new\(\s*std::net::IpAddr::V4\(Ipv4Addr::(\w+)\)\s*,\s*(\d+)\s*\)/g,
    "DEFAULT_BIND",
  );

  const proxied = new URL(origin);
  assert.equal(
    proxied.port,
    port,
    "the bundler's development proxy would forward to a port the Server Process does not bind",
  );
  assert.equal(host, "LOCALHOST", "DEFAULT_BIND no longer binds loopback");
  assert.equal(proxied.hostname, "127.0.0.1", "the development proxy no longer targets loopback");
  assert.equal(proxied.protocol, "http:");
  // Nothing but the origin may live in the constant: a path would be silently
  // dropped by the proxy targets in vite.config.ts.
  assert.equal(proxied.pathname, "/");
});

test("the Server Process serves the directory the bundler writes the Web Target to", async () => {
  const [, webOutputDirectory, desktopOutputDirectory] = captureOnly(
    await read("vite.config.ts"),
    /outDir:\s*target === "web" \? "([^"]+)" : "([^"]+)"/g,
    "the bundler's per-Target output directory",
  );
  const [, webRoot] = captureOnly(
    await read("crates/web-server/src/config.rs"),
    /pub const DEFAULT_WEB_ROOT:\s*&str\s*=\s*"([^"]+)"/g,
    "DEFAULT_WEB_ROOT",
  );
  const [, checkedWebBundle] = captureOnly(
    await read("scripts/check-target-bundles.sh"),
    /^web_bundle="([^"]+)"$/gm,
    "web_bundle in scripts/check-target-bundles.sh",
  );

  assert.equal(
    webRoot,
    webOutputDirectory,
    "the binary's default --web-root names a directory the bundler no longer writes, so every route would 404",
  );
  assert.equal(
    checkedWebBundle,
    webOutputDirectory,
    "the bundle separation check inspects a directory the bundler no longer writes",
  );
  // The claim DEFAULT_WEB_ROOT's own comment makes: the Desktop Target's assets
  // are a different directory and are never served.
  assert.notEqual(webOutputDirectory, desktopOutputDirectory);
});

// `dev-web.sh` passes `--store` explicitly so the override can exist, which
// means its default is a second declaration of the binary's own. Were they to
// diverge, `dev:web` and a hand-run Server Process would open different Stores
// and development data would appear to vanish between them.
test("the development Store default names the file the Server Process opens by default", async () => {
  const [, developmentStore] = captureOnly(
    await read("scripts/dev-web.sh"),
    /^store="\$\{WEB_TARGET_DEV_STORE:-([^}]+)\}"$/gm,
    "the development Store default in scripts/dev-web.sh",
  );
  const [, defaultStore] = captureOnly(
    await read("crates/web-server/src/config.rs"),
    /pub const DEFAULT_STORE:\s*&str\s*=\s*"([^"]+)"/g,
    "DEFAULT_STORE",
  );
  assert.equal(
    developmentStore,
    defaultStore,
    "dev:web would open a different Store than the Server Process run by hand",
  );
});

test("both Transports enforce the same request ceiling", async () => {
  const declaration = /pub const MAX_REQUEST_BYTES:\s*usize\s*=\s*([^;]+);/g;
  const [, webExpression] = captureOnly(
    await read("crates/web-server/src/graphql_route.rs"),
    declaration,
    "MAX_REQUEST_BYTES in the web-server crate",
  );
  const [, desktopExpression] = captureOnly(
    await read("crates/tauri-graphql-transport/src/endpoint.rs"),
    new RegExp(declaration.source, "g"),
    "MAX_REQUEST_BYTES in the tauri-graphql-transport crate",
  );

  assert.equal(
    rustIntegerProduct(webExpression, "the Server Process request ceiling"),
    rustIntegerProduct(desktopExpression, "the Desktop Transport request ceiling"),
    "the two Transports would accept requests of different sizes",
  );
});
