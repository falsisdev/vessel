import SwiftUI
import AppKit

@main
public struct VesselApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    public init() {}

    public var body: some Scene {
        WindowGroup {
            MainView()
                .preferredColorScheme(.dark)
                .frame(minWidth: 1000, minHeight: 650)
        }
        .windowStyle(.titleBar)
        .windowToolbarStyle(.unified(showsTitle: false))
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandMenu("Playback") {
                Button("Play / Pause") {
                    PlayerState.shared.togglePlayPause()
                }
                .keyboardShortcut(.space, modifiers: [])

                Button("Stop Playback") {
                    PlayerState.shared.stop()
                }
                .keyboardShortcut("w", modifiers: [.command])
            }
        }
    }
}

public class AppDelegate: NSObject, NSApplicationDelegate {
    public func applicationDidFinishLaunching(_ notification: Notification) {
        VesselCoreDaemon.shared.startHealthCheck()
    }

    public func applicationWillTerminate(_ notification: Notification) {
        VesselCoreDaemon.shared.shutdown()
    }
}
