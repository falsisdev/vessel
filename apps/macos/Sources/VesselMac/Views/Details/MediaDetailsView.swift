import SwiftUI

public struct MediaDetailsView: View {
    public let item: MediaItem
    public var onDismiss: () -> Void

    @State private var streams: [StreamSource] = []
    @State private var isLoadingStreams: Bool = true
    @State private var inLibrary: Bool = false

    public init(item: MediaItem, onDismiss: @escaping () -> Void) {
        self.item = item
        self.onDismiss = onDismiss
    }

    public var body: some View {
        ZStack(alignment: .topTrailing) {
            ScrollView(.vertical, showsIndicators: true) {
                VStack(alignment: .leading, spacing: 20) {
                    // Header / Backdrop Banner
                    HStack(alignment: .top, spacing: 24) {
                        // Poster
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
                                        Image(systemName: item.isReading ? "book" : "film")
                                            .font(.system(size: 32))
                                            .foregroundColor(.vesselTextMuted)
                                    )
                            @unknown default:
                                EmptyView()
                            }
                        }
                        .frame(width: 180, height: 260)
                        .clipShape(RoundedRectangle(cornerRadius: 14))
                        .overlay(
                            RoundedRectangle(cornerRadius: 14)
                                .stroke(Color.vesselBorderSubtle, lineWidth: 1)
                        )

                        // Metadata
                        VStack(alignment: .leading, spacing: 10) {
                            Text(item.title)
                                .font(.system(size: 26, weight: .black, design: .rounded))
                                .foregroundColor(.vesselTextPrimary)

                            HStack(spacing: 8) {
                                if let typeName = item.typeName {
                                    Text(typeName)
                                        .font(.system(size: 11, weight: .bold))
                                        .padding(.horizontal, 8)
                                        .padding(.vertical, 4)
                                        .background(Color.vesselAccent.opacity(0.15))
                                        .foregroundColor(.vesselAccent)
                                        .clipShape(RoundedRectangle(cornerRadius: 6))
                                }

                                if let score = item.score, score > 0 {
                                    HStack(spacing: 3) {
                                        Image(systemName: "star.fill")
                                            .font(.system(size: 9))
                                            .foregroundColor(.vesselWarning)
                                        Text(String(format: "%.1f", score))
                                            .font(.system(size: 11, weight: .bold))
                                            .foregroundColor(.vesselTextPrimary)
                                    }
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 4)
                                    .background(Color.vesselCard)
                                    .clipShape(RoundedRectangle(cornerRadius: 6))
                                }
                            }

                            if let overview = item.overview, !overview.isEmpty {
                                Text(overview)
                                    .font(.system(size: 13))
                                    .foregroundColor(.vesselTextSecondary)
                                    .lineSpacing(4)
                                    .padding(.top, 4)
                            }

                            // Library Action Button
                            Button(action: toggleLibrary) {
                                HStack(spacing: 6) {
                                    Image(systemName: inLibrary ? "checkmark" : "plus")
                                    Text(inLibrary ? "In Library" : "Add to Library")
                                }
                                .font(.system(size: 12, weight: .semibold))
                                .padding(.horizontal, 14)
                                .padding(.vertical, 8)
                                .background(inLibrary ? Color.vesselSuccess.opacity(0.2) : Color.vesselCard)
                                .foregroundColor(inLibrary ? Color.vesselSuccess : Color.vesselTextPrimary)
                                .clipShape(RoundedRectangle(cornerRadius: 8))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 8)
                                        .stroke(inLibrary ? Color.vesselSuccess : Color.vesselBorderSubtle, lineWidth: 1)
                                )
                            }
                            .buttonStyle(.plain)
                            .padding(.top, 8)
                        }
                    }

                    Divider()
                        .background(Color.vesselBorderSubtle)

                    // Available Streams / Reading section
                    VStack(alignment: .leading, spacing: 12) {
                        Text(item.isReading ? "Chapters & Reading" : "Available Streams & Sources")
                            .font(.system(size: 16, weight: .bold))
                            .foregroundColor(.vesselTextPrimary)

                        if isLoadingStreams {
                            HStack {
                                ProgressView()
                                    .scaleEffect(0.8)
                                Text("Resolving fast streams...")
                                    .font(.system(size: 12))
                                    .foregroundColor(.vesselTextMuted)
                            }
                            .padding(.vertical, 10)
                        } else if streams.isEmpty {
                            Text("No streams currently available.")
                                .font(.system(size: 12))
                                .foregroundColor(.vesselTextMuted)
                        } else {
                            VStack(spacing: 8) {
                                ForEach(streams) { s in
                                    StreamRowView(stream: s) {
                                        PlayerState.shared.play(stream: s, media: item)
                                    }
                                }
                            }
                        }
                    }
                }
                .padding(28)
            }

            // Close button
            Button(action: onDismiss) {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 22))
                    .foregroundColor(.vesselTextMuted)
            }
            .buttonStyle(.plain)
            .padding(20)
        }
        .frame(minWidth: 640, minHeight: 480)
        .background(Color.vesselBase)
        .task {
            await loadStreams()
        }
    }

    private func loadStreams() async {
        isLoadingStreams = true
        do {
            self.streams = try await VesselAPIClient.shared.fetchStreams(id: item.id, providerId: item.providerId)
            self.isLoadingStreams = false
        } catch {
            self.isLoadingStreams = false
        }
    }

    private func toggleLibrary() {
        inLibrary.toggle()
        Task {
            try? await VesselAPIClient.shared.addToLibrary(item: item)
        }
    }
}

public struct StreamRowView: View {
    public let stream: StreamSource
    public var action: () -> Void

    @State private var isHovered: Bool = false

    public var body: some View {
        Button(action: action) {
            HStack {
                HStack(spacing: 8) {
                    Image(systemName: "play.circle.fill")
                        .foregroundColor(.vesselAccent)
                    Text(stream.name)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundColor(.vesselTextPrimary)
                        .lineLimit(1)
                }

                Spacer()

                HStack(spacing: 8) {
                    if let q = stream.quality {
                        Text(q)
                            .font(.system(size: 10, weight: .bold))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.vesselAccent.opacity(0.15))
                            .foregroundColor(.vesselAccent)
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }

                    if let debrid = stream.isDebrid, debrid {
                        Text("⚡ DEBRID")
                            .font(.system(size: 9, weight: .black))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.vesselSuccess.opacity(0.15))
                            .foregroundColor(.vesselSuccess)
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                    }

                    Image(systemName: "arrow.right.circle")
                        .foregroundColor(.vesselTextMuted)
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .background(isHovered ? Color.vesselElevated : Color.vesselCard)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isHovered ? Color.vesselAccent : Color.vesselBorderSubtle, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .onHover { hovering in
            isHovered = hovering
        }
    }
}
