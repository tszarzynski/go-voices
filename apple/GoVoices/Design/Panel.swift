import App
import SwiftUI

/// A flat button: grey, square-cornered, small caps. `on` fills it with the
/// accent, for toggles; `compact` makes it shorter, for the title bar.
struct FlatButtonStyle: ButtonStyle {
    var on = false
    var compact = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(GoKeysText.label.font)
            .tracking(GoKeysText.label.tracking)
            .textCase(.uppercase)
            .foregroundStyle(on ? .white : GoKeys.text)
            .lineLimit(1)
            .padding(.horizontal, GoKeys.s(compact ? 8 : 10))
            .frame(minHeight: GoKeys.s(compact ? 22 : 28))
            .background(on ? GoKeys.accent : (configuration.isPressed ? GoKeys.controlPressed : GoKeys.control))
            .contentShape(Rectangle())
            .focusEffectDisabled()
    }
}

/// A round button as on the GO:KEYS panel: the panel's grey, with a red
/// rim and a white symbol. Pressed, the face lightens.
struct RoundButtonStyle: ButtonStyle {
    var size: CGFloat = GoKeys.s(52)

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(.white)
            .frame(width: size, height: size)
            .background {
                // The rim, then the face set into it: two fills draw a
                // cleaner ring than a stroke.
                ZStack {
                    Circle().fill(GoKeys.accent)
                    Circle()
                        .fill(configuration.isPressed ? GoKeys.controlPressed : GoKeys.panel)
                        .padding(2.5)
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
    var size: CGFloat = GoKeys.s(7)

    var body: some View {
        Circle().fill(color).frame(width: size, height: size)
    }
}

/// The display: a backlit amber LCD with dark dot-matrix text.
struct Display<Content: View>: View {
    /// One of two displays, not the one being edited.
    var dimmed = false
    @ViewBuilder let content: Content

    var body: some View {
        content
            .padding(.horizontal, GoKeys.s(10))
            .padding(.vertical, GoKeys.s(6))
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(GoKeys.display.opacity(dimmed ? 0.45 : 1))
            .overlay { Rectangle().strokeBorder(Color.black, lineWidth: 2) }
    }
}

/// A flat button that opens a menu of choices.
struct FlatMenu<Content: View>: View {
    let title: String
    /// What it sets, for VoiceOver.
    let label: String
    @ViewBuilder let content: Content

    var body: some View {
        Menu {
            content
        } label: {
            HStack(spacing: GoKeys.s(6)) {
                Text(title)
                Image(systemName: "chevron.up.chevron.down").font(.system(size: GoKeys.s(9), weight: .bold))
            }
        }
        .menuStyle(.button)
        .menuIndicator(.hidden)
        .buttonStyle(FlatButtonStyle())
        .fixedSize()
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
                .font(.system(size: GoKeys.s(12)))
                .foregroundStyle(color)
                .frame(width: target, height: target)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
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
            Spacer(minLength: GoKeys.s(4))
            Button(action: dismiss) {
                Image(systemName: "xmark")
                    .font(.system(size: GoKeys.s(10), weight: .bold))
                    .foregroundStyle(GoKeys.displayText)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Dismiss")
        }
    }
}

/// Buttons joined by 1px lines, as one control.
struct ToggleGroup<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        HStack(spacing: 1) { content }
            .padding(1)
            .background(GoKeys.line)
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

    static let size = GoKeys.s(44)
    /// Points of drag for the whole range.
    private static let travel: CGFloat = 160

    var body: some View {
        VStack(spacing: GoKeys.s(4)) {
            Text(caps: control.label).goKeys(.label, color: GoKeys.dim).lineLimit(1)
            dial
                .frame(width: Self.size, height: Self.size)
                .contentShape(Circle())
                .gesture(drag)
                .onTapGesture(count: 2, perform: reset)
                .contextMenu { Button("Reset", action: reset) }
            Text(control.text).goKeys(.detail, color: control.edited ? GoKeys.text : GoKeys.dim)
        }
        .frame(maxWidth: .infinity)
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

/// A setting with choices, as a grid cell: its label, then joined buttons
/// where they fit, or a menu, at the height of a knob's dial so the cells
/// of a row line up.
struct ChoiceCell: View {
    let control: ControlView
    let set: (Int16) -> Void

    var body: some View {
        VStack(spacing: GoKeys.s(4)) {
            Text(caps: control.label).goKeys(.label, color: GoKeys.dim).lineLimit(1)
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
        .accessibilityElement(children: .contain)
        .accessibilityLabel(control.label)
    }
}

/// Groups of settings on one grid of equal cells: a knob takes a cell; a
/// choice or an envelope's shape takes two. Groups go left to right in
/// order, each starting on a column and moving to the next row whole when
/// it doesn't fit, so knobs line up down the page and across groups.
struct ControlGrid: View {
    let groups: [GroupView]
    /// The keyboard, to draw a layer's keys on.
    var keyboard: KeyAxis?
    let send: (Event) -> Void

    /// The grid every Edit panel view lines up on.
    static let grid = GridLayout(minCell: GoKeys.s(76), gap: GoKeys.s(14))

    var body: some View {
        Self.grid {
            ForEach(Array(groups.enumerated()), id: \.element.title) { index, group in
                HStack(spacing: GoKeys.gap) {
                    // Never cut short: the group is made wide enough.
                    Text(caps: group.title).goKeys(.label, color: GoKeys.dim).fixedSize()
                    GoKeys.line.frame(minWidth: GoKeys.s(16), maxHeight: 1)
                }
                .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .heading))
                ForEach(group.controls, id: \.setting) { control in
                    if control.choices.isEmpty {
                        Knob(control: control) { send(control.set($0)) } reset: { send(control.reset) }
                            .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(1)))
                    } else {
                        ChoiceCell(control: control) { send(control.set($0)) }
                            .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(control.cells)))
                    }
                }
                if let keys = group.keys, let keyboard {
                    KeyRangeStrip(range: keys, keyboard: keyboard, send: send)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(6)))
                }
                if !group.envelope.isEmpty {
                    EnvelopeShape(points: group.envelope)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .cell(2)))
                }
                if !group.note.isEmpty {
                    Text(group.note).goKeys(.row, color: GoKeys.dim)
                        .fixedSize(horizontal: false, vertical: true)
                        .layoutValue(key: GridSlot.self, value: GridSlot(group: index, kind: .note))
                }
            }
        }
    }
}

extension ControlView {
    /// Cells a choice takes: one when its options are a few letters
    /// (`Off`/`On`, an octave, a key), two otherwise.
    var cells: Int { choices.allSatisfy { $0.label.count <= 3 } ? 1 : 2 }
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
        let width = proposal.width ?? minCell * 8
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
        let columns = max(1, Int((width + gap) / (minCell + gap)))
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
        let headingGap = GoKeys.s(14)
        let lineGap = GoKeys.s(16)
        let rowGap = GoKeys.s(32)

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
            if column > 0, column + groupColumns > columns {
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
                frames[member] = CGRect(x: left, y: bottom + GoKeys.s(10), width: groupWidth, height: height)
                bottom += GoKeys.s(10) + height
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

/// The zone map: one lane per part across the keyboard's keys, each part a
/// band from its lowest to its highest key. Tap a band to edit that part;
/// drag either end of a band to move it. Each part's keys move on their
/// own: overlapping bands are layered.
struct ZoneMap: View {
    let layout: LayoutView
    let send: (Event) -> Void

    private var keyboard: KeyAxis { KeyAxis(first: layout.first, last: layout.last) }

    var body: some View {
        VStack(spacing: GoKeys.s(3)) {
            ForEach(layout.zones, id: \.part) { zone in
                ZoneLane(zone: zone, keyboard: keyboard, send: send)
            }
            KeyRuler(ticks: layout.ticks, keyboard: keyboard)
        }
    }
}

/// The octaves marked under the lanes.
private struct KeyRuler: View {
    let ticks: [NoteTick]
    let keyboard: KeyAxis

    var body: some View {
        GeometryReader { proxy in
            ForEach(ticks, id: \.note) { tick in
                Text(tick.label)
                    .goKeys(.detail, color: GoKeys.dim)
                    .fixedSize()
                    .offset(x: keyboard.x(Int(tick.note), width: proxy.size.width))
            }
        }
        .frame(height: GoKeys.s(12))
        .padding(.leading, ZoneLane.labelWidth + GoKeys.s(6))
        .accessibilityHidden(true)
    }
}

/// One part's lane: its number, then its band.
private struct ZoneLane: View {
    let zone: ZoneView
    let keyboard: KeyAxis
    let send: (Event) -> Void

    static let labelWidth = GoKeys.s(14)
    /// How near an end a drag must start to move it.
    private static let grip = GoKeys.s(14)

    /// The end being dragged, and the key it's on, while dragging.
    @State private var dragging: (end: End, note: Int)?

    private enum End { case low, high }

    private var low: Int { keyboard.clamp(Int(zone.low)) }
    private var high: Int { keyboard.clamp(Int(zone.high)) }

    var body: some View {
        HStack(spacing: GoKeys.s(6)) {
            Text(zone.label)
                .goKeys(.label, color: zone.editing ? GoKeys.text : GoKeys.dim)
                .frame(width: Self.labelWidth, alignment: .trailing)
            GeometryReader { proxy in
                let size = proxy.size
                ZStack(alignment: .topLeading) {
                    Rectangle().fill(GoKeys.control)
                    band(in: size)
                }
                .contentShape(Rectangle())
                .gesture(drag(in: size))
            }
        }
        .frame(height: GoKeys.s(22))
        .contextMenu {
            Button("Edit \(zone.title)") { send(.editPart(zone.part)) }
            if let remove = zone.remove {
                Button(remove, role: .destructive) { send(.removePart(zone.part)) }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(zone.title), \(zone.sound)")
        .accessibilityValue(zone.keys)
        .accessibilityAddTraits(zone.editing ? [.isButton, .isSelected] : .isButton)
        .accessibilityAction { send(.editPart(zone.part)) }
    }

    private func band(in size: CGSize) -> some View {
        let from = keyboard.x(low, width: size.width)
        let to = keyboard.x(high + 1, width: size.width)
        let text: Color = zone.editing ? .white : GoKeys.text
        return ZStack(alignment: .leading) {
            Rectangle().fill(zone.editing ? GoKeys.accent : GoKeys.dim.opacity(0.45))
            HStack(spacing: GoKeys.s(6)) {
                Text(zone.sound).goKeys(.detail, color: text).lineLimit(1)
                // The core's words for its keys, updated as an end moves.
                Text(zone.keys)
                    .goKeys(.detail, color: text.opacity(dragging == nil ? 0.75 : 1))
                    .lineLimit(1)
                    .layoutPriority(dragging == nil ? -1 : 1)
            }
            .padding(.horizontal, GoKeys.s(6))
            // Grips at the ends.
            HStack {
                Rectangle().fill(text.opacity(0.6)).frame(width: 2)
                Spacer(minLength: 0)
                Rectangle().fill(text.opacity(0.6)).frame(width: 2)
            }
            .padding(.vertical, GoKeys.s(5))
            .padding(.horizontal, GoKeys.s(2))
        }
        .frame(width: max(to - from, 4), height: size.height)
        .clipped()
        .offset(x: from)
    }

    private func drag(in size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { gesture in
                let width = size.width
                let end: End
                if let dragging {
                    end = dragging.end
                } else {
                    // Which end it started near, if either.
                    let start = gesture.startLocation.x
                    let (from, to) = (keyboard.x(low, width: width), keyboard.x(high + 1, width: width))
                    let near = [(End.low, abs(start - from)), (End.high, abs(start - to))]
                        .filter { $0.1 <= Self.grip }
                        .min { $0.1 < $1.1 }
                    guard let near, abs(gesture.translation.width) > 2 else { return }
                    end = near.0
                }
                // An end sits between keys: the key on its side of the line.
                let half = keyboard.keyWidth(width) / 2
                let at = keyboard.note(at: gesture.location.x + (end == .low ? half : -half), width: width)
                guard at != dragging?.note else { return }
                dragging = (end, at)
                switch end {
                case .low: send(.setKeys(part: zone.part, low: UInt8(at), high: zone.high))
                case .high: send(.setKeys(part: zone.part, low: zone.low, high: UInt8(at)))
                }
            }
            .onEnded { _ in
                if dragging == nil { send(.editPart(zone.part)) }
                dragging = nil
            }
    }
}
