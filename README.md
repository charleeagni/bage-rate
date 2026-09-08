# bage-rate

Desktop pets for macOS. A transparent, always-on-top, click-through window
covers your screen and renders any HTML you give it. Clicks fall through to
whatever is underneath, except on elements you mark `.clickable`.

## Add a pet

1. Write a self-contained HTML file and put it in `public/pets/`.
   Transparent background, position things with CSS, use `class="clickable"`
   on anything that should catch the mouse. See `public/pets/blob.html`.
2. List it in `public/pets/manifest.json`:

   ```json
   [
     { "file": "blob.html" },
     { "file": "cat.html", "x": 0, "y": 900, "width": 400, "height": 200 }
   ]
   ```

   Without a box the frame covers the whole screen and the pet positions
   itself. With one, the pet is confined to that rectangle.
3. `bun run dev` (or `npm run dev`).

A pet may draw with WebGL and run WebAssembly: `public/pets/psych.html` is
a shader plasma whose warp factor comes from `psych.wasm`. Put the `.wasm`
next to the HTML and `fetch` it by relative path. three.js is available too:
`import * as THREE from "./vendor/three.module.min.js"` as in
`public/pets/orbit.html` (the file is copied from `node_modules` on install).

Pets stay visible over fullscreen apps too. The exceptions are games that
capture the display exclusively, and fullscreen apps on a second monitor:
the overlay covers the primary one only.

Bundled pets run in separate iframe documents and share the host origin for
click detection. Treat a bundled pet as trusted code: it is part of the app's
same-origin content. Imported lock-in animations instead run in an isolated
sandbox. While dragging, add `clickable` to `<html>` so the cursor is not lost
between frames; `blob.html` shows the pattern.

## Lock-in mode

`Cmd+Shift+L` is the default shortcut. Change it in Settings, available from
the app menu with `Cmd+,` or from the tray. Switch away from the anchored app
for five seconds and the original baby animation plays. Press the shortcut
again to unlock. Settings also accepts self-contained HTML animations and
provides a preview with playback errors. Imported animations are served by
Rust through a custom protocol without opening a port. See
[playback validation](docs/lockin-validation.md) for the test procedure.

## Privacy and trust

Lock-in mode reads the name of the foreground macOS app about twice a second
while it is enabled. The name is used locally to detect whether you have left
the anchored app and to bring that app forward again. bage-rate does not send
those names over the network.

Only import an animation you trust. Its HTML can run JavaScript, but it is
served from a separate custom origin with a restrictive policy: it cannot make
network requests, submit forms, embed frames, or read the app's origin. The
animation itself is stored in bage-rate's local application preferences.

The optional `web-server` executable is for local development and testing. It
has no authentication or authorization and binds to `127.0.0.1` by default.
Keep it on loopback. If you deliberately bind it to a network interface, every
client that can reach it can access and modify its data.

## Layout

- `public/pets/`: your pets, plus the manifest.
- `packages/tauri-overlay/`: the React half of the reusable overlay; the Rust crate comes from https://github.com/charleeagni/tauri-overlay
  (iframe host, cursor hit-testing, window setup). See its README.
- `src/pets/`: loads the manifest into the overlay.
- `src/lockin/`: lock-in mode.

## Under the hood

bage-rate is built on a Tauri + GraphQL template. Its docs, including `verify`,
Modules, and the two build targets, moved to
[`docs/tauri-graphql-template.md`](docs/tauri-graphql-template.md).

## macOS releases without a Developer ID

`npm run build:app` verifies the repository, builds an app with Tauri's ad-hoc
signing identity (`-`), checks its bundle signature, and creates a standard DMG.
It also checks the app signature inside the mounted DMG before succeeding.
Finder automation permission is not required.

Ad-hoc signing seals the app's contents but does not identify the developer or
provide Apple notarization. Downloaded copies can still be blocked by Gatekeeper.
Testers must explicitly approve an app they trust using the options available
on their macOS version. See [Apple's guidance on opening apps safely](https://support.apple.com/en-us/102445).
The build never changes Gatekeeper settings or removes download quarantine.

## License

MIT. See [LICENSE](LICENSE). Third-party components retain their respective licenses.
