import SwiftUI

public struct CinemaHomeView: View {
    public var onSelectMedia: (MediaItem) -> Void

    @State private var catalogs: [CatalogRow] = []
    @State private var isLoading: Bool = true
    @State private var errorMessage: String?

    public init(onSelectMedia: @escaping (MediaItem) -> Void) {
        self.onSelectMedia = onSelectMedia
    }

    public var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 24) {
                // AI Discovery Header (scoped to Cinema)
                AIDiscoveryView(defaultDomain: "cinema") { item in
                    onSelectMedia(item)
                }
                .padding(.top, 4)

                if isLoading {
                    HStack {
                        Spacer()
                        ProgressView("Loading Cinema Catalogs...")
                            .foregroundColor(.vesselTextMuted)
                        Spacer()
                    }
                    .padding(.vertical, 40)
                } else if let err = errorMessage {
                    Text("Error loading catalogs: \(err)")
                        .foregroundColor(.vesselError)
                        .padding()
                } else {
                    // Catalog Rows
                    ForEach(catalogs) { row in
                        CatalogRowView(row: row, onSelectMedia: onSelectMedia)
                    }
                }
            }
            .padding(24)
        }
        .background(Color.vesselBase)
        .task {
            await loadCatalogs()
        }
    }

    private func loadCatalogs() async {
        isLoading = true
        do {
            self.catalogs = try await VesselAPIClient.shared.fetchCatalogs(domain: "cinema")
            self.isLoading = false
        } catch {
            self.errorMessage = error.localizedDescription
            self.isLoading = false
        }
    }
}

public struct CatalogRowView: View {
    public let row: CatalogRow
    public var onSelectMedia: (MediaItem) -> Void

    public var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text(row.name)
                    .font(.system(size: 18, weight: .bold, design: .rounded))
                    .foregroundColor(.vesselTextPrimary)

                if let prov = row.providerName {
                    Text(prov)
                        .font(.system(size: 10, weight: .semibold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(Color.vesselCard)
                        .foregroundColor(.vesselAccent)
                        .clipShape(Capsule())
                }

                Spacer()
            }

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 16) {
                    ForEach(row.items) { item in
                        MediaPosterCard(item: item) {
                            onSelectMedia(item)
                        }
                    }
                }
                .padding(.vertical, 4)
            }
        }
    }
}

public struct MediaPosterCard: View {
    public let item: MediaItem
    public var action: () -> Void

    @State private var isHovered: Bool = false

    public var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                ZStack(alignment: .bottomLeading) {
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
                                        .font(.system(size: 28))
                                        .foregroundColor(.vesselTextMuted)
                                )
                        @unknown default:
                            EmptyView()
                        }
                    }
                    .frame(width: 155, height: 230)
                    .clipShape(RoundedRectangle(cornerRadius: 12))

                    // Rating badge if available
                    if let score = item.score, score > 0 {
                        HStack(spacing: 3) {
                            Image(systemName: "star.fill")
                                .font(.system(size: 8))
                                .foregroundColor(.vesselWarning)
                            Text(String(format: "%.1f", score))
                                .font(.system(size: 10, weight: .bold))
                                .foregroundColor(.vesselTextPrimary)
                        }
                        .padding(.horizontal, 6)
                        .padding(.vertical, 3)
                        .background(Color.vesselBase.opacity(0.85))
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                        .padding(8)
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(isHovered ? Color.vesselAccent : Color.vesselBorderSubtle, lineWidth: 1.5)
                )

                // Title
                VStack(alignment: .leading, spacing: 2) {
                    Text(item.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundColor(.vesselTextPrimary)
                        .lineLimit(1)

                    if let typeName = item.typeName {
                        Text(typeName)
                            .font(.system(size: 11))
                            .foregroundColor(.vesselTextMuted)
                    }
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
