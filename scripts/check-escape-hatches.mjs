// The valve guard: an escape hatch is admitted, counted, and made to justify
// itself.
//
// The primitives deliberately cannot express work that reads repeatedly,
// branches on what it read, and ends in a write whose shape follows from that.
// A Module admits such work through one declared registration rather than by
// inventing its own seam — and this is what stops the declaration from
// becoming the normal way to write a Module:
//
//   1. Every declared hatch needs a written exception, in the same shape
//      AGENTS.md demands for replacement CRUD.
//   2. A Module may declare at most CAP of them. A second Module reaching for
//      the same shape is the signal to design a primitive and delete both
//      exceptions, not to raise this number.

import { spawnSync } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

import { repository, report } from "./module-tree.mjs";

const CAP = 2;

const REQUIRED_SECTIONS = [
  "## Which primitive was insufficient",
  "## Why",
  "## The smallest seam taken",
  "## Its drift-prevention test",
];

const argument = (flag, fallback) => {
  const argv = process.argv.slice(2);
  const index = argv.indexOf(flag);
  return index === -1 ? fallback : argv[index + 1];
};

/** What each Module declared, read from the seam rather than from source text. */
async function declarations() {
  const file = argument("--declarations", null);
  if (file) return JSON.parse(await readFile(file, "utf8"));

  const exported = spawnSync(
    "cargo",
    ["run", "--locked", "--quiet", "--package", "app-schema", "--bin", "export_escape_hatches"],
    { cwd: repository, encoding: "utf8" },
  );
  if (exported.status !== 0) {
    console.error(exported.stderr);
    throw new Error("could not read the declared escape hatches");
  }
  return JSON.parse(exported.stdout);
}

const exceptionsDirectory = path.resolve(
  argument("--exceptions", path.join(repository, "docs", "exceptions")),
);

const records = new Set(
  await readdir(exceptionsDirectory).catch(() => []),
);

const declared = await declarations();
const failures = [];
let total = 0;

for (const [module, hatches] of Object.entries(declared)) {
  total += hatches.length;
  if (hatches.length > CAP) {
    failures.push(
      `module "${module}" declares ${hatches.length} escape hatches, above the cap of ${CAP}: ` +
        `${hatches.join(", ")}. Design a primitive instead of raising the cap.`,
    );
  }
  for (const hatch of hatches) {
    const record = `${module}--${hatch}.md`;
    if (!records.has(record)) {
      failures.push(
        `module "${module}" declares escape hatch "${hatch}" with no written exception at ` +
          `${path.relative(repository, path.join(exceptionsDirectory, record))}`,
      );
      continue;
    }
    const text = await readFile(path.join(exceptionsDirectory, record), "utf8");
    for (const section of REQUIRED_SECTIONS) {
      if (!text.includes(section)) {
        failures.push(`${record}: has no "${section}" section`);
      }
    }
  }
}

report(
  failures,
  total === 0
    ? "no Module declares an escape hatch"
    : `${total} declared escape hatch(es), each within the cap and backed by a written exception`,
);
