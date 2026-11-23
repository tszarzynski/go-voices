import App
import SwiftUI

/// The Layers tab: the sound's four layers as cards (tap one to edit it,
/// switch it on or off, play it alone), its wave, then its settings in
/// groups, with each envelope drawn above its knobs.
struct LayersContent: View {
    let layers: LayersView
    let send: (Event) -> Void

    private var keyboard: KeyAxis { KeyAxis(first: layers.first, last: layers.last) }

    var body: some View {
        if let status = layers.status {
            Text(status)
                .goKeys(.row, color: GoKeys.dim)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, GoKeys.s(40))
        } else {
            VStack(alignment: .leading, spacing: GoKeys.s(28)) {
                // Two cells each, on the knobs' columns.
                ControlGrid.grid {
                    ForEach(layers.cards, id: \.layer) { card in
                        LayerCard(card: card, keyboard: keyboard, paste: layers.paste, send: send)
                            .layoutValue(key: GridSlot.self, value: GridSlot(group: 0, kind: .cell(2)))
                    }
                }
                if let wave = layers.wave {
                    WaveRow(wave: wave, send: send)
                }
                ControlGrid(groups: layers.groups, keyboard: keyboard, send: send)
            }
        }
    }
}

/// One layer: its number and wave; filled red while it's the one edited,
/// dimmed while it's off. Its buttons switch it on or off and play it alone;
/// its menu copies it, pastes a layer copied onto it, and resets it.
private struct LayerCard: View {
    let card: LayerCardView
    let keyboard: KeyAxis
    /// The layer copied, to paste onto this one.
    let paste: String?
    let send: (Event) -> Void

    var body: some View {
        let text: Color = card.selected ? .white : GoKeys.text
        VStack(alignment: .leading, spacing: GoKeys.s(6)) {
            HStack(spacing: GoKeys.s(6)) {
                LedDot(color: card.on ? GoKeys.green : GoKeys.dim.opacity(0.5))
                Text(caps: card.label).goKeys(.label, color: text)
                if card.edited {
                    Text("•").goKeys(.label, color: text)
                        .accessibilityLabel("Changed")
                }
                Spacer(minLength: 0)
                Text(card.level).goKeys(.detail, color: card.selected ? .white.opacity(0.8) : GoKeys.dim)
                    .accessibilityLabel("Level \(card.level)")
            }
            Text(card.wave)
                .goKeys(.row, color: text)
                .lineLimit(1)
                .opacity(card.on ? 1 : 0.5)
            KeyBar(low: Int(card.low), high: Int(card.high), keyboard: keyboard,
                   color: card.selected ? .white : GoKeys.accent)
                .opacity(card.on ? 1 : 0.5)
            Text(card.detail)
                .goKeys(.detail, color: card.selected ? .white.opacity(0.8) : GoKeys.dim)
                .lineLimit(1)
            HStack(spacing: 1) {
                Button(card.on ? "On" : "Off") { send(.toggleLayer(card.layer)) }
                .buttonStyle(FlatButtonStyle(on: card.on, compact: true))
                .accessibilityLabel(card.on ? "Switch \(card.label) off" : "Switch \(card.label) on")
                Button("Solo") { send(.soloLayer(card.soloed ? nil : card.layer)) }
                    .buttonStyle(FlatButtonStyle(on: card.soloed, compact: true))
                    .accessibilityAddTraits(card.soloed ? .isSelected : [])
            }
        }
        .padding(GoKeys.s(10))
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(card.selected ? GoKeys.accent : GoKeys.panel)
        .overlay { Rectangle().strokeBorder(GoKeys.line, lineWidth: 1) }
        .contentShape(Rectangle())
        .onTapGesture { send(.selectLayer(card.layer)) }
        .contextMenu {
            Button("Copy \(card.label)") { send(.copyLayer(card.layer)) }
            Button(paste.map { "Paste \($0)" } ?? "Paste") { send(.pasteLayer(card.layer)) }
                .disabled(paste == nil)
            Divider()
            Button("Reset \(card.label)") { send(.resetLayers(card.layer)) }
                .disabled(!card.edited)
        }
        .accessibilityElement(children: .contain)
        .accessibilityAddTraits(card.selected ? [.isButton, .isSelected] : .isButton)
    }
}

/// Which of the keyboard's keys a layer plays, as a lit stretch of a bar.
private struct KeyBar: View {
    let low: Int
    let high: Int
    let keyboard: KeyAxis
    let color: Color

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let from = keyboard.x(keyboard.clamp(low), width: width)
            let to = keyboard.x(keyboard.clamp(high) + 1, width: width)
            ZStack(alignment: .leading) {
                Rectangle().fill(GoKeys.line)
                Rectangle().fill(color)
                    .frame(width: max(0, to - from))
                    .offset(x: from)
            }
        }
        .frame(height: GoKeys.s(4))
        .accessibilityHidden(true)
    }
}

/// The selected layer's wave: step through waves with ‹ ›, or choose one
/// from the list.
private struct WaveRow: View {
    let wave: WaveView
    let send: (Event) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: GoKeys.s(10)) {
            HStack(spacing: GoKeys.gap) {
                Text(caps: "Wave").goKeys(.label, color: GoKeys.dim)
                GoKeys.line.frame(height: 1)
            }
            HStack(spacing: GoKeys.gap) {
                ToggleGroup {
                    Button { send(.stepWave(false)) } label: { Image(systemName: "chevron.left") }
                        .buttonStyle(FlatButtonStyle())
                        .accessibilityLabel("Previous wave")
                    Button { send(.stepWave(true)) } label: { Image(systemName: "chevron.right") }
                        .buttonStyle(FlatButtonStyle())
                        .accessibilityLabel("Next wave")
                }
                .fixedSize()
                VStack(alignment: .leading, spacing: 0) {
                    Text(wave.name).goKeys(.row).lineLimit(1)
                    Text(wave.detail).goKeys(.detail, color: GoKeys.dim).lineLimit(1)
                }
                Spacer(minLength: GoKeys.gap)
                Button("Choose…") { send(.searchWaves("")) }
                    .buttonStyle(FlatButtonStyle())
            }
        }
    }
}

/// An envelope's shape: the points (0–1000 across and up) joined, and
/// filled below.
struct EnvelopeShape: View {
    let points: [PointView]

    var body: some View {
        GeometryReader { proxy in
            let size = proxy.size
            let at = { (point: PointView) in
                CGPoint(
                    x: CGFloat(point.x) / 1000 * size.width,
                    y: size.height - CGFloat(point.y) / 1000 * size.height
                )
            }
            let line = Path { path in
                guard let first = points.first else { return }
                path.move(to: at(first))
                for point in points.dropFirst() { path.addLine(to: at(point)) }
            }
            ZStack {
                Path { path in
                    path.addPath(line)
                    path.addLine(to: CGPoint(x: size.width, y: size.height))
                    path.addLine(to: CGPoint(x: 0, y: size.height))
                    path.closeSubpath()
                }
                .fill(GoKeys.accent.opacity(0.15))
                line.stroke(GoKeys.accent, style: StrokeStyle(lineWidth: 2, lineJoin: .round))
            }
        }
        .padding(GoKeys.s(6))
        .background(GoKeys.panel)
        .accessibilityHidden(true)
    }
}

/// Every wave a layer can have: search, and tap one to hear it on the
/// selected layer; the list stays open to try others.
struct WavePicker: View {
    let picker: WavePickerView
    let send: (Event) -> Void

    @State private var query = ""

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(caps: "Waves").goKeys(.label, color: .white)
                Spacer()
                Button("Done") { send(.searchWaves(nil)) }
                    .buttonStyle(FlatButtonStyle())
                    .keyboardShortcut(.defaultAction)
            }
            .padding(GoKeys.page)
            .background(GoKeys.panel)
            TextField(picker.prompt, text: $query)
                .textFieldStyle(.plain)
                .font(GoKeysText.row.font)
                .foregroundStyle(GoKeys.text)
                .padding(GoKeys.s(8))
                .background(GoKeys.control)
                .padding(GoKeys.page)
                .onChange(of: query) { _, query in send(.searchWaves(query)) }
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0) {
                        if let empty = picker.empty {
                            Text(empty).goKeys(.row, color: GoKeys.dim).padding(.vertical, GoKeys.s(40))
                        }
                        ForEach(picker.rows, id: \.self) { row in
                            Button { send(.setWave(group: row.group, number: row.number)) } label: {
                                HStack {
                                    Text(row.name).goKeys(.row, color: row.current ? .white : GoKeys.text)
                                    Spacer(minLength: GoKeys.gap)
                                    Text(row.detail)
                                        .goKeys(.detail, color: row.current ? .white.opacity(0.8) : GoKeys.dim)
                                }
                                .frame(minHeight: GoKeys.s(32))
                                .padding(.horizontal, GoKeys.page)
                                .background(row.current ? GoKeys.accent : GoKeys.background)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .id(row)
                            .accessibilityAddTraits(row.current ? .isSelected : [])
                        }
                    }
                }
                .onAppear {
                    if let current = picker.rows.first(where: \.current) { proxy.scrollTo(current, anchor: .center) }
                }
            }
        }
        .background(GoKeys.background)
        .onAppear { query = picker.query }
        #if os(macOS)
        .frame(minWidth: 420, minHeight: 520)
        #endif
    }
}

extension ControlView {
    /// The event that sets this control to `value`.
    func set(_ value: Int16) -> Event {
        switch setting {
        case let .part(control): .setControl(control: control, value: value)
        case let .layer(control): .setLayerControl(control: control, value: value)
        }
    }

    /// The event that puts it back.
    var reset: Event {
        switch setting {
        case let .part(control): .resetControl(control)
        case let .layer(control): .resetLayerControl(control)
        }
    }
}

/// The selected layer's keys on a drawn keyboard, lit from its lowest to its
/// highest. Click or drag on a key to move the nearer end there; at the
/// keyboard's ends, a range takes in every key past them too. Double-click
/// (or the menu) puts both ends back.
struct KeyRangeStrip: View {
    let range: KeyRangeView
    let keyboard: KeyAxis
    let send: (Event) -> Void

    /// The end being dragged, and the key it's on.
    @State private var dragging: LayerControl?
    @State private var sent: Int?

    private var first: Int { keyboard.first }
    private var last: Int { keyboard.last }
    private var low: Int { keyboard.clamp(Int(range.low.value)) }
    private var high: Int { keyboard.clamp(Int(range.high.value)) }
    private var edited: Bool { range.low.edited || range.high.edited }

    private static func isBlack(_ note: Int) -> Bool { [1, 3, 6, 8, 10].contains(note % 12) }

    var body: some View {
        VStack(spacing: GoKeys.s(6)) {
            GeometryReader { proxy in
                keys(in: proxy.size)
                    .contentShape(Rectangle())
                    .gesture(drag(in: proxy.size))
                    .simultaneousGesture(TapGesture(count: 2).onEnded(reset))
            }
            Text(range.text).goKeys(.detail, color: edited ? GoKeys.text : GoKeys.dim).lineLimit(1)
        }
        .contextMenu { Button("Reset keys", action: reset) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Keys")
        .accessibilityValue(range.text)
    }

    private func reset() { send(.resetLayerKeys) }

    private var whites: [Int] { (first ... last).filter { !Self.isBlack($0) } }

    /// The white keys side by side, the black ones over them; lit in range.
    private func keys(in size: CGSize) -> some View {
        let whites = whites
        let width = size.width / CGFloat(whites.count)
        return ZStack(alignment: .topLeading) {
            ForEach(Array(whites.enumerated()), id: \.element) { index, note in
                Rectangle()
                    .fill(Color(white: 0.82))
                    .overlay { Rectangle().fill(GoKeys.accent.opacity(strength(note))) }
                    .overlay(alignment: .trailing) { Rectangle().fill(GoKeys.background).frame(width: 1) }
                    .frame(width: width, height: size.height)
                    .offset(x: CGFloat(index) * width)
            }
            ForEach((first ... last).filter(Self.isBlack), id: \.self) { note in
                Rectangle()
                    .fill(Color(white: 0.12))
                    .overlay { Rectangle().fill(GoKeys.accent.opacity(0.7 * strength(note))) }
                    .frame(width: width * 0.6, height: size.height * 0.6)
                    .offset(x: blackX(note, width: width))
            }
        }
        .frame(width: size.width, height: size.height, alignment: .topLeading)
        .clipped()
    }

    /// How loud a key plays the layer, 0 to 1: full in its range, fading
    /// over the fade widths either side.
    private func strength(_ note: Int) -> Double {
        let raw = (low: Int(range.low.value), high: Int(range.high.value))
        if (raw.low ... max(raw.low, raw.high)).contains(note) { return 1 }
        let below = Int(range.fadeLow), above = Int(range.fadeHigh)
        if note < raw.low, below > 0, raw.low - note <= below {
            return 1 - Double(raw.low - note) / Double(below + 1)
        }
        if note > raw.high, above > 0, note - raw.high <= above {
            return 1 - Double(note - raw.high) / Double(above + 1)
        }
        return 0
    }

    /// A black key's left edge: across the line between its two white keys.
    private func blackX(_ note: Int, width: CGFloat) -> CGFloat {
        let before = whites.filter { $0 < note }.count
        return CGFloat(before) * width - width * 0.3
    }

    /// The key under a point: a black one in the top part, else a white one.
    private func note(at point: CGPoint, in size: CGSize) -> Int {
        let width = size.width / CGFloat(whites.count)
        if point.y < size.height * 0.6,
           let black = (first ... last).filter(Self.isBlack).first(where: {
               let x = blackX($0, width: width)
               return point.x >= x && point.x < x + width * 0.6
           }) {
            return black
        }
        let index = min(max(Int(point.x / width), 0), whites.count - 1)
        return whites[index]
    }

    private func drag(in size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { gesture in
                let note = note(at: gesture.location, in: size)
                let end = dragging ?? {
                    if note <= low { return .keyLow }
                    if note >= high { return .keyHigh }
                    return note - low < high - note ? .keyLow : .keyHigh
                }()
                dragging = end
                guard note != sent else { return }
                sent = note
                send(.setLayerControl(control: end, value: Int16(note)))
            }
            .onEnded { _ in
                dragging = nil
                sent = nil
            }
    }
}
