import SwiftUI
import AppKit

public struct PluginsView: View {
    @ObservedObject private var theme = ThemeManager.shared
    @ObservedObject private var loc = LocalizationManager.shared

    @State private var selectedTab: PluginTab = .installed
    @State private var installedPlugins: [PluginInfo] = []
    @State private var availablePlugins: [AvailablePluginInfo] = []
    @State private var isLoading: Bool = true
    @State private var errorMessage: String?
    @State private var toastMessage: String?
    @State private var toastIsError: Bool = false

    // Install inputs
    @State private var installUrl: String = ""
    @State private var installPath: String = ""
    @State private var isInstalling: Bool = false

    public enum PluginTab: String, CaseIterable {
        case installed = "installed"
        case discover = "discover"
    }

    public init() {}

    public var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 24) {
                // Header & Tabs
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(loc.t("plugins_title"))
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundColor(theme.tokens.textPrimary)
                        Text(loc.t("plugins_subtitle"))
                            .font(.system(size: 12))
                            .foregroundColor(theme.tokens.textMuted)
                    }

                    Spacer()

                    // Tab Selector
                    HStack(spacing: 4) {
                        Button(action: { selectedTab = .installed }) {
                            HStack(spacing: 6) {
                                Image(systemName: "checkmark.circle.fill")
                                Text(loc.t("plugins_tab_installed"))
                            }
                            .font(.system(size: 12, weight: .semibold))
                            .padding(.horizontal, 14)
                            .padding(.vertical, 7)
                            .background(selectedTab == .installed ? theme.tokens.accentPrimary : Color.clear)
                            .foregroundColor(selectedTab == .installed ? Color(hex: "#11111B") : theme.tokens.textPrimary)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .buttonStyle(.plain)

                        Button(action: { selectedTab = .discover }) {
                            HStack(spacing: 6) {
                                Image(systemName: "plus.circle.fill")
                                Text(loc.t("plugins_tab_discover"))
                            }
                            .font(.system(size: 12, weight: .semibold))
                            .padding(.horizontal, 14)
                            .padding(.vertical, 7)
                            .background(selectedTab == .discover ? theme.tokens.accentPrimary : Color.clear)
                            .foregroundColor(selectedTab == .discover ? Color(hex: "#11111B") : theme.tokens.textPrimary)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .buttonStyle(.plain)
                    }
                    .padding(3)
                    .background(theme.tokens.bgCard)
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(theme.tokens.borderSubtle, lineWidth: 1)
                    )
                }

                // Toast Banner
                if let msg = toastMessage {
                    HStack(spacing: 8) {
                        Image(systemName: toastIsError ? "exclamationmark.circle.fill" : "checkmark.circle.fill")
                            .foregroundColor(toastIsError ? Color.vesselError : Color.vesselSuccess)
                        Text(msg)
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(theme.tokens.textPrimary)
                        Spacer()
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(toastIsError ? Color.vesselError.opacity(0.12) : Color.vesselSuccess.opacity(0.12))
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                    .transition(.move(edge: .top).combined(with: .opacity))
                }

                // Error Banner
                if let err = errorMessage {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(Color.vesselError)
                        Text(err)
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(theme.tokens.textPrimary)
                        Spacer()
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(Color.vesselError.opacity(0.12))
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                }

                if isLoading {
                    HStack {
                        Spacer()
                        ProgressView()
                            .scaleEffect(1.2)
                        Spacer()
                    }
                    .padding(.vertical, 60)
                } else if selectedTab == .installed {
                    installedView
                } else {
                    discoverView
                }
            }
            .padding(24)
        }
        .background(theme.tokens.bgBase)
        .task {
            await loadPlugins()
        }
    }

    // MARK: - Installed Tab
    @ViewBuilder
    private var installedView: some View {
        if installedPlugins.isEmpty {
            VStack(spacing: 12) {
                Image(systemName: "puzzlepiece.extension")
                    .font(.system(size: 40))
                    .foregroundColor(theme.tokens.textMuted)
                Text("No plugins installed.")
                    .font(.system(size: 14))
                    .foregroundColor(theme.tokens.textMuted)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 60)
        } else {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 320, maximum: 440), spacing: 16)], spacing: 16) {
                ForEach(Array(installedPlugins.enumerated()), id: \.element.id) { index, plugin in
                    InstalledPluginCard(
                        plugin: plugin,
                        canMoveUp: index > 0,
                        canMoveDown: index < installedPlugins.count - 1,
                        onMoveUp: { movePlugin(from: index, to: index - 1) },
                        onMoveDown: { movePlugin(from: index, to: index + 1) },
                        onToggle: {
                            Task {
                                await togglePlugin(plugin)
                            }
                        }
                    )
                }
            }
        }
    }

    // MARK: - Discover & Install Tab
    @ViewBuilder
    private var discoverView: some View {
        VStack(alignment: .leading, spacing: 24) {
            // Install Options (URL & Local)
            HStack(alignment: .top, spacing: 16) {
                // Card 1: URL Install
                VStack(alignment: .leading, spacing: 12) {
                    HStack(spacing: 8) {
                        Image(systemName: "network")
                            .foregroundColor(theme.tokens.accentPrimary)
                        Text(loc.t("plugins_install_url_title"))
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(theme.tokens.textPrimary)
                    }

                    Text(loc.t("plugins_install_url_desc"))
                        .font(.system(size: 11))
                        .foregroundColor(theme.tokens.textMuted)

                    HStack(spacing: 8) {
                        TextField("https://example.com/manifest.json", text: $installUrl)
                            .textFieldStyle(.plain)
                            .foregroundColor(theme.tokens.textPrimary)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 8)
                            .background(theme.tokens.bgBase)
                            .clipShape(RoundedRectangle(cornerRadius: 8))

                        Button(action: installFromUrl) {
                            Text(loc.t("btn_install"))
                                .font(.system(size: 12, weight: .semibold))
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(theme.tokens.accentPrimary)
                                .foregroundColor(Color(hex: "#11111B"))
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .buttonStyle(.plain)
                        .disabled(installUrl.isEmpty || isInstalling)
                    }
                }
                .padding(16)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme.tokens.bgSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(theme.tokens.borderSubtle, lineWidth: 1)
                )

                // Card 2: Local Install
                VStack(alignment: .leading, spacing: 12) {
                    HStack(spacing: 8) {
                        Image(systemName: "folder.badge.gearshape")
                            .foregroundColor(theme.tokens.accentSecondary)
                        Text(loc.t("plugins_install_local_title"))
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(theme.tokens.textPrimary)
                    }

                    Text(loc.t("plugins_install_local_desc"))
                        .font(.system(size: 11))
                        .foregroundColor(theme.tokens.textMuted)

                    HStack(spacing: 8) {
                        TextField("/path/to/plugin folder or binary", text: $installPath)
                            .textFieldStyle(.plain)
                            .foregroundColor(theme.tokens.textPrimary)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 8)
                            .background(theme.tokens.bgBase)
                            .clipShape(RoundedRectangle(cornerRadius: 8))

                        Button(action: selectLocalPluginFolder) {
                            Text("Browse...")
                                .font(.system(size: 12, weight: .medium))
                                .padding(.horizontal, 10)
                                .padding(.vertical, 8)
                                .background(theme.tokens.bgCard)
                                .foregroundColor(theme.tokens.textPrimary)
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .buttonStyle(.plain)

                        Button(action: installFromLocalPath) {
                            Text("Load")
                                .font(.system(size: 12, weight: .semibold))
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(theme.tokens.accentSecondary)
                                .foregroundColor(Color(hex: "#11111B"))
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        .buttonStyle(.plain)
                        .disabled(installPath.isEmpty || isInstalling)
                    }
                }
                .padding(16)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(theme.tokens.bgSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(theme.tokens.borderSubtle, lineWidth: 1)
                )
            }

            // Curated Official Plugins Section
            VStack(alignment: .leading, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(loc.t("plugins_curated_title"))
                        .font(.system(size: 18, weight: .bold, design: .rounded))
                        .foregroundColor(theme.tokens.textPrimary)
                    Text(loc.t("plugins_curated_desc"))
                        .font(.system(size: 12))
                        .foregroundColor(theme.tokens.textMuted)
                }

                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320, maximum: 440), spacing: 16)], spacing: 16) {
                    ForEach(availablePlugins) { p in
                        CuratedPluginCard(plugin: p) {
                            Task {
                                await installCuratedPlugin(p)
                            }
                        }
                    }
                }
            }
        }
    }

    // MARK: - API Actions
    private func loadPlugins() async {
        isLoading = true
        do {
            async let inst = VesselAPIClient.shared.fetchPlugins()
            async let avail = VesselAPIClient.shared.fetchAvailablePlugins()
            let (loadedInst, loadedAvail) = try await (inst, avail)

            // Reorder installed if user previously prioritized
            var sortedInst = loadedInst
            if let orderData = UserDefaults.standard.data(forKey: "vessel_plugin_order"),
               let savedOrder = try? JSONDecoder().decode([String].self, from: orderData) {
                sortedInst.sort { a, b in
                    let idxA = savedOrder.firstIndex(of: a.id) ?? 999
                    let idxB = savedOrder.firstIndex(of: b.id) ?? 999
                    return idxA < idxB
                }
            }

            DispatchQueue.main.async {
                self.installedPlugins = sortedInst
                self.availablePlugins = loadedAvail
                self.isLoading = false
            }
        } catch {
            DispatchQueue.main.async {
                self.errorMessage = error.localizedDescription
                self.isLoading = false
            }
        }
    }

    private func togglePlugin(_ plugin: PluginInfo) async {
        let newState = !plugin.enabled
        do {
            try await VesselAPIClient.shared.togglePlugin(id: plugin.id, enabled: newState)
            showToast(newState ? "Plugin enabled" : "Plugin disabled", isError: false)
            await loadPlugins()
        } catch {
            showToast(error.localizedDescription, isError: true)
        }
    }

    private func movePlugin(from: Int, to: Int) {
        guard from >= 0, from < installedPlugins.count, to >= 0, to < installedPlugins.count else { return }
        installedPlugins.swapAt(from, to)
        let order = installedPlugins.map { $0.id }
        if let data = try? JSONEncoder().encode(order) {
            UserDefaults.standard.set(data, forKey: "vessel_plugin_order")
        }
        showToast("Plugin priority updated", isError: false)
    }

    private func installFromUrl() {
        guard !installUrl.isEmpty else { return }
        isInstalling = true
        Task {
            do {
                try await VesselAPIClient.shared.installPlugin(source: installUrl)
                DispatchQueue.main.async {
                    self.installUrl = ""
                    self.isInstalling = false
                    self.showToast("Plugin installed successfully!", isError: false)
                }
                await loadPlugins()
            } catch {
                DispatchQueue.main.async {
                    self.isInstalling = false
                    self.showToast(error.localizedDescription, isError: true)
                }
            }
        }
    }

    private func installFromLocalPath() {
        guard !installPath.isEmpty else { return }
        isInstalling = true
        Task {
            do {
                try await VesselAPIClient.shared.installPlugin(path: installPath)
                DispatchQueue.main.async {
                    self.installPath = ""
                    self.isInstalling = false
                    self.showToast("Local plugin loaded successfully!", isError: false)
                }
                await loadPlugins()
            } catch {
                DispatchQueue.main.async {
                    self.isInstalling = false
                    self.showToast(error.localizedDescription, isError: true)
                }
            }
        }
    }

    private func installCuratedPlugin(_ plugin: AvailablePluginInfo) async {
        isInstalling = true
        do {
            try await VesselAPIClient.shared.installPlugin(id: plugin.id)
            DispatchQueue.main.async {
                self.isInstalling = false
                self.showToast("Installed \(plugin.name)", isError: false)
            }
            await loadPlugins()
        } catch {
            DispatchQueue.main.async {
                self.isInstalling = false
                self.showToast(error.localizedDescription, isError: true)
            }
        }
    }

    private func selectLocalPluginFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        if panel.runModal() == .OK, let url = panel.url {
            self.installPath = url.path
        }
    }

    private func showToast(_ message: String, isError: Bool) {
        self.toastMessage = message
        self.toastIsError = isError
        DispatchQueue.main.asyncAfter(deadline: .now() + 3.5) {
            if self.toastMessage == message {
                self.toastMessage = nil
            }
        }
    }
}

// MARK: - Installed Plugin Card
public struct InstalledPluginCard: View {
    @ObservedObject private var theme = ThemeManager.shared
    @ObservedObject private var loc = LocalizationManager.shared

    public let plugin: PluginInfo
    public let canMoveUp: Bool
    public let canMoveDown: Bool
    public let onMoveUp: () -> Void
    public let onMoveDown: () -> Void
    public let onToggle: () -> Void

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Header
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(plugin.name)
                        .font(.system(size: 15, weight: .bold))
                        .foregroundColor(theme.tokens.textPrimary)

                    Text("v\(plugin.version ?? "1.0.0") • \(plugin.author ?? "Vessel")")
                        .font(.system(size: 10))
                        .foregroundColor(theme.tokens.textMuted)
                }

                Spacer()

                HStack(spacing: 6) {
                    // Domain badge
                    Text(plugin.domainName)
                        .font(.system(size: 10, weight: .bold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(theme.tokens.accentPrimary.opacity(0.15))
                        .foregroundColor(theme.tokens.accentPrimary)
                        .clipShape(Capsule())

                    // Active badge
                    Text(plugin.enabled ? loc.t("btn_active") : loc.t("btn_disable"))
                        .font(.system(size: 10, weight: .bold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(plugin.enabled ? Color.vesselSuccess.opacity(0.18) : Color.vesselCard)
                        .foregroundColor(plugin.enabled ? Color.vesselSuccess : theme.tokens.textMuted)
                        .clipShape(Capsule())
                }
            }

            if let desc = plugin.description, !desc.isEmpty {
                Text(desc)
                    .font(.system(size: 12))
                    .foregroundColor(theme.tokens.textSecondary)
                    .lineLimit(2)
            }

            Divider()
                .background(theme.tokens.borderSubtle)

            // Footer actions
            HStack {
                Text(plugin.isBuiltin == true ? loc.t("plugin_builtin") : loc.t("plugin_external"))
                    .font(.system(size: 11))
                    .foregroundColor(theme.tokens.textMuted)

                Spacer()

                // Reorder buttons
                HStack(spacing: 4) {
                    Button(action: onMoveUp) {
                        Image(systemName: "chevron.up")
                            .font(.system(size: 10, weight: .bold))
                            .frame(width: 24, height: 24)
                            .background(theme.tokens.bgCard)
                            .foregroundColor(canMoveUp ? theme.tokens.textPrimary : theme.tokens.textMuted.opacity(0.4))
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                    .buttonStyle(.plain)
                    .disabled(!canMoveUp)

                    Button(action: onMoveDown) {
                        Image(systemName: "chevron.down")
                            .font(.system(size: 10, weight: .bold))
                            .frame(width: 24, height: 24)
                            .background(theme.tokens.bgCard)
                            .foregroundColor(canMoveDown ? theme.tokens.textPrimary : theme.tokens.textMuted.opacity(0.4))
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                    .buttonStyle(.plain)
                    .disabled(!canMoveDown)
                }

                // Enable/Disable Toggle button
                Button(action: onToggle) {
                    Text(plugin.enabled ? loc.t("btn_disable") : loc.t("btn_enable"))
                        .font(.system(size: 11, weight: .semibold))
                        .padding(.horizontal, 12)
                        .padding(.vertical, 5)
                        .background(plugin.enabled ? Color.vesselCard : theme.tokens.accentPrimary)
                        .foregroundColor(plugin.enabled ? theme.tokens.textPrimary : Color(hex: "#11111B"))
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(16)
        .background(theme.tokens.bgSurface)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(theme.tokens.borderSubtle, lineWidth: 1)
        )
    }
}

// MARK: - Curated Plugin Card
public struct CuratedPluginCard: View {
    @ObservedObject private var theme = ThemeManager.shared
    @ObservedObject private var loc = LocalizationManager.shared

    public let plugin: AvailablePluginInfo
    public var onInstall: () -> Void

    public var isInstalled: Bool {
        plugin.installed == true
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(plugin.name)
                        .font(.system(size: 15, weight: .bold))
                        .foregroundColor(theme.tokens.textPrimary)

                    Text("v\(plugin.version ?? "1.0.0") • \(plugin.author ?? "Vessel")")
                        .font(.system(size: 10))
                        .foregroundColor(theme.tokens.textMuted)
                }

                Spacer()

                HStack(spacing: 6) {
                    if let lang = plugin.languageDisplay {
                        Text(lang)
                            .font(.system(size: 10, weight: .semibold))
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .background(theme.tokens.accentSecondary.opacity(0.15))
                            .foregroundColor(theme.tokens.accentSecondary)
                            .clipShape(Capsule())
                    }

                    if let dom = plugin.domain {
                        Text(dom.uppercased())
                            .font(.system(size: 10, weight: .bold))
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .background(theme.tokens.accentPrimary.opacity(0.15))
                            .foregroundColor(theme.tokens.accentPrimary)
                            .clipShape(Capsule())
                    }
                }
            }

            if let desc = plugin.description, !desc.isEmpty {
                Text(desc)
                    .font(.system(size: 12))
                    .foregroundColor(theme.tokens.textSecondary)
                    .lineLimit(2)
            }

            Divider()
                .background(theme.tokens.borderSubtle)

            HStack {
                Text(plugin.isBuiltin == true ? loc.t("plugin_builtin") : "Community")
                    .font(.system(size: 11))
                    .foregroundColor(theme.tokens.textMuted)

                Spacer()

                Button(action: onInstall) {
                    HStack(spacing: 4) {
                        if isInstalled {
                            Image(systemName: "checkmark")
                            Text("Installed")
                        } else {
                            Image(systemName: "arrow.down.circle")
                            Text(loc.t("btn_install"))
                        }
                    }
                    .font(.system(size: 11, weight: .semibold))
                    .padding(.horizontal, 14)
                    .padding(.vertical, 6)
                    .background(isInstalled ? Color.vesselCard : theme.tokens.accentPrimary)
                    .foregroundColor(isInstalled ? theme.tokens.textMuted : Color(hex: "#11111B"))
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                }
                .buttonStyle(.plain)
                .disabled(isInstalled)
            }
        }
        .padding(16)
        .background(theme.tokens.bgSurface)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(theme.tokens.borderSubtle, lineWidth: 1)
        )
    }
}
