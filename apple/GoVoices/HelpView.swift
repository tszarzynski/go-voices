import SwiftUI

/// The app's icon, drawn: the keyboard's round button with GO:V on it.
struct AppBadge: View {
    var size: CGFloat = GoKeys.s(96)

    var body: some View {
        ZStack {
            Circle().fill(GoKeys.accent)
            Circle().fill(GoKeys.panel).padding(size * 0.05)
            Text("GO:V")
                .font(.custom(GoKeys.labelFace, fixedSize: size * 0.27).weight(.bold))
                .foregroundStyle(GoKeys.selectedText)
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

/// Done, on iPhone and iPad, where About and Help are sheets.
private struct DoneButton: View {
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        #if os(iOS)
        Button("Done") { dismiss() }
            .buttonStyle(FlatButtonStyle())
        #endif
    }
}

/// Room for the Mac's window buttons over the hidden title bar.
private var titleBarRoom: CGFloat {
    #if os(macOS)
    GoKeys.s(20)
    #else
    0
    #endif
}

// MARK: - About

/// About Go:Voices: what it is, its version, and whose work it builds on.
struct AboutView: View {
    private struct Credit: Identifiable {
        let name: String
        let author: String?
        let role: String
        let url: String?
        var id: String { name }
    }

    private let credits = [
        Credit(name: "GO:Plus", author: "waldt",
               role: "The sounds, their banks and programs, and what the keyboard takes over MIDI",
               url: "https://github.com/waldt/goplus"),
        Credit(name: "GO Tone Bridge", author: "madbadware",
               role: "The waves' names, and the JUNO-DS patch map",
               url: "https://github.com/madbadware/go-tone-bridge"),
        Credit(name: "Crux", author: "Red Badger",
               role: "The core the app is built around",
               url: "https://github.com/redbadger/crux"),
        Credit(name: "Fonts", author: nil,
               role: "Quicksand, Space Grotesk and JetBrains Mono, under the SIL Open Font License",
               url: nil),
    ]

    private var version: String {
        let info = Bundle.main.infoDictionary
        let short = info?["CFBundleShortVersionString"] as? String ?? "–"
        let build = info?["CFBundleVersion"] as? String ?? "–"
        return "\(short) (\(build))"
    }

    var body: some View {
        ScrollView {
            VStack(spacing: GoKeys.section) {
                VStack(spacing: GoKeys.page) {
                    AppBadge(size: GoKeys.s(88))
                    VStack(spacing: GoKeys.tight) {
                        Text("Go:Voices")
                            .font(.ui(24, weight: .bold, relativeTo: .title, family: GoKeys.labelFace))
                            .foregroundStyle(GoKeys.selectedText)
                        Text("Version \(version)").goKeys(.readout, color: GoKeys.dim)
                    }
                    Text("Browse, arrange and shape the sounds of a Roland GO:KEYS, over MIDI: Bluetooth or USB.")
                        .goKeys(.row, color: GoKeys.dim)
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: GoKeys.s(300))
                        .fixedSize(horizontal: false, vertical: true)
                }

                Card(title: "Built on") {
                    ForEach(Array(credits.enumerated()), id: \.element.id) { index, credit in
                        if index > 0 { Hairline() }
                        row(credit)
                    }
                }

                Text("Roland, GO:KEYS, GO:PIANO and JUNO-DS are trademarks of Roland Corporation. Go:Voices isn't affiliated with or endorsed by Roland.")
                    .goKeys(.detail, color: GoKeys.dim)
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.horizontal, GoKeys.section)
            .padding(.top, GoKeys.section + titleBarRoom)
            .padding(.bottom, GoKeys.section)
        }
        .overlay(alignment: .topTrailing) { DoneButton().padding(GoKeys.page) }
        .background(GoKeys.panel)
    }

    @ViewBuilder
    private func row(_ credit: Credit) -> some View {
        let content = HStack(alignment: .firstTextBaseline, spacing: GoKeys.gap) {
            VStack(alignment: .leading, spacing: GoKeys.tight) {
                HStack(alignment: .firstTextBaseline, spacing: GoKeys.tight) {
                    Text(credit.name).goKeys(.name)
                    if let author = credit.author {
                        Text("by \(author)").goKeys(.detail, color: GoKeys.dim)
                    }
                }
                Text(credit.role).goKeys(.detail, color: GoKeys.dim)
                    .multilineTextAlignment(.leading)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
            if credit.url != nil {
                Image(systemName: "arrow.up.right")
                    .font(.system(size: GoKeys.t(10), weight: .bold))
                    .foregroundStyle(GoKeys.dim)
            }
        }
        .padding(.horizontal, GoKeys.page)
        .padding(.vertical, GoKeys.page)
        .contentShape(Rectangle())

        if let url = credit.url, let link = URL(string: url) {
            Link(destination: link) { content }
                .buttonStyle(.plain)
                .help(url)
        } else {
            content
        }
    }
}

// MARK: - Help

/// How to use the app, a topic at a time: each a card of short points,
/// the thing to do first, then how.
struct HelpView: View {
    private struct Point: Hashable {
        let lead: String
        let text: String
    }

    private struct Topic: Identifiable {
        let icon: String
        let title: String
        let points: [Point]
        var id: String { title }
    }

    private static func point(_ lead: String, _ text: String) -> Point { Point(lead: lead, text: text) }

    private let topics: [Topic] = [
        Topic(icon: "dot.radiowaves.left.and.right", title: "Connecting", points: [
            point("Pair once.", pairing),
            point("Then it connects by itself.", "The keyboard button's light is green when connected, amber while connecting and red when not."),
            point("Channel 1,", "unless you pick another in the keyboard sheet."),
        ]),
        Topic(icon: "magnifyingglass", title: "Finding sounds", points: [
            point("Browse", "a bank or category in the tree, or All sounds, Favourites or Recent."),
            point("Search", "finds sounds by name in whatever is listed."),
            point("\(Wording.tap) a sound", "to send it to the keyboard\(Wording.steps)."),
            point("Star a sound", "to keep it in Favourites."),
        ]),
        Topic(icon: "pianokeys", title: "Splits and layers", points: [
            point("Each lane is a part:", "a sound on its own keys, up to four. The lit one is selected: the sound you pick goes to it."),
            point("Drag a lane's ends", "to set its keys, or its middle to move it. Side by side, parts split the keyboard; over the same keys, they layer."),
            point("Add part", "splits the selected part in two. Remove, in a lane's menu, gives its keys to a neighbour."),
            point("Back to one sound,", "in the menu on the preset's name, keeps only the selected part, on every key."),
            point("\(Wording.tap == "Tap" ? "Double-tap" : "Double-click") a lane", "to edit its part."),
        ]),
        Topic(icon: "slider.horizontal.3", title: "Editing a part", points: [
            point("EDIT,", "on the lit lane, opens the Edit panel for its part."),
            point("Sound:", "level, pan, octave, tuning, tone, envelope, vibrato and effect sends."),
            point("Layers:", "the sound itself, up to four layers, each with its wave, keys, velocities, filter, envelopes and LFOs. Solo one to hear it alone."),
            point("Changed settings", "show a red ring; Reset puts the tab back as the sound has it."),
        ]),
        Topic(icon: "square.stack", title: "Presets", points: [
            point("A preset keeps everything:", "the parts, their sounds, keys and edits."),
            point("Save one", "from the menu on its name, at the top left; once it's changed, Save updates it."),
            point("Play a set:", "presets are in the tree, in order\(Wording.steps)."),
        ]),
        Topic(icon: "gearshape", title: "The keyboard sheet", points: [
            point("Keyboards", "to connect, the MIDI channel, and the reverb and chorus every part shares."),
            point("Seamless switching:", "notes you're holding keep their sound when you pick another."),
        ]),
    ]

    private static let pairing: String = {
        #if os(macOS)
        "In Audio MIDI Setup: Window › Show MIDI Studio › Configure Bluetooth."
        #else
        "Tap the keyboard button at the top right, then Pair keyboard…"
        #endif
    }()

    private let shortcuts: [([String], String)] = [
        (["⌘", "F"], "Search"),
        (["esc"], "Clear the search"),
        (["↑"], "Previous sound or preset (or ⌘←)"),
        (["↓"], "Next sound or preset (or ⌘→)"),
        (["⌘", "1–4"], "Select a part"),
        (["⌘", "E"], "Open or close the Edit panel"),
        (["⌘", "S"], "Save the preset"),
        (["⇧", "⌘", "S"], "Save as a new preset"),
        (["⌘", "−"], "Smaller text (Compact)"),
        (["⌘", "="], "Bigger text (Comfortable)"),
        (["⌘", "?"], "This help"),
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: GoKeys.section) {
                HStack(spacing: GoKeys.page) {
                    AppBadge(size: GoKeys.s(44))
                    VStack(alignment: .leading, spacing: GoKeys.tight) {
                        Text("Go:Voices Help").goKeys(.title)
                        Text("Pick, arrange and shape the GO:KEYS's sounds.").goKeys(.detail, color: GoKeys.dim)
                    }
                    Spacer(minLength: 0)
                    DoneButton()
                }
                ForEach(topics) { topic in
                    Card(title: topic.title, icon: topic.icon) {
                        ForEach(Array(topic.points.enumerated()), id: \.offset) { index, point in
                            if index > 0 { Hairline() }
                            Text("\(Text(point.lead).fontWeight(.semibold).foregroundStyle(GoKeys.text)) \(point.text)")
                                .font(GoKeysText.row.font)
                                .foregroundStyle(GoKeys.dim)
                                .lineSpacing(GoKeys.s(3))
                                .fixedSize(horizontal: false, vertical: true)
                                .padding(.horizontal, GoKeys.page)
                                .padding(.vertical, GoKeys.page)
                        }
                    }
                }
                #if os(macOS)
                Card(title: "Shortcuts", icon: "command") {
                    ForEach(Array(shortcuts.enumerated()), id: \.offset) { index, shortcut in
                        if index > 0 { Hairline() }
                        HStack(spacing: GoKeys.page) {
                            HStack(spacing: GoKeys.joined) {
                                ForEach(shortcut.0, id: \.self) { Keycap(key: $0) }
                            }
                            .frame(width: GoKeys.s(84), alignment: .leading)
                            Text(shortcut.1).goKeys(.row)
                        }
                        .padding(.horizontal, GoKeys.page)
                        .padding(.vertical, GoKeys.gap)
                        .accessibilityElement(children: .combine)
                    }
                }
                #endif
            }
            .frame(maxWidth: GoKeys.s(560), alignment: .leading)
            .padding(.horizontal, GoKeys.section)
            .padding(.top, GoKeys.section + titleBarRoom)
            .padding(.bottom, GoKeys.section)
            .frame(maxWidth: .infinity)
        }
        .background(GoKeys.panel)
    }
}

/// A key of a shortcut, as a small raised cap.
private struct Keycap: View {
    let key: String

    var body: some View {
        Text(key)
            .goKeys(.readout, color: GoKeys.text)
            .frame(minWidth: GoKeys.s(20), minHeight: GoKeys.s(20))
            .padding(.horizontal, GoKeys.tight)
            .background(GoKeys.control, in: RoundedRectangle(cornerRadius: GoKeys.radius))
            .overlay {
                RoundedRectangle(cornerRadius: GoKeys.radius).strokeBorder(GoKeys.line, lineWidth: 1)
            }
    }
}
