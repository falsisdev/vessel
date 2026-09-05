import Foundation

public class VesselAPIClient {
    public static let shared = VesselAPIClient()

    private init() {}

    private var computedBaseURL: URL {
        let saved = UserDefaults.standard.string(forKey: "vessel_api_base_url") ?? "http://127.0.0.1:8080"
        return URL(string: saved) ?? URL(string: "http://127.0.0.1:8080")!
    }

    public func get<T: Decodable>(path: String, queryItems: [URLQueryItem]? = nil) async throws -> T {
        var components = URLComponents(url: computedBaseURL.appendingPathComponent(path), resolvingAgainstBaseURL: true)!
        if let queryItems = queryItems {
            components.queryItems = queryItems
        }
        guard let url = components.url else {
            throw URLError(.badURL)
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.timeoutInterval = 15.0

        let (data, response) = try await URLSession.shared.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse, (200...299).contains(httpResponse.statusCode) else {
            throw URLError(.badServerResponse)
        }

        let decoder = JSONDecoder()
        return try decoder.decode(T.self, from: data)
    }

    public func post<T: Decodable, B: Encodable>(path: String, body: B) async throws -> T {
        let url = computedBaseURL.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(body)
        request.timeoutInterval = 20.0

        let (data, response) = try await URLSession.shared.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse, (200...299).contains(httpResponse.statusCode) else {
            throw URLError(.badServerResponse)
        }

        let decoder = JSONDecoder()
        return try decoder.decode(T.self, from: data)
    }

    // MARK: - Generic Standard Response
    public struct StandardResponse: Codable {
        public let success: Bool?
        public let message: String?
    }

    // MARK: - Catalogs
    public struct CatalogsResponse: Codable {
        public let catalogs: [CatalogRow]?
    }

    public func fetchCatalogs(domain: String = "cinema") async throws -> [CatalogRow] {
        let response: CatalogsResponse = try await get(path: "api/catalogs", queryItems: [
            URLQueryItem(name: "domain", value: domain)
        ])
        return response.catalogs ?? []
    }

    // MARK: - Search
    public struct SearchResponse: Codable {
        public let items: [MediaItem]?
        public let total: Int?
    }

    public func search(query: String, domain: String = "all") async throws -> [MediaItem] {
        var queryItems = [
            URLQueryItem(name: "q", value: query),
            URLQueryItem(name: "query", value: query)
        ]
        if domain != "all" {
            queryItems.append(URLQueryItem(name: "domain", value: domain))
        }
        let res: SearchResponse = try await get(path: "api/search", queryItems: queryItems)
        return res.items ?? []
    }

    // MARK: - Streams
    public struct StreamsResponse: Codable {
        public let streams: [StreamSource]?
    }

    public func fetchStreams(id: String, providerId: String, season: Int? = nil, episode: Int? = nil) async throws -> [StreamSource] {
        var query = [
            URLQueryItem(name: "id", value: id),
            URLQueryItem(name: "provider_id", value: providerId)
        ]
        if let s = season { query.append(URLQueryItem(name: "season", value: String(s))) }
        if let e = episode { query.append(URLQueryItem(name: "episode", value: String(e))) }

        let response: StreamsResponse = try await get(path: "api/streams", queryItems: query)
        return response.streams ?? []
    }

    // MARK: - Local AI Engine
    public struct MoodsResponse: Codable {
        public let moods: [MoodPreset]?
    }

    public func fetchAIMoods() async throws -> [MoodPreset] {
        let response: MoodsResponse = try await get(path: "api/ai/moods")
        return response.moods ?? []
    }

    public func fetchAITasteProfile() async throws -> TasteProfile {
        return try await get(path: "api/ai/taste-profile")
    }

    public struct AIDiscoverRequest: Codable {
        public let query: String
        public let mood: String
        public let domain: String
        public let limit: Int
    }

    public struct RecommendationsResponse: Codable {
        public let recommendations: [RecommendationItem]?
    }

    public func discoverByMood(query: String = "", mood: String = "", domain: String = "all", limit: Int = 12) async throws -> [RecommendationItem] {
        let req = AIDiscoverRequest(query: query, mood: mood, domain: domain, limit: limit)
        let res: RecommendationsResponse = try await post(path: "api/ai/discover", body: req)
        return res.recommendations ?? []
    }

    // MARK: - IPTV Channels
    public func fetchIPTVChannels(country: String = "ALL") async throws -> [MediaItem] {
        let q = country == "ALL" ? "popular" : "country:\(country)"
        let res: SearchResponse = try await get(path: "api/search", queryItems: [
            URLQueryItem(name: "domain", value: "7"),
            URLQueryItem(name: "query", value: q),
            URLQueryItem(name: "q", value: q)
        ])
        return res.items ?? []
    }

    // MARK: - Plugins Hub
    public struct PluginsResponse: Codable {
        public let plugins: [PluginInfo]?
    }

    public struct AvailablePluginsResponse: Codable {
        public let plugins: [AvailablePluginInfo]?
    }

    public func fetchPlugins() async throws -> [PluginInfo] {
        let res: PluginsResponse = try await get(path: "api/plugins")
        return res.plugins ?? []
    }

    public func fetchAvailablePlugins() async throws -> [AvailablePluginInfo] {
        let res: AvailablePluginsResponse = try await get(path: "api/plugins/available")
        return res.plugins ?? []
    }

    public struct TogglePluginRequest: Codable {
        public let id: String
        public let enabled: Bool
    }

    public func togglePlugin(id: String, enabled: Bool) async throws {
        let req = TogglePluginRequest(id: id, enabled: enabled)
        let _: StandardResponse = try await post(path: "api/plugins/toggle", body: req)
    }

    public struct InstallPluginRequest: Codable {
        public let id: String?
        public let source: String?
        public let path: String?
    }

    public func installPlugin(id: String? = nil, source: String? = nil, path: String? = nil) async throws {
        let req = InstallPluginRequest(id: id, source: source, path: path)
        let _: StandardResponse = try await post(path: "api/plugins/install", body: req)
    }

    // MARK: - Debrid Streaming Configuration
    public struct DebridConfigRequest: Codable {
        public let provider: String
        public let api_key: String
    }

    public func configureDebrid(provider: String, apiKey: String) async throws {
        let req = DebridConfigRequest(provider: provider, api_key: apiKey)
        let _: StandardResponse = try await post(path: "api/debrid/configure", body: req)
    }

    // MARK: - Reading Content
    public func fetchChapter(id: String, providerId: String, chapterNum: Int? = nil, chapterId: String? = nil) async throws -> ChapterData {
        var query = [
            URLQueryItem(name: "provider", value: providerId),
            URLQueryItem(name: "media", value: id)
        ]
        if let cNum = chapterNum { query.append(URLQueryItem(name: "chapter_num", value: String(cNum))) }
        if let cId = chapterId { query.append(URLQueryItem(name: "chapter", value: cId)) }
        return try await get(path: "api/chapter", queryItems: query)
    }

    // MARK: - Library
    public struct LibraryResponse: Codable {
        public let items: [LibraryItem]?
        public let total: Int?
    }

    public func fetchLibrary() async throws -> [LibraryItem] {
        let response: LibraryResponse = try await get(path: "api/library")
        return response.items ?? []
    }

    public struct AddLibraryRequest: Codable {
        public let id: String
        public let provider_id: String
        public let title: String
        public let poster_url: String?
        public let domain: Int
    }

    public func addToLibrary(item: MediaItem) async throws {
        let req = AddLibraryRequest(
            id: item.id,
            provider_id: item.providerId,
            title: item.title,
            poster_url: item.posterUrl,
            domain: item.domain
        )
        let _: StandardResponse = try await post(path: "api/library/add", body: req)
    }

    public struct ProgressRequest: Codable {
        public let id: String
        public let provider_id: String
        public let title: String
        public let progress_percent: Double
        public let position_seconds: Double
    }

    public func updatePlaybackProgress(item: MediaItem, percent: Double, positionSeconds: Double) async throws {
        let req = ProgressRequest(
            id: item.id,
            provider_id: item.providerId,
            title: item.title,
            progress_percent: percent,
            position_seconds: positionSeconds
        )
        let _: StandardResponse = try await post(path: "api/library/progress", body: req)
    }

    public func setBaseURL(_ urlString: String) {
        UserDefaults.standard.set(urlString, forKey: "vessel_api_base_url")
    }
}
