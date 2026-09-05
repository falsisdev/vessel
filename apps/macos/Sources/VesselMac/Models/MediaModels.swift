import Foundation

public struct MediaItem: Identifiable, Codable, Hashable {
    public let id: String
    public let providerId: String
    public let title: String
    public let posterUrl: String?
    public let type: Int
    public let typeName: String?
    public let domain: Int
    public let overview: String?
    public let score: Double?
    public let releaseDate: String?

    public init(
        id: String,
        providerId: String = "",
        title: String,
        posterUrl: String? = nil,
        type: Int = 1,
        typeName: String? = nil,
        domain: Int = 1,
        overview: String? = nil,
        score: Double? = nil,
        releaseDate: String? = nil
    ) {
        self.id = id
        self.providerId = providerId
        self.title = title
        self.posterUrl = posterUrl
        self.type = type
        self.typeName = typeName
        self.domain = domain
        self.overview = overview
        self.score = score
        self.releaseDate = releaseDate
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: DynamicCodingKeys.self)

        self.id = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "id")!))
            ?? String((try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "id")!)) ?? 0)

        self.providerId = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "provider_id")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "provider")!))
            ?? ""

        self.title = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "title")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "name")!))
            ?? "Untitled"

        self.posterUrl = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "poster_url")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "poster")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "logo")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "thumbnail")!))

        self.type = (try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "type")!)) ?? 1
        self.typeName = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "type_name")!)

        if let dInt = try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "domain")!) {
            self.domain = dInt
        } else if let dStr = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "domain")!) {
            switch dStr.lowercased() {
            case "reading": self.domain = 2
            case "iptv", "live": self.domain = 7
            default: self.domain = 1
            }
        } else {
            self.domain = 1
        }

        self.overview = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "overview")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "description")!))

        if let s = try? container.decode(Double.self, forKey: DynamicCodingKeys(stringValue: "score")!) {
            self.score = s
        } else if let r = try? container.decode(Double.self, forKey: DynamicCodingKeys(stringValue: "rating")!) {
            self.score = r
        } else if let rStr = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "rating")!), let rD = Double(rStr) {
            self.score = rD
        } else {
            self.score = nil
        }

        if let y = try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "year")!) {
            self.releaseDate = String(y)
        } else if let rd = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "release_date")!) {
            self.releaseDate = rd
        } else {
            self.releaseDate = nil
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: DynamicCodingKeys.self)
        try container.encode(id, forKey: DynamicCodingKeys(stringValue: "id")!)
        try container.encode(providerId, forKey: DynamicCodingKeys(stringValue: "provider_id")!)
        try container.encode(title, forKey: DynamicCodingKeys(stringValue: "title")!)
        try container.encodeIfPresent(posterUrl, forKey: DynamicCodingKeys(stringValue: "poster_url")!)
        try container.encode(type, forKey: DynamicCodingKeys(stringValue: "type")!)
        try container.encodeIfPresent(typeName, forKey: DynamicCodingKeys(stringValue: "type_name")!)
        try container.encode(domain, forKey: DynamicCodingKeys(stringValue: "domain")!)
        try container.encodeIfPresent(overview, forKey: DynamicCodingKeys(stringValue: "overview")!)
        try container.encodeIfPresent(score, forKey: DynamicCodingKeys(stringValue: "score")!)
        try container.encodeIfPresent(releaseDate, forKey: DynamicCodingKeys(stringValue: "release_date")!)
    }

    public var isReading: Bool {
        return domain == 2 || type == 4 || type == 5 || type == 6
    }

    public var channelCountry: String {
        if let atIdx = id.firstIndex(of: "@"), let dotIdx = id[..<atIdx].lastIndex(of: ".") {
            let code = id[id.index(after: dotIdx)..<atIdx]
            return code.uppercased()
        }
        if let ov = overview?.uppercased() {
            for c in ["TR", "AZ", "DE", "FR", "US", "UK", "ES", "IT", "RU"] {
                if ov.contains(" \(c) ") || ov.contains("• \(c) ") {
                    return c
                }
            }
        }
        return "GLOBAL"
    }
}

public struct CatalogRow: Identifiable, Codable {
    public let id: String
    public let providerId: String
    public let providerName: String?
    public let name: String
    public let catalogType: String
    public let domain: Int
    public let items: [MediaItem]

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: DynamicCodingKeys.self)

        self.id = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "id")!)) ?? UUID().uuidString
        self.providerId = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "provider_id")!)) ?? ""
        self.providerName = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "provider_name")!)

        self.name = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "title")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "name")!))
            ?? "Catalog"

        self.catalogType = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "catalog_type")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "type")!))
            ?? "general"

        self.domain = (try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "domain")!)) ?? 1

        if let rawItems = try? container.decode([MediaItem].self, forKey: DynamicCodingKeys(stringValue: "items")!) {
            self.items = rawItems
        } else {
            self.items = []
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: DynamicCodingKeys.self)
        try container.encode(id, forKey: DynamicCodingKeys(stringValue: "id")!)
        try container.encode(providerId, forKey: DynamicCodingKeys(stringValue: "provider_id")!)
        try container.encodeIfPresent(providerName, forKey: DynamicCodingKeys(stringValue: "provider_name")!)
        try container.encode(name, forKey: DynamicCodingKeys(stringValue: "title")!)
        try container.encode(catalogType, forKey: DynamicCodingKeys(stringValue: "catalog_type")!)
        try container.encode(domain, forKey: DynamicCodingKeys(stringValue: "domain")!)
        try container.encode(items, forKey: DynamicCodingKeys(stringValue: "items")!)
    }
}

public struct StreamSource: Identifiable, Codable, Hashable {
    public let id: String
    public let name: String
    public let url: String
    public let quality: String?
    public let sizeBytes: Int64?
    public let seeders: Int?
    public let isDebrid: Bool?

    public init(id: String, name: String, url: String, quality: String? = nil, sizeBytes: Int64? = nil, seeders: Int? = nil, isDebrid: Bool? = nil) {
        self.id = id
        self.name = name
        self.url = url
        self.quality = quality
        self.sizeBytes = sizeBytes
        self.seeders = seeders
        self.isDebrid = isDebrid
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: DynamicCodingKeys.self)
        self.id = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "id")!)) ?? UUID().uuidString
        self.name = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "title")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "name")!))
            ?? "Stream"
        self.url = (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "url")!))
            ?? (try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "stream_url")!))
            ?? ""
        self.quality = try? container.decode(String.self, forKey: DynamicCodingKeys(stringValue: "quality")!)
        self.sizeBytes = try? container.decode(Int64.self, forKey: DynamicCodingKeys(stringValue: "size_bytes")!)
        self.seeders = try? container.decode(Int.self, forKey: DynamicCodingKeys(stringValue: "seeders")!)
        self.isDebrid = try? container.decode(Bool.self, forKey: DynamicCodingKeys(stringValue: "is_debrid")!)
    }
}

public struct PluginInfo: Identifiable, Codable {
    public let id: String
    public let name: String
    public let version: String?
    public let description: String?
    public let author: String?
    public let domain: Int?
    public let enabled: Bool
    public let isBuiltin: Bool?

    enum CodingKeys: String, CodingKey {
        case id, name, version, description, author, domain, enabled
        case isBuiltin = "is_builtin"
    }

    public var domainName: String {
        switch domain {
        case 1: return "Cinema"
        case 2: return "Reading"
        case 7: return "Live TV"
        default: return "Universal"
        }
    }
}

public struct AvailablePluginInfo: Identifiable, Codable {
    public let id: String
    public let name: String
    public let version: String?
    public let description: String?
    public let author: String?
    public let domain: String?
    public let installed: Bool?
    public let isBuiltin: Bool?
    public let languageDisplay: String?

    enum CodingKeys: String, CodingKey {
        case id, name, version, description, author, domain, installed
        case isBuiltin = "is_builtin"
        case languageDisplay = "language_display"
    }
}

public struct ChapterData: Codable {
    public let id: String?
    public let title: String?
    public let pages: [String]?
    public let contentText: String?
    public let pdfUrl: String?

    enum CodingKeys: String, CodingKey {
        case id, title, pages
        case contentText = "content_text"
        case pdfUrl = "pdf_url"
    }
}

public struct LibraryItem: Identifiable, Codable, Hashable {
    public let id: String
    public let providerId: String?
    public let mediaId: String?
    public let title: String
    public let posterUrl: String?
    public let status: String
    public let progressPercent: Double?
    public let domain: Int?
    public let lastUpdated: String?

    enum CodingKeys: String, CodingKey {
        case id
        case providerId = "provider_id"
        case mediaId = "media_id"
        case title
        case posterUrl = "poster_url"
        case status
        case progressPercent = "progress_percent"
        case domain
        case lastUpdated = "last_updated"
    }
}

// Helper for dynamic key decoding
private struct DynamicCodingKeys: CodingKey {
    var stringValue: String
    init?(stringValue: String) { self.stringValue = stringValue }
    var intValue: Int?
    init?(intValue: Int) { return nil }
}
