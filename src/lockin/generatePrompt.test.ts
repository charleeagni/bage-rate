import assert from "node:assert/strict";
import { test } from "node:test";

import { PET_PROMPT, chatGptUrl, claudeUrl } from "./generatePrompt.ts";

test("deep links carry the prompt in q, decodable and https", () => {
  for (const url of [chatGptUrl(), claudeUrl()]) {
    const parsed = new URL(url);
    assert.equal(parsed.protocol, "https:");
    assert.equal(parsed.searchParams.get("q"), PET_PROMPT);
  }
});
