import SwiftUI

public struct LiveTVView: View {
    @ObservedObject private var theme = ThemeManager.shared
    @ObservedObject private var loc = LocalizationManager.shared

    @State private var channels: [MediaItem] = []
    @State private var searchText: String = ""
    @State private var selectedCountry: String = "ALL"
    @State private var isLoading: Bool = true
    @State private var errorMessage: String?

    public init() {}

    public var filteredChannels: [MediaItem] {
        channels.filter { ch in
            let matchesSearch = searchText.isEmpty || ch.title.localizedCaseInsensitiveContains(searchText) || (ch.overview?.localizedCaseInsensitiveContains(searchText) ?? false)
            let matchesCountry = (selectedCountry == "ALL") || (ch.channelCountry == selectedCountry)
            return matchesSearch && matchesCountry
        }
    }

    public var availableCountries: [String] {
        var set = Set<String>()
        for ch in channels {
            let c = ch.channelCountry
            if !c.isEmpty && c != "GLOBAL" {
                set.insert(c)
            }
        }
        return ["ALL"] + Array(set).sorted()
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            // Header & Search
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(loc.t("iptv_guide_title"))
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundColor(theme.tokens.textPrimary)
                    Text(loc.t("iptv_guide_subtitle"))
                        .font(.system(size: 12))
                        .foregroundColor(theme.tokens.textMuted)
                }

                Spacer()

                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(theme.tokens.textMuted)
                    TextField(loc.t("search_placeholder"), text: $searchText)
                        .textFieldStyle(.plain)
                        .foregroundColor(theme.tokens.textPrimary)
                    if !searchText.isEmpty {
                        Button(action: { searchText = "" }) {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(theme.tokens.textMuted)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(theme.tokens.bgCard)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(theme.tokens.borderSubtle, lineWidth: 1)
                )
                .frame(width: 280)
            }

            // Country Filter Pills
            if !availableCountries.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(availableCountries, id: \.self) { country in
                            let isSelected = selectedCountry == country
                            Button(action: {
                                selectedCountry = country
                            }) {
                                HStack(spacing: 4) {
                                    if country == "ALL" {
                                        Text("🌍")
                                        Text("All")
                                    } else {
                                        Text(flagEmoji(country))
                                        Text(country)
                                    }
                                }
                                .font(.system(size: 12, weight: .semibold))
                                .padding(.horizontal, 12)
                                .padding(.vertical, 6)
                                .background(isSelected ? theme.tokens.accentPrimary : theme.tokens.bgCard)
                                .foregroundColor(isSelected ? Color(hex: "#11111B") : theme.tokens.textPrimary)
                                .clipShape(Capsule())
                                .overlay(
                                    Capsule().stroke(isSelected ? theme.tokens.accentPrimary : theme.tokens.borderSubtle, lineWidth: 1)
                                )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(.vertical, 2)
                }
            }

            if isLoading {
                Spacer()
                HStack {
                    Spacer()
                    ProgressView()
                        .scaleEffect(1.2)
                    Spacer()
                }
                Spacer()
            } else if let err = errorMessage {
                Spacer()
                VStack(spacing: 8) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 32))
                        .foregroundColor(Color.vesselError)
                    Text(err)
                        .font(.system(size: 13))
                        .foregroundColor(theme.tokens.textMuted)
                }
                .frame(maxWidth: .infinity)
                Spacer()
            } else if filteredChannels.isEmpty {
                Spacer()
                VStack(spacing: 8) {
                    Image(systemName: "tv.slash")
                        .font(.system(size: 36))
                        .foregroundColor(theme.tokens.textMuted)
                    Text("No channels found")
                        .font(.system(size: 14, weight: .medium))
                        .foregroundColor(theme.tokens.textMuted)
                }
                .frame(maxWidth: .infinity)
                Spacer()
            } else {
                // Channel Grid
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 180, maximum: 220), spacing: 14)], spacing: 14) {
                        ForEach(filteredChannels) { ch in
                            ChannelCard(channel: ch) {
                                playChannel(ch)
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
        }
        .padding(24)
        .background(theme.tokens.bgBase)
        .task {
            await loadChannels()
        }
    }

    private func loadChannels() async {
        isLoading = true
        errorMessage = nil
        do {
            let chs = try await VesselAPIClient.shared.fetchIPTVChannels(country: "ALL")
            DispatchQueue.main.async {
                self.channels = chs
                self.isLoading = false
            }
        } catch {
            DispatchQueue.main.async {
                self.errorMessage = error.localizedDescription
                self.isLoading = false
            }
        }
    }

    private func playChannel(_ ch: MediaItem) {
        let provId = ch.providerId.isEmpty ? "com.vessel.iptv" : ch.providerId
        Task {
            do {
                let streams = try await VesselAPIClient.shared.fetchStreams(id: ch.id, providerId: provId)
                if let first = streams.first {
                    DispatchQueue.main.async {
                        PlayerState.shared.play(stream: first, media: ch)
                    }
                }
            } catch {
                print("Failed to resolve IPTV stream: \(error)")
            }
        }
    }

    private func flagEmoji(_ countryCode: String) -> String {
        let base: UInt32 = 127397
        let upper = countryCode.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard upper.count == 2 else { return "" }
        var scalars = String.UnicodeScalarView()
        for v in upper.unicodeScalars {
            guard let scalar = UnicodeScalar(base + v.value) else { return "" }
            scalars.append(scalar)
        }
        let flag = String(scalars)
        return flag
    }
}

public struct ChannelCard: View {
    @ObservedObject private var theme = ThemeManager.shared
    public let channel: MediaItem
    public var onSelect: () -> Void

    @State private var isHovered: Bool = false

    public init(channel: MediaItem, onSelect: @escaping () -> Void) {
        self.channel = channel
        self.onSelect = onSelect
    }

    public var body: some View {
        Button(action: onSelect) {
            VStack(alignment: .leading, spacing: 10) {
                ZStack {
                    RoundedRectangle(cornerRadius: 10)
                        .fill(theme.tokens.bgCard)
                        .frame(height: 90)

                    if let logo = channel.posterUrl, !logo.isEmpty {
                        AsyncImage(url: URL(string: logo)) { phase in
                            switch phase {
                            case .success(let image):
                                image
                                    .resizable()
                                    .aspectRatio(contentMode: .fit)
                                    .padding(12)
                            case .failure(_), .empty:
                                Image(systemName: "tv")
                                    .font(.system(size: 24))
                                    .foregroundColor(theme.tokens.textMuted)
                            @unknown default:
                                EmptyView()
                            }
                        }
                    } else {
                        Image(systemName: "tv")
                            .font(.system(size: 24))
                            .foregroundColor(theme.tokens.textMuted)
                    }

                    if isHovered {
                        Circle()
                            .fill(theme.tokens.accentPrimary)
                            .frame(width: 36, height: 36)
                            .overlay(
                                Image(systemName: "play.fill")
                                    .font(.system(size: 14))
                                    .foregroundColor(Color(hex: "#11111B"))
                            )
                            .transition(.scale.combined(with: .opacity))
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(isHovered ? theme.tokens.accentPrimary : theme.tokens.borderSubtle, lineWidth: 1.5)
                )

                // Channel Info
                VStack(alignment: .leading, spacing: 2) {
                    Text(channel.title)
                        .font(.system(size: 12, weight: .bold))
                        .foregroundColor(theme.tokens.textPrimary)
                        .lineLimit(1)

                    HStack(spacing: 6) {
                        Text(channel.channelCountry)
                            .font(.system(size: 9, weight: .bold))
                            .foregroundColor(theme.tokens.accentPrimary)

                        if let ov = channel.overview, !ov.isEmpty {
                            Text(ov)
                                .font(.system(size: 9))
                                .foregroundColor(theme.tokens.textMuted)
                                .lineLimit(1)
                        }
                    }
                }
            }
            .scaleEffect(isHovered ? 1.02 : 1.0)
            .animation(.spring(response: 0.25, dampingFraction: 0.7), value: isHovered)
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            isHovered = hovering
        }
    }
}
