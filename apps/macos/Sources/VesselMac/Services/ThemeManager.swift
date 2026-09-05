import SwiftUI
import Combine

public struct ThemeTokens {
    public var bgBase: Color
    public var bgSurface: Color
    public var bgCard: Color
    public var bgElevated: Color
    public var bgSidebar: Color
    public var accentPrimary: Color
    public var accentSecondary: Color
    public var textPrimary: Color
    public var textSecondary: Color
    public var textMuted: Color
    public var borderSubtle: Color
    public var borderDefault: Color

    public static let catppuccinMocha = ThemeTokens(
        bgBase: Color(hex: "#1E1E2E"),
        bgSurface: Color(hex: "#181825"),
        bgCard: Color(hex: "#313244"),
        bgElevated: Color(hex: "#313244"),
        bgSidebar: Color(hex: "#181825"),
        accentPrimary: Color(hex: "#CBA6F7"),
        accentSecondary: Color(hex: "#F5C2E7"),
        textPrimary: Color(hex: "#CDD6F4"),
        textSecondary: Color(hex: "#BAC2DE"),
        textMuted: Color(hex: "#A6ADC8"),
        borderSubtle: Color(hex: "#CDD6F4").opacity(0.08),
        borderDefault: Color(hex: "#CDD6F4").opacity(0.16)
    )
}

public struct AvailableTheme: Identifiable, Codable {
    public let id: String
    public let name: String
    public let description: String?
    public let author: String?
    public let activeVariant: String?
    public let isBuiltin: Bool?

    enum CodingKeys: String, CodingKey {
        case id, name, description, author
        case activeVariant = "active_variant"
        case isBuiltin = "is_builtin"
    }
}

public class ThemeManager: ObservableObject {
    public static let shared = ThemeManager()

    @Published public var tokens: ThemeTokens = .catppuccinMocha
    @Published public var activeThemeId: String = "catppuccin"
    @Published public var activeVariantId: String = "mocha"
    @Published public var availableThemes: [AvailableTheme] = []

    private init() {
        Task {
            await reloadThemes()
        }
    }

    @MainActor
    public func reloadThemes() async {
        do {
            struct ThemesResponse: Codable {
                let themes: [AvailableTheme]?
            }
            let res: ThemesResponse = try await VesselAPIClient.shared.get(path: "api/themes")
            self.availableThemes = res.themes ?? []

            struct ActiveThemeResponse: Codable {
                let theme: AvailableTheme?
                let active_variant: String?
                let tokens: [String: String]?
            }
            let active: ActiveThemeResponse = try await VesselAPIClient.shared.get(path: "api/theme/active")
            if let th = active.theme {
                self.activeThemeId = th.id
            }
            if let v = active.active_variant {
                self.activeVariantId = v
            }
            if let t = active.tokens {
                self.applyTokens(t)
            }
        } catch {
            print("ThemeManager reload error: \(error)")
        }
    }

    @MainActor
    public func setTheme(themeId: String, variantId: String = "") async {
        do {
            struct SetThemeReq: Codable {
                let theme_id: String
                let variant_id: String
            }
            struct ActiveThemeResponse: Codable {
                let active_variant: String?
                let tokens: [String: String]?
            }
            let req = SetThemeReq(theme_id: themeId, variant_id: variantId)
            let res: ActiveThemeResponse = try await VesselAPIClient.shared.post(path: "api/theme/active", body: req)
            self.activeThemeId = themeId
            if let v = res.active_variant {
                self.activeVariantId = v
            }
            if let t = res.tokens {
                self.applyTokens(t)
            }
        } catch {
            print("Failed to set theme: \(error)")
        }
    }

    private func applyTokens(_ dict: [String: String]) {
        let base = Color(hex: dict["bg-base"] ?? "#1E1E2E")
        let surface = Color(hex: dict["bg-surface"] ?? "#181825")
        let card = Color(hex: dict["bg-card"] ?? "#313244")
        let elevated = Color(hex: dict["bg-elevated"] ?? "#313244")
        let sidebar = Color(hex: dict["bg-sidebar"] ?? "#181825")
        let primary = Color(hex: dict["accent-primary"] ?? "#CBA6F7")
        let secondary = Color(hex: dict["accent-secondary"] ?? "#F5C2E7")
        let textP = Color(hex: dict["text-primary"] ?? "#CDD6F4")
        let textS = Color(hex: dict["text-secondary"] ?? "#BAC2DE")
        let textM = Color(hex: dict["text-muted"] ?? "#A6ADC8")

        self.tokens = ThemeTokens(
            bgBase: base,
            bgSurface: surface,
            bgCard: card,
            bgElevated: elevated,
            bgSidebar: sidebar,
            accentPrimary: primary,
            accentSecondary: secondary,
            textPrimary: textP,
            textSecondary: textS,
            textMuted: textM,
            borderSubtle: textP.opacity(0.08),
            borderDefault: textP.opacity(0.16)
        )
    }
}
