import SwiftUI
import AppKit

public struct MainView: View {
    @State private var selection: NavigationItem? = .cinema
    @State private var selectedMedia: MediaItem?
    @ObservedObject private var playerState = PlayerState.shared
    @ObservedObject private var theme = ThemeManager.shared

    public init() {}

    public var body: some View {
        ZStack {
            WindowAccessor()
                .frame(width: 0, height: 0)

            NavigationSplitView {
                SidebarView(selection: $selection)
                    .navigationSplitViewColumnWidth(min: 200, ideal: 230, max: 270)
            } detail: {
                ZStack {
                    switch selection {
                    case .cinema:
                        CinemaHomeView { item in
                            selectedMedia = item
                        }
                    case .reading:
                        ReadingHomeView { item in
                            selectedMedia = item
                        }
                    case .liveTV:
                        LiveTVView()
                    case .library:
                        LibraryView { item in
                            selectedMedia = item
                        }
                    case .aiDiscovery:
                        ScrollView {
                            AIDiscoveryView(defaultDomain: "all") { item in
                                selectedMedia = item
                            }
                            .padding(24)
                        }
                        .background(theme.tokens.bgBase)
                    case .plugins:
                        PluginsView()
                    case .settings:
                        SettingsView()
                    case .none:
                        CinemaHomeView { item in
                            selectedMedia = item
                        }
                    }
                }
                .background(theme.tokens.bgBase)
            }
            .sheet(item: $selectedMedia) { item in
                MediaDetailsView(item: item) {
                    selectedMedia = nil
                }
            }

            if playerState.isPlayerPresented {
                VideoPlayerView()
                    .transition(.opacity)
                    .zIndex(100)
            }
        }
        .animation(.easeInOut(duration: 0.25), value: playerState.isPlayerPresented)
        .background(theme.tokens.bgBase)
    }
}

public struct WindowAccessor: NSViewRepresentable {
    public func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async {
            if let window = view.window {
                window.collectionBehavior.insert([.fullScreenPrimary, .fullScreenAllowsTiling])
                window.styleMask.insert([.resizable, .miniaturizable, .closable, .titled])
                window.standardWindowButton(.zoomButton)?.isEnabled = true
                window.titleVisibility = .hidden
                window.titlebarAppearsTransparent = true
            }
        }
        return view
    }

    public func updateNSView(_ nsView: NSView, context: Context) {}
}
