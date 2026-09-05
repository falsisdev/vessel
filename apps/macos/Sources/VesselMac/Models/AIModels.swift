import Foundation

public struct MoodPreset: Identifiable, Codable, Hashable {
    public let id: String
    public let icon: String
    public let titleTr: String
    public let titleEn: String
    public let descTr: String
    public let descEn: String

    enum CodingKeys: String, CodingKey {
        case id
        case icon
        case titleTr = "title_tr"
        case titleEn = "title_en"
        case descTr = "desc_tr"
        case descEn = "desc_en"
    }

    public func localizedTitle(isTurkish: Bool = false) -> String {
        return isTurkish ? titleTr : titleEn
    }
}

public struct TasteProfile: Codable {
    public let activeTraitIds: [String]?
    public let activeTraitsTr: [String]?
    public let activeTraitsEn: [String]?
    public let topKeywords: [String]?
    public let totalItemsCount: Int?
    public let tasteAffinity: [String: Int]?

    enum CodingKeys: String, CodingKey {
        case activeTraitIds = "active_trait_ids"
        case activeTraitsTr = "active_traits_tr"
        case activeTraitsEn = "active_traits_en"
        case topKeywords = "top_keywords"
        case totalItemsCount = "total_items_count"
        case tasteAffinity = "taste_affinity"
    }
}

public struct RecommendationItem: Identifiable, Codable, Hashable {
    public let id: String
    public let providerId: String
    public let title: String
    public let posterUrl: String?
    public let domain: String?
    public let type: String?
    public let matchScore: Int?
    public let reasonTr: String?
    public let reasonEn: String?
    public let overview: String?

    enum CodingKeys: String, CodingKey {
        case id
        case providerId = "provider_id"
        case title
        case posterUrl = "poster_url"
        case domain
        case type
        case matchScore = "match_score"
        case reasonTr = "reason_tr"
        case reasonEn = "reason_en"
        case overview
    }

    public func localizedReason(isTurkish: Bool = false) -> String {
        let score = matchScore ?? 85
        let defaultReason = isTurkish ? "%\(score) Eşleşme" : "\(score)% Match"
        if isTurkish {
            return reasonTr ?? defaultReason
        }
        return reasonEn ?? defaultReason
    }

    public func toMediaItem() -> MediaItem {
        let isReading = domain == "reading" || type == "manga" || type == "webtoon"
        return MediaItem(
            id: id,
            providerId: providerId,
            title: title,
            posterUrl: posterUrl,
            type: isReading ? 4 : 1,
            typeName: type ?? (isReading ? "Manga" : "Movie"),
            domain: isReading ? 2 : 1,
            overview: overview
        )
    }
}
