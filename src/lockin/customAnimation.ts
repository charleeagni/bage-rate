import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Frame } from "tauri-overlay";
import { babyPetSrc } from "./babyPet.ts";

export type CustomAnimation = { name: string; html: string } | null;
export type Preferences = { animation: CustomAnimation; shortcut: string; shortcutError: string | null };
export const loadPreferences = () => invoke<Preferences>("lockin_preferences");
export const setAnimation = (animation: CustomAnimation) => invoke<Preferences>("lockin_animation", { animation });
export const setShortcut = (shortcut: string) => invoke<Preferences>("lockin_shortcut", { shortcut });
export const onPreferences = (handler: (value: Preferences) => void) =>
  listen<Preferences>("lockin:preferences", e => handler(e.payload));

// Move the previous Settings value once, after Rust has accepted and saved it.
export async function migrateAnimation(): Promise<Preferences> {
  const preferences = await loadPreferences();
  const raw = localStorage.getItem("cute.lockin.animation");
  if (!raw) return preferences;
  let next = preferences;
  if (!preferences.animation) {
    const legacy: unknown = JSON.parse(raw);
    if (legacy && typeof legacy === "object" && "name" in legacy && "html" in legacy &&
        typeof legacy.name === "string" && typeof legacy.html === "string") {
      next = await setAnimation({ name: legacy.name, html: legacy.html });
    }
  }
  localStorage.removeItem("cute.lockin.animation");
  return next;
}

export function distractionFrame(index: number, custom: CustomAnimation): Frame {
  if (custom) return { key: `distraction-${index}`, src: `${convertFileSrc("current", "animation")}?play=${index}`, sandbox: "allow-scripts" };
  return { key: `distraction-${index}`, src: babyPetSrc(index) };
}
