import App
import SwiftUI

/// The one screen: a red title bar, the tree of places beside the list of
/// sounds (on a narrow screen, the tree first and the list after it), and
/// the display with − and + at the bottom.
/// Everything it shows comes from the core's view model; taps become events.
struct BrowserScreen: View {
    let view: ViewModel
    let send: (Event) -> Void

    @State private var showingDevices = false
    /// Narrow screens: the list is showing, rather than the tree.
    @State private var showingList = false

    /// Below this width (iPhone, narrow iPad), the tree and the list take
    /// turns.
    private static let splitWidth: CGFloat = 620

    /// The tree beside the list. The Mac always uses it (its window is kept
    /// wide enough): swapping layouts while resizing tears the list's table
    /// view down mid-layout, which AppKit flags as reentrant and can crash.
    private var split: some View {
        HStack(spacing: 0) {
            PlaceTree(view: view, send: send) {}
                .frame(width: GoKeys.s(240))
            GoKeys.line.frame(width: 1)
            withEditor {
                VStack(spacing: 0) {
                    SearchField(view: view, send: send)
                    PlaceHeader(view: view, back: nil)
                    ListColumn(view: view, send: send)
                }
            }
        }
    }

    /// The Edit panel, while open, over `content`: the list stays underneath
    /// so it keeps its scroll position.
    private func withEditor(@ViewBuilder _ content: () -> some View) -> some View {
        ZStack {
            content().accessibilityHidden(view.editor != nil)
            if let editor = view.editor {
                EditorPanel(editor: editor, waves: view.waves, send: send)
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            TitleBar(connection: view.connection) { showingDevices = true }
            #if os(macOS)
            split
            #else
            GeometryReader { proxy in
                if proxy.size.width >= Self.splitWidth {
                    split
                } else {
                    withEditor {
                        VStack(spacing: 0) {
                            SearchField(view: view, send: send)
                            if showingList {
                                PlaceHeader(view: view) { showingList = false }
                                ListColumn(view: view, send: send)
                            } else {
                                PlaceTree(view: view, send: send) { showingList = true }
                            }
                        }
                    }
                }
            }
            #endif
            GoKeys.line.frame(height: 1)
            NowPlaying(view: view, send: send)
        }
        .background(GoKeys.panel)
        .tint(GoKeys.accent)
        .onChange(of: view.query) { _, query in
            // Typing on the tree shows what it finds.
            if !query.isEmpty { showingList = true }
        }
        .sheet(isPresented: $showingDevices) {
            DevicesSheet(view: view, send: send) { showingDevices = false }
        }
    }
}

// MARK: - Title bar

/// A bar along the top holding the keyboard button on the right, padded
/// like the footer.
private struct TitleBar: View {
    let connection: ConnectionView
    let openDevices: () -> Void

    var body: some View {
        HStack {
            Spacer(minLength: 0)
            KeyboardButton(connection: connection, open: openDevices)
        }
        .padding(GoKeys.page)
        .background(GoKeys.panel)
        .overlay(alignment: .bottom) { GoKeys.line.frame(height: 1) }
    }
}

/// Which keyboard sounds go to, with its LED; opens the keyboards sheet.
private struct KeyboardButton: View {
    let connection: ConnectionView
    let open: () -> Void

    var body: some View {
        Button(action: open) {
            HStack(spacing: GoKeys.s(6)) {
                LedDot(color: connection.state.led)
                Text(connection.label)
            }
        }
        .buttonStyle(FlatButtonStyle(compact: true))
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
                Text(caps: "Banks")
                    .goKeys(.label, color: GoKeys.dim)
                    .padding(.horizontal, GoKeys.page)
                    .padding(.top, GoKeys.s(14))
                    .padding(.bottom, GoKeys.s(4))
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
            .padding(.vertical, GoKeys.s(6))
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
/// name; the count. The place the list shows is filled red.
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
    private static let indent: CGFloat = GoKeys.s(22)

    var body: some View {
        HStack(spacing: GoKeys.s(6)) {
            if let disclosure {
                Button { toggle?() } label: {
                    Image(systemName: "chevron.right")
                        .font(.system(size: GoKeys.s(9), weight: .bold))
                        .rotationEffect(.degrees(disclosure ? 90 : 0))
                        .frame(width: GoKeys.s(16), height: GoKeys.s(24))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(place.selected ? .white : GoKeys.dim)
                .accessibilityLabel(disclosure ? "Close \(place.label)" : "Open \(place.label)")
            } else if let icon {
                Image(systemName: icon)
                    .font(.system(size: GoKeys.s(11)))
                    .frame(width: GoKeys.s(16))
                    .foregroundStyle(place.selected ? .white : GoKeys.dim)
            }
            Button(action: action) {
                HStack {
                    Text(place.label)
                        .goKeys(.row, color: place.selected ? .white : GoKeys.text)
                        .lineLimit(1)
                    Spacer(minLength: GoKeys.gap)
                    Text(place.count).goKeys(.detail, color: place.selected ? .white.opacity(0.8) : GoKeys.dim)
                }
                .frame(minHeight: GoKeys.s(30))
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityAddTraits(place.selected ? .isSelected : [])
        }
        .padding(.leading, GoKeys.page + CGFloat(depth) * Self.indent)
        .padding(.trailing, PatchRow.scrollBarInset) // clear of the scroll bar
        .background(place.selected ? GoKeys.accent : .clear)
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
    let back: (() -> Void)?

    var body: some View {
        HStack(spacing: GoKeys.s(6)) {
            if let back {
                Button(action: back) {
                    Image(systemName: "chevron.left").font(.system(size: GoKeys.s(12), weight: .semibold))
                        .frame(width: GoKeys.s(24), height: GoKeys.s(24))
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .foregroundStyle(GoKeys.text)
                .accessibilityLabel("Places")
            }
            Text(caps: view.placeTitle).goKeys(.label).lineLimit(1)
            Spacer(minLength: GoKeys.gap)
            Text(view.count).goKeys(.detail, color: GoKeys.dim).lineLimit(1)
        }
        .padding(.horizontal, GoKeys.page)
        .frame(minHeight: GoKeys.s(30))
        .background(GoKeys.panel)
    }
}

/// The search field. ⌘F focuses it; Escape clears it.
private struct SearchField: View {
    let view: ViewModel
    let send: (Event) -> Void

    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: GoKeys.s(6)) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: GoKeys.s(12)))
                .foregroundStyle(GoKeys.dim)
            TextField(
                "",
                text: Binding(get: { view.query }, set: { send(.search($0)) }),
                prompt: Text(view.searchPrompt).foregroundStyle(GoKeys.dim)
            )
            .textFieldStyle(.plain)
            .font(.inter(14))
            .foregroundStyle(GoKeys.text)
            .autocorrectionDisabled()
            #if os(iOS)
            .textInputAutocapitalization(.never)
            .submitLabel(.search)
            #endif
            .focused($focused)
            .onKeyPress(.escape) {
                guard !view.query.isEmpty else { return .ignored }
                send(.search(""))
                return .handled
            }
            if !view.query.isEmpty {
                Button { send(.search("")) } label: {
                    Image(systemName: "xmark.circle.fill").foregroundStyle(GoKeys.dim)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Clear search")
            }
        }
        .padding(.horizontal, GoKeys.s(8))
        .frame(height: GoKeys.s(30))
        .background(GoKeys.background)
        .overlay {
            Rectangle().strokeBorder(focused ? GoKeys.accent : GoKeys.line, lineWidth: 1)
        }
        .background {
            // ⌘F, without a visible button.
            Button("Search") { focused = true }
                .keyboardShortcut("f", modifiers: .command)
                .hidden()
        }
        .padding(GoKeys.page)
        .background(GoKeys.panel)
    }
}

// MARK: - List

/// The list: sounds, or in Presets, the presets.
private struct ListColumn: View {
    let view: ViewModel
    let send: (Event) -> Void

    var body: some View {
        if let presets = view.presets {
            PresetList(presets: presets, empty: view.empty, send: send)
        } else {
            PatchList(view: view, send: send)
        }
    }
}

/// The presets: tap one to play it; its menu updates, renames or deletes it.
/// What's playing is filled red, as the current sound in the list.
private struct PresetList: View {
    let presets: [PresetView]
    let empty: String?
    let send: (Event) -> Void

    @State private var renaming: PresetView?
    @State private var name = ""

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                if let empty {
                    Text(empty)
                        .goKeys(.row, color: GoKeys.dim)
                        .frame(maxWidth: .infinity, alignment: .center)
                        .padding(.vertical, GoKeys.s(40))
                }
                ForEach(presets, id: \.id) { preset in
                    row(preset)
                }
            }
        }
        .background(GoKeys.background)
        .alert("Rename preset", isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            TextField("Name", text: $name)
            Button("Rename") {
                if let renaming { send(.renamePreset(id: renaming.id, name: name)) }
            }
            Button("Cancel", role: .cancel) {}
        }
    }

    private func row(_ preset: PresetView) -> some View {
        Button { send(.recallPreset(preset.id)) } label: {
            HStack(spacing: GoKeys.gap) {
                Text(preset.name)
                    .goKeys(.row, color: preset.current ? .white : GoKeys.text)
                    .lineLimit(1)
                Spacer(minLength: GoKeys.gap)
                if preset.edited {
                    Text(caps: "Edited").goKeys(.label, color: .white)
                }
                Text(preset.detail)
                    .goKeys(.detail, color: preset.current ? .white.opacity(0.8) : GoKeys.dim)
                    .lineLimit(1)
            }
            .frame(minHeight: GoKeys.s(36))
            .padding(.leading, GoKeys.page)
            .padding(.trailing, PatchRow.scrollBarInset)
            .background(preset.current ? GoKeys.accent : GoKeys.background)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(preset.current ? .isSelected : [])
        .contextMenu {
            Button("Update with what's playing") { send(.updatePreset(preset.id)) }
            Button("Rename…") {
                name = preset.name
                renaming = preset
            }
            Button("Delete", role: .destructive) { send(.deletePreset(preset.id)) }
        }
    }
}

private struct PatchList: View {
    let view: ViewModel
    let send: (Event) -> Void

    // A lazy stack in a scroll view, like the tree, rather than a `List`:
    // on the Mac a List is a table view, which warned of reentrant layout
    // and crashed while the window was being resized.
    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 0) {
                    if let empty = view.empty {
                        Text(empty)
                            .goKeys(.row, color: GoKeys.dim)
                            .frame(maxWidth: .infinity, alignment: .center)
                            .padding(.vertical, GoKeys.s(40))
                    }
                    // A category can come in several runs, so titles repeat: by position.
                    ForEach(Array(view.sections.enumerated()), id: \.offset) { _, section in
                        if !section.title.isEmpty {
                            Text(caps: section.title)
                                .goKeys(.label, color: GoKeys.dim)
                                .frame(maxWidth: .infinity, minHeight: GoKeys.s(26), alignment: .leading)
                                .padding(.horizontal, GoKeys.page)
                                .background(GoKeys.panel)
                                .accessibilityAddTraits(.isHeader)
                        }
                        ForEach(section.rows, id: \.id) { row in
                            PatchRow(row: row, send: send)
                                .id(row.id)
                        }
                    }
                }
            }
            .background(GoKeys.background)
            .onChange(of: view.current?.id) { _, id in
                guard let id else { return }
                withAnimation { proxy.scrollTo(id) }
            }
        }
    }
}

/// One sound on one line: tap to send it, star to keep it. The current one
/// is filled red.
private struct PatchRow: View {
    let row: RowView
    let send: (Event) -> Void

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            Button { send(.select(row.id)) } label: {
                HStack(spacing: GoKeys.gap) {
                    Text(row.name)
                        .goKeys(.row, color: row.current ? .white : GoKeys.text)
                        .lineLimit(1)
                    Spacer(minLength: GoKeys.gap)
                    if !row.category.isEmpty {
                        Text(row.category).goKeys(.detail, color: secondary).lineLimit(1)
                    }
                    Text(row.address).goKeys(.detail, color: secondary)
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            StarButton(on: row.favourite, color: star, target: GoKeys.s(28)) { send(.toggleFavourite(row.id)) }
        }
        .frame(minHeight: GoKeys.s(36))
        // The trailing inset keeps the star clear of the scroll bar.
        .padding(.leading, GoKeys.page)
        .padding(.trailing, Self.scrollBarInset)
        // The current sound is filled red, as the selected place in the tree.
        .background(row.current ? GoKeys.accent : GoKeys.background)
    }

    /// Room on the right for an overlay scroll bar.
    static let scrollBarInset: CGFloat = GoKeys.s(16)

    /// Small print: dim, or light on the red of the current row.
    private var secondary: Color { row.current ? .white.opacity(0.8) : GoKeys.dim }

    /// The star: red when starred, white when starred on the red row.
    private var star: Color {
        switch (row.favourite, row.current) {
        case (true, true): .white
        case (true, false): GoKeys.accent
        case (false, true): .white.opacity(0.7)
        case (false, false): GoKeys.dim.opacity(0.6)
        }
    }
}

// MARK: - Now playing

/// The display, as the keyboard shows the sound, with the round − and +
/// buttons beside it, as on the keyboard's panel.
private struct NowPlaying: View {
    let view: ViewModel
    let send: (Event) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: GoKeys.gap) {
            ZoneMap(layout: view.layout, send: send)
                .padding(.bottom, GoKeys.s(4))
            HStack(spacing: GoKeys.gap) {
                // One per part, lowest keys first: tap one to edit it.
                LazyVGrid(
                    columns: Array(repeating: GridItem(.flexible(), spacing: GoKeys.gap), count: columns),
                    spacing: GoKeys.gap
                ) {
                    ForEach(view.displays, id: \.part) { display in
                        let screen = Display(dimmed: display.dimmed) { DisplayText(display: display, send: send) }
                        if display.several {
                            screen
                                .contentShape(Rectangle())
                                .onTapGesture { send(.editPart(display.part)) }
                                .contextMenu {
                                    if let remove = display.remove {
                                        Button(remove, role: .destructive) { send(.removePart(display.part)) }
                                    }
                                }
                                .accessibilityElement(children: .combine)
                                .accessibilityAddTraits(display.dimmed ? .isButton : [.isButton, .isSelected])
                        } else {
                            screen
                        }
                    }
                }
                HStack(spacing: GoKeys.s(10)) {
                    stepButton("minus", label: "Previous", event: .previous, key: .leftArrow)
                    stepButton("plus", label: "Next", event: .next, key: .rightArrow)
                }
            }
            LayoutControls(
                layout: view.layout,
                editorOpen: view.editor != nil,
                loaded: view.loaded,
                presetName: view.presetName,
                send: send
            )
        }
        .padding(GoKeys.page)
        .background(GoKeys.panel.ignoresSafeArea(edges: .bottom))
    }

    #if os(iOS)
    @Environment(\.horizontalSizeClass) private var sizeClass
    #endif

    /// The displays side by side; two to a row on a narrow screen.
    private var columns: Int {
        let count = max(view.displays.count, 1)
        #if os(iOS)
        if sizeClass == .compact { return min(count, 2) }
        #endif
        return count
    }

    private func stepButton(_ symbol: String, label: String, event: Event, key: KeyEquivalent) -> some View {
        Button { send(event) } label: {
            Image(systemName: symbol)
                .font(.system(size: GoKeys.s(17), weight: .bold))
        }
        .buttonStyle(RoundButtonStyle())
        .keyboardShortcut(key, modifiers: .command)
        .accessibilityLabel(label)
    }
}

/// The Edit panel, in place of the sound list: the settings of the part
/// being edited, then the whole keyboard's. The groups flow into as many
/// columns as fit.
private struct EditorPanel: View {
    let editor: EditorView
    /// The wave picker, while it's showing.
    let waves: WavePickerView?
    let send: (Event) -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: GoKeys.s(28)) {
                HStack(spacing: GoKeys.gap) {
                    ToggleGroup {
                        ForEach(editor.tabs, id: \.tab) { tab in
                            Button(tab.label) { send(.showTab(tab.tab)) }
                                .buttonStyle(FlatButtonStyle(on: tab.selected))
                                .accessibilityAddTraits(tab.selected ? .isSelected : [])
                        }
                    }
                    .fixedSize()
                    Text(caps: editor.title).goKeys(.label).lineLimit(1)
                    Spacer(minLength: GoKeys.gap)
                    Button("Reset") { send(.resetEditor) }
                        .buttonStyle(FlatButtonStyle())
                        .disabled(!editor.edited)
                        .opacity(editor.edited ? 1 : 0.4)
                }
                if let layers = editor.layers {
                    LayersContent(layers: layers, send: send)
                } else {
                    columns(editor.groups)
                    Text(caps: editor.keyboardTitle).goKeys(.label)
                        .lineLimit(1)
                        .padding(.top, GoKeys.s(16))
                    columns(editor.keyboard)
                }
            }
            .padding(GoKeys.page)
            .padding(.trailing, PatchRow.scrollBarInset - GoKeys.page)
        }
        .background(GoKeys.background)
        .sheet(isPresented: Binding(get: { waves != nil }, set: { if !$0 { send(.searchWaves(nil)) } })) {
            if let waves { WavePicker(picker: waves, send: send) }
        }
    }

    private func columns(_ groups: [GroupView]) -> some View {
        ControlGrid(groups: groups, send: send)
    }
}

/// Adding a part, starting from a layout, saving presets, and the Edit
/// panel.
private struct LayoutControls: View {
    let layout: LayoutView
    let editorOpen: Bool
    /// The preset Save updates, if one is loaded.
    let loaded: LoadedView?
    /// A name to start from when saving a new preset.
    let presetName: String
    let send: (Event) -> Void

    @State private var saving = false
    @State private var name = ""

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            Button("+ Part") { send(.addPart) }
                .buttonStyle(FlatButtonStyle())
                .disabled(!layout.canAdd)
                .opacity(layout.canAdd ? 1 : 0.4)
                .accessibilityHint("Splits the part being edited in two")
            FlatMenu(title: "Start from", label: "Start from") {
                ForEach(layout.layouts, id: \.label) { option in
                    Button(option.label) { send(.startLayout(option.layout)) }
                }
            }
            Spacer(minLength: 0)
            if let loaded {
                Button("Save") { send(.updatePreset(loaded.id)) }
                    .buttonStyle(FlatButtonStyle())
                    .keyboardShortcut("s", modifiers: .command)
                    .disabled(!loaded.edited)
                    .opacity(loaded.edited ? 1 : 0.4)
                    .accessibilityHint(loaded.name)
            }
            Button("Save new") {
                name = presetName
                saving = true
            }
            .buttonStyle(FlatButtonStyle())
            .keyboardShortcut("s", modifiers: loaded == nil ? .command : [.command, .shift])
            .alert("Save new preset", isPresented: $saving) {
                TextField("Name", text: $name)
                Button("Save") { send(.savePreset(name: name)) }
                Button("Cancel", role: .cancel) {}
            }
            Button("Edit") { send(.toggleEditor) }
                .buttonStyle(FlatButtonStyle(on: editorOpen))
                .keyboardShortcut("e", modifiers: .command)
                .accessibilityAddTraits(editorOpen ? .isSelected : [])
        }
    }
}

/// One display: `[HEADING]` and the address, the title and the star, and
/// the footnote or an error.
private struct DisplayText: View {
    let display: DisplayView
    let send: (Event) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: GoKeys.s(6)) {
                Text("[\(display.heading.uppercased())]")
                    .goKeys(.displaySmall, color: GoKeys.displayText)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                Spacer(minLength: GoKeys.s(4))
                if let sound = display.sound {
                    // The bank goes first when space is short.
                    ViewThatFits(in: .horizontal) {
                        Text("\(sound.bank) \(sound.address)").goKeys(.displaySmall, color: GoKeys.displayText)
                        Text(sound.address).goKeys(.displaySmall, color: GoKeys.displayText)
                    }
                    .lineLimit(1)
                }
            }
            HStack(spacing: GoKeys.s(6)) {
                Text(display.title)
                    .goKeys(.displayLarge, color: GoKeys.displayText)
                    .lineLimit(1)
                    .minimumScaleFactor(0.5)
                Spacer(minLength: 0)
                if let sound = display.sound {
                    StarButton(on: sound.favourite) { send(.toggleFavourite(sound.id)) }
                }
            }
            if let status = display.status {
                StatusLine(status: status) { send(.dismissStatus) }
            } else {
                Text(display.footnote)
                    .goKeys(.displaySmall, color: GoKeys.displayText.opacity(0.6))
                    .lineLimit(1)
                    .minimumScaleFactor(0.6)
            }
        }
    }
}
