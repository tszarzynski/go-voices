import App
import SwiftUI

@main
struct GoVoicesApp: App {
    @State private var core: Core
    @State private var pairing: Pairing
    /// The Mac's Text Size; every window is drawn again when it changes.
    @AppStorage(TextSize.key) private var textSize = TextSize.comfortable

    init() {
        let pairing = Pairing()
        let core = Core(handler: MidiShell(pairing: pairing))
        #if os(macOS)
        core.update(.start(platform: .mac))
        #else
        core.update(.start(platform: .ios))
        #endif
        _core = State(initialValue: core)
        _pairing = State(initialValue: pairing)
        GoKeys.registerFonts()
    }

    var body: some Scene {
        WindowGroup("Go:Voices") {
            BrowserScreen(view: core.view, send: core.update)
                .preferredColorScheme(.dark)
                #if os(iOS)
                .sheet(isPresented: $pairing.isPresented) {
                    PairingSheet { pairing.isPresented = false }
                        .ignoresSafeArea()
                }
                #else
                .frame(minWidth: 860, minHeight: 600)
                .id(textSize)
                #endif
        }
        #if os(macOS)
        .defaultSize(width: 1320, height: 880)
        .windowStyle(.hiddenTitleBar)
        .commands {
            CommandGroup(after: .toolbar) {
                Picker("Text Size", selection: Binding(get: { textSize }, set: resize)) {
                    ForEach(TextSize.allCases) { Text($0.label).tag($0) }
                }
                Button("Smaller") { resize(.compact) }
                    .keyboardShortcut("-", modifiers: .command)
                    .disabled(textSize == .compact)
                Button("Bigger") { resize(.comfortable) }
                    .keyboardShortcut("+", modifiers: .command)
                    .disabled(textSize == .comfortable)
                Divider()
            }
            CommandGroup(replacing: .appInfo) { OpenWindowButton(title: "About Go:Voices", id: "about") }
            CommandGroup(replacing: .help) {
                OpenWindowButton(title: "Go:Voices Help", id: "help")
                    .keyboardShortcut("?", modifiers: .command)
            }
        }
        #endif

        #if os(macOS)
        Window("About Go:Voices", id: "about") {
            AboutView()
                .id(textSize)
                .frame(width: 440, height: 600)
                .preferredColorScheme(.dark)
        }
        .windowResizability(.contentSize)
        .windowStyle(.hiddenTitleBar)

        Window("Go:Voices Help", id: "help") {
            HelpView()
                .id(textSize)
                .frame(minWidth: 480, minHeight: 480)
                .preferredColorScheme(.dark)
        }
        .defaultSize(width: 608, height: 760)
        .windowStyle(.hiddenTitleBar)
        #endif
    }

    /// Sizes are read as the views are drawn: the new one is set before
    /// they're drawn again.
    private func resize(_ size: TextSize) {
        GoKeys.size = size
        textSize = size
    }
}

#if os(macOS)
/// A menu item opening one of the app's other windows.
private struct OpenWindowButton: View {
    let title: String
    let id: String
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Button(title) { openWindow(id: id) }
    }
}
#endif
