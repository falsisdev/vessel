import SwiftUI

extension Color {
    init(hex: String) {
        let cleanHex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: cleanHex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch cleanHex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (255, 0, 0, 0)
        }
        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }

    // Catppuccin Mocha Tokens
    static let vesselBase = Color(hex: "#1E1E2E")
    static let vesselSurface = Color(hex: "#181825")
    static let vesselCard = Color(hex: "#313244")
    static let vesselElevated = Color(hex: "#313244")
    static let vesselOverlay = Color(hex: "#1E1E2E").opacity(0.88)
    static let vesselSidebar = Color(hex: "#181825")
    static let vesselPlayer = Color(hex: "#181825")

    static let vesselAccent = Color(hex: "#CBA6F7")       // Catppuccin Mauve
    static let vesselAccentPink = Color(hex: "#F5C2E7")   // Pink
    static let vesselAccentBlue = Color(hex: "#89B4FA")   // Blue
    static let vesselAccentHover = Color(hex: "#B4BEFE")  // Lavender

    static let vesselTextPrimary = Color(hex: "#CDD6F4")
    static let vesselTextSecondary = Color(hex: "#BAC2DE")
    static let vesselTextMuted = Color(hex: "#A6ADC8")
    static let vesselTextInverse = Color(hex: "#11111B")

    static let vesselSuccess = Color(hex: "#A6E3A1")
    static let vesselWarning = Color(hex: "#F9E2AF")
    static let vesselError = Color(hex: "#F38BA8")
    static let vesselInfo = Color(hex: "#89DCEB")

    static let vesselBorderSubtle = Color(hex: "#CDD6F4").opacity(0.08)
    static let vesselBorderDefault = Color(hex: "#CDD6F4").opacity(0.16)
    static let vesselBorderStrong = Color(hex: "#CDD6F4").opacity(0.28)
}
