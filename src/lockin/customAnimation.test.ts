import assert from "node:assert/strict";
import { test } from "node:test";
import { mockConvertFileSrc } from "@tauri-apps/api/mocks";
import { BABY_PET_DOCUMENT } from "./babyPet.ts";
import { distractionFrame } from "./customAnimation.ts";
import { shortcutFromKey } from "./shortcut.ts";

test("without a custom animation, the original baby plays", () => {
  assert.equal(BABY_PET_DOCUMENT, "/pets/baby-overlay.html");
  const frame = distractionFrame(0, null);
  assert.ok(frame.src?.startsWith(BABY_PET_DOCUMENT));
  assert.equal(frame.srcDoc, undefined);
});

test("custom animations use the isolated protocol on macOS and Windows", () => {
  const previous = globalThis.window;
  Object.assign(globalThis, { window: {} });
  try {
    for (const platform of ["macos", "windows"] as const) {
      mockConvertFileSrc(platform);
      const frame = distractionFrame(2, { name: "me.html", html: "<script>run()</script>" });
      assert.equal(frame.src, platform === "macos" ? "animation://localhost/current?play=2" : "http://animation.localhost/current?play=2");
      assert.equal(frame.srcDoc, undefined);
      assert.equal(frame.sandbox, "allow-scripts");
    }
  } finally { Object.assign(globalThis, { window: previous }); }
});

test("shortcut recorder accepts modifiers with physical keys and rejects incomplete chords", () => {
  const base = { metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, code: "KeyK" };
  assert.equal(shortcutFromKey(base), null);
  assert.equal(shortcutFromKey({ ...base, metaKey: true, shiftKey: true }), "Super+Shift+K");
  assert.equal(shortcutFromKey({ ...base, ctrlKey: true, code: "Digit2" }), "Control+2");
  assert.equal(shortcutFromKey({ ...base, altKey: true, code: "ArrowUp" }), "Alt+Up");
  assert.equal(shortcutFromKey({ ...base, metaKey: true, code: "MetaLeft" }), null);
});
