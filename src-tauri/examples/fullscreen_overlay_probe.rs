//! GUI regression probe. Launches a disposable second app into fullscreen.
//! Run with CUTE_FULLSCREEN_FIXTURE pointing to the compiled Swift fixture.
//! --baseline reproduces the old NSWindow configuration for comparison.

#[cfg(target_os = "macos")]
fn main() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use tauri::{Manager, PhysicalPosition};

    let fixture = std::env::var("CUTE_FULLSCREEN_FIXTURE").expect("set CUTE_FULLSCREEN_FIXTURE");
    let baseline = std::env::args().any(|arg| arg == "--baseline");
    let mut context = tauri::generate_context!();
    // Reuse the production overlay config without loading app state or settings.
    context
        .config_mut()
        .app
        .windows
        .retain(|w| w.label == "main");
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().unwrap());
    let app = tauri::Builder::default()
        .plugin(tauri_overlay::init())
        .on_page_load(|webview, _| {
            webview.eval("document.body.innerHTML = '<div style=\"position:fixed;top:100px;left:100px;padding:24px;background:#ef4565;color:white;font:24px sans-serif\">Fullscreen overlay probe</div>'").unwrap();
        })
        .build(context)
        .unwrap();
    app.run(move |app, event| {
        if !matches!(event, tauri::RunEvent::Ready) {
            return;
        }
        let window = app.get_webview_window("main").unwrap();
        if baseline {
            if let Some(monitor) = window.primary_monitor().unwrap() {
                window.set_position(PhysicalPosition::new(0, 0)).unwrap();
                window.set_size(*monitor.size()).unwrap();
            }
            window.set_ignore_cursor_events(true).unwrap();
            use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as B};
            // SAFETY: Ready is on the main thread and Tauri owns this window.
            let native = unsafe { &*window.ns_window().unwrap().cast::<NSWindow>() };
            native.setCollectionBehavior(
                B::CanJoinAllSpaces | B::FullScreenAuxiliary | B::CanJoinAllApplications,
            );
        } else {
            tauri_overlay::cover_primary_monitor(&window).unwrap();
        }
        let app = app.clone();
        let fixture = fixture.clone();
        std::thread::spawn(move || {
            let mut child = Command::new(fixture).stdout(Stdio::piped()).spawn().unwrap();
            let pid = child.id();
            let mut phases = 0;
            let mut failed = false;
            for line in BufReader::new(child.stdout.take().unwrap()).lines() {
                let phase = line.unwrap();
                println!("fixture: {phase}");
                if !matches!(phase.as_str(), "ordinary" | "fullscreen" | "returned") {
                    continue;
                }
                phases += 1;
                let (tx, rx) = std::sync::mpsc::channel();
                let window = window.clone();
                app.run_on_main_thread(move || {
                    use objc2_app_kit::{NSWindow, NSWindowOcclusionState, NSWorkspace};
                    assert!(objc2::MainThreadMarker::new().is_some());
                    // SAFETY: Tauri owns the window, borrowed on AppKit's main thread.
                    let native = unsafe { &*window.ns_window().unwrap().cast::<NSWindow>() };
                    let frontmost = NSWorkspace::sharedWorkspace().frontmostApplication();
                    let fixture_focused = frontmost.is_some_and(|a| a.processIdentifier() == pid as i32);
                    let visible = native.isOnActiveSpace()
                        && native.isVisible()
                        && native.occlusionState().contains(NSWindowOcclusionState::Visible);
                    let click_through = native.ignoresMouseEvents();
                    let no_focus = !native.isKeyWindow() && !native.canBecomeKeyWindow();
                    println!("{phase}: visible={visible}, fixture_focused={fixture_focused}, click_through={click_through}, no_focus={no_focus}");
                    tx.send(visible && fixture_focused && click_through && no_focus).unwrap();
                }).unwrap();
                failed |= !rx.recv().unwrap();
            }
            let status = child.wait().unwrap();
            let code = i32::from(failed || phases != 3 || !status.success());
            println!("probe: phases={phases}, exit={code}");
            // Exit explicitly: AppKit termination can otherwise discard the test exit code.
            std::process::exit(code);
        });
    });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This probe requires a macOS GUI session.");
}
