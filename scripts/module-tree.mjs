// Where the Module checks agree about what a Module is.
//
// Each check takes `--modules <directory>` so its own tests can point it at a
// fixture tree instead of at this repository's `modules/`. Everything else
// about a Module's shape — which files are hand-authored, which are generated,
// where each half's manifest lives — is decided once, here.

import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const repository = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

/** The modules directory a check was pointed at, defaulting to this repository's. */
export function modulesDirectory(argv = process.argv.slice(2)) {
  const flag = argv.indexOf("--modules");
  return flag === -1
    ? path.join(repository, "modules")
    : path.resolve(argv[flag + 1]);
}

/** Every Module folder under `directory`, sorted by name. */
export async function modules(directory) {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch {
    return [];
  }
  return entries
    .filter((entry) => entry.isDirectory())
    .map((entry) => ({ name: entry.name, root: path.join(directory, entry.name) }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

/**
 * Every file under `root` matching `extensions`, skipping `generated/` and
 * `entities/` — a Module does not write those, so holding them to the seal
 * would fail on the codegen's own output.
 */
export async function handAuthoredFiles(root, extensions) {
  const found = [];
  const walk = async (directory) => {
    let entries;
    try {
      entries = await readdir(directory, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const child = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === "generated" || entry.name === "entities") continue;
        if (entry.name === "node_modules" || entry.name === "target") continue;
        await walk(child);
      } else if (extensions.some((extension) => entry.name.endsWith(extension))) {
        found.push(child);
      }
    }
  };
  await walk(root);
  return found.sort();
}

export async function exists(target) {
  try {
    await stat(target);
    return true;
  } catch {
    return false;
  }
}

/** Print each failure and exit non-zero, or print `success` and exit zero. */
export function report(failures, success) {
  if (failures.length > 0) {
    for (const failure of failures) console.error(`ERROR: ${failure}`);
    console.error(`${failures.length} violation(s)`);
    process.exit(1);
  }
  console.log(success);
}
