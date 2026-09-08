// "Generate an animation" deep links: open ChatGPT or Claude in the default
// browser with the contract pre-filled, so the reply can be pasted straight
// into Settings.
import { emit } from "@tauri-apps/api/event";

export const PET_PROMPT = `You are helping me make a "get back to work" animation for an app called bage-rate. When I drift away from the app I locked in on, bage-rate plays a single HTML file full-screen on a transparent, always-on-top overlay. When the animation finishes, bage-rate pulls me back to my work app.

First, before writing any code, ask me what I want the animation to be: what character or scene, its look and colours, what it does to nag me back to work, how long it should run (10-30 seconds is ideal) and any personality. Ask in one short message, then wait for my answer.

Once I have answered, give me the complete HTML in one code block so I can copy it and paste it straight into bage-rate's Settings window. If you can also offer it as a downloadable file, do that too. Finish by telling me to copy the code and paste it into bage-rate's Settings.

Rules for the file:
- One .html file only: inline CSS and JS, no external files, fonts, images or libraries. Inline SVG is fine.
- html and body: margin 0, height 100%, background transparent, overflow hidden. Nothing may paint a background over the whole screen; a small darkened vignette or speech bubble is fine.
- Start playing immediately on load and play once. Animate with CSS keyframes and/or JS, using transforms and opacity only.
- When the animation is over, call exactly: window.parent.postMessage({ source: "babyOverlay", event: "finish" }, "*"). bage-rate uses this to refocus my work app. Then fade everything out.
- The overlay is click-through by default; only elements with the class "clickable" receive the mouse. You do not need any.
- No sound, no network, no alerts, no console spam.`;

const encode = (prompt: string) => encodeURIComponent(prompt);

export const chatGptUrl = (prompt = PET_PROMPT) => `https://chatgpt.com/?q=${encode(prompt)}`;
export const claudeUrl = (prompt = PET_PROMPT) => `https://claude.ai/new?q=${encode(prompt)}`;

export function openInBrowser(url: string): Promise<void> {
  return emit("open-url", url);
}
