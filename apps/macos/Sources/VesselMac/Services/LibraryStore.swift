import Foundation
import Combine

public class LibraryStore: ObservableObject {
    public static let shared = LibraryStore()

    @Published public var items: [LibraryItem] = []
    @Published public var selectedFilter: String = "all"
    @Published public var isLoading: Bool = false

    private init() {
        Task {
            await reload()
        }
    }

    @MainActor
    public func reload() async {
        isLoading = true
        defer { isLoading = false }
        do {
            self.items = try await VesselAPIClient.shared.fetchLibrary()
        } catch {
            print("Failed to load library: \(error)")
        }
    }

    public var filteredItems: [LibraryItem] {
        if selectedFilter == "all" {
            return items
        }
        return items.filter { $0.status.lowercased() == selectedFilter.lowercased() }
    }
}
