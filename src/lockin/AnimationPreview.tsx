import { useEffect, useRef, useState } from "react";
import { distractionFrame, type CustomAnimation } from "./customAnimation.ts";

export function AnimationPreview({ animation, run }: { animation: CustomAnimation; run: number }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [status, setStatus] = useState("Loading animation…");
  const [error, setError] = useState(false);
  const source = distractionFrame(run, animation);
  useEffect(() => {
    let failed = false;
    const timeout = window.setTimeout(() => {
      failed = true; setError(true);
      setStatus("Animation did not load within 5 seconds. Check the HTML and try again.");
    }, 5000);
    const onMessage = (event: MessageEvent) => {
      if (event.source !== frame.current?.contentWindow) return;
      const m = event.data;
      if (m?.source === "cuteAnimation" && m.event === "error") {
        clearTimeout(timeout); failed = true; setError(true); setStatus(String(m.detail));
      } else if ((m?.source === "cuteAnimation" && m.event === "ready") || (m?.source === "babyOverlay" && m.event === "start")) {
        clearTimeout(timeout); if (!failed) setStatus("Document loaded. Check that the animation plays correctly below.");
      } else if (m?.source === "babyOverlay" && m.event === "finish") {
        clearTimeout(timeout); if (!failed) setStatus("Animation finished.");
      }
    };
    window.addEventListener("message", onMessage);
    return () => { clearTimeout(timeout); window.removeEventListener("message", onMessage); };
  }, []);
  return <section className="card">
    <p role="status" className={error ? "error" : "card-hint"}>{status}</p>
    <iframe ref={frame} title="Animation preview" src={source.src} sandbox={source.sandbox ?? "allow-scripts"} className="animation-preview" />
  </section>;
}
