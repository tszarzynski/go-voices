import App
import SwiftUI
#if os(macOS)
import AppKit
#endif

/// The one dotted outline: on what's selected in a list, and on Add part.
struct DottedOutline: View {
    var body: some View {
        RoundedRectangle(cornerRadius: GoKeys.radius)
            .strokeBorder(Pad.silver.base, style: StrokeStyle(lineWidth: 2, lineCap: .round, dash: [0, 4]))
    }
}

extension View {
    /// A panel set into the red body: its corners rounded.
    func panel() -> some View {
        clipShape(RoundedRectangle(cornerRadius: GoKeys.s(12)))
    }
}

/// The mark on what's selected in a list: a dotted grey border, as Add
/// part's.
struct SelectedRim: ViewModifier {
    let on: Bool

    func body(content: Content) -> some View {
        content.overlay {
            if on {
                DottedOutline()
                    .padding(.horizontal, GoKeys.tight)
                    .allowsHitTesting(false)
            }
        }
    }
}

extension View {
    /// A dotted grey border while `on`.
    func selectedRim(_ on: Bool) -> some View { modifier(SelectedRim(on: on)) }
}

/// A pad button, as the keyboard's: small caps on a dotted pad, silver
/// when `on` (selected), wine-red otherwise. `compact` makes it shorter.
struct FlatButtonStyle: ButtonStyle {
    var on = false
    var compact = false

    func makeBody(configuration: Configuration) -> some View {
        let pad: Pad = on ? .silver : .wine
        return configuration.label
            .font(GoKeysText.label.font)
            .tracking(GoKeysText.label.tracking)
            .textCase(.uppercase)
            .foregroundStyle(pad.text)
            .lineLimit(1)
            .padding(.horizontal, GoKeys.gap)
            .frame(minHeight: GoKeys.s(compact ? 22 : 28))
            .background { PadFill(pad: pad).brightness(configuration.isPressed ? 0.08 : 0) }
            .contentShape(RoundedRectangle(cornerRadius: GoKeys.radius))
            .focusEffectDisabled()
    }
}

/// A round button as on the GO:KEYS panel: the panel's grey, with a red
/// rim and a white symbol. Pressed, the face lightens; `on`, it's red.
struct RoundButtonStyle: ButtonStyle {
    var on = false

    private var size: CGFloat { GoKeys.s(52) }
    /// The red ring's width.
    private let rim: CGFloat = 2.5

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(GoKeys.selectedText)
            .frame(width: size, height: size)
            .background {
                // The rim, then the face set into it: two fills draw a
                // cleaner ring than a stroke.
                ZStack {
                    Circle().fill(GoKeys.accent)
                    Circle()
                        .fill(configuration.isPressed ? GoKeys.controlPressed : (on ? GoKeys.accent : GoKeys.panel))
                        .padding(rim)
                }
            }
            .scaleEffect(configuration.isPressed ? 0.94 : 1)
            .animation(.easeOut(duration: 0.08), value: configuration.isPressed)
            .contentShape(Circle())
            .focusEffectDisabled()
    }
}

/// A small LED.
struct LedDot: View {
    let color: Color

    private var size: CGFloat { GoKeys.s(7) }

    var body: some View {
        // A light rim so it shows on the wine pads too.
        Circle().fill(color)
            .overlay { Circle().strokeBorder(GoKeys.text.opacity(0.5), lineWidth: 1) }
            .frame(width: size, height: size)
    }
}

/// The display: a backlit amber LCD with dark dot-matrix text.
struct Display<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        content
            .padding(.horizontal, GoKeys.gap)
            .padding(.vertical, GoKeys.s(6))
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(GoKeys.display, in: RoundedRectangle(cornerRadius: GoKeys.radius))
            .overlay { RoundedRectangle(cornerRadius: GoKeys.radius).strokeBorder(GoKeys.line, lineWidth: 2) }
    }
}

/// A value with choices, as a readout in a dark well like the search
/// field, in the knobs' values' type: click it for the menu.
struct FlatMenu<Content: View>: View {
    let title: String
    /// What it sets, for VoiceOver.
    let label: String
    @ViewBuilder let content: Content

    var body: some View {
        Menu {
            content
        } label: {
            HStack(spacing: GoKeys.tight) {
                Text(title)
                    .goKeys(.readout, color: GoKeys.text)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                    .truncationMode(.middle)
                Image(systemName: "chevron.up.chevron.down")
                    .font(.system(size: GoKeys.t(8), weight: .bold))
                    .foregroundStyle(GoKeys.dim)
            }
            .padding(.horizontal, GoKeys.gap)
            .frame(minHeight: GoKeys.s(28))
            .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
            .overlay { RoundedRectangle(cornerRadius: GoKeys.radius).strokeBorder(GoKeys.line, lineWidth: 1) }
            .contentShape(RoundedRectangle(cornerRadius: GoKeys.radius))
        }
        .menuStyle(.button)
        .menuIndicator(.hidden)
        .buttonStyle(.plain)
        // As wide as its words where there's room, never wider than its
        // cell: a long value shrinks, then is cut in the middle.
        .fixedSize(horizontal: false, vertical: true)
        .help("\(label): click to choose")
        .accessibilityLabel("\(label): \(title)")
    }
}

/// A menu item, ticked when it's the one in use.
struct MenuChoice: View {
    let title: String
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            if selected {
                Label(title, systemImage: "checkmark")
            } else {
                Text(title)
            }
        }
    }
}

/// A star: filled when the sound is starred.
struct StarButton: View {
    let on: Bool
    var color: Color = GoKeys.displayText
    /// A larger tap target, for list rows.
    var target: CGFloat?
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: on ? "star.fill" : "star")
                .font(.system(size: GoKeys.t(12)))
                .foregroundStyle(color)
                .frame(width: target, height: target)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(on ? "Unstar: take it out of Favourites" : "Star: keep it in Favourites")
        .accessibilityLabel(on ? "Unstar" : "Star")
    }
}

/// An error, on a display, with a button to dismiss it.
struct StatusLine: View {
    let status: String
    let dismiss: () -> Void

    var body: some View {
        HStack {
            Text("! \(status)").goKeys(.displaySmall, color: GoKeys.displayText).lineLimit(1)
            Spacer(minLength: GoKeys.tight)
            Button(action: dismiss) {
                Image(systemName: "xmark")
                    .font(.system(size: GoKeys.t(10), weight: .bold))
                    .foregroundStyle(GoKeys.displayText)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Dismiss")
        }
    }
}

/// Pads side by side, a little apart, as one control: as the keyboard's
/// row of pads.
struct ToggleGroup<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        HStack(spacing: GoKeys.joined) { content }
    }
}

/// A knob for one setting: drag up or down to turn it, double-click (or
/// double-tap) or use its menu to reset it. While turning, it sends at most
/// every 40 ms, so a Bluetooth keyboard isn't flooded, then the final value.
struct Knob: View {
    let control: ControlView
    let set: (Int16) -> Void
    let reset: () -> Void

    @State private var turning: Double?
    @State private var start: Double = 0
    @State private var lastSent = Date.distantPast

    static var size: CGFloat { GoKeys.s(44) }
    /// Points of drag for the whole range.
    private static let travel: CGFloat = 160

    var body: some View {
        VStack(spacing: GoKeys.tight) {
            CellLabel(text: control.label)
            dial
                .frame(width: Self.size, height: Self.size)
                .contentShape(Circle())
                .gesture(drag)
                .onTapGesture(count: 2, perform: reset)
                .contextMenu { Button("Reset", action: reset) }
            Text(control.text).goKeys(.readout, color: control.edited ? GoKeys.text : GoKeys.dim)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .help("\(control.label): drag up or down to change it, double-click to put it back")
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(control.label)
        .accessibilityValue(control.text)
        .accessibilityAdjustableAction { direction in
            let step = max(1, Int16((Int(control.max) - Int(control.min)) / 32))
            switch direction {
            case .increment: set(min(control.max, control.value + step))
            case .decrement: set(max(control.min, control.value - step))
            @unknown default: break
            }
        }
    }

    private var shown: Double { turning ?? Double(control.value) }

    /// How far round, 0 to 1.
    private var fraction: Double {
        let span = Double(control.max) - Double(control.min)
        return span > 0 ? (shown - Double(control.min)) / span : 0
    }

    /// A ring of 270°, open at the bottom: the track, the value in red (dim
    /// at the keyboard's own), and a pointer.
    private var dial: some View {
        ZStack {
            Circle()
                .trim(from: 0, to: 0.75)
                .stroke(GoKeys.line, style: StrokeStyle(lineWidth: GoKeys.s(4), lineCap: .round))
                .rotationEffect(.degrees(135))
            Circle()
                .trim(from: 0, to: 0.75 * fraction)
                .stroke(control.edited ? GoKeys.accent : GoKeys.dim,
                        style: StrokeStyle(lineWidth: GoKeys.s(4), lineCap: .round))
                .rotationEffect(.degrees(135))
            Capsule()
                .fill(GoKeys.text)
                .frame(width: GoKeys.s(2.5), height: Self.size * 0.3)
                .offset(y: -Self.size * 0.17)
                .rotationEffect(.degrees(-135 + 270 * fraction))
        }
        .padding(GoKeys.s(3))
        .background(Circle().fill(GoKeys.control))
    }

    private var drag: some Gesture {
        DragGesture(minimumDistance: 1)
            .onChanged { gesture in
                if turning == nil { start = Double(control.value) }
                let span = Double(control.max) - Double(control.min)
                let value = (start - gesture.translation.height / Self.travel * span)
                    .clamped(Double(control.min), Double(control.max))
                turning = value
                let now = Date()
                if now.timeIntervalSince(lastSent) >= 0.04 {
                    lastSent = now
                    set(Int16(value.rounded()))
                }
            }
            .onEnded { _ in
                if let turning { set(Int16(turning.rounded())) }
                turning = nil
            }
    }
}

/// A cell's label over its knob or choice: one line, shrunk a little
/// rather than cut short.
struct CellLabel: View {
    let text: String

    var body: some View {
        Text(caps: text).goKeys(.label, color: GoKeys.dim)
            .lineLimit(1)
            .minimumScaleFactor(0.75)
    }
}

/// A setting with choices, as one grid cell: its label, then joined
/// buttons where they fit, or a menu, at the height of a knob's dial so the
/// cells of a row line up.
struct ChoiceCell: View {
    let control: ControlView
    let set: (Int16) -> Void

    var body: some View {
        VStack(spacing: GoKeys.tight) {
            CellLabel(text: control.label)
            if let toggle = control.toggle {
                // Two choices: one pad that switches between them, silver
                // on the second (On), as the keyboard's buttons.
                Button(control.text) { set(toggle.next) }
                    .buttonStyle(FlatButtonStyle(on: toggle.on))
                    .help("\(control.label): click to switch")
                    .accessibilityValue(control.text)
                    .frame(maxWidth: .infinity, minHeight: Knob.size, maxHeight: Knob.size)
            } else {
                ViewThatFits(in: .horizontal) {
                    ToggleGroup {
                        ForEach(control.choices, id: \.value) { choice in
                            Button(choice.label) { set(choice.value) }
                                .buttonStyle(FlatButtonStyle(on: choice.value == control.value, compact: true))
                                .accessibilityAddTraits(choice.value == control.value ? .isSelected : [])
                        }
                    }
                    .fixedSize()
                    FlatMenu(title: control.text, label: control.label) {
                        ForEach(control.choices, id: \.value) { choice in
                            MenuChoice(title: choice.label, selected: choice.value == control.value) { set(choice.value) }
                        }
                    }
                }
            .frame(maxWidth: .infinity, minHeight: Knob.size, maxHeight: Knob.size)
            }
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(control.label)
    }
}

/// Groups of settings on one grid of equal cells: a knob or a choice takes
/// a cell; an envelope's shape takes two. Each group has a row of its own,
/// so knobs line up down the page.
struct ControlGrid: View {
    let groups: [GroupView]
    /// The keyboard, to draw a layer's keys on, and its octaves.
    var keyboard: KeyAxis?
    var ticks: [NoteTick] = []
    /// Just the controls: the headings and notes shown elsewhere.
    var bare = false
    let send: (Event) -> Void

    /// The grid every Edit panel view lines up on.
    static var grid: GridLayout { GridLayout(minCell: GoKeys.s(70), gap: GoKeys.page) }

    var body: some View {
        Self.grid {
            ForEach(Array(groups.enumerated()), id: \.element.title) { index, group in
                if !bare {
                    // Never cut short: the group is made wide enough.
                    Text(group.title).goKeys(.heading).fixedSize()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .heading))
                }
                ForEach(group.controls, id: \.setting) { control in
                    if control.choices.isEmpty {
                        Knob(control: control) { send(control.set($0)) } reset: { send(control.reset) }
                            .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(1)))
                    } else {
                        ChoiceCell(control: control) { send(control.set($0)) }
                            .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(1)))
                    }
                }
                if let keys = group.keys, let keyboard {
                    KeyRangeStrip(range: keys, keyboard: keyboard, ticks: ticks, send: send)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(6)))
                }
                if !group.envelope.isEmpty {
                    EnvelopeShape(points: group.envelope)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(2)))
                }
                if !bare, !group.note.isEmpty {
                    Text(group.note).goKeys(.row, color: GoKeys.dim)
                        .fixedSize(horizontal: false, vertical: true)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .note))
                }
            }
        }
    }
}

extension ControlView {
    /// For a setting of two choices (Off / On): whether it's on the second,
    /// and the value a press sets.
    var toggle: (on: Bool, next: Int16)? {
        guard choices.count == 2 else { return nil }
        let on = value == choices[1].value
        return (on, on ? choices[0].value : choices[1].value)
    }
}

/// Where a view goes in a `GridLayout`: its group, and whether it's the
/// group's heading, a cell (so many columns wide) or its note.
struct GridSlot: LayoutValueKey, Equatable {
    enum Kind: Equatable {
        case heading
        case cell(Int)
        case note
    }

    static let defaultValue = GridSlot(group: 0, kind: .cell(1))

    var group: Int
    var kind: Kind
}

/// As many columns as fit at `minCell` or wider, filling the width. Each
/// group's heading spans its cells (and the group is at least as wide as
/// its heading's words); its cells all have the height of the
/// tallest one-column cell, wrapping onto more lines only when the group is
/// wider than the grid; its note goes under them.
struct GridLayout: SwiftUI.Layout {
    var minCell: CGFloat
    var gap: CGFloat

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache _: inout ()) -> CGSize {
        // Asked how wide it could be (no width, or an infinite one): as
        // wide as eight cells, not a column count it can't hold.
        let width = proposal.width.flatMap { $0.isFinite ? $0 : nil } ?? minCell * 8
        return CGSize(width: width, height: frames(width: width, subviews: subviews).height)
    }

    func placeSubviews(in bounds: CGRect, proposal _: ProposedViewSize, subviews: Subviews, cache _: inout ()) {
        for (subview, frame) in zip(subviews, frames(width: bounds.width, subviews: subviews).frames) {
            subview.place(
                at: CGPoint(x: bounds.minX + frame.minX, y: bounds.minY + frame.minY),
                proposal: ProposedViewSize(frame.size)
            )
        }
    }

    private func frames(width: CGFloat, subviews: Subviews) -> (frames: [CGRect], height: CGFloat) {
        let columns = max(1, min(Int(((width + gap) / (minCell + gap)).rounded(.down)), 64))
        let cell = (width - gap * CGFloat(columns - 1)) / CGFloat(columns)
        let x = { (column: Int) in CGFloat(column) * (cell + gap) }
        let span = { (count: Int) in CGFloat(count) * cell + CGFloat(count - 1) * gap }
        let slots = subviews.map { $0[GridSlot.self] }
        let cellHeight = zip(subviews, slots)
            .compactMap { subview, slot -> CGFloat? in
                guard case let .cell(count) = slot.kind else { return nil }
                return subview.sizeThatFits(ProposedViewSize(width: span(min(count, columns)), height: nil)).height
            }
            .max() ?? 0
        // One rhythm: a heading close to its controls, their lines a little
        // apart, the next row of groups clearly so.
        let headingGap = GoKeys.page
        let lineGap = GoKeys.gap * 2
        let rowGap = GoKeys.section

        var frames = Array(repeating: CGRect.zero, count: subviews.count)
        var column = 0
        var rowTop: CGFloat = 0
        var rowBottom: CGFloat = 0
        var index = 0
        while index < subviews.count {
            // One group: its views run on while they share its index.
            let group = slots[index].group
            var end = index
            while end < subviews.count, slots[end].group == group { end += 1 }
            let members = Array(index ..< end)
            let wide = members.reduce(0) { total, member in
                if case let .cell(count) = slots[member].kind { total + min(count, columns) } else { total }
            }
            // Wide enough for its heading too.
            let heading = members.first { slots[$0].kind == .heading }
            let headingWidth = heading.map { subviews[$0].sizeThatFits(.unspecified).width } ?? 0
            let headingColumns = Int(((headingWidth + gap) / (cell + gap)).rounded(.up))
            let groupColumns = min(max(wide, headingColumns, 1), columns)
            // One group a row: each starts on a new one.
            if column > 0 {
                column = 0
                rowTop = rowBottom + rowGap
            }
            let left = x(column)
            let groupWidth = span(groupColumns)
            var top = rowTop
            if let heading {
                let height = subviews[heading].sizeThatFits(ProposedViewSize(width: groupWidth, height: nil)).height
                frames[heading] = CGRect(x: left, y: top, width: groupWidth, height: height)
                top += height + headingGap
            }
            var line = 0
            var lineColumn = 0
            var bottom = top
            for member in members {
                guard case let .cell(wanted) = slots[member].kind else { continue }
                let count = min(wanted, groupColumns)
                if lineColumn + count > groupColumns {
                    line += 1
                    lineColumn = 0
                }
                let y = top + CGFloat(line) * (cellHeight + lineGap)
                frames[member] = CGRect(x: left + x(lineColumn), y: y, width: span(count), height: cellHeight)
                bottom = y + cellHeight
                lineColumn += count
            }
            for member in members where slots[member].kind == .note {
                let height = subviews[member].sizeThatFits(ProposedViewSize(width: groupWidth, height: nil)).height
                frames[member] = CGRect(x: left, y: bottom + GoKeys.gap, width: groupWidth, height: height)
                bottom += GoKeys.gap + height
            }
            rowBottom = max(rowBottom, bottom)
            column += groupColumns
            index = end
        }
        return (frames, rowBottom)
    }
}

extension Double {
    func clamped(_ low: Double, _ high: Double) -> Double { Swift.min(Swift.max(self, low), high) }
}

/// The keyboard's keys across a width: which key is where, and where each
/// key is. Every key strip, bar and map draws on one.
struct KeyAxis {
    let first: Int
    let last: Int

    init(first: UInt8, last: UInt8) {
        self.first = Int(first)
        self.last = Int(last)
    }

    var count: Int { last - first + 1 }

    /// A key brought onto the keyboard: one past an end is at that end.
    func clamp(_ note: Int) -> Int { min(max(note, first), last) }

    /// Where a key's left edge is.
    func x(_ note: Int, width: CGFloat) -> CGFloat {
        CGFloat(note - first) / CGFloat(count) * width
    }

    /// The key at `x`.
    func note(at x: CGFloat, width: CGFloat) -> Int {
        first + min(max(Int(x / width * CGFloat(count)), 0), count - 1)
    }

    /// One key's width.
    func keyWidth(_ width: CGFloat) -> CGFloat { width / CGFloat(count) }
}

/// Dragging a range of keys drawn as a band: a part's lane, a layer's
/// keys. Near either end, the drag moves that end; on the band, the whole
/// range, its width kept.
struct RangeGrip {
    enum Grip: Equatable {
        case low, high
        /// The whole band, from where its ends were.
        case band(low: Int, high: Int)
    }

    let keyboard: KeyAxis
    /// Its ends as drawn: on the keyboard.
    let low: Int
    let high: Int

    /// How near an end a drag must start to move it.
    static var reach: CGFloat { GoKeys.s(14) }
    /// How far a drag must go before it moves anything: a finger's tap
    /// wobbles a little, and should stay a tap.
    static var slop: CGFloat { GoKeys.s(6) }

    init(keyboard: KeyAxis, low: UInt8, high: UInt8) {
        self.keyboard = keyboard
        self.low = keyboard.clamp(Int(low))
        self.high = keyboard.clamp(Int(high))
    }

    /// The band's left and right edges.
    func edges(width: CGFloat) -> (from: CGFloat, to: CGFloat) {
        (keyboard.x(low, width: width), keyboard.x(high + 1, width: width))
    }

    /// What a drag starting at `x` takes: an end near it, else the band if
    /// it's on it.
    func grab(at x: CGFloat, width: CGFloat) -> Grip? {
        let (from, to) = edges(width: width)
        let near = [(Grip.low, abs(x - from)), (Grip.high, abs(x - to))]
            .filter { $0.1 <= Self.reach }
            .min { $0.1 < $1.1 }
        if let near { return near.0 }
        return (from ... to).contains(x) ? .band(low: low, high: high) : nil
    }

    /// Whether a drag with `grip` has gone far enough to move it: the
    /// band, a whole key at least, so a tap on it stays a tap.
    func moves(_ grip: Grip, by moved: CGFloat, width: CGFloat) -> Bool {
        guard abs(moved) > Self.slop else { return false }
        if case .band = grip { return target(grip, at: 0, moved: moved, width: width) != 0 }
        return true
    }

    /// The key an end is dragged to, or the keys the band is moved by.
    func target(_ grip: Grip, at x: CGFloat, moved: CGFloat, width: CGFloat) -> Int {
        // An end sits between keys: the key on its side of the line.
        let half = keyboard.keyWidth(width) / 2
        switch grip {
        case .low: return keyboard.note(at: x + half, width: width)
        case .high: return keyboard.note(at: x - half, width: width)
        case let .band(from, to):
            // Whole keys, keeping the band on the keyboard.
            let keys = Int((moved / keyboard.keyWidth(width)).rounded())
            return min(max(keys, keyboard.first - from), keyboard.last - to)
        }
    }
}

/// The marks at a band's ends that say they can be dragged.
struct GripMarks: View {
    let color: Color

    var body: some View {
        HStack {
            Rectangle().fill(color).frame(width: 2)
            Spacer(minLength: 0)
            Rectangle().fill(color).frame(width: 2)
        }
        .padding(.vertical, GoKeys.s(6))
        .padding(.horizontal, GoKeys.s(3))
        .allowsHitTesting(false)
    }
}

extension View {
    /// On the Mac, the pointer over a band says what a drag would do: a
    /// resize arrow at its ends, an open hand on it.
    func rangeCursor(_ grip: @escaping (CGFloat) -> RangeGrip.Grip?) -> some View {
        #if os(macOS)
        onContinuousHover { phase in
            switch phase {
            case let .active(point):
                switch grip(point.x) {
                case .low, .high: NSCursor.resizeLeftRight.set()
                case .band: NSCursor.openHand.set()
                case nil: NSCursor.arrow.set()
                }
            case .ended:
                NSCursor.arrow.set()
            }
        }
        #else
        self
        #endif
    }
}

/// The arrangement: one lane per part across the keyboard's keys, each
/// part a band in its colour from its lowest to its highest key. Tap a band
/// to pick sounds for that part, double-tap to edit it; drag either end to
/// move that end, or the middle to move the whole band. Each part's keys
/// move on their own: overlapping bands are layered.
struct ZoneMap: View {
    let layout: LayoutView
    let send: (Event) -> Void

    private var keyboard: KeyAxis { KeyAxis(first: layout.first, last: layout.last) }

    // Every row (the lanes, Add part and the keyboard) a row high, a gap
    // apart.
    var body: some View {
        VStack(spacing: GoKeys.gap) {
            ForEach(layout.zones, id: \.part) { zone in
                ZoneLane(zone: zone, keyboard: keyboard, send: send)
            }
            if layout.canAdd {
                AddPartLane(send: send)
            }
            // Right under the lanes, so their ends line up with keys.
            KeyRuler(ticks: layout.ticks, keyboard: keyboard)
        }
        .fixedSize(horizontal: false, vertical: true)
    }
}

/// `+ Add part`, an empty lane under the parts: splits the selected part
/// in two.
private struct AddPartLane: View {
    let send: (Event) -> Void

    var body: some View {
        Button { send(.addPart) } label: {
            HStack(spacing: GoKeys.tight) {
                Image(systemName: "plus").font(.system(size: GoKeys.t(10), weight: .bold))
                Text("Add part").font(GoKeysText.name.font)
                Spacer(minLength: 0)
            }
            .foregroundStyle(GoKeys.dim)
            .padding(.horizontal, GoKeys.gap)
            .frame(maxWidth: .infinity, minHeight: GoKeys.row, maxHeight: GoKeys.row)
            .overlay {
                DottedOutline()
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help("Add a part: splits the selected part in two")
        .accessibilityHint("Splits the selected part in two")
    }
}

/// The octaves marked under the lanes, drawn on a keyboard.
struct KeyRuler: View {
    let ticks: [NoteTick]
    let keyboard: KeyAxis

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let key = keyboard.keyWidth(width)
            ZStack(alignment: .topLeading) {
                Rectangle().fill(GoKeys.keys)
                // Black keys over the white.
                ForEach(keyboard.first ... keyboard.last, id: \.self) { note in
                    if [1, 3, 6, 8, 10].contains(note % 12) {
                        Rectangle().fill(GoKeys.keysBlack)
                            .frame(width: key, height: proxy.size.height * 0.5)
                            .offset(x: keyboard.x(note, width: width))
                    }
                }
                // Under the black keys; the last kept inside the end.
                ForEach(ticks, id: \.note) { tick in
                    Text(tick.label)
                        .goKeys(.tick, color: GoKeys.keysBlack)
                        .fixedSize()
                        .offset(
                            x: min(keyboard.x(Int(tick.note), width: width) + 2, width - GoKeys.t(18)),
                            y: proxy.size.height * 0.52
                        )
                }
            }
        }
        .frame(height: GoKeys.row)
        .clipShape(RoundedRectangle(cornerRadius: GoKeys.radius))
        .accessibilityHidden(true)
    }
}

/// One part's lane: its band, over the keys it plays.
private struct ZoneLane: View {
    let zone: ZoneView
    let keyboard: KeyAxis
    let send: (Event) -> Void

    /// What's being dragged, and the key it's on, while dragging.
    @State private var dragging: (grip: RangeGrip.Grip, at: Int)?
    /// When the band was last tapped, for a double tap.
    @State private var tapped: Date?

    private var range: RangeGrip { RangeGrip(keyboard: keyboard, low: zone.low, high: zone.high) }

    var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            ZStack(alignment: .topLeading) {
                // Keys the part doesn't play: nothing drawn, still a drag
                // target.
                Color.clear
                band(in: size)
            }
            .contentShape(Rectangle())
            .gesture(drag(in: size))
            .rangeCursor { range.grab(at: $0, width: size.width) }
        }
        .frame(height: GoKeys.row)
        .help("\(zone.title): click to pick its sound, double-click to edit it. Drag an end to set its keys, or the middle to move them.")
        .contextMenu {
            Button("Edit \(zone.title)") { send(.openEditor(zone.part)) }
            if let remove = zone.remove {
                Button(remove, role: .destructive) { send(.removePart(zone.part)) }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(zone.title), \(zone.sound)")
        .accessibilityValue(zone.keys)
        .accessibilityAddTraits(zone.editing ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction { send(.editPart(zone.part)) }
        .accessibilityAction(named: "Edit") { send(.openEditor(zone.part)) }
    }

    private func band(in size: CGSize) -> some View {
        let (from, to) = range.edges(width: size.width)
        // The selected part in the keyboard's grey pads, as a selection in
        // the lists; the others in its wine-red ones.
        let pad: Pad = zone.editing ? .silver : .wine
        let text = pad.text
        let dim = pad.dim
        return ZStack(alignment: .leading) {
            PadFill(pad: pad)
            HStack(spacing: GoKeys.gap) {
                Text(zone.sound)
                    .font(GoKeysText.name.font)
                    .foregroundStyle(text)
                    .lineLimit(1)
                    .layoutPriority(2)
                if !zone.category.isEmpty {
                    Text(zone.category).goKeys(.detail, color: dim).lineLimit(1)
                }
                Spacer(minLength: 0)
                // The core's words for its keys, updated as it moves.
                Text(zone.keys)
                    .goKeys(.readout, color: dragging == nil ? dim : text)
                    .lineLimit(1)
                    .layoutPriority(dragging == nil ? 0 : 3)
            }
            .padding(.horizontal, GoKeys.gap)
            GripMarks(color: dim)
        }
        .frame(width: max(to - from, 4), height: size.height)
        .clipped()
        .offset(x: from)
    }

    private func drag(in size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { gesture in
                let width = size.width
                let grip: RangeGrip.Grip
                if let dragging {
                    grip = dragging.grip
                } else {
                    guard let found = range.grab(at: gesture.startLocation.x, width: width),
                          range.moves(found, by: gesture.translation.width, width: width)
                    else { return }
                    grip = found
                }
                let at = range.target(grip, at: gesture.location.x, moved: gesture.translation.width, width: width)
                guard at != dragging?.at else { return }
                dragging = (grip, at)
                let (low, high): (Int, Int) = switch grip {
                case .low: (at, Int(zone.high))
                case .high: (Int(zone.low), at)
                case let .band(from, to): (from + at, to + at)
                }
                send(.setKeys(part: zone.part, low: UInt8(low), high: UInt8(high)))
            }
            .onEnded { _ in
                defer { dragging = nil }
                guard dragging == nil else { return }
                if let tapped, Date().timeIntervalSince(tapped) < 0.35 {
                    self.tapped = nil
                    send(.openEditor(zone.part))
                } else {
                    tapped = Date()
                    send(.editPart(zone.part))
                }
            }
    }
}

/// The GO:KEYS pads' two finishes: dotted, silver-grey and wine-red.
enum Pad {
    case silver, wine

    private var baseHex: UInt32 {
        switch self {
        case .silver: GoKeys.silver
        case .wine: GoKeys.wine
        }
    }

    private var dotsHex: UInt32 {
        switch self {
        case .silver: GoKeys.silverDots
        case .wine: GoKeys.wineDots
        }
    }

    var base: Color { Color(hex: baseHex) }

    /// Text on it.
    var text: Color {
        switch self {
        case .silver: GoKeys.onSilver
        case .wine: GoKeys.onWine
        }
    }

    /// Small print on it.
    var dim: Color {
        switch self {
        case .silver: GoKeys.onSilverDim
        case .wine: GoKeys.onWineDim
        }
    }

    /// One repeat of its dots, drawn once: a pad is filled with it rather
    /// than with thousands of dots drawn one by one.
    var tile: CGImage {
        switch self {
        case .silver: Self.silverTile
        case .wine: Self.wineTile
        }
    }

    private static let silverTile = Pad.silver.makeTile()
    private static let wineTile = Pad.wine.makeTile()

    /// Two rows of dots a pitch apart, the second offset half a pitch, as a
    /// perforated sheet; at twice the points, for sharp dots.
    private func makeTile() -> CGImage {
        let pixels = 2.0
        let pitch = 3.0 * pixels
        let dot = 1.3 * pixels
        let width = Int(pitch), height = Int(pitch * 2)
        let cgColor = { (hex: UInt32) in
            CGColor(
                srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
                green: CGFloat((hex >> 8) & 0xFF) / 255,
                blue: CGFloat(hex & 0xFF) / 255,
                alpha: 1
            )
        }
        let context = CGContext(
            data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
            space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )!
        context.setFillColor(cgColor(baseHex))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        context.setFillColor(cgColor(dotsHex))
        for (x, y) in [(pitch / 2, pitch / 2), (0, pitch * 1.5), (pitch, pitch * 1.5)] {
            context.fillEllipse(in: CGRect(x: x - dot / 2, y: y - dot / 2, width: dot, height: dot))
        }
        return context.makeImage()!
    }
}

/// A pad's face: its colour, dotted all over like the keyboard's pads,
/// with slightly rounded corners.
struct PadFill: View {
    let pad: Pad

    var body: some View {
        Rectangle()
            .fill(ImagePaint(image: Image(decorative: pad.tile, scale: 2)))
            .clipShape(RoundedRectangle(cornerRadius: GoKeys.radius))
            .accessibilityHidden(true)
    }
}

/// A setup's parts as thin key-grey bars across the keyboard, one a line.
struct MiniZones: View {
    let zones: [KeysView]
    let keyboard: KeyAxis

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let lane = (proxy.size.height - CGFloat(max(zones.count - 1, 0))) / CGFloat(max(zones.count, 1))
            ZStack(alignment: .topLeading) {
                Rectangle().fill(GoKeys.control)
                ForEach(Array(zones.enumerated()), id: \.offset) { row, zone in
                    let low = keyboard.clamp(Int(zone.low))
                    let high = keyboard.clamp(Int(zone.high))
                    let from = keyboard.x(low, width: width)
                    Rectangle().fill(GoKeys.keys)
                        .frame(width: max(keyboard.x(high + 1, width: width) - from, 2), height: lane)
                        .offset(x: from, y: CGFloat(row) * (lane + 1))
                }
            }
        }
        .accessibilityHidden(true)
    }
}

/// A sheet's title, with Done (Return) on the right.
struct SheetHeader: View {
    let title: String
    let done: () -> Void

    var body: some View {
        HStack {
            Text(title).goKeys(.title)
            Spacer()
            Button("Done", action: done)
                .buttonStyle(FlatButtonStyle())
                .keyboardShortcut(.defaultAction)
        }
        .padding(GoKeys.page)
    }
}

/// A search field in a dark well: its prompt, a count or clear button on
/// the right, and Esc to clear it. `focused` is the field's focus, for
/// shortcuts that shouldn't fire while typing.
struct SearchBox<Trailing: View>: View {
    @Binding var text: String
    let prompt: String
    var focused: FocusState<Bool>.Binding
    @ViewBuilder var trailing: Trailing

    var body: some View {
        HStack(spacing: GoKeys.tight) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: GoKeys.t(12)))
                .foregroundStyle(GoKeys.dim)
            TextField("", text: $text, prompt: Text(prompt).foregroundStyle(GoKeys.dim))
                .textFieldStyle(.plain)
                .font(GoKeysText.row.font)
                .foregroundStyle(GoKeys.text)
                .autocorrectionDisabled()
                #if os(iOS)
                .textInputAutocapitalization(.never)
                .submitLabel(.search)
                #endif
                .focused(focused)
                .onKeyPress(.escape) {
                    guard !text.isEmpty else { return .ignored }
                    text = ""
                    return .handled
                }
            trailing
            if !text.isEmpty {
                Button { text = "" } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(GoKeys.dim)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Clear search")
            }
        }
        .padding(.horizontal, GoKeys.gap)
        .frame(height: GoKeys.row)
        .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
        .overlay {
            RoundedRectangle(cornerRadius: GoKeys.radius)
                .strokeBorder(focused.wrappedValue ? Pad.silver.base : GoKeys.line, lineWidth: 1)
        }
    }
}

/// What an empty list says, centred in it.
struct EmptyMessage: View {
    let text: String

    var body: some View {
        Text(text)
            .goKeys(.row, color: GoKeys.dim)
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity, alignment: .center)
            .padding(.vertical, GoKeys.empty)
    }
}
