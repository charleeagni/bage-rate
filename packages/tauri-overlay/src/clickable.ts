import { cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";

// The overlay window ignores the cursor, so the page never sees mouse events.
// Instead we poll the OS cursor, hit-test the DOM ourselves (reaching into the
// same-origin frames), and stop ignoring the cursor only while it is over
// an element marked `.clickable`. Everything else stays click-through.
export const CLICKABLE_CLASS = "clickable";

// ponytail: IPC poll at 30Hz; move hit-testing to Rust if it ever shows on a profiler.
const POLL_MS = 33;

export function isOverClickable(x: number, y: number, doc: Document = document): boolean {
  const hit = doc.elementFromPoint(x, y);
  if (!hit) return false;
  if (hit.closest(`.${CLICKABLE_CLASS}`)) return true;
  if (hit instanceof HTMLIFrameElement && hit.contentDocument) {
    const box = hit.getBoundingClientRect();
    return isOverClickable(x - box.left, y - box.top, hit.contentDocument);
  }
  return false;
}

// Returns a stop function.
export function watchClickable(): () => void {
  const win = getCurrentWindow();
  let ignoring = true;
  const tick = async () => {
    const [cursor, origin, scale] = await Promise.all([
      cursorPosition(),
      win.outerPosition(),
      win.scaleFactor(),
    ]);
    const x = (cursor.x - origin.x) / scale;
    const y = (cursor.y - origin.y) / scale;
    const ignore = !isOverClickable(x, y);
    if (ignore !== ignoring) {
      ignoring = ignore;
      await win.setIgnoreCursorEvents(ignore);
    }
  };
  const id = setInterval(() => void tick().catch(console.error), POLL_MS);
  return () => clearInterval(id);
}
