import XCTest
import SwiftUI
@testable import VesselMac

final class VesselMacTests: XCTestCase {
    func testMediaItemReadingDomain() {
        let movie = MediaItem(id: "1", providerId: "cinemasis", title: "Inception", type: 1, domain: 1)
        XCTAssertFalse(movie.isReading)

        let manga = MediaItem(id: "2", providerId: "mangile", title: "Chainsaw Man", type: 4, domain: 2)
        XCTAssertTrue(manga.isReading)
    }

    func testMoodPresetLocalization() {
        let preset = MoodPreset(
            id: "mind_bending",
            icon: "🔮",
            titleTr: "Zihin Yakan & Ters Köşe",
            titleEn: "Mind-Bending & Plot Twists",
            descTr: "Kafanızı karıştıracak yapımlar",
            descEn: "Twists and psychology"
        )
        XCTAssertEqual(preset.localizedTitle(isTurkish: true), "Zihin Yakan & Ters Köşe")
        XCTAssertEqual(preset.localizedTitle(isTurkish: false), "Mind-Bending & Plot Twists")
    }

    func testRecommendationItemConversion() {
        let rec = RecommendationItem(
            id: "tt123",
            providerId: "cinemasis",
            title: "Dark",
            posterUrl: "https://example.com/dark.jpg",
            domain: "cinema",
            type: "series",
            matchScore: 98,
            reasonTr: "Zaman yolculuğu ve gizem",
            reasonEn: "Time travel and deep mystery",
            overview: "A missing child sets four families on a frantic hunt."
        )
        XCTAssertEqual(rec.localizedReason(isTurkish: true), "Zaman yolculuğu ve gizem")
        XCTAssertEqual(rec.localizedReason(isTurkish: false), "Time travel and deep mystery")

        let media = rec.toMediaItem()
        XCTAssertEqual(media.id, "tt123")
        XCTAssertEqual(media.title, "Dark")
        XCTAssertEqual(media.domain, 1)
        XCTAssertFalse(media.isReading)
    }

    func testCatppuccinThemeColors() {
        XCTAssertNotNil(Color.vesselBase)
        XCTAssertNotNil(Color.vesselSurface)
        XCTAssertNotNil(Color.vesselCard)
        XCTAssertNotNil(Color.vesselAccent)
    }
}
