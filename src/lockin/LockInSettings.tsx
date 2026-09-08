import { useEffect, useState, type DragEvent } from "react";

import { migrateAnimation, onPreferences, setAnimation, setShortcut, type CustomAnimation, type Preferences } from "./customAnimation.ts";
import { AnimationPreview } from "./AnimationPreview.tsx";
import { shortcutFromKey, shortcutLabel } from "./shortcut.ts";
import { chatGptUrl, claudeUrl, openInBrowser } from "./generatePrompt.ts";

// The settings window: the animation that plays when
// you drift (drop or paste an HTML file, one button to restore the default),
// and deep links that ask an assistant to write one.
export function LockInSettings() {
  const [preferences, setPreferences] = useState<Preferences | null>(null);
  const animation = preferences?.animation ?? null;
  const [recording, setRecording] = useState(false);
  const [preview, setPreview] = useState(0);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    let active = true;
    const apply = (value: Preferences) => { if (active) setPreferences(value); };
    const stop = onPreferences(apply);
    void stop.then(() => migrateAnimation()).then(apply).catch(e => { if (active) setError(String(e)); });
    return () => { active = false; void stop.then(fn => fn()); };
  }, []);
  const [over, setOver] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = (work: Promise<unknown>) => {
    setError(null);
    work.catch((reason: unknown) => setError(String(reason)));
  };
  const apply = async (next: CustomAnimation) => {
    setSaving(true); setError(null); setPreview(0);
    try { setPreferences(await setAnimation(next)); }
    finally { setSaving(false); }
  };
  const onDrop = (event: DragEvent) => {
    event.preventDefault();
    setOver(false);
    const file = event.dataTransfer.files[0];
    if (!file) return;
    if (!/\.html?$/i.test(file.name)) return setError("That isn't an .html file.");
    run(file.text().then((html) => apply({ name: file.name, html })));
  };

  // Paste lands on the document, not on any element, so listen at the window.
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const html = event.clipboardData?.getData("text/plain").trim();
      if (!html) return;
      event.preventDefault();
      if (!/<(html|body|svg|div|style|script)\b/i.test(html)) return setError("That doesn't look like HTML.");
      run(apply({ name: "Pasted animation", html }));
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  });

  return (
    <main className="settings">
      <header className="settings-header">
        <p className="eyebrow">bage-rate</p>
        <h2>Lock in</h2>
        <p className="lede">
          Press the shortcut while in the app you want to stay in. Drift somewhere else for a few seconds
          and the animation plays, then drags you back. Press it again to unlock.
        </p>
      </header>

      <section className="card card-row">
        <div>
          <strong>Shortcut</strong>
          <p className="card-hint">Toggles lock-in from anywhere</p>
        </div>
        <button className="hotkey" disabled={!preferences || saving}
          aria-label="Change lock-in shortcut"
          onClick={event => { event.currentTarget.focus(); setRecording(true); setError(null); }}
          onBlur={() => setRecording(false)}
          onKeyDown={event => {
            if (!recording) return;
            if (event.key === "Escape") { event.preventDefault(); setRecording(false); return; }
            if (event.key === "Tab") { setRecording(false); return; }
            event.preventDefault(); event.stopPropagation();
            if (event.repeat) return;
            const shortcut = shortcutFromKey(event);
            if (!shortcut) { setError("Use Command, Control, or Option with a letter, number, arrow, space, or function key."); return; }
            setRecording(false); setSaving(true);
            run(setShortcut(shortcut).then(setPreferences).finally(() => setSaving(false)));
          }}>
          {recording ? "Press shortcut… Esc cancels" : preferences ? shortcutLabel(preferences.shortcut) : "Loading…"}
        </button>
      </section>

      <section
        className={`dropzone${over ? " dropzone-over" : ""}${animation ? " dropzone-custom" : ""}`}
        onDragOver={(event) => {
          event.preventDefault();
          setOver(true);
        }}
        onDragLeave={() => setOver(false)}
        onDrop={onDrop}
      >
        <span className="dropzone-icon" aria-hidden>
          {animation ? "🎬" : "👶"}
        </span>
        <strong>{animation ? animation.name : "Default: Baby sneeze & lick"}</strong>
        <span className="dropzone-hint">
          {animation
            ? "Drop or paste another .html to swap the distraction animation"
            : "Drop an .html file here, or paste the HTML with ⌘V, to change what plays when you drift"}
        </span>
        {animation && (
          <button className="ghost" disabled={saving} onClick={() => run(apply(null))}>
            Restore default animation
          </button>
        )}
      </section>

      <button disabled={!preferences || saving} onClick={() => setPreview(n => n + 1)}>Preview animation</button>
      {preview > 0 && <AnimationPreview key={preview} run={preview} animation={animation} />}
      {preferences?.shortcutError && <p className="error" role="alert">{preferences.shortcutError}</p>}

      <section className="card generate">
        <p className="eyebrow">Want your own?</p>
        <p className="card-hint">
          Opens your browser with a ready-made prompt. The assistant asks what you want and hands you the
          HTML. Paste it above.
        </p>
        <div className="generate-buttons">
          <button className="link-button" onClick={() => run(openInBrowser(chatGptUrl()))}>
            Generate with ChatGPT ↗
          </button>
          <button className="link-button" onClick={() => run(openInBrowser(claudeUrl()))}>
            Generate with Claude ↗
          </button>
        </div>
      </section>

      {error && <p role="alert" className="error">{error}</p>}
    </main>
  );
}
