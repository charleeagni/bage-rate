# Lock-in playback checks

The default animation is `public/pets/baby-overlay.html`. Lock-in follows the
frontmost application, not browser tabs. The intended dwell before playback is
five seconds, plus foreground-app detection and rendering time. Chrome URL
lookup is no longer part of the transition.

Imported HTML is saved in `lockin-settings.json` in the app data directory.
Rust serves it at `animation://localhost/current` on macOS, with no listening
port. Windows uses Tauri's `http://animation.localhost/current` equivalent.
The frame allows scripts but does not share the app origin. Its response policy
allows inline animation code, inline styles and embedded assets, and blocks
network requests and nested frames. Imported files must be self-contained.
The main app keeps its existing script policy.

Preview reports document readiness, JavaScript errors, rejected promises, and
blocked resources. Readiness means the document loaded; a person must still
check that it draws and animates correctly. Distraction playback accepts finish
messages only from its currently mounted frame. Preview never refocuses an app.

Shortcut preferences are saved by Rust. A replacement is registered before the
old shortcut is released. Registration failure leaves the old shortcut in place.
The recorder uses physical keys, requires Command, Control or Option, and
supports Escape to cancel and Tab to leave. The shortcut does not activate
lock-in while Settings is focused.

## Packaged Mac checks

Build with `npm exec tauri build -- --debug --bundles app --ci`. This bundles
frontend assets and exercises the packaged content policy without a Vite server.
A debug bundle is not a signed, notarized release.

1. Open Settings. Confirm Baby is the default and Preview plays it.
2. Paste this HTML and preview it. Confirm the moving dot appears, then the
   preview reports completion without switching apps.

   ```html
   <html><style>
   body { margin:0; background:transparent; }
   div { width:40px; height:40px; background:tomato; border-radius:50%; animation:move 2s linear; }
   @keyframes move { to { transform:translateX(200px); } }
   </style><div></div><script>
   setTimeout(() => parent.postMessage({source:'babyOverlay',event:'finish'}, '*'), 2000);
   </script></html>
   ```

3. Preview `<html><script>throw Error('Preview error test')</script></html>`.
   Confirm the error is visible. Test an external script too; it should report
   a blocked resource instead of silently failing.
4. Restore Baby. Record a different shortcut. Cancel a second change with
   Escape. Restart and confirm the saved shortcut still works. Try a shortcut
   already owned by another app and check that failures preserve the old one.
5. Lock into one app, switch to another and measure time until the first visible
   animation frame. Return to the anchor during the dwell and confirm playback
   cancels. Toggle off during playback. Repeat with fast repeated shortcut presses.
6. Lock into a terminal on an ordinary desktop, then switch to Chrome already
   in native fullscreen. Wait for the dwell and confirm the baby appears there,
   without leaving fullscreen or taking focus. Return during playback, repeat,
   and let playback finish to check automatic refocus. Repeat with a fullscreen
   anchor and fullscreen distracting app. Test each
   monitor. The existing overlay covers the primary monitor only; secondary
   monitor coverage is not implemented by this change.
7. Repeat on the affected Mac and a second Mac. Record macOS version, hardware,
   app build, animation file, time until first frame and any error shown.

`npm run verify` checks generated contracts, TypeScript, bundle separation,
formatting, Clippy, Rust tests and the web server. It does not establish
fullscreen behavior, multi-monitor coverage or performance on another machine.

## Native fullscreen regression

Run `./scripts/check-fullscreen-overlay.sh` in a macOS GUI session. It opens a
temporary native application, enters fullscreen, and returns to the desktop.
Leave it focused while the probe runs. It does not load the production database
or preferences. Swift and Cargo are required.

The probe uses the production overlay window configuration and panel setup.
At each stage it checks active-Space membership, window visibility and occlusion,
mouse pass-through, and that the fixture still owns focus. It fails if a stage
is missing. `./scripts/check-fullscreen-overlay.sh --baseline` instead uses the
previous NSWindow configuration and should fail in fullscreen. This GUI check is
separate from `npm run verify`, which also runs on machines without a GUI session.

The overlay is a floating, non-activating NSPanel that remains visible when the
application deactivates. Collection flags alone on Tauri's ordinary NSWindow
were insufficient. Conversion uses the pinned tauri-nspanel integration, retaining
the same native object for the webview and Tauri's cursor commands. Settings is
not converted. The application keeps its existing activation policy.

## Observed in this checkout

A packaged debug validation copy with a separate app identifier was exercised
through the macOS UI. The original baby preview reported start and completion.
An imported self-contained HTML animation rendered a red dot through
`animation://localhost/current` and reported completion. An imported throwing
script displayed `Script error.` in Preview. Command-Shift-K was recorded and
saved; Escape cancelled another recording. The shortcut and restored Baby
selection survived a complete app restart. The recorder explicitly focuses its
button on click because WebKit did not focus it during the first capture test.

`CARGO_INCREMENTAL=0 npm run verify` passed after clearing the checkout's
incremental compiler cache to recover disk space. No second Mac or secondary-monitor
coverage has been verified. The affected machine's reported 20-second delay still
needs a timing measurement.

On macOS 26.2, the native fullscreen probe reproduced the failure with the
previous NSWindow: visible on the desktop, invisible in the fixture's fullscreen
Space, then visible on return. The NSPanel passed all three stages while the
fixture kept focus and the overlay remained click-through. This checks native
window behavior; the full lock-in animation sequence in Chrome remains a separate
manual check above.
