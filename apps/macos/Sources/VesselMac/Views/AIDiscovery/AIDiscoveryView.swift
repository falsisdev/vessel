import SwiftUI

public struct AIDiscoveryView: View {
    public let defaultDomain: String
    public var onSelectMedia: ((MediaItem) -> Void)?

    @State private var moodPresets: [MoodPreset] = []
    @State private var tasteProfile: TasteProfile?
    @State private var selectedMood: String?
    @State private var selectedDomain: String = "all"
    @State private var promptText: String = ""
    @State private var recommendations: [RecommendationItem] = []
    @State private var isLoading: Bool = false
    @State private var errorMessage: String?

    public init(defaultDomain: String = "all", onSelectMedia: ((MediaItem) -> Void)? = nil) {
        self.defaultDomain = defaultDomain
        self._selectedDomain = State(initialValue: defaultDomain)
        self.onSelectMedia = onSelectMedia
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            // AI Header Box
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 8) {
                            HStack(spacing: 5) {
                                Text("✨")
                                Text("LOCAL AI DISCOVERY")
                                    .font(.system(size: 11, weight: .bold))
                                    .foregroundColor(.vesselAccent)
                            }
                            .padding(.horizontal, 10)
                            .padding(.vertical, 4)
                            .background(Color.vesselAccent.opacity(0.12))
                            .clipShape(Capsule())
                            .overlay(
                                Capsule().stroke(Color.vesselAccent.opacity(0.3), lineWidth: 1)
                            )

                            HStack(spacing: 5) {
                                Circle()
                                    .fill(Color.vesselSuccess)
                                    .frame(width: 6, height: 6)
                                Text("Vessel-Embed-v2.4 (MiniLM-64D Core)")
                                    .font(.system(size: 10, weight: .medium))
                                    .foregroundColor(.vesselTextMuted)
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.vesselSurface.opacity(0.6))
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                        }

                        Text("Discover by Mood & Vibe")
                            .font(.system(size: 20, weight: .bold, design: .rounded))
                            .foregroundColor(.vesselTextPrimary)

                        Text("Your Taste DNA calculated locally on device (100% Offline & Private).")
                            .font(.system(size: 12))
                            .foregroundColor(.vesselTextMuted)
                    }

                    Spacer()

                    // Live Taste DNA tags
                    if let traits = tasteProfile?.activeTraitsEn ?? tasteProfile?.activeTraitsTr {
                        VStack(alignment: .trailing, spacing: 6) {
                            ForEach(traits.prefix(3), id: \.self) { trait in
                                Text(trait)
                                    .font(.system(size: 11, weight: .medium))
                                    .foregroundColor(.vesselTextSecondary)
                                    .padding(.horizontal, 10)
                                    .padding(.vertical, 4)
                                    .background(Color.vesselCard)
                                    .clipShape(RoundedRectangle(cornerRadius: 8))
                                    .overlay(
                                        RoundedRectangle(cornerRadius: 8)
                                            .stroke(Color.vesselBorderSubtle, lineWidth: 1)
                                    )
                            }
                        }
                    }
                }

                // Mood Pills Carousel
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(moodPresets) { mood in
                            Button(action: {
                                if selectedMood == mood.id {
                                    selectedMood = nil
                                } else {
                                    selectedMood = mood.id
                                    triggerDiscovery(moodId: mood.id)
                                }
                            }) {
                                HStack(spacing: 6) {
                                    Text(mood.icon)
                                    Text(mood.titleEn)
                                        .font(.system(size: 12, weight: .semibold))
                                }
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(selectedMood == mood.id ? Color.vesselAccent : Color.vesselCard)
                                .foregroundColor(selectedMood == mood.id ? Color.vesselTextInverse : Color.vesselTextPrimary)
                                .clipShape(Capsule())
                                .overlay(
                                    Capsule().stroke(
                                        selectedMood == mood.id ? Color.vesselAccent : Color.vesselBorderSubtle,
                                        lineWidth: 1
                                    )
                                )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(.vertical, 2)
                }

                // Domain Scope & Prompt Input Bar
                HStack(spacing: 10) {
                    // Domain Filter
                    Picker("Domain", selection: $selectedDomain) {
                        Text("✨ All").tag("all")
                        Text("🎬 Cinema").tag("cinema")
                        Text("📖 Reading").tag("reading")
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 220)

                    // Text Input
                    HStack(spacing: 8) {
                        Image(systemName: "magnifyingglass")
                            .foregroundColor(.vesselTextMuted)
                        TextField("Describe what you want (e.g. 'mind-bending psychological anime')...", text: $promptText)
                            .textFieldStyle(.plain)
                            .foregroundColor(.vesselTextPrimary)
                            .onSubmit {
                                triggerDiscovery()
                            }
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(Color.vesselBase)
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(Color.vesselBorderSubtle, lineWidth: 1)
                    )

                    // Submit Button
                    Button(action: {
                        triggerDiscovery()
                    }) {
                        HStack(spacing: 6) {
                            if isLoading {
                                ProgressView()
                                    .scaleEffect(0.7)
                            } else {
                                Image(systemName: "sparkles")
                            }
                            Text("Find Matches")
                                .font(.system(size: 12, weight: .semibold))
                        }
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(Color.vesselAccent)
                        .foregroundColor(.vesselTextInverse)
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                    }
                    .buttonStyle(.plain)
                    .disabled(isLoading)
                }
            }
            .padding(18)
            .background(Color.vesselSurface)
            .clipShape(RoundedRectangle(cornerRadius: 16))
            .overlay(
                RoundedRectangle(cornerRadius: 16)
                    .stroke(Color.vesselBorderDefault, lineWidth: 1)
            )

            // Results Section
            if !recommendations.isEmpty {
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Text("✨ AI Curated Matches")
                            .font(.system(size: 16, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)

                        Spacer()

                        Button("Clear") {
                            recommendations.removeAll()
                            selectedMood = nil
                        }
                        .font(.system(size: 12))
                        .foregroundColor(.vesselTextMuted)
                        .buttonStyle(.plain)
                    }

                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 14) {
                            ForEach(recommendations) { item in
                                AIMediaCard(item: item) {
                                    onSelectMedia?(item.toMediaItem())
                                }
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
            }
        }
        .task {
            await loadInitialAIData()
        }
    }

    private func loadInitialAIData() async {
        do {
            async let moods = VesselAPIClient.shared.fetchAIMoods()
            async let profile = VesselAPIClient.shared.fetchAITasteProfile()
            let (loadedMoods, loadedProfile) = try await (moods, profile)
            self.moodPresets = loadedMoods
            self.tasteProfile = loadedProfile
        } catch {
            print("Failed to load initial AI data: \(error)")
        }
    }

    private func triggerDiscovery(moodId: String? = nil) {
        let m = moodId ?? selectedMood ?? ""
        isLoading = true
        errorMessage = nil

        Task {
            do {
                let results = try await VesselAPIClient.shared.discoverByMood(
                    query: promptText,
                    mood: m,
                    domain: selectedDomain,
                    limit: 12
                )
                DispatchQueue.main.async {
                    self.recommendations = results
                    self.isLoading = false
                }
            } catch {
                DispatchQueue.main.async {
                    self.errorMessage = error.localizedDescription
                    self.isLoading = false
                }
            }
        }
    }
}

public struct AIMediaCard: View {
    public let item: RecommendationItem
    public var action: () -> Void

    @State private var isHovered: Bool = false

    public var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                ZStack(alignment: .topTrailing) {
                    // Poster image
                    AsyncImage(url: URL(string: item.posterUrl ?? "")) { phase in
                        switch phase {
                        case .success(let image):
                            image
                                .resizable()
                                .aspectRatio(contentMode: .fill)
                        case .failure(_), .empty:
                            Rectangle()
                                .fill(Color.vesselCard)
                                .overlay(
                                    Image(systemName: "film")
                                        .font(.system(size: 28))
                                        .foregroundColor(.vesselTextMuted)
                                )
                        @unknown default:
                            EmptyView()
                        }
                    }
                    .frame(width: 155, height: 230)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

                    // Match badge
                    HStack(spacing: 3) {
                        Image(systemName: "bolt.fill")
                            .font(.system(size: 9))
                        Text("\(item.matchScore ?? 85)%")
                            .font(.system(size: 10, weight: .black))
                    }
                    .padding(.horizontal, 7)
                    .padding(.vertical, 4)
                    .background(Color.vesselBase.opacity(0.85))
                    .foregroundColor(.vesselAccent)
                    .clipShape(Capsule())
                    .padding(8)
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(isHovered ? Color.vesselAccent : Color.vesselBorderSubtle, lineWidth: 1.5)
                )

                // Title & reason
                VStack(alignment: .leading, spacing: 3) {
                    Text(item.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundColor(.vesselTextPrimary)
                        .lineLimit(1)

                    Text("✦ \(item.localizedReason())")
                        .font(.system(size: 11))
                        .foregroundColor(.vesselAccent)
                        .lineLimit(1)
                }
                .frame(width: 155, alignment: .leading)
            }
            .scaleEffect(isHovered ? 1.03 : 1.0)
            .animation(.spring(response: 0.25, dampingFraction: 0.7), value: isHovered)
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            isHovered = hovering
        }
    }
}
