// The CSP that lets frames run WebAssembly is tauri-overlay's job (tested in
// the crate); this only proves the demo module loads and behaves.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("psych.wasm exports warp(x) = 2x", async () => {
  const bytes = await readFile(new URL("../../public/pets/psych.wasm", import.meta.url));
  const { instance } = await WebAssembly.instantiate(bytes);
  const warp = instance.exports.warp as (x: number) => number;
  assert.equal(warp(1.5), 3);
});
