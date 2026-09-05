import SwiftUI
import AppKit

public enum NavigationItem: String, CaseIterable, Identifiable {
    case cinema = "cinema"
    case reading = "reading"
    case liveTV = "live"
    case aiDiscovery = "ai"
    case library = "library"
    case plugins = "plugins"
    case settings = "settings"

    public var id: String { rawValue }

    public var icon: String {
        switch self {
        case .cinema: return "film"
        case .reading: return "book.closed"
        case .liveTV: return "tv"
        case .aiDiscovery: return "sparkles"
        case .library: return "books.vertical"
        case .plugins: return "puzzlepiece.extension"
        case .settings: return "gearshape"
        }
    }

    public func localizedTitle(loc: LocalizationManager) -> String {
        switch self {
        case .cinema: return loc.t("nav_cinema")
        case .reading: return loc.t("nav_reading")
        case .liveTV: return loc.t("nav_live")
        case .aiDiscovery: return loc.t("nav_ai")
        case .library: return loc.t("nav_library")
        case .plugins: return loc.t("nav_plugins")
        case .settings: return loc.t("nav_settings")
        }
    }
}

public struct SidebarView: View {
    @Binding public var selection: NavigationItem?
    @ObservedObject public var daemon = VesselCoreDaemon.shared
    @ObservedObject private var theme = ThemeManager.shared
    @ObservedObject private var loc = LocalizationManager.shared

    public init(selection: Binding<NavigationItem?>) {
        self._selection = selection
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // App Branding Header & Full-screen button
            HStack(spacing: 10) {
                ZStack {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(
                            LinearGradient(
                                colors: [theme.tokens.accentPrimary, theme.tokens.accentSecondary],
                                startPoint: .topLeading,
                                endPoint: .bottomTrailing
                            )
                        )
                        .frame(width: 32, height: 32)
                    Image(systemName: "circle.circle.fill")
                        .font(.system(size: 16, weight: .bold))
                        .foregroundColor(Color(hex: "#11111B"))
                }

                VStack(alignment: .leading, spacing: 2) {
                    Text("VESSEL")
                        .font(.system(size: 15, weight: .black, design: .rounded))
                        .foregroundColor(theme.tokens.textPrimary)
                        .tracking(1.5)
                    Text("Native macOS")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundColor(theme.tokens.textMuted)
                }

                Spacer()

                // Full Screen Toggle Button
                Button(action: {
                    if let window = NSApp.windows.first(where: { $0.isKeyWindow }) ?? NSApp.mainWindow {
                        window.toggleFullScreen(nil)
                    }
                }) {
                    Image(systemName: "arrow.up.left.and.arrow.down.right")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundColor(theme.tokens.textMuted)
                        .frame(width: 24, height: 24)
                        .background(theme.tokens.bgCard)
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain)
                .help("Toggle Full Screen")
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)

            Divider()
                .background(theme.tokens.borderSubtle)

            // Navigation List
            List(NavigationItem.allCases, selection: $selection) { item in
                NavigationLink(value: item) {
                    HStack(spacing: 12) {
                        Image(systemName: item.icon)
                            .font(.system(size: 14, weight: .semibold))
                            .frame(width: 20)
                            .foregroundColor(selection == item ? theme.tokens.accentPrimary : theme.tokens.textSecondary)
                        Text(item.localizedTitle(loc: loc))
                            .font(.system(size: 13, weight: selection == item ? .semibold : .regular))
                            .foregroundColor(selection == item ? theme.tokens.textPrimary : theme.tokens.textSecondary)
                    }
                    .padding(.vertical, 5)
                }
            }
            .listStyle(.sidebar)
            .scrollContentBackground(.hidden)

            Spacer()

            // Daemon status footer
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    Circle()
                        .fill(daemon.isCoreRunning ? Color.vesselSuccess : Color.vesselWarning)
                        .frame(width: 8, height: 8)
                    Text(daemon.coreStatusText)
                        .font(.system(size: 11))
                        .foregroundColor(theme.tokens.textMuted)
                        .lineLimit(1)
                }
            }
            .padding(14)
            .background(theme.tokens.bgSurface.opacity(0.8))
        }
        .background(theme.tokens.bgSidebar)
    }
}
