import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

import { BABY_PET_DOCUMENT, babyPetSrc, babyTaunt } from "./babyPet.ts";

const params = (index: number) => new URLSearchParams(babyPetSrc(index).split("?")[1]);

test("a spawn points at the overlay document and plays once", () => {
  assert.ok(babyPetSrc(0).startsWith(`${BABY_PET_DOCUMENT}?`));
  assert.equal(params(0).get("loop"), null);
});

test("each distraction gets its own splat", () => {
  assert.notEqual(params(1).get("seed"), params(0).get("seed"));
});

test("the taunt names the distracting site, without the www", () => {
  assert.equal(babyTaunt("https://www.youtube.com/watch?v=1"), "youtube.com again?");
});

test("no url, or one that will not parse, still taunts", () => {
  assert.equal(babyTaunt(null), "back to work.");
  assert.equal(babyTaunt("not a url"), "back to work.");
});

test("only painted baby pixels refocus the locked app", async () => {
  const [baby, overlay] = await Promise.all([
    readFile(new URL("../../public/pets/baby-overlay.html", import.meta.url), "utf8"),
    readFile(new URL("../pets/PetsOverlay.tsx", import.meta.url), "utf8"),
  ]);
  assert.match(baby, /#babyRoot\{pointer-events:visiblePainted;cursor:pointer;\}/);
  assert.match(baby, /<g id="babyRoot" class="clickable">/);
  assert.match(baby, /babyRoot\.addEventListener\('click'.*emit\('click'/);
  assert.doesNotMatch(overlay, /lockin-catch/);
});
