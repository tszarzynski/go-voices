import App
import SwiftUI

/// The Layers tab: the sound's four layers as cards (tap one to edit it,
/// switch it on or off, play it alone), its wave, then its settings in
/// groups, with each envelope drawn above its knobs.
struct LayersContent: View {
    let layers: LayersView
    /// The octaves marked on the keys.
    let ticks: [NoteTick]
    let send: (Event) -> Void

    private var keyboard: KeyAxis { KeyAxis(first: layers.first, last: layers.last) }

    var body: some View {
        if let status = layers.status {
            Text(status)
                .goKeys(.row, color: GoKeys.dim)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, GoKeys.empty)
        } else {
            VStack(alignment: .leading, spacing: GoKeys.section) {
                // Two by two, however narrow: a card fits half a sheet.
                Grid(horizontalSpacing: GoKeys.page, verticalSpacing: GoKeys.page) {
                    ForEach(Array(stride(from: 0, to: layers.cards.count, by: 2)), id: \.self) { row in
                        GridRow {
                            ForEach(layers.cards[row ..< min(row + 2, layers.cards.count)], id: \.layer) { card in
                                LayerCard(card: card, keyboard: keyboard, paste: layers.paste, send: send)
                            }
                        }
                    }
                }
                if let wave = layers.wave {
                    WaveRow(wave: wave, send: send)
                }
                ControlGrid(groups: layers.groups, keyboard: keyboard, ticks: ticks, send: send)
            }
        }
    }
}

/// One layer, in a dark well: its number and level, its wave, the keys it
/// plays as a bar, and what else limits it; the dotted outline while it's
/// the one edited, as a selection in the lists; dimmed while it's off. Its
/// pads switch it on or off and play it alone; its menu copies it, pastes
/// a layer copied onto it, and resets it.
private struct LayerCard: View {
    let card: LayerCardView
    let keyboard: KeyAxis
    /// The layer copied, to paste onto this one.
    let paste: String?
    let send: (Event) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: GoKeys.gap) {
            HStack(spacing: GoKeys.tight) {
                Text(caps: card.label).goKeys(.label, color: GoKeys.dim)
                if card.edited {
                    Text("•").goKeys(.label, color: GoKeys.accent)
                        .accessibilityLabel("Changed")
                }
                Spacer(minLength: 0)
                Text(card.level).goKeys(.readout, color: GoKeys.dim)
                    .accessibilityLabel("Level \(card.level)")
            }
            Group {
                Text(card.wave).goKeys(.name).lineLimit(1)
                KeyBar(low: Int(card.low), high: Int(card.high), keyboard: keyboard)
                Text(card.detail).goKeys(.readout, color: GoKeys.dim)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            .opacity(card.on ? 1 : GoKeys.disabled)
            ToggleGroup {
                Button(card.on ? "On" : "Off") { send(.toggleLayer(card.layer)) }
                    .buttonStyle(FlatButtonStyle(on: card.on, compact: true))
                    .help(card.on ? "Switch this layer off" : "Switch this layer on")
                    .accessibilityLabel(card.on ? "Switch \(card.label) off" : "Switch \(card.label) on")
                Button("Solo") { send(.soloLayer(card.soloed ? nil : card.layer)) }
                    .buttonStyle(FlatButtonStyle(on: card.soloed, compact: true))
                    .help(card.soloed ? "Play every layer again" : "Play this layer alone")
                    .accessibilityAddTraits(card.soloed ? .isSelected : [])
            }
        }
        .padding(GoKeys.gap)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
        .overlay { if card.selected { DottedOutline() } }
        .contentShape(RoundedRectangle(cornerRadius: GoKeys.radius))
        .onTapGesture { send(.selectLayer(card.layer)) }
        .help("\(card.label): click to edit it; right-click to copy, paste or reset it")
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

/// Which of the keyboard's keys a layer plays: a key-grey stretch of a
/// dark bar, as a preset's parts in the list.
private struct KeyBar: View {
    let low: Int
    let high: Int
    let keyboard: KeyAxis

    var body: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let from = keyboard.x(keyboard.clamp(low), width: width)
            let to = keyboard.x(keyboard.clamp(high) + 1, width: width)
            ZStack(alignment: .leading) {
                Capsule().fill(GoKeys.control)
                Capsule().fill(GoKeys.keys)
                    .frame(width: max(0, to - from))
                    .offset(x: from)
            }
        }
        .frame(height: GoKeys.s(5))
        .accessibilityHidden(true)
    }
}

/// The selected layer's wave: step through waves with ‹ ›, or choose one
/// from the list.
private struct WaveRow: View {
    let wave: WaveView
    let send: (Event) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: GoKeys.page) {
            Text("Wave").goKeys(.heading)
            HStack(spacing: GoKeys.gap) {
                ToggleGroup {
                    Button { send(.stepWave(false)) } label: { Image(systemName: "chevron.left") }
                        .buttonStyle(FlatButtonStyle())
                        .help("Previous wave")
                        .accessibilityLabel("Previous wave")
                    Button { send(.stepWave(true)) } label: { Image(systemName: "chevron.right") }
                        .buttonStyle(FlatButtonStyle())
                        .help("Next wave")
                        .accessibilityLabel("Next wave")
                }
                .fixedSize()
                VStack(alignment: .leading, spacing: 0) {
                    Text(wave.name).goKeys(.row).lineLimit(1)
                    Text(wave.detail).goKeys(.readout, color: GoKeys.dim).lineLimit(1)
                }
                Spacer(minLength: GoKeys.gap)
                Button("Choose…") { send(.searchWaves("")) }
                    .buttonStyle(FlatButtonStyle())
                    .help("Choose from every wave, by name")
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
                .fill(Pad.silver.base.opacity(0.15))
                line.stroke(Pad.silver.base, style: StrokeStyle(lineWidth: 2, lineJoin: .round))
            }
        }
        .padding(GoKeys.gap)
        .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
        .accessibilityHidden(true)
    }
}

/// Every wave a layer can have: search, and tap one to hear it on the
/// selected layer; the list stays open to try others.
struct WavePicker: View {
    let picker: WavePickerView
    let send: (Event) -> Void

    @State private var query = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(spacing: 0) {
            // As the keyboard sheet: its title, then Done.
            HStack {
                Text("Waves").goKeys(.title)
                Spacer()
                Button("Done") { send(.searchWaves(nil)) }
                    .buttonStyle(FlatButtonStyle())
                    .keyboardShortcut(.defaultAction)
            }
            .padding(GoKeys.page)
            // As the sound list's search field.
            HStack(spacing: GoKeys.tight) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: GoKeys.t(12)))
                    .foregroundStyle(GoKeys.dim)
                TextField("", text: $query, prompt: Text(picker.prompt).foregroundStyle(GoKeys.dim))
                    .textFieldStyle(.plain)
                    .font(GoKeysText.row.font)
                    .foregroundStyle(GoKeys.text)
                    .autocorrectionDisabled()
                    .focused($focused)
                    .onChange(of: query) { _, query in send(.searchWaves(query)) }
            }
            .padding(.horizontal, GoKeys.gap)
            .frame(height: GoKeys.row)
            .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
            .overlay {
                RoundedRectangle(cornerRadius: GoKeys.radius)
                    .strokeBorder(focused ? Pad.silver.base : GoKeys.line, lineWidth: 1)
            }
            .padding([.horizontal, .bottom], GoKeys.page)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0) {
                        if let empty = picker.empty {
                            Text(empty).goKeys(.row, color: GoKeys.dim).padding(.vertical, GoKeys.empty)
                        }
                        // As the sound list: a row high, the one in use
                        // outlined.
                        ForEach(picker.rows, id: \.self) { row in
                            Button { send(.setWave(group: row.group, number: row.number)) } label: {
                                HStack {
                                    Text(row.name)
                                        .goKeys(.row, color: row.current ? GoKeys.selectedText : GoKeys.text)
                                        .fontWeight(row.current ? .semibold : .regular)
                                    Spacer(minLength: GoKeys.gap)
                                    Text(row.detail).goKeys(.readout, color: GoKeys.dim)
                                }
                                .frame(height: GoKeys.row)
                                .padding(.horizontal, GoKeys.page)
                                .selectedRim(row.current)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .id(row)
                            .accessibilityAddTraits(row.current ? .isSelected : [])
                        }
                    }
                    .padding(.bottom, GoKeys.page)
                }
                .onAppear {
                    if let current = picker.rows.first(where: \.current) { proxy.scrollTo(current, anchor: .center) }
                }
            }
        }
        .background(GoKeys.panel)
        .tint(GoKeys.accent)
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

/// The selected layer's keys, drawn as a part's on the arrangement: a
/// silver pad over the same keyboard, its fades shading off either side.
/// Drag an end to move it, or the middle to move both; drag from a key off
/// the pad to bring the nearer end there. At the keyboard's ends, a range
/// takes in every key past them too. Double-click (or the menu) puts both
/// ends back.
struct KeyRangeStrip: View {
    let range: KeyRangeView
    let keyboard: KeyAxis
    let ticks: [NoteTick]
    let send: (Event) -> Void

    /// What's being dragged, and where to, while dragging.
    @State private var dragging: (grip: RangeGrip.Grip, at: Int)?

    private var grip: RangeGrip {
        RangeGrip(keyboard: keyboard, low: UInt8(clamping: range.low.value), high: UInt8(clamping: range.high.value))
    }

    var body: some View {
        VStack(spacing: ZoneMap.spacing) {
            GeometryReader { proxy in
                band(in: proxy.size)
                    .contentShape(Rectangle())
                    .gesture(drag(in: proxy.size))
                    .simultaneousGesture(TapGesture(count: 2).onEnded(reset))
                    .rangeCursor { grip.grab(at: $0, width: proxy.size.width) }
            }
            .frame(height: ZoneMap.row)
            KeyRuler(ticks: ticks, keyboard: keyboard)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .help("The layer's keys: drag an end to set it, or the middle to move both. Double-click to put them back.")
        .contextMenu { Button("Reset keys", action: reset) }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Keys")
        .accessibilityValue(range.text)
    }

    private func reset() { send(.resetLayerKeys) }

    /// The pad from the lowest key to the highest, its fades either side,
    /// with the keys in words on it and grips at its ends.
    private func band(in size: CGSize) -> some View {
        let width = size.width
        let key = keyboard.keyWidth(width)
        let (from, to) = grip.edges(width: width)
        let below = min(CGFloat(range.fadeLow) * key, from)
        let above = min(CGFloat(range.fadeHigh) * key, width - to)
        let silver = Pad.silver.base
        return ZStack(alignment: .topLeading) {
            if below > 0 {
                LinearGradient(colors: [silver.opacity(0), silver.opacity(0.6)], startPoint: .leading, endPoint: .trailing)
                    .frame(width: below, height: size.height)
                    .offset(x: from - below)
            }
            if above > 0 {
                LinearGradient(colors: [silver.opacity(0.6), silver.opacity(0)], startPoint: .leading, endPoint: .trailing)
                    .frame(width: above, height: size.height)
                    .offset(x: to)
            }
            ZStack(alignment: .leading) {
                PadFill(pad: .silver)
                Text(range.text)
                    .goKeys(.readout, color: Pad.silver.text)
                    .lineLimit(1)
                    .padding(.horizontal, GoKeys.gap)
                GripMarks(color: Pad.silver.dim)
                // Changed from the sound's: a red rim, as a changed knob's ring.
                if range.low.edited || range.high.edited {
                    RoundedRectangle(cornerRadius: GoKeys.radius)
                        .strokeBorder(GoKeys.accent, lineWidth: 2)
                }
            }
            .frame(width: max(to - from, 4), height: size.height)
            .offset(x: from)
        }
        .frame(width: width, height: size.height, alignment: .topLeading)
        .clipped()
    }

    private func drag(in size: CGSize) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { gesture in
                let width = size.width
                let held: RangeGrip.Grip
                if let dragging {
                    held = dragging.grip
                } else if let found = grip.grab(at: gesture.startLocation.x, width: width) {
                    held = found
                } else {
                    // Off the pad: the nearer end comes to the key.
                    held = gesture.startLocation.x < grip.edges(width: width).from ? .low : .high
                }
                // Off the pad or on an end, the end comes at once; the band
                // only once it's dragged a key.
                if dragging == nil, case .band = held, !grip.moves(held, by: gesture.translation.width, width: width) { return }
                let at = grip.target(held, at: gesture.location.x, moved: gesture.translation.width, width: width)
                guard at != dragging?.at else { return }
                dragging = (held, at)
                switch held {
                case .low: send(.setLayerControl(control: .keyLow, value: Int16(at)))
                case .high: send(.setLayerControl(control: .keyHigh, value: Int16(at)))
                // Both ends in one change, so neither holds the other back.
                case let .band(from, to): send(.setLayerKeys(low: UInt8(from + at), high: UInt8(to + at)))
                }
            }
            .onEnded { _ in dragging = nil }
    }
}
