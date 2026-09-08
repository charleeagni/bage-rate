![bage-rate app demo](docs/demo.gif)

<p align="center">
  <a href="https://github.com/charleeagni/bage-rate/releases/download/v0.1.3/bage-rate_0.1.3_aarch64.dmg">
    <img src="https://img.shields.io/badge/Download_bage--rate_0.1.3-macOS_Apple_silicon-2675f5?style=for-the-badge&logo=apple&logoColor=white" alt="Download bage-rate 0.1.3 for Apple silicon Macs">
  </a>
</p>

# bage-rate

Lock into the app you want to work in. Wander off and bage-rate puts a baby
on your screen to remind you, then brings you back. For macOS.

## Use it

1. Open the app you want to stay in and press `Cmd+Shift+L`.
2. Switch to another app for five seconds and the reminder plays.
3. When it finishes, bage-rate brings your work app forward again.
4. Press `Cmd+Shift+L` again when you're done.

It follows apps, so switching tabs inside the same browser won't trigger it.
The reminder appears over fullscreen apps too, on the primary monitor.
Games that take exclusive control of the display may cover it.

## Make the reminder yours

Open Settings from the tray or the app menu with `Cmd+,`. You can change the
shortcut there, too.

The default baby sneezes and licks the screen. If that's not your idea of
encouragement, drop an HTML file into Settings or paste its contents. Preview
it there before using it. You can restore the baby with one button.

Settings also has buttons that open ChatGPT or Claude with a prompt for making
an animation. Describe what you want, then paste the HTML you get back.

## What stays on your Mac

While lock-in is on, bage-rate checks the name of the foreground app about
twice a second. It uses that name to detect when you leave and bring you back.
Those names aren't sent over the network.

Your shortcut and imported animation are saved locally. Imported HTML can run
JavaScript, so only use files you trust. It runs in an isolated frame that
blocks network requests, forms, nested frames and access to the app's origin.
Keep all the animation's assets in the HTML file.

## Work on the app

Run `npm install`, then `npm run dev` to start the desktop app. Lock-in code
lives in `src/lockin/`; bundled animations live in `public/pets/`.

The transparent window comes from [tauri-overlay](packages/tauri-overlay/README.md).
That's the reusable part if you want to put your own HTML or desktop pets over
other apps. Its Rust crate lives in the
[tauri-overlay repository](https://github.com/charleeagni/tauri-overlay).

Run `npm run verify` to check the code. See
[playback validation](docs/lockin-validation.md) for manual app checks.
The database, module and build documentation is in the
[Tauri + GraphQL template guide](docs/tauri-graphql-template.md).

The optional `web-server` is for local development and testing. It has no
authentication and binds to `127.0.0.1` by default. Keep it on loopback.
If you bind it to a network interface, anyone who can reach it can read and
change its data.

## Build a macOS release

`npm run build:app` verifies the repository, signs and notarizes the app,
creates a signed and notarized DMG, staples both tickets, and checks the app's
signature inside the mounted DMG. Finder automation permission isn't needed.

## License

MIT. See [LICENSE](LICENSE). Third-party components keep their own licenses.
