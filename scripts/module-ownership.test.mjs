// Ownership is enforced at generate time (ADR-0006): each Module's documents
// are generated against a mini-schema holding only its own registrations, so
// naming another Module's types fails generation rather than review. These
// tests pin the wiring that delivers that enforcement — the env seams in
// codegen.ts, the per-module steps in generate.sh, the drift groups that keep
// the generated output honest — and prove behaviourally that the codegen
// toolchain rejects an operation naming a type its schema does not define.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
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

test("codegen honours the schema, documents, and output environment seams", async () => {
  const codegen = await read("codegen.ts");
  const [, documentsDefault] = captureOnly(
    codegen,
    /process\.env\.CODEGEN_DOCUMENTS \?\? "([^"]+)"/g,
    "CODEGEN_DOCUMENTS default in codegen.ts",
  );
  assert.equal(documentsDefault, "./src/**/*.graphql");
  captureOnly(codegen, /process\.env\.CODEGEN_SCHEMA/g, "CODEGEN_SCHEMA in codegen.ts");
  captureOnly(codegen, /process\.env\.CODEGEN_OUTPUT/g, "CODEGEN_OUTPUT in codegen.ts");
});

test("generate runs each Module's codegen against its own mini-schema", async () => {
  const generate = await read("scripts/generate.sh");
  // The mini-schema export, per discovered Module.
  captureOnly(
    generate,
    /--bin export_schema -- \\\n\s+--module "\$\{module_name\}"/g,
    "per-module export_schema --module step in generate.sh",
  );
  // The codegen run wired to that mini-schema and the Module's own documents.
  const [perModuleCodegen] = captureOnly(
    generate,
    /CODEGEN_SCHEMA="\$\{scratch_directory\}\/\$\{module_name\}\.graphql" \\\n\s+CODEGEN_DOCUMENTS="\.\/\$\{module_directory\}\/operations\/\*\*\/\*\.graphql" \\\n\s+CODEGEN_OUTPUT="\.\/\$\{module_directory\}\/src\/generated\/graphql\.ts" \\/g,
    "per-module codegen invocation in generate.sh",
  );
  assert.ok(perModuleCodegen.includes("CODEGEN_SCHEMA"));
});

test("the drift check covers every generated composition artifact", async () => {
  const drift = await read("scripts/check-generated-drift.sh");
  captureOnly(drift, /==> module registry drift/g, "registry drift group");
  captureOnly(drift, /==> entity drift/g, "per-module entity drift group");
  captureOnly(drift, /==> module index drift/g, "module index drift group");
  captureOnly(
    drift,
    /==> per-module GraphQL document drift/g,
    "per-module document drift group",
  );
  // Per-module document drift must regenerate from a fresh mini-schema, not
  // from the composed SDL, or a cross-module reference would slip through.
  captureOnly(
    drift,
    /--module "\$\{module_name\}" "\$\{scratch_directory\}\/\$\{module_name\}-mini\.graphql"/g,
    "fresh mini-schema export in the drift check",
  );
  captureOnly(
    drift,
    /diff -u "\$\{module_directory\}\/src\/generated\/graphql\.ts"/g,
    "per-module generated document diff in the drift check",
  );
  captureOnly(
    drift,
    /diff -u src\/generated\/modules\.ts/g,
    "module index diff in the drift check",
  );
});

// The enforcement mechanism itself: graphql-codegen fails when an operation
// names a field its schema does not define. A Module's mini-schema holds only
// that Module's registrations, so this exit code is what makes cross-module
// references fail generation.
test("codegen rejects an operation naming a type outside its schema", async () => {
  const scratch = await mkdtemp(path.join(tmpdir(), "module-ownership-"));
  const runCodegen = (documentsDirectory) =>
    spawnSync(
      "./scripts/run-js.sh",
      ["node_modules/@graphql-codegen/cli/esm/bin.js", "--config", "codegen.ts"],
      {
        cwd: repository,
        encoding: "utf8",
        env: {
          ...process.env,
          CODEGEN_SCHEMA: "./schema.graphql",
          CODEGEN_DOCUMENTS: path.join(documentsDirectory, "*.graphql"),
          CODEGEN_OUTPUT: path.join(documentsDirectory, "graphql.ts"),
        },
      },
    );
  try {
    // The control: an in-schema operation under the identical invocation must
    // generate, without which the rejection below could be blamed on the
    // setup rather than on the out-of-schema field.
    const inSchema = path.join(scratch, "in-schema");
    await mkdir(inSchema);
    await writeFile(
      path.join(inSchema, "valid.graphql"),
      "query InSchema {\n  projects {\n    nodes {\n      id\n    }\n  }\n}\n",
    );
    assert.equal(runCodegen(inSchema).status, 0, "the control operation failed to generate");

    const trespassing = path.join(scratch, "trespassing");
    await mkdir(trespassing);
    await writeFile(
      path.join(trespassing, "trespass.graphql"),
      "query Trespass {\n  someOtherModulesRootField {\n    id\n  }\n}\n",
    );
    assert.notEqual(
      runCodegen(trespassing).status,
      0,
      "codegen accepted an operation whose root field the schema does not define",
    );
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
});
