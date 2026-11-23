import App
import SwiftUI

@main
struct GoVoicesApp: App {
    @State private var core: Core
    @State private var pairing: Pairing

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
        WindowGroup("GO Voices") {
            BrowserScreen(view: core.view, send: core.update)
                .preferredColorScheme(.dark)
                #if os(iOS)
                .sheet(isPresented: $pairing.isPresented) {
                    PairingSheet { pairing.isPresented = false }
                        .ignoresSafeArea()
                }
                #else
                .frame(minWidth: 640, minHeight: 480)
                #endif
        }
        #if os(macOS)
        .defaultSize(width: 980, height: 780)
        #endif
    }
}
