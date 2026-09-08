import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  DEFAULT_TARGET,
  TARGETS,
  parseTarget,
  transportModuleForTarget,
} from "./targetSelection.ts";

const repositoryRoot = new URL("../../../", import.meta.url);

test("an unset build-time flag selects the Desktop Target", () => {
  assert.equal(parseTarget(undefined), "desktop");
  assert.equal(parseTarget(""), "desktop");
  assert.equal(DEFAULT_TARGET, "desktop");
});

test("each named Target selects itself", () => {
  for (const target of TARGETS) {
    assert.equal(parseTarget(target), target);
  }
});

test("an unrecognised Target is refused rather than defaulted", () => {
  assert.throws(() => parseTarget("browser"), /APP_TARGET/);
});

test("every Target maps to a Transport module that exists", () => {
  const modules = TARGETS.map((target) => transportModuleForTarget(target));

  for (const module of modules) {
    assert.ok(
      existsSync(fileURLToPath(new URL(module, repositoryRoot))),
      `${module} is missing`,
    );
  }

  assert.equal(new Set(modules).size, TARGETS.length, "two Targets share a Transport");
});
