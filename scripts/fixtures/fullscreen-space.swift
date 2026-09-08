// Disposable second application for the native fullscreen overlay regression probe.
import AppKit

final class Fixture: NSObject, NSApplicationDelegate, NSWindowDelegate {
    var window: NSWindow!
    func report(_ phase: String) {
        FileHandle.standardOutput.write(Data((phase + "\n").utf8))
    }
    func applicationDidFinishLaunching(_ notification: Notification) {
        window = NSWindow(contentRect: NSRect(x: 180, y: 180, width: 800, height: 500),
                          styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.title = "Fullscreen overlay test"
        window.backgroundColor = .darkGray
        window.collectionBehavior = [.fullScreenPrimary]
        window.delegate = self
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { self.report("ordinary") }
        DispatchQueue.main.asyncAfter(deadline: .now() + 4) { self.report("entering"); self.window.toggleFullScreen(nil) }
        // Fail closed if a fullscreen transition never finishes.
        DispatchQueue.main.asyncAfter(deadline: .now() + 20) { NSApp.terminate(nil) }
    }
    func windowDidFailToEnterFullScreen(_ window: NSWindow) { report("fullscreen-failed") }
    func windowDidEnterFullScreen(_ notification: Notification) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { self.report("fullscreen") }
        DispatchQueue.main.asyncAfter(deadline: .now() + 4) { self.window.toggleFullScreen(nil) }
    }
    func windowDidExitFullScreen(_ notification: Notification) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { self.report("returned") }
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { NSApp.terminate(nil) }
    }
}
let app = NSApplication.shared
let fixture = Fixture()
app.setActivationPolicy(.regular)
app.delegate = fixture
app.run()
