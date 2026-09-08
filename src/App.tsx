import { APP_TITLE } from "./app-config.ts";
import { modules } from "./generated/modules.ts";
import { lazy, Suspense } from "react";

declare const __DESKTOP__: boolean;
const LockInSettings = __DESKTOP__ ? lazy(() => import("./lockin/LockInSettings.tsx").then(m => ({ default: m.LockInSettings }))) : null;
const PetsOverlay = __DESKTOP__ ? lazy(() => import("./pets/PetsOverlay.tsx").then(m => ({ default: m.PetsOverlay }))) : null;
import { currentView } from "./view.ts";

// The app shell. The overlay window draws the pets and nothing else; any
// other window shows the Module panels.
export function App() {
  const view = currentView();
  if (view === "overlay" && PetsOverlay) return <Suspense fallback={null}><PetsOverlay /></Suspense>;
  if (view === "settings" && LockInSettings) return <Suspense fallback={null}><LockInSettings /></Suspense>;
  return (
    <main>
      <header>
        <p className="eyebrow">Desktop pets</p>
        <h1>{APP_TITLE}</h1>
      </header>

      {modules.map((module) => (
        <module.Component key={module.name} />
      ))}
    </main>
  );
}
