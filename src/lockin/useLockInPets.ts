import { useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import type { Frame } from "tauri-overlay";
import { distractionFrame, loadPreferences, onPreferences, type CustomAnimation } from "./customAnimation.ts";
import { devLog, devLogError } from "./devLog.ts";
import { lockIn, type Unlock } from "./lockIn.ts";

const DWELL_MS = 5000;
export function useLockInPets(): { anchor: string | null; pets: Frame[]; refocus: () => void; error: string | null } {
  const [pets, setPets] = useState<Frame[]>([]);
  const [anchor, setAnchor] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const animation = useRef<CustomAnimation>(null);
  const anchorRef = useRef<string | null>(null);
  const refocus = () => {
    if (!anchorRef.current) return;
    void emit("lockin:refocus", { app: anchorRef.current }).catch(e => setError(String(e)));
  };
  useEffect(() => {
    let active = true;
    let unlock: Unlock | null = null;
    let distractions = 0;
    let queue = Promise.resolve();
    let startup: ReturnType<typeof setTimeout> | undefined;
    const clear = () => { clearTimeout(startup); setPets([]); };
    const fail = (reason: unknown) => { if (active) { clear(); setError(String(reason)); } };
    const preferences = onPreferences(next => { animation.current = next.animation; });
    void preferences.then(() => loadPreferences()).then(next => { if (active) animation.current = next.animation; }).catch(fail);
    const onMessage = (event: MessageEvent) => {
      // Only the currently mounted distraction may finish/refocus or report an error.
      const frames = document.querySelectorAll<HTMLIFrameElement>('iframe[title^="distraction-"]');
      if (![...frames].some(frame => frame.contentWindow === event.source)) return;
      const m = event.data;
      if (m?.source === "cuteAnimation" && m.event === "error") { fail(m.detail); return; }
      if (m?.source === "cuteAnimation" && m.event === "ready") clearTimeout(startup);
      if (m?.source === "babyOverlay" && m.event === "finish") refocus();
    };
    window.addEventListener("message", onMessage);
    const toggle = async () => {
      if (!active) return;
      if (unlock) {
        await unlock(); unlock = null; anchorRef.current = null; setAnchor(null); clear(); return;
      }
      setError(null);
      unlock = await lockIn({ dwellMs: DWELL_MS }, transition => {
        if (!active) return;
        devLog(`${transition.state}: ${transition.app}`);
        if (transition.state === "locked") { anchorRef.current = transition.app; setAnchor(transition.app); return; }
        clear();
        if (transition.state === "focused") return;
        setError(null);
        setPets([distractionFrame(distractions++, animation.current)]);
        if (animation.current) startup = setTimeout(() => fail("Animation did not load within 5 seconds. Open Settings and preview it."), 5000);
      });
    };
    const stop = listen("lockin:toggle", () => { queue = queue.then(toggle).catch(fail); });
    void stop.catch(fail);
    return () => {
      active = false; clearTimeout(startup);
      window.removeEventListener("message", onMessage);
      void stop.then(fn => fn()).catch(devLogError("shortcut listener cleanup failed"));
      void preferences.then(fn => fn()).catch(devLogError("settings listener cleanup failed"));
      void queue.then(() => unlock?.()).catch(devLogError("unlock failed"));
    };
  }, []);
  return { anchor, pets, refocus, error };
}
