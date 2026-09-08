# tauri-overlay

A transparent, always-on-top, click-through window for Tauri on macOS.
It covers your screen and renders the HTML you give it. Clicks fall through to
whatever is underneath, except on elements you mark `.clickable`.

Use it for desktop pets, reminders, or anything else you want to draw over
other apps. bage-rate uses it for the animation that appears when you leave
your work app. The overlay itself doesn't decide when to remind you.

## Set up the window

```rust
// On macOS, add `.plugin(tauri_overlay::init())` to the Tauri builder.

app.run(|app, event| {
    if let tauri::RunEvent::Ready = event {
        let window = app.get_webview_window("main").unwrap();
        tauri_overlay::cover_primary_monitor(&window).unwrap();
    }
});
```

Frames may run WebAssembly when the app's `script-src` policy in
`tauri.conf.json` includes `'wasm-unsafe-eval'`.

The [Rust crate's README](https://github.com/charleeagni/tauri-overlay) lists the
`tauri.conf.json` window flags and capability permissions you need.

## Put HTML on screen

```tsx
import { Overlay } from "tauri-overlay";

<Overlay frames={[{ key: "blob", src: "/pets/blob.html" }]}>
  {/* Optional controls drawn over the frames. */}
</Overlay>
```

A frame covers the whole screen unless given `x`, `y`, `width`, `height`. Use
`src` for a served document or `srcDoc` for inline HTML. Inside a frame, add
the `clickable` class to anything that should receive the cursor; add it to
`<html>` while dragging so the cursor is not lost between frames.

## Make a pet

Write an HTML file with a transparent background and put it in your app's
public assets. Add its URL to `frames`, as above. The pet positions itself
with CSS; the overlay handles the window and mouse clicks.

The examples in this repository are in [`public/pets/`](../../public/pets/).
Start with `blob.html` for a draggable pet. `orbit.html` uses three.js, and
`psych.html` runs a WebGL shader with WebAssembly. The app copies three.js
into `public/pets/vendor/` during installation.

For the bundled pets in this app, add the file to
[`manifest.json`](../../public/pets/manifest.json):

```json
[
  { "file": "blob.html" },
  { "file": "cat.html", "x": 0, "y": 900, "width": 400, "height": 200 }
]
```

The manifest loader belongs to this app. In another app, pass your own list
of frames to `Overlay`.

## Frame access and screen limits

By default, frames allow scripts and share the host origin so the overlay
can detect `.clickable` elements. Use that default for trusted bundled HTML.
It doesn't isolate untrusted code from your app.

bage-rate serves imported animations from a separate origin and passes
`sandbox="allow-scripts"`. Click detection inside those frames isn't available
to the host. See [playback validation](../../docs/lockin-validation.md) for how
the app loads and restricts them.

The overlay stays visible over fullscreen apps on the primary monitor.
Secondary monitors aren't covered, and games that take exclusive control
of the display may cover the overlay.
