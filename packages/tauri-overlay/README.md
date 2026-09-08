# tauri-overlay

A click-through, always-on-top, transparent overlay for Tauri on macOS. Drop
any HTML document onto the screen; it renders in a sandboxed frame above every
window and lets clicks fall through, except over elements marked `.clickable`.

## Rust

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

See https://github.com/charleeagni/tauri-overlay for the `tauri.conf.json` window flags
and capability permissions the window needs.

## React

```tsx
import { Overlay } from "tauri-overlay";

<Overlay frames={[{ key: "blob", src: "/pets/blob.html" }]}>
  {/* optional host chrome drawn over the frames */}
</Overlay>
```

A frame covers the whole screen unless given `x`, `y`, `width`, `height`. Use
`src` for a served document or `srcDoc` for inline HTML. Inside a frame, add
the `clickable` class to anything that should receive the cursor; add it to
`<html>` while dragging so the cursor is not lost between frames.
