import CoreText
import SwiftUI

/// Design tokens: flat greys with the GO:KEYS red as the one accent, in the
/// spirit of a DAW's browser. The display is a Roland-style backlit orange
/// LCD. One look: no light/dark variants.
enum GoKeys {
    /// The keyboard's red: whatever is selected.
    static let accent = Color(hex: 0xE0242B)

    static let background = Color(hex: 0x1C1C1C) // the list
    static let panel = Color(hex: 0x272727) // the strips above and below it
    static let control = Color(hex: 0x353535) // buttons and toggles
    static let controlPressed = Color(hex: 0x444444)
    static let line = Color(hex: 0x111111) // 1px borders between controls

    static let text = Color(hex: 0xE4E4E4)
    static let dim = Color(hex: 0x8E8E8E)

    // The display: a backlit amber LCD with dark pixels, after the JUNO-G.
    static let display = Color(hex: 0xFF7A0F)
    static let displayText = Color(hex: 0x2E0F00)

    static let green = Color(hex: 0x4CD964)
    static let amber = Color(hex: 0xFFB020)
    static let red = Color(hex: 0xFF3B3B)

    /// The Mac draws everything at the size the iPad version has there:
    /// macOS shows iPad apps at 77%. Elsewhere, 1.
    #if os(macOS)
    static let scale: CGFloat = 0.77
    #else
    static let scale: CGFloat = 1
    #endif

    /// A size in points, at `scale`.
    static func s(_ points: CGFloat) -> CGFloat { points * scale }

    static let page: CGFloat = GoKeys.s(12)
    static let gap: CGFloat = GoKeys.s(8)

    /// Registers the bundled fonts (both SIL OFL) with CoreText: Inter for
    /// the interface, VT323 for the display's dot matrix.
    static func registerFonts() {
        for name in ["Inter-Variable", "VT323-Regular"] {
            guard let url = Bundle.main.url(forResource: name, withExtension: "ttf") else { continue }
            CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
        }
    }
}

extension Color {
    init(hex: UInt32) {
        self.init(
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}

extension Font {
    /// Inter at `weight` (one variable font), at `GoKeys.scale` and scaled
    /// with Dynamic Type relative to `style`.
    static func inter(_ size: CGFloat, weight: Font.Weight = .regular, relativeTo style: Font.TextStyle = .body) -> Font {
        .custom("Inter", size: GoKeys.s(size), relativeTo: style).weight(weight)
    }
}

/// Text roles.
enum GoKeysText {
    /// Small caps on controls and headings.
    case label
    /// A patch name in the list.
    case row
    /// Addresses and counts, with even-width digits.
    case detail
    /// The display's small line.
    case displaySmall
    /// The display's patch name.
    case displayLarge

    var font: Font {
        switch self {
        case .label: .inter(11, weight: .semibold, relativeTo: .caption).monospacedDigit()
        case .row: .inter(14)
        case .detail: .inter(11, relativeTo: .caption).monospacedDigit()
        case .displaySmall: .custom("VT323-Regular", size: GoKeys.s(18), relativeTo: .callout)
        case .displayLarge: .custom("VT323-Regular", size: GoKeys.s(30), relativeTo: .title2)
        }
    }

    var tracking: CGFloat {
        switch self {
        case .label: 0.6
        case .row, .detail, .displaySmall, .displayLarge: 0
        }
    }
}

extension Text {
    /// An upper-case label.
    init(caps text: String) { self.init(text.uppercased()) }

    func goKeys(_ role: GoKeysText, color: Color = GoKeys.text) -> some View {
        font(role.font)
            .tracking(role.tracking)
            .foregroundStyle(color)
    }
}
