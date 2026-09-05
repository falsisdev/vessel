import SwiftUI

public struct SettingsView: View {
    @ObservedObject public var daemon = VesselCoreDaemon.shared

    @AppStorage("vessel_selected_theme") private var selectedTheme: String = "catppuccin"
    @AppStorage("vessel_selected_variant") private var selectedVariant: String = "mocha"
    @AppStorage("vessel_selected_locale") private var selectedLang: String = "tr"
    @AppStorage("vessel_realdebrid_token") private var rdToken: String = ""
    @AppStorage("vessel_torbox_token") private var torboxToken: String = ""
    @AppStorage("vessel_api_base_url") private var apiBaseUrl: String = "http://127.0.0.1:8080"
    @AppStorage("vessel_core_binary_path") private var coreBinaryPath: String = ""
    @AppStorage("vessel_core_host") private var coreHost: String = "127.0.0.1"
    @AppStorage("vessel_core_port") private var corePort: String = "8080"

    public init() {}

    public var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 24) {
                // Header
                VStack(alignment: .leading, spacing: 4) {
                    Text("Settings")
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundColor(.vesselTextPrimary)
                    Text("Configure themes, debrid providers, languages, and core engine")
                        .font(.system(size: 12))
                        .foregroundColor(.vesselTextMuted)
                }

                // Theme Engine Section
                VStack(alignment: .leading, spacing: 14) {
                    HStack {
                        Image(systemName: "paintpalette.fill")
                            .foregroundColor(.vesselAccent)
                        Text("Theme Engine")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    Picker("Active Theme", selection: $selectedTheme) {
                        Text("Catppuccin Mocha (Default)").tag("catppuccin")
                        Text("Midnight OLED").tag("midnight-oled")
                        Text("Vessel Dark").tag("vessel-dark")
                        Text("Dracula").tag("dracula")
                        Text("Nord").tag("nord")
                        Text("Vessel Light").tag("vessel-light")
                    }
                    .pickerStyle(.radioGroup)

                    Picker("Variant", selection: $selectedVariant) {
                        Text("Mocha").tag("mocha")
                        Text("Latte").tag("latte")
                        Text("Frappe").tag("frappe")
                        Text("Macchiato").tag("macchiato")
                    }
                    .pickerStyle(.segmented)

                    Button("Apply Theme") {
                        Task {
                            await ThemeManager.shared.setTheme(themeId: selectedTheme, variantId: selectedVariant)
                        }
                    }
                    .buttonStyle(.borderedProminent)
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.vesselSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))

                // Torrent & Debrid Streaming
                VStack(alignment: .leading, spacing: 14) {
                    HStack {
                        Image(systemName: "bolt.horizontal.fill")
                            .foregroundColor(.vesselSuccess)
                        Text("Torrent & Debrid Cloud Streaming")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Real-Debrid API Key")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.vesselTextSecondary)
                        SecureField("Paste your Real-Debrid API token here...", text: $rdToken)
                            .textFieldStyle(.plain)
                            .padding(10)
                            .background(Color.vesselBase)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("TorBox API Key")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.vesselTextSecondary)
                        SecureField("Paste your TorBox API token here...", text: $torboxToken)
                            .textFieldStyle(.plain)
                            .padding(10)
                            .background(Color.vesselBase)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.vesselSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))

                // Language Selector
                VStack(alignment: .leading, spacing: 14) {
                    HStack {
                        Image(systemName: "globe")
                            .foregroundColor(.vesselAccentPink)
                        Text("Language & Localization")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    Picker("Application Language", selection: $selectedLang) {
                        Text("English").tag("en")
                        Text("Türkçe").tag("tr")
                        Text("Deutsch").tag("de")
                        Text("Français").tag("fr")
                        Text("Español").tag("es")
                        Text("Português").tag("pt")
                        Text("Русский").tag("ru")
                        Text("日本語").tag("ja")
                        Text("中文").tag("zh")
                        Text("العربية").tag("ar")
                        Text("فارسی").tag("fa")
                        Text("Azərbaycan").tag("az")
                    }
                    .pickerStyle(.menu)
                    .onChange(of: selectedLang) { _, newValue in
                        LocalizationManager.shared.currentLocale = newValue
                    }
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.vesselSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))

                // Backend / Core Configuration
                VStack(alignment: .leading, spacing: 14) {
                    HStack {
                        Image(systemName: "link")
                            .foregroundColor(.vesselInfo)
                        Text("Backend & Core Configuration")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("API Base URL")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.vesselTextSecondary)
                        TextField("http://127.0.0.1:8080", text: $apiBaseUrl)
                            .textFieldStyle(.plain)
                            .padding(10)
                            .background(Color.vesselBase)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }

                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Core Host")
                                .font(.system(size: 12, weight: .medium))
                                .foregroundColor(.vesselTextSecondary)
                            TextField("127.0.0.1", text: $coreHost)
                                .textFieldStyle(.plain)
                                .padding(10)
                                .background(Color.vesselBase)
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                        VStack(alignment: .leading, spacing: 8) {
                            Text("Core Port")
                                .font(.system(size: 12, weight: .medium))
                                .foregroundColor(.vesselTextSecondary)
                            TextField("8080", text: $corePort)
                                .textFieldStyle(.plain)
                                .padding(10)
                                .background(Color.vesselBase)
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                        }
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Core Binary Path")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.vesselTextSecondary)
                        HStack {
                            TextField("/path/to/bin/vessel", text: $coreBinaryPath)
                                .textFieldStyle(.plain)
                                .padding(10)
                                .background(Color.vesselBase)
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                            Button("Browse...") {
                                let panel = NSOpenPanel()
                                panel.canChooseFiles = true
                                panel.canChooseDirectories = false
                                panel.allowsMultipleSelection = false
                                if panel.runModal() == .OK, let url = panel.url {
                                    coreBinaryPath = url.path
                                }
                            }
                        }
                    }

                    HStack {
                        Button("Save & Apply") {
                            VesselAPIClient.shared.setBaseURL(apiBaseUrl)
                            // Persist URL also in daemon for health checks
                            let _ = VesselCoreDaemon.shared.baseURL // triggers computed
                        }
                        .buttonStyle(.borderedProminent)

                        Button("Restart Core") {
                            VesselCoreDaemon.shared.shutdown()
                            VesselCoreDaemon.shared.startHealthCheck()
                        }
                    }
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.vesselSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))

                // Core Daemon Status
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Image(systemName: "server.rack")
                            .foregroundColor(.vesselInfo)
                        Text("Vessel Core Daemon")
                            .font(.system(size: 15, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    HStack(spacing: 8) {
                        Circle()
                            .fill(daemon.isCoreRunning ? Color.vesselSuccess : Color.vesselWarning)
                            .frame(width: 10, height: 10)
                        Text(daemon.coreStatusText)
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(.vesselTextPrimary)
                    }

                    Text("Backend binary: ./bin/vessel (Go Core, SQLite, Vector Engine, gRPC/REST Gateway)")
                        .font(.system(size: 11))
                        .foregroundColor(.vesselTextMuted)
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.vesselSurface)
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(24)
        }
        .background(Color.vesselBase)
    }
}
