import assert from "node:assert/strict";
import { test } from "node:test";

import { isOverClickable } from "../src/clickable.ts";

// Minimal stand-ins: the hit-test only uses elementFromPoint, closest, and,
// for frames, getBoundingClientRect + contentDocument.
const element = (classes: string[] = []) => ({
  closest: (selector: string) => (classes.includes(selector.slice(1)) ? {} : null),
});
const doc = (hit: unknown) => ({ elementFromPoint: () => hit }) as unknown as Document;

// clickable.ts is browser code; give it the one global it instanceof-checks.
class HTMLIFrameElement {}
Object.assign(globalThis, { HTMLIFrameElement });

test("a plain element is click-through", () => {
  assert.equal(isOverClickable(1, 1, doc(element())), false);
});

test("a .clickable element takes the cursor", () => {
  assert.equal(isOverClickable(1, 1, doc(element(["clickable"]))), true);
});

test("a dragging pet marks its whole document clickable, seen through its frame", () => {
  const frame = Object.assign(new HTMLIFrameElement(), {
    closest: () => null,
    getBoundingClientRect: () => ({ left: 0, top: 0 }),
    contentDocument: doc(element(["clickable"])), // <html class="clickable">
  });
  assert.equal(isOverClickable(5, 5, doc(frame)), true);
});
