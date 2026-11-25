import App
import SwiftUI

/// The one screen: the title bar; the arrangement, a lane per part across
/// the keys; then the browser picking sounds (and presets) for the selected
/// part (the tree beside the list; on a narrow screen, one at
/// a time), with the Edit panel beside it while it's open (a sheet on a
/// narrow screen).
/// Everything it shows comes from the core's view model; taps become events.
struct BrowserScreen: View {
    let view: ViewModel
    let send: (Event) -> Void

    /// iPhone: the keyboard sheet waits for the Edit sheet to close, as
    /// one sheet can't open over another from the same screen.
    @State private var devicesAfterEditor = false
    /// iPhone and narrow iPad: the Edit panel is a sheet.
    @State private var editorIsSheet = false
    /// Narrow screens: the list is showing, rather than the tree.
    @State private var showingList = false
    /// The search field has the keyboard: ⌘← and ⌘→ move its cursor then,
    /// rather than stepping sounds.
    @FocusState private var searching: Bool
    /// The frame's colour, from the View menu.
    @AppStorage(Look.key) private var look = Look.red

    /// Below this width (iPhone, narrow iPad), the tree and the list take
    /// turns.
    private static let splitWidth: CGFloat = 620
    /// From this width (iPad), the Edit panel sits beside the list.
    private static let inspectorWidth: CGFloat = 900

    /// The tree beside the list, and the Edit panel beside them while
    /// open. The Mac always uses it (its window is
    /// kept wide enough): swapping layouts while resizing tears the list's
    /// table view down mid-layout, which AppKit flags as reentrant and can
    /// crash.
    private func split(inspector: Bool) -> some View {
        HStack(spacing: GoKeys.frame) {
            // The tree at a fixed width; the list and the Edit panel share
            // the rest equally.
            PlaceTree(view: view, send: send) {}
                .frame(width: GoKeys.tree)
                .panel()
            VStack(spacing: 0) {
                TargetBar(view: view, searching: $searching, send: send)
                ListColumn(view: view, send: send)
            }
            .frame(minWidth: GoKeys.s(300), maxWidth: .infinity)
            .panel()
            if inspector, let editor = view.editor {
                EditorPanel(editor: editor, waves: view.waves, ticks: view.layout.ticks, send: send)
                    .frame(minWidth: GoKeys.s(300), maxWidth: .infinity)
                    .panel()
            }
        }
    }

    var body: some View {
        // Panels set into the keyboard's red body, as its black panel is (or
        // into a dark one).
        VStack(spacing: GoKeys.frame) {
            Arrangement(view: view, send: send, openDevices: openDevices)
                .panel()
            #if os(macOS)
            split(inspector: true)
            #else
            GeometryReader { proxy in
                let inspector = proxy.size.width >= Self.inspectorWidth
                Group {
                    if proxy.size.width >= Self.splitWidth {
                        split(inspector: inspector)
                    } else {
                        VStack(spacing: 0) {
                            TargetBar(view: view, searching: $searching, send: send)
                            if showingList {
                                PlaceHeader(view: view) { showingList = false }
                                ListColumn(view: view, send: send)
                            } else {
                                PlaceTree(view: view, send: send) { showingList = true }
                            }
                        }
                        .panel()
                    }
                }
                .onChange(of: inspector, initial: true) { _, inspector in editorIsSheet = !inspector }
                .sheet(isPresented: Binding(
                    get: { !inspector && view.editor != nil },
                    // Closed by the person, not by the window growing past
                    // the width where the panel sits beside the list.
                    set: { if !$0, view.editor != nil, !inspector { send(.toggleEditor) } }
                ), onDismiss: {
                    if devicesAfterEditor {
                        devicesAfterEditor = false
                        send(.showKeyboard(true))
                    }
                }) {
                    if let editor = view.editor {
                        EditorPanel(editor: editor, waves: view.waves, ticks: view.layout.ticks, send: send)
                            .presentationDetents([.medium, .large])
                            .presentationBackgroundInteraction(.enabled(upThrough: .medium))
                    }
                }
            }
            #endif
        }
        .padding(GoKeys.frame)
        // Under the Mac's hidden title bar too: the traffic lights sit on
        // the frame.
        .background(look.frame.ignoresSafeArea())
        .tint(GoKeys.accent)
        .onChange(of: view.query) { _, query in
            // Typing on the tree shows what it finds.
            if !query.isEmpty { showingList = true }
        }
        .sheet(isPresented: Binding(
            get: { view.keyboard != nil },
            set: { if !$0, view.keyboard != nil { send(.showKeyboard(false)) } }
        )) {
            DevicesSheet(view: view, send: send) { send(.showKeyboard(false)) }
        }
    }

    /// Opens the keyboard sheet; on iPhone, once the Edit sheet has closed.
    private func openDevices() {
        if editorIsSheet, view.editor != nil {
            devicesAfterEditor = true
            send(.toggleEditor)
        } else {
            send(.showKeyboard(true))
        }
    }
}

// MARK: - Keyboard button

/// Which keyboard sounds go to, with its LED; opens the keyboards sheet.
private struct KeyboardButton: View {
    let connection: ConnectionView
    let open: () -> Void

    var body: some View {
        Button(action: open) {
            HStack(spacing: GoKeys.tight) {
                LedDot(color: connection.state.led)
                Text(connection.label)
            }
        }
        .buttonStyle(FlatButtonStyle(compact: true))
        .help("Keyboards, MIDI channel, seamless switching and effects")
        .accessibilityLabel("Keyboard: \(connection.label)")
    }
}

extension DeviceState {
    /// The LED colour for a connection.
    var led: Color {
        switch self {
        case .connected: GoKeys.green
        case .connecting: GoKeys.amber
        case .disconnected: GoKeys.red
        }
    }
}

// MARK: - Tree

/// All sounds, Favourites and Recent, then each bank, which opens to show
/// its categories. Tapping a place lists it; `opened` follows.
private struct PlaceTree: View {
    let view: ViewModel
    let send: (Event) -> Void
    let opened: () -> Void

    /// Banks shown open.
    @State private var expanded: Set<Place> = []

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(view.places, id: \.place) { place in
                    TreeRow(place: place, icon: place.place.icon) { open(place) }
                }
                // As the list's category headings: small dim capitals, low
                // in a row's height, close to what they head.
                Text(caps: "Banks")
                    .goKeys(.label, color: GoKeys.dim)
                    .frame(height: GoKeys.row, alignment: .bottom)
                    .padding(.horizontal, GoKeys.page)
                    .accessibilityAddTraits(.isHeader)
                ForEach(view.banks, id: \.bank.place) { node in
                    let bank = node.bank.place
                    let isOpen = expanded.contains(bank)
                    TreeRow(place: node.bank, disclosure: isOpen) {
                        open(node.bank)
                        expanded.insert(bank)
                    } toggle: {
                        if isOpen { expanded.remove(bank) } else { expanded.insert(bank) }
                    }
                    if isOpen {
                        ForEach(node.categories, id: \.place) { category in
                            TreeRow(place: category, depth: 1) { open(category) }
                        }
                    }
                }
            }
            .padding(.vertical, GoKeys.page)
        }
        .background(GoKeys.panel)
        .onAppear { expandSelection() }
        .onChange(of: view.banks.map(\.holdsSelection)) { _, _ in expandSelection() }
    }

    private func open(_ place: PlaceView) {
        send(.open(place.place))
        opened()
    }

    /// The bank holding what the list shows stays open.
    private func expandSelection() {
        for node in view.banks where node.holdsSelection {
            expanded.insert(node.bank.place)
        }
    }
}

/// One row of the tree: a disclosure arrow for a bank, or an icon; the
/// name; the count. The place the list shows is filled lighter.
private struct TreeRow: View {
    let place: PlaceView
    var icon: String?
    /// For a bank: whether it's open.
    var disclosure: Bool?
    var depth = 0
    let action: () -> Void
    var toggle: (() -> Void)?

    /// A category's indent: its name lines up with its bank's, past the
    /// disclosure arrow.
    private static var indent: CGFloat { GoKeys.s(22) }

    var body: some View {
        HStack(spacing: GoKeys.tight) {
            if let disclosure {
                Button { toggle?() } label: {
                    Image(systemName: "chevron.right")
                        .font(.system(size: GoKeys.t(9), weight: .bold))
                        .rotationEffect(.degrees(disclosure ? 90 : 0))
                        .frame(width: GoKeys.s(16), height: GoKeys.s(24))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(GoKeys.dim)
                .accessibilityLabel(disclosure ? "Close \(place.label)" : "Open \(place.label)")
            } else if let icon {
                Image(systemName: icon)
                    .font(.system(size: GoKeys.t(11)))
                    .frame(width: GoKeys.s(16))
                    .foregroundStyle(place.selected ? GoKeys.selectedText : GoKeys.dim)
            }
            Button(action: action) {
                HStack {
                    Text(place.label)
                        .goKeys(.row, color: place.selected ? GoKeys.selectedText : GoKeys.text)
                        .lineLimit(1)
                    Spacer(minLength: GoKeys.gap)
                    Text(place.count).goKeys(.readout, color: GoKeys.dim)
                }
                .frame(height: GoKeys.row)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityAddTraits(place.selected ? .isSelected : [])
        }
        .padding(.leading, GoKeys.page + CGFloat(depth) * Self.indent)
        .padding(.trailing, PatchRow.scrollBarInset) // clear of the scroll bar
        .selectedRim(place.selected)
    }
}

extension Place {
    /// The tree's icon for the top places.
    var icon: String? {
        switch self {
        case .all: "music.note.list"
        case .favourites: "star"
        case .recent: "clock"
        case .presets: "square.stack"
        case .bank, .category: nil
        }
    }
}

/// Above the list: where it is and how many it holds; on a narrow screen,
/// a way back to the tree.
private struct PlaceHeader: View {
    let view: ViewModel
    let back: () -> Void

    var body: some View {
        HStack(spacing: GoKeys.tight) {
            Button(action: back) {
                Image(systemName: "chevron.left").font(.system(size: GoKeys.t(12), weight: .semibold))
                    .frame(width: GoKeys.s(24), height: GoKeys.s(24))
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .foregroundStyle(GoKeys.text)
            .accessibilityLabel("Places")
            Text(caps: view.placeTitle).goKeys(.label).lineLimit(1)
            Spacer(minLength: GoKeys.gap)
        }
        .padding(.horizontal, GoKeys.page)
        .frame(minHeight: GoKeys.row)
        .background(GoKeys.panel)
    }
}

/// The search field. ⌘F focuses it; Escape clears it.
private struct SearchField: View {
    let view: ViewModel
    var focused: FocusState<Bool>.Binding
    let send: (Event) -> Void

    var body: some View {
        SearchBox(
            text: Binding(get: { view.query }, set: { send(.search($0)) }),
            prompt: view.searchPrompt,
            focused: focused
        ) {
            // How many are listed: the header's count, now the list has none.
            Text(view.count).goKeys(.readout, color: GoKeys.dim).lineLimit(1).fixedSize()
        }
        .background {
            // ⌘F, without a visible button.
            Button("Search") { focused.wrappedValue = true }
                .keyboardShortcut("f", modifiers: .command)
                .hidden()
        }
    }
}

// MARK: - List

/// The list: sounds, or in Presets, the presets.
private struct ListColumn: View {
    let view: ViewModel
    let send: (Event) -> Void

    var body: some View {
        if let presets = view.presets {
            PresetList(
                presets: presets,
                empty: view.empty,
                keyboard: KeyAxis(first: view.layout.first, last: view.layout.last),
                send: send
            )
        } else {
            PatchList(sections: view.sections, empty: view.empty, current: view.current?.id, send: send)
                .equatable()
        }
    }
}

/// The presets: tap one to play it; its menu updates, renames or deletes it.
/// What's playing is filled lighter, as the current sound in the list.
private struct PresetList: View {
    let presets: [PresetView]
    let empty: String?
    let keyboard: KeyAxis
    let send: (Event) -> Void

    /// The preset being renamed, and its new name.
    @State private var renaming: UInt32?
    @State private var name = ""

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                if let empty { EmptyMessage(text: empty) }
                ForEach(presets, id: \.id) { preset in
                    row(preset)
                }
            }
        }
        .background(GoKeys.panel)
        .renamePresetAlert(renaming: $renaming, name: $name, send: send)
    }

    private func row(_ preset: PresetView) -> some View {
        Button { send(.recallPreset(preset.id)) } label: {
            // One line, a row high, as the sounds.
            HStack(spacing: GoKeys.gap) {
                MiniZones(zones: preset.zones, keyboard: keyboard)
                    .frame(width: GoKeys.s(56), height: GoKeys.row - GoKeys.s(12))
                    .clipShape(RoundedRectangle(cornerRadius: GoKeys.s(2)))
                Text(preset.name)
                    .goKeys(.row, color: preset.current ? GoKeys.selectedText : GoKeys.text)
                    .fontWeight(preset.current ? .semibold : .regular)
                    .lineLimit(1)
                if preset.edited {
                    Text(caps: "Edited").goKeys(.label, color: GoKeys.accent)
                }
                Spacer(minLength: GoKeys.gap)
                Text(preset.detail)
                    .goKeys(.detail, color: GoKeys.dim)
                    .lineLimit(1)
                    .layoutPriority(-1)
            }
            .frame(height: GoKeys.row)
            .padding(.leading, GoKeys.page)
            .padding(.trailing, PatchRow.scrollBarInset)
            .selectedRim(preset.current)
            .background(GoKeys.panel)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(preset.current ? .isSelected : [])
        .contextMenu {
            Button("Update with what's playing") { send(.updatePreset(preset.id)) }
            Button("Rename…") {
                name = preset.name
                renaming = preset.id
            }
            Button("Delete", role: .destructive) { send(.deletePreset(preset.id)) }
        }
    }
}

/// The sounds listed. Drawn again only when what it shows changes, not on
/// every redraw of the screen (a knob turning, a lane dragged).
private struct PatchList: View, Equatable {
    let sections: [SectionView]
    let empty: String?
    /// The sound picked for the part selected.
    let current: UInt32?
    let send: (Event) -> Void

    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.sections == rhs.sections && lhs.empty == rhs.empty && lhs.current == rhs.current
    }

    // A lazy stack in a scroll view, like the tree, rather than a `List`:
    // on the Mac a List is a table view, which warned of reentrant layout
    // and crashed while the window was being resized.
    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                // Each category's heading stays at the top while its sounds
                // scroll under it.
                LazyVStack(spacing: 0, pinnedViews: [.sectionHeaders]) {
                    if let empty { EmptyMessage(text: empty) }
                    // A category can come in several runs, so a title can
                    // repeat: each run by its title and first sound.
                    ForEach(sections, id: \.key) { section in
                        SwiftUI.Section {
                            ForEach(section.rows, id: \.id) { row in
                                PatchRow(row: row, current: row.id == current, send: send)
                                    .id(row.id)
                            }
                        } header: {
                            // Small dim capitals, a little apart from the
                            // sounds above: markers, the sounds carry the
                            // list. Each the same height, as the rows are.
                            if !section.title.isEmpty {
                                Text(caps: section.title)
                                    .goKeys(.label, color: GoKeys.dim)
                                    .frame(maxWidth: .infinity, alignment: .bottomLeading)
                                    .frame(height: GoKeys.row)
                                    .padding(.horizontal, GoKeys.page)
                                    .background(GoKeys.panel)
                                    .accessibilityAddTraits(.isHeader)
                            }
                        }
                    }
                }
            }
            .background(GoKeys.panel)
            .onChange(of: current) { old, id in
                guard let id else { return }
                let ease = Animation.easeOut(duration: 0.35)
                let ids = sections.flatMap(\.rows).map(\.id)
                guard let to = ids.firstIndex(of: id) else { return }
                // Every row is the same height, so one scroll lands true.
                if let from = old.flatMap({ ids.firstIndex(of: $0) }), abs(to - from) <= 8 {
                    // Nearby (a step, a click): only as far as shows it and
                    // the row past it, so it's clear of the pinned heading
                    // going up; eased.
                    let past = ids[min(max(to + (to < from ? -1 : 1), 0), ids.count - 1)]
                    withAnimation(ease) { proxy.scrollTo(past) }
                } else {
                    // Far off, or not listed before: straight to a few rows
                    // short of it, then eased the rest of the way, to the
                    // middle.
                    let from = old.flatMap { ids.firstIndex(of: $0) } ?? 0
                    let short = ids[min(max(to + (to > from ? -3 : 3), 0), ids.count - 1)]
                    proxy.scrollTo(short, anchor: .center)
                    DispatchQueue.main.async {
                        withAnimation(ease) { proxy.scrollTo(id, anchor: .center) }
                    }
                }
            }
        }
    }
}

/// One sound on one line: tap to send it, star to keep it. The current one
/// is filled lighter.
private struct PatchRow: View {
    let row: RowView
    let current: Bool
    let send: (Event) -> Void

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            Button { send(.select(row.id)) } label: {
                HStack(spacing: GoKeys.gap) {
                    Text(row.name)
                        .goKeys(.row, color: current ? GoKeys.selectedText : GoKeys.text)
                        .fontWeight(current ? .semibold : .regular)
                        .lineLimit(1)
                    Spacer(minLength: GoKeys.gap)
                    if !row.category.isEmpty {
                        Text(row.category).goKeys(.detail, color: GoKeys.dim).lineLimit(1)
                    }
                    Text(row.address).goKeys(.readout, color: GoKeys.dim)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            StarButton(on: row.favourite, color: star, target: GoKeys.s(28)) { send(.toggleFavourite(row.id)) }
        }
        .frame(height: GoKeys.row)
        // The trailing inset keeps the star clear of the scroll bar.
        .padding(.leading, GoKeys.page)
        .padding(.trailing, Self.scrollBarInset)
        // The current sound is filled lighter, as the selected place in the tree.
        .selectedRim(current)
        .background(GoKeys.panel)
    }

    /// Room on the right for an overlay scroll bar.
    static var scrollBarInset: CGFloat { GoKeys.s(16) }

    /// The star: amber when starred.
    private var star: Color {
        switch (row.favourite, current) {
        case (true, _): GoKeys.amber
        case (false, true): GoKeys.dim
        case (false, false): GoKeys.faint
        }
    }
}

// MARK: - Arrangement

/// The parts across the keys, under the preset they make and the keyboard
/// they play on.
private struct Arrangement: View {
    let view: ViewModel
    let send: (Event) -> Void
    let openDevices: () -> Void

    var body: some View {
        // The preset's row a little apart from the strips, so it reads as
        // their title, not one of them.
        VStack(alignment: .leading, spacing: GoKeys.page + GoKeys.tight) {
            HStack(spacing: GoKeys.gap) {
                ArrangementBar(view: view, send: send)
                KeyboardButton(connection: view.connection, open: openDevices)
            }
            ZoneMap(layout: view.layout, current: view.current, editorOpen: view.editor != nil, send: send)
        }
        .padding(GoKeys.page)
        .background(GoKeys.panel)
    }
}

/// The preset: its name, opening a menu to save it as a new preset or
/// rename it; Save beside it once it's changed.
private struct ArrangementBar: View {
    let view: ViewModel
    let send: (Event) -> Void

    @State private var saving = false
    @State private var renaming: UInt32?
    @State private var name = ""

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            Menu {
                Button("Save as New Preset…", action: saveNew)
                if let loaded = view.loaded {
                    Button("Rename…") {
                        name = loaded.name
                        renaming = loaded.id
                    }
                }
                if view.layout.zones.count > 1 {
                    Divider()
                    // Every part but the selected one gone; it plays on
                    // every key.
                    Button("Back to One Sound") { send(.startLayout(.one)) }
                }
            } label: {
                HStack(spacing: GoKeys.tight) {
                    Image(systemName: "square.stack").font(.system(size: GoKeys.t(12)))
                    Text(view.loaded?.name ?? "Not saved")
                        .font(GoKeysText.title.font)
                        .foregroundStyle(view.loaded == nil ? GoKeys.dim : GoKeys.text)
                        .lineLimit(1)
                    if view.loaded?.edited == true {
                        Text(caps: "Edited").goKeys(.label, color: GoKeys.accent)
                    }
                    Image(systemName: "chevron.down").font(.system(size: GoKeys.t(9), weight: .bold))
                        .foregroundStyle(GoKeys.dim)
                }
                .contentShape(Rectangle())
            }
            .menuStyle(.button)
            .buttonStyle(.plain)
            .menuIndicator(.hidden)
            .fixedSize()
            .help("The preset playing: save it as a new one, rename it, or go back to one sound")
            .accessibilityLabel("Preset: \(view.loaded?.name ?? "not saved")")
            if let loaded = view.loaded, loaded.edited {
                Button("Save") { send(.updatePreset(loaded.id)) }
                    .buttonStyle(FlatButtonStyle(compact: true))
                    .help("Update \(loaded.name) with what's playing (⌘S)")
                    .accessibilityHint("Updates \(loaded.name)")
            }
            Spacer(minLength: 0)
        }
        // The shortcuts, kept out of the menu, where they'd only work while
        // it's open: ⌘S saves the preset loaded (or a new one if none is),
        // ⌘⇧S always a new one.
        .background {
            Group {
                Button("Save") {
                    if let loaded = view.loaded {
                        if loaded.edited { send(.updatePreset(loaded.id)) }
                    } else {
                        saveNew()
                    }
                }
                .keyboardShortcut("s", modifiers: .command)
                Button("Save as New Preset", action: saveNew)
                    .keyboardShortcut("s", modifiers: [.command, .shift])
            }
            .hidden()
            .accessibilityHidden(true)
        }
        .alert("Save new preset", isPresented: $saving) {
            TextField("Name", text: $name)
            Button("Save") { send(.savePreset(name: name)) }
            Button("Cancel", role: .cancel) {}
        }
        .renamePresetAlert(renaming: $renaming, name: $name, send: send)
    }

    private func saveNew() {
        name = view.presetName
        saving = true
    }
}

extension View {
    /// The one Rename preset alert: while `renaming` holds a preset's id,
    /// asks for its new `name`.
    func renamePresetAlert(renaming: Binding<UInt32?>, name: Binding<String>, send: @escaping (Event) -> Void) -> some View {
        alert("Rename preset", isPresented: Binding(get: { renaming.wrappedValue != nil }, set: { if !$0 { renaming.wrappedValue = nil } })) {
            TextField("Name", text: name)
            Button("Rename") {
                if let id = renaming.wrappedValue { send(.renamePreset(id: id, name: name.wrappedValue)) }
            }
            Button("Cancel", role: .cancel) {}
        }
    }
}

// MARK: - Target

/// Over the list: an error, if there is one, and the search. With no
/// buttons: ↑ and ↓ (or ⌘← and ⌘→) step the selected part's sound, ⌘1 to
/// ⌘4 select a part.
private struct TargetBar: View {
    let view: ViewModel
    /// The search field has the keyboard: ⌘← and ⌘→ are its, then.
    var searching: FocusState<Bool>.Binding
    let send: (Event) -> Void

    var body: some View {
        VStack(spacing: GoKeys.page) {
            if let status = view.status {
                StatusLine(status: status.message) { send(.dismissStatus) }
            }
            bar
        }
        .padding(GoKeys.page)
        .background(GoKeys.panel)
    }

    private var bar: some View {
        SearchField(view: view, focused: searching, send: send)
            // In the field too: it has one line, so ↑ and ↓ are free.
            .onKeyPress(.upArrow) {
                send(.previous)
                return .handled
            }
            .onKeyPress(.downArrow) {
                send(.next)
                return .handled
            }
            .background {
                // ↑ and ↓, or ⌘← and ⌘→, step the selected part's sound
                // through the list; ⌘1 to ⌘4 pick a part. No buttons.
                ZStack {
                    step(.previous, key: .upArrow, modifiers: [])
                    step(.next, key: .downArrow, modifiers: [])
                    step(.previous, key: .leftArrow, modifiers: .command)
                    step(.next, key: .rightArrow, modifiers: .command)
                    // ⌘1 to ⌘4 select that part, searching or not.
                    ForEach(view.layout.zones, id: \.part) { zone in
                        Button(zone.title) { send(.editPart(zone.part)) }
                            .keyboardShortcut(KeyEquivalent(Character("\(zone.part + 1)")), modifiers: .command)
                            .opacity(0)
                            .allowsHitTesting(false)
                            .accessibilityHidden(true)
                    }
                }
            }
    }

    /// Off while searching: the keys move the field's cursor then.
    private func step(_ event: Event, key: KeyEquivalent, modifiers: EventModifiers) -> some View {
        Button(event == .next ? "Next sound" : "Previous sound") { send(event) }
            .keyboardShortcut(searching.wrappedValue ? nil : KeyboardShortcut(key, modifiers: modifiers))
            .opacity(0)
            .allowsHitTesting(false)
            .accessibilityHidden(true)
    }
}

// MARK: - Edit panel

/// The Edit panel, beside the list: the selected part's settings, or its
/// layers. It follows the part selected.
private struct EditorPanel: View {
    let editor: EditorView
    /// The wave picker, while it's showing.
    let waves: WavePickerView?
    /// The octaves marked on the keys, as under the lanes.
    let ticks: [NoteTick]
    let send: (Event) -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: GoKeys.gap) {
                ToggleGroup {
                    ForEach(editor.tabs, id: \.tab) { tab in
                        Button(tab.tab.label) { send(.showTab(tab.tab)) }
                            .buttonStyle(FlatButtonStyle(on: tab.selected))
                            .help(tab.tab == .layers ? "Layers: the sound's own four layers" : "Sound: the part's level, play, tune and tone")
                            .accessibilityAddTraits(tab.selected ? .isSelected : [])
                    }
                }
                .fixedSize()
                // What it edits, for VoiceOver: the lit lane shows it. On the tabs alone, so the controls keep their own names.
                .accessibilityElement(children: .contain)
                .accessibilityLabel(editor.title)
                Spacer(minLength: GoKeys.gap)
                // Closed with Edit, on the lit lane.
                Button("Reset") { send(.resetEditor) }
                .help("Put this tab's settings back as the sound has them")
                .buttonStyle(FlatButtonStyle())
                .disabled(!editor.edited)
                .opacity(editor.edited ? 1 : GoKeys.disabled)
            }
            .padding(GoKeys.page)
            .background(GoKeys.panel)
            ScrollView {
                Group {
                    if let layers = editor.layers {
                        LayersContent(layers: layers, ticks: ticks, send: send)
                    } else {
                        ControlGrid(groups: editor.groups, send: send)
                    }
                }
                .padding(GoKeys.page)
            }
        }
        .background(GoKeys.panel)
        .sheet(isPresented: Binding(get: { waves != nil }, set: { if !$0 { send(.searchWaves(nil)) } })) {
            if let waves { WavePicker(picker: waves, send: send) }
        }
    }
}

extension SectionView {
    /// Which run of the list it is: a category can come in several runs,
    /// so its title and first sound.
    var key: String { "\(title)·\(rows.first?.id ?? 0)" }
}
