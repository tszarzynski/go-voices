import App
import Foundation

/// The shell's words for what the core sends as values: errors, states,
/// hints and the display's lines. The core works in values; the chrome is
/// worded here.
enum Wording {
    #if os(macOS)
    /// "Click" on the Mac, "Tap" elsewhere.
    static let tap = "Click"
    #else
    static let tap = "Tap"
    #endif

    /// `Channel 4`.
    static func channel(_ channel: UInt8) -> String { "Channel \(channel)" }

    static let seamless = "Seamless switching"
    static let seamlessHint = "Held notes keep their sound when you pick a new one. Uses two more of the keyboard's parts; untested with a sustain pedal."
    static let channelHint = "The keyboard listens on channel 1 unless its settings were changed."

    /// What to say while no keyboard is listed.
    static var devicesHint: String {
        #if os(macOS)
        "Switch on your keyboard, or connect it with a USB cable."
        #else
        "Pair your keyboard below, or connect it with a USB cable."
        #endif
    }

    /// The Bluetooth section's hint and button: the system pairs, on Apple
    /// platforms.
    static var bluetoothHint: String {
        #if os(macOS)
        "Pair in Audio MIDI Setup: Window › Show MIDI Studio › Configure Bluetooth."
        #else
        "Pair a Bluetooth MIDI keyboard once; after that it connects by itself."
        #endif
    }

    static var bluetoothAction: String {
        #if os(macOS)
        "Open MIDI Setup"
        #else
        "Pair keyboard…"
        #endif
    }
}

extension MidiError {
    /// What to tell the person.
    var message: String {
        switch self {
        case .notFound: "Device not found"
        case .notResponding: "Not responding"
        case .notMidi: "No MIDI from this device"
        case .disconnected: "Disconnected while connecting"
        case .pairingFailed: "Pairing failed"
        case .bluetoothOff: "Turn on Bluetooth"
        case .permissionDenied: "Allow Bluetooth access"
        case .unsupported: "Bluetooth isn't available"
        case .busy: "Disconnect the other device first"
        case let .other(message): message
        }
    }
}

extension LayersStatus {
    var message: String {
        switch self {
        case .drums: "Drum kits have no layers to edit here"
        case .notConnected: "Connect your keyboard to edit this sound's layers"
        case .reading: "Reading the sound from the keyboard…"
        }
    }
}

extension Tab {
    var label: String {
        switch self {
        case .sound: "Sound"
        case .layers: "Layers"
        }
    }
}

extension DeviceView {
    /// How it's attached, or where its connection is.
    var detail: String {
        switch state {
        case .connected: "Connected"
        case .connecting: "Connecting…"
        case .disconnected:
            switch transport {
            case .usb: "USB"
            case .bluetooth: "Bluetooth"
            case .virtual: "Virtual"
            case .other: "MIDI"
            }
        }
    }

    /// What toggling it does.
    var action: String {
        switch state {
        case .connected: "Disconnect"
        case .connecting: "Cancel"
        case .disconnected: "Connect"
        }
    }
}

extension PasteView {
    /// `Layer 2 of JUNO Piano 1`.
    var label: String { "Layer \(layer + 1) of \(sound ?? "this sound")" }
}

extension DisplayView {
    /// `2 · Acoustic piano`, `Acoustic piano`, or the app's name.
    var heading: String {
        switch (sound, several) {
        case (nil, false): "Go:Voices"
        case (nil, true): "\(part)"
        case let (sound?, false): sound.category
        case let (sound?, true): "\(part) · \(sound.category)"
        }
    }

    /// The sound's name; with nothing picked, how many are listed.
    func title(count: String) -> String {
        switch (sound, several) {
        case let (sound?, _): sound.name
        case (nil, false): count
        case (nil, true): "—"
        }
    }

    /// The part's keys when there are several; otherwise what happens to a
    /// pick.
    func footnote(connected: Bool, channel: UInt8) -> String {
        switch (sound != nil, several, connected) {
        case (_, true, _): keys
        case (false, false, _): "\(Wording.tap) a sound to play it on your keyboard"
        case (true, false, true): "Sent on channel \(channel)"
        case (true, false, false): "Sent when a keyboard connects"
        }
    }
}
