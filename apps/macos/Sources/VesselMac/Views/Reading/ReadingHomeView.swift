import SwiftUI
import AppKit

public struct ReadingHomeView: View {
    public var onSelectMedia: (MediaItem) -> Void

    @State private var catalogs: [CatalogRow] = []
    @State private var isLoading: Bool = true
    @State private var errorMessage: String?
    @State private var localFilePath: String?

    public init(onSelectMedia: @escaping (MediaItem) -> Void) {
        self.onSelectMedia = onSelectMedia
    }

    public var body: some View {
        ScrollView(.vertical, showsIndicators: true) {
            VStack(alignment: .leading, spacing: 24) {
                // Header with Open Local File Action
                HStack {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Manga & E-Books")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundColor(.vesselTextPrimary)
                        Text("Explore webtoons, manga catalogs, or open any local comic & document")
                            .font(.system(size: 12))
                            .foregroundColor(.vesselTextMuted)
                    }

                    Spacer()

                    // Open Local File Button
                    Button(action: selectLocalFile) {
                        HStack(spacing: 8) {
                            Image(systemName: "folder.badge.plus")
                            Text("Open Local File (PDF, CBZ, TXT...)")
                                .font(.system(size: 12, weight: .semibold))
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(Color.vesselCard)
                        .foregroundColor(.vesselTextPrimary)
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                        .overlay(
                            RoundedRectangle(cornerRadius: 10)
                                .stroke(Color.vesselAccent, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                }

                // AI Discovery Header (scoped to Reading)
                AIDiscoveryView(defaultDomain: "reading") { item in
                    onSelectMedia(item)
                }

                if isLoading {
                    HStack {
                        Spacer()
                        ProgressView("Loading Reading Catalogs...")
                            .foregroundColor(.vesselTextMuted)
                        Spacer()
                    }
                    .padding(.vertical, 40)
                } else if let err = errorMessage {
                    Text("Error loading reading catalogs: \(err)")
                        .foregroundColor(.vesselError)
                        .padding()
                } else {
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
            self.catalogs = try await VesselAPIClient.shared.fetchCatalogs(domain: "reading")
            self.isLoading = false
        } catch {
            self.errorMessage = error.localizedDescription
            self.isLoading = false
        }
    }

    private func selectLocalFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.canCreateDirectories = false
        panel.canChooseFiles = true
        panel.allowedContentTypes = [] // allow all formats (pdf, cbz, cbr, docx, txt, md, zip)

        if panel.runModal() == .OK, let url = panel.url {
            let item = MediaItem(
                id: "local:\(url.path)",
                providerId: "local",
                title: url.lastPathComponent,
                posterUrl: nil,
                type: 4,
                typeName: "Local Document",
                domain: 2,
                overview: "Local file: \(url.path)"
            )
            onSelectMedia(item)
        }
    }
}
