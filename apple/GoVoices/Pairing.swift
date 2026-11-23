import Observation
import SwiftUI
#if os(iOS)
import CoreAudioKit
#endif

/// Whether the system's Bluetooth MIDI pairing sheet is up (iOS). The core
/// asks for it with `OpenBluetoothSettings`; the shell sets this.
@MainActor
@Observable
final class Pairing {
    var isPresented = false
}

#if os(iOS)
/// Apple's Bluetooth LE MIDI pairing screen (`CABTMIDICentralViewController`)
/// in a navigation controller with a Done button. A keyboard paired here
/// becomes an ordinary CoreMIDI device, which the core then connects.
struct PairingSheet: UIViewControllerRepresentable {
    let done: () -> Void

    func makeUIViewController(context: Context) -> UINavigationController {
        let central = CABTMIDICentralViewController()
        central.navigationItem.rightBarButtonItem = UIBarButtonItem(
            systemItem: .done,
            primaryAction: UIAction { _ in done() }
        )
        return UINavigationController(rootViewController: central)
    }

    func updateUIViewController(_ controller: UINavigationController, context: Context) {}
}
#endif
