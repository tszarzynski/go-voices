import CoreText
import OSLog
import SwiftUI

/// Design tokens: flat greys with the GO:KEYS red as the one accent, in the
/// spirit of a DAW's browser. The display is a Roland-style backlit orange
/// LCD. One look: no light/dark variants.
enum GoKeys {
    /// The keyboard's red: its body, the round buttons' rims, what's on,
    /// and what's changed (Edited, a changed knob's ring).
    static let accent = Color(hex: 0xD82D19)

    static let background = Color(hex: 0x1C1C1C) // the list
    static let panel = Color(hex: 0x272727) // the strips above and below it
    static let control = Color(hex: 0x353535) // wells: knob faces, key bars, key caps
    static let controlPressed = Color(hex: 0x444444)
    static let line = Color(hex: 0x111111) // 1px borders between controls
    static let inset = Color(hex: 0x141414) // a readout or graph set into a card

    static let text = Color(hex: 0xE4E4E4)
    /// Text selected in a list: brighter than the rest.
    static let selectedText = Color.white
    static let dim = Color(hex: 0xA3A3A3) // small print and labels
    static let faint = Color(hex: 0x5E5E5E) // an unset star, the least there is

    /// What's switched off or can't be used yet, faded by this.
    static let disabled = 0.4

    // A keyboard's keys: the ruler's white and black keys; the white is
    // also any range of keys that's played but not selected (a preset's
    // parts, a layer's keys).
    static let keys = Color(hex: 0x6E6E6E)
    static let keysBlack = background

    // The pads' two dotted finishes and the text on them.
    static let silver: UInt32 = 0x9C9C9C
    static let silverDots: UInt32 = 0x808080
    static let onSilver = Color(hex: 0x1A1A1A)
    static let onSilverDim = Color(hex: 0x3D3D3D)
    static let wine: UInt32 = 0x4A1418
    static let wineDots: UInt32 = 0x6A2228
    static let onWine = text
    static let onWineDim = Color(hex: 0xC29A9D)

    // The display: a backlit amber LCD with dark pixels, after the JUNO-G.
    static let display = Color(hex: 0xFF7A0F)
    static let displayText = Color(hex: 0x2E0F00)
    static let displayDim = Color(hex: 0x823A06) // its unlit pixels' shade

    // LEDs, and the favourite star's amber.
    static let green = Color(hex: 0x4CD964)
    static let amber = Color(hex: 0xFFB020)
    static let red = Color(hex: 0xFF4D4D)

    /// The size everything is drawn at on the Mac, chosen in the View
    /// menu; kept between launches. Set before the views are drawn again.
    nonisolated(unsafe) static var size = TextSize.saved

    /// Spacing and sizes. On the Mac, at `size`; elsewhere, 1.
    static var scale: CGFloat {
        #if os(macOS)
        size.scale
        #else
        1
        #endif
    }

    /// A size in points, at `scale`.
    static func s(_ points: CGFloat) -> CGFloat { points * scale }

    /// Text and icons. On the Mac, at `size`; elsewhere, 1.
    static var textScale: CGFloat {
        #if os(macOS)
        size.textScale
        #else
        1
        #endif
    }

    /// A text or icon size in points, at `textScale`.
    static func t(_ points: CGFloat) -> CGFloat { points * textScale }

    // One spacing scale: every gap and inset is one of these.
    /// Pads joined in a group.
    static var joined: CGFloat { GoKeys.s(3) }
    /// An icon to its text, inside a control; a heading to its hint.
    static var tight: CGFloat { GoKeys.s(4) }
    /// Between controls in a row, lanes and a card's lines; inside
    /// controls, wells and cards.
    static var gap: CGFloat { GoKeys.s(8) }
    /// Every panel's edge; a heading to what it heads.
    static var page: CGFloat { GoKeys.s(12) }
    /// Between sections and groups.
    static var section: CGFloat { GoKeys.s(24) }
    /// The red between panels, and round them.
    static var frame: CGFloat { GoKeys.s(10) }
    /// One row's height: a lane, Add part, the keyboard, a row of a list
    /// (and so its selection's outline).
    static var row: CGFloat { GoKeys.s(32) }
    /// The corners of everything inside a panel: pads, fields, readouts.
    static var radius: CGFloat { GoKeys.s(4) }
    /// The tree's width: 220 points at Comfortable.
    static var tree: CGFloat { GoKeys.s(244) }
    /// Round an empty list's message.
    static var empty: CGFloat { GoKeys.s(40) }

    /// The typeface for the controls and text: rows, names, buttons.
    static let textFace = "Space Grotesk"
    /// The typeface for the panel's labels: titles and group headings.
    static let labelFace = "Quicksand"
    /// The typeface for readouts: addresses, values, keys, counts.
    static let readoutFace = "JetBrains Mono"

    /// Registers the bundled fonts (all SIL OFL) with CoreText, after the
    /// GO:KEYS panel: Quicksand for its labels, Space Grotesk for controls
    /// and text, JetBrains Mono for readouts, VT323 for the display's dot
    /// matrix.
    static func registerFonts() {
        for name in ["SpaceGrotesk-Variable", "Quicksand-Variable", "JetBrainsMono-Variable", "VT323-Regular"] {
            // Missing or refused, the system font stands in: say so.
            guard let url = Bundle.main.url(forResource: name, withExtension: "ttf") else {
                fontLog.error("font \(name, privacy: .public) isn't in the app")
                continue
            }
            var error: Unmanaged<CFError>?
            if !CTFontManagerRegisterFontsForURL(url as CFURL, .process, &error) {
                let reason = error?.takeRetainedValue().localizedDescription ?? "unknown"
                fontLog.error("font \(name, privacy: .public) not registered: \(reason, privacy: .public)")
            }
        }
    }
}

private let fontLog = Logger(subsystem: "com.tszarzynski.govoices", category: "fonts")

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
    /// `family` (text unless given) at `weight`, at `GoKeys.textScale` and
    /// scaled with Dynamic Type relative to `style`.
    static func ui(
        _ size: CGFloat,
        weight: Font.Weight = .regular,
        relativeTo style: Font.TextStyle = .body,
        family: String = GoKeys.textFace
    ) -> Font {
        .custom(family, size: GoKeys.t(size), relativeTo: style).weight(weight)
    }
}

/// The frame round the panels: the GO:KEYS red, or dark, easier on the
/// eyes for long sessions. The Mac's View menu's Theme.
enum Look: String, CaseIterable, Identifiable {
    case red
    case dark

    /// Where it's kept.
    static let key = "look"

    var id: Self { self }

    var label: String {
        switch self {
        case .red: "Red"
        case .dark: "Dark"
        }
    }

    /// The frame's colour: darker than the panels it holds.
    var frame: Color {
        switch self {
        case .red: GoKeys.accent
        case .dark: Color(hex: 0x111111)
        }
    }

    /// The LCD's backlight: in the dark, turned down a little so it
    /// doesn't glare.
    var display: Color {
        switch self {
        case .red: GoKeys.display
        case .dark: Color(hex: 0xE06A10)
        }
    }
}

/// How big the Mac draws everything: its View menu's Text Size.
enum TextSize: String, CaseIterable, Identifiable {
    /// As the iPad app looks on a Mac: iPad sizes at 77%.
    case compact
    /// A little under the iPad's: spacing at 90%, and text at 93% so a row
    /// is the Mac's 13 points and nothing is under 9.
    case comfortable

    /// Where it's kept.
    static let key = "textSize"

    static var saved: TextSize {
        UserDefaults.standard.string(forKey: key).flatMap(TextSize.init(rawValue:)) ?? .comfortable
    }

    var id: Self { self }

    var label: String {
        switch self {
        case .compact: "Compact"
        case .comfortable: "Comfortable"
        }
    }

    var scale: CGFloat {
        switch self {
        case .compact: 0.77
        case .comfortable: 0.9
        }
    }

    var textScale: CGFloat {
        switch self {
        case .compact: 0.77
        case .comfortable: 0.93
        }
    }
}

/// Text roles: every piece of text is one of these.
enum GoKeysText {
    /// The preset's name over the lanes; a sheet's title.
    case title
    /// A patch name in the list; the search field.
    case row
    /// A part's sound on its lane; Add part.
    case name
    /// A group's heading: the Edit panel's groups, the list's categories.
    case heading
    /// Small caps on controls and headings.
    case label
    /// Small print in words: a category, a device's state.
    case detail
    /// Numbers and keys read off the keyboard: an address, a value, a
    /// count, a range.
    case readout
    /// The note names on the drawn keyboard.
    case tick
    /// The display's small line.
    case displaySmall
    /// The display's patch name.
    case displayLarge

    var font: Font {
        switch self {
        case .title: .ui(16, weight: .bold, relativeTo: .headline, family: GoKeys.labelFace)
        case .row: .ui(14)
        case .name: .ui(13, weight: .semibold, relativeTo: .subheadline)
        case .heading: .ui(14, weight: .bold, relativeTo: .subheadline, family: GoKeys.labelFace)
        case .label: .ui(11, weight: .semibold, relativeTo: .caption).monospacedDigit()
        case .detail: .ui(11, relativeTo: .caption)
        case .readout: .ui(11, relativeTo: .caption, family: GoKeys.readoutFace)
        case .tick: .ui(10, weight: .bold, relativeTo: .caption2, family: GoKeys.labelFace)
        case .displaySmall: .custom("VT323-Regular", size: GoKeys.t(18), relativeTo: .callout)
        case .displayLarge: .custom("VT323-Regular", size: GoKeys.t(30), relativeTo: .title2)
        }
    }

    var tracking: CGFloat {
        switch self {
        case .label: 0.6
        case .title: 1.2
        case .row, .name, .heading, .detail, .readout, .tick, .displaySmall, .displayLarge: 0
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
