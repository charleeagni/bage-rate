import { useEffect } from "react";

import { watchClickable } from "./clickable.ts";
import "./overlay.css";

// Frames have separate documents. Trusted bundled frames share the host origin
// for click hit-testing; imported animations use an isolated origin and sandbox. A frame covers the whole screen unless
// given a box. Anything the frame marks `.clickable` takes the cursor; the rest
// of the overlay stays click-through.
export type Frame = {
  key: string;
  src?: string;
  srcDoc?: string;
  sandbox?: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
};

export function Overlay({ frames, children }: { frames: Frame[]; children?: React.ReactNode }) {
  useEffect(watchClickable, []);
  return (
    <div className="overlay">
      {frames.map((frame) => (
        <iframe
          key={frame.key}
          className="overlay-frame"
          sandbox={frame.sandbox ?? "allow-scripts allow-same-origin"}
          src={frame.src}
          srcDoc={frame.srcDoc}
          title={frame.key}
          style={{
            left: frame.x ?? 0,
            top: frame.y ?? 0,
            width: frame.width ?? "100%",
            height: frame.height ?? "100%",
          }}
        />
      ))}
      {children}
    </div>
  );
}
