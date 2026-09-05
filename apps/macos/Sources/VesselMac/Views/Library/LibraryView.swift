import SwiftUI

public struct LibraryView: View {
    @ObservedObject public var store = LibraryStore.shared
    public var onSelectMedia: (MediaItem) -> Void

    public init(onSelectMedia: @escaping (MediaItem) -> Void) {
        self.onSelectMedia = onSelectMedia
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            // Header & Category Pills
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("My Library")
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundColor(.vesselTextPrimary)
                    Text("Track what you're watching, reading, and planning next")
                        .font(.system(size: 12))
                        .foregroundColor(.vesselTextMuted)
                }

                Spacer()

                // Filter tabs
                HStack(spacing: 8) {
                    filterButton("All", filter: "all")
                    filterButton("Watching", filter: "watching")
                    filterButton("Plan to Watch", filter: "plan")
                    filterButton("Completed", filter: "completed")
                    filterButton("Favorites", filter: "favorites")
                }
            }

            if store.isLoading {
                Spacer()
                HStack {
                    Spacer()
                    ProgressView("Loading library...")
                        .foregroundColor(.vesselTextMuted)
                    Spacer()
                }
                Spacer()
            } else if store.filteredItems.isEmpty {
                Spacer()
                VStack(spacing: 12) {
                    Image(systemName: "books.vertical")
                        .font(.system(size: 40))
                        .foregroundColor(.vesselTextMuted)
                    Text("No items in this collection yet.")
                        .font(.system(size: 14, weight: .medium))
                        .foregroundColor(.vesselTextMuted)
                }
                .frame(maxWidth: .infinity)
                Spacer()
            } else {
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 155, maximum: 175), spacing: 16)], spacing: 16) {
                        ForEach(store.filteredItems) { item in
                            LibraryItemCard(item: item) {
                                let media = MediaItem(
                                    id: item.id,
                                    providerId: "library",
                                    title: item.title,
                                    posterUrl: item.posterUrl,
                                    domain: item.domain ?? 1
                                )
                                onSelectMedia(media)
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
        }
        .padding(24)
        .background(Color.vesselBase)
        .task {
            await store.reload()
        }
    }

    private func filterButton(_ title: String, filter: String) -> some View {
        Button(action: {
            store.selectedFilter = filter
        }) {
            Text(title)
                .font(.system(size: 12, weight: .semibold))
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .background(store.selectedFilter == filter ? Color.vesselAccent : Color.vesselCard)
                .foregroundColor(store.selectedFilter == filter ? Color.vesselTextInverse : Color.vesselTextPrimary)
                .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }
}

public struct LibraryItemCard: View {
    public let item: LibraryItem
    public var action: () -> Void

    @State private var isHovered: Bool = false

    public var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 8) {
                ZStack(alignment: .bottom) {
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

                    // Progress bar if in progress
                    if let p = item.progressPercent, p > 0 {
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                Rectangle()
                                    .fill(Color.black.opacity(0.6))
                                Rectangle()
                                    .fill(Color.vesselAccent)
                                    .frame(width: geo.size.width * CGFloat(min(p / 100.0, 1.0)))
                            }
                        }
                        .frame(height: 4)
                        .clipShape(RoundedRectangle(cornerRadius: 2))
                    }
                }
                .overlay(
                    RoundedRectangle(cornerRadius: 12)
                        .stroke(isHovered ? Color.vesselAccent : Color.vesselBorderSubtle, lineWidth: 1.5)
                )

                // Title & status
                VStack(alignment: .leading, spacing: 2) {
                    Text(item.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundColor(.vesselTextPrimary)
                        .lineLimit(1)

                    Text(item.status.capitalized)
                        .font(.system(size: 10, weight: .bold))
                        .foregroundColor(.vesselAccent)
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
