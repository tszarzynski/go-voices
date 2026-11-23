import App
import CoreMIDI
import os
#if os(macOS)
import AppKit
#endif

let midiLog = Logger(subsystem: "com.tszarzynski.govoices", category: "midi")

/// The core's effects, over CoreMIDI and `UserDefaults`. Adapted from
/// chord-detector's `MidiShell`; it adds sending, pairing on iOS and
/// settings.
///
/// USB and Bluetooth LE keyboards are both ordinary CoreMIDI devices once
/// paired (macOS: Audio MIDI Setup; iOS: the system sheet `Pairing`
/// presents). Device ids given to the core are the CoreMIDI unique ids of
/// their sources, as strings; sounds go to the destination of the same
/// entity.
///
/// The generated `Core` dispatches effects on the main actor, and CoreMIDI's
/// callbacks hop there, so all state is main-actor isolated.
@MainActor
final class MidiShell: EffectHandler {
    private var client = MIDIClientRef()
    private var input = MIDIPortRef()
    private var output = MIDIPortRef()
    private var devicesSink: EffectSink<MidiDevices>?
    /// Open connections, each with the stream the core reads it from.
    private var connections: [MIDIUniqueID: EffectSink<Connection>] = [:]
    /// SysEx arriving in several packets, by source, until its end.
    private var incoming: [MIDIUniqueID: [UInt8]] = [:]
    private let pairing: Pairing
    private let defaults: UserDefaults

    init(pairing: Pairing, defaults: UserDefaults = .standard) {
        self.pairing = pairing
        self.defaults = defaults
        MIDIClientCreateWithBlock("GoVoices" as CFString, &client) { @Sendable [weak self] notification in
            guard notification.pointee.messageID == .msgSetupChanged else { return }
            Task { @MainActor in self?.setupChanged() }
        }
        // The core reads the keyboard's settings, which come back as SysEx;
        // that is all that's passed on (the keys and the panel aren't needed).
        // Each source is connected with its unique id as the reference.
        MIDIInputPortCreateWithProtocol(client, "Input" as CFString, ._1_0, &input) { @Sendable [weak self] list, reference in
            let source = MIDIUniqueID(truncatingIfNeeded: Int(bitPattern: reference))
            var packets: [(status: UInt32, data: [UInt8])] = []
            for packet in list.unsafeSequence() {
                let count = Int(packet.pointee.wordCount)
                let words = withUnsafeBytes(of: packet.pointee.words) { Array($0.bindMemory(to: UInt32.self).prefix(count)) }
                packets += Self.sysExPackets(words)
            }
            guard !packets.isEmpty else { return }
            DispatchQueue.main.async { MainActor.assumeIsolated { self?.received(packets, from: source) } }
        }
        MIDIOutputPortCreate(client, "Output" as CFString, &output)
    }

    // MARK: - EffectHandler: MIDI

    nonisolated func watchDevices(_ operation: WatchDevices, into sink: EffectSink<MidiDevices>) {
        MainActor.assumeIsolated {
            devicesSink = sink
            sink.send(MidiDevices(value: devices()))
        }
    }

    nonisolated func connectMidi(_ operation: ConnectMidi, into sink: EffectSink<Connection>) {
        MainActor.assumeIsolated { connect(operation.deviceId, sink) }
    }

    nonisolated func disconnectMidi(_ operation: DisconnectMidi) {
        MainActor.assumeIsolated {
            guard let uid = MIDIUniqueID(operation.deviceId) else { return }
            connections.removeValue(forKey: uid)
            if let source = source(uid) {
                MIDIPortDisconnectSource(input, source)
            }
        }
    }

    /// Pairings are kept by the system, not the app.
    nonisolated func forgetDevice(_ operation: ForgetDevice) {}

    /// Not requested: Apple platforms start the core without scanning.
    nonisolated func scanBluetooth(_ operation: ScanBluetooth, into sink: EffectSink<ScanEvent>) {
        sink.send(.failed(.unsupported))
    }

    nonisolated func stopBluetoothScan(_ operation: StopBluetoothScan) {}

    nonisolated func openBluetoothSettings(_ operation: OpenBluetoothSettings) {
        MainActor.assumeIsolated {
            #if os(macOS)
            let url = URL(fileURLWithPath: "/System/Applications/Utilities/Audio MIDI Setup.app")
            NSWorkspace.shared.openApplication(at: url, configuration: .init())
            #else
            pairing.isPresented = true
            #endif
        }
    }

    nonisolated func sendMidi(_ operation: SendMidi) {
        MainActor.assumeIsolated { send(operation.bytes, to: operation.deviceId) }
    }

    // MARK: - EffectHandler: settings

    /// The settings, as the core wrote them (JSON).
    private static let settingsKey = "settings.json"

    nonisolated func loadSettings(_ operation: LoadSettings) async -> StoredSettings {
        await MainActor.run { StoredSettings(value: defaults.string(forKey: Self.settingsKey)) }
    }

    nonisolated func saveSettings(_ operation: SaveSettings) {
        MainActor.assumeIsolated { defaults.set(operation.value, forKey: Self.settingsKey) }
    }

    // MARK: - Private

    private func devices() -> [MidiDevice] {
        sources().map { source in
            MidiDevice(
                id: String(Self.uniqueId(source)),
                name: Self.string(source, kMIDIPropertyDisplayName) ?? "MIDI device",
                transport: Self.transport(source)
            )
        }
    }

    private func connect(_ id: String, _ sink: EffectSink<Connection>) {
        guard let uid = MIDIUniqueID(id), let source = source(uid) else {
            return sink.send(.failed(.notFound))
        }
        guard destination(for: source) != nil else {
            return sink.send(.failed(.other("This device can't receive sounds")))
        }
        let status = MIDIPortConnectSource(input, source, UnsafeMutableRawPointer(bitPattern: Int(uid)))
        midiLog.debug("connect \(id, privacy: .public) status \(status)")
        guard status == noErr else {
            return sink.send(.failed(.other("Couldn't connect (\(status))")))
        }
        connections[uid] = sink
        sink.send(.connected)
    }

    /// SysEx packets from a source: whole messages go to its connection.
    private func received(_ packets: [(status: UInt32, data: [UInt8])], from source: MIDIUniqueID) {
        for (status, data) in packets {
            switch status {
            case 0: deliver(data, from: source) // complete in one packet
            case 1: incoming[source] = data // start
            case 2: incoming[source]?.append(contentsOf: data) // continue
            default: // end
                guard let start = incoming.removeValue(forKey: source) else { continue }
                deliver(start + data, from: source)
            }
        }
    }

    private func deliver(_ data: [UInt8], from source: MIDIUniqueID) {
        connections[source]?.send(.bytes([0xF0] + data + [0xF7]))
    }

    private func send(_ bytes: [UInt8], to id: String) {
        guard let uid = MIDIUniqueID(id), connections[uid] != nil,
              let source = source(uid), let destination = destination(for: source) else {
            midiLog.error("send to \(id, privacy: .public): not connected")
            return
        }
        let messages = Self.umpMessages(bytes)
        guard !messages.isEmpty else { return }
        // Room for every message: a list header, then per packet a
        // timestamp, a word count and the words.
        let size = 64 + messages.reduce(0) { $0 + 16 + 4 * $1.count }
        let buffer = UnsafeMutableRawPointer.allocate(byteCount: size, alignment: 8)
        defer { buffer.deallocate() }
        let list = buffer.bindMemory(to: MIDIEventList.self, capacity: 1)
        var packet: UnsafeMutablePointer<MIDIEventPacket>? = MIDIEventListInit(list, ._1_0)
        for var words in messages {
            guard let current = packet else { break }
            packet = MIDIEventListAdd(list, size, current, 0, words.count, &words)
        }
        guard packet != nil else {
            midiLog.error("send to \(id, privacy: .public): \(bytes.count) bytes didn't fit")
            return
        }
        let status = MIDISendEventList(output, destination, list)
        midiLog.debug("sent \(bytes.count) bytes to \(id, privacy: .public) status \(status)")
    }

    /// A device was added or removed. The new list goes first: the core ends
    /// a connection whose device has gone, rather than reconnecting it.
    private func setupChanged() {
        let devices = devices()
        devicesSink?.send(MidiDevices(value: devices))
        let present = Set(devices.compactMap { MIDIUniqueID($0.id) })
        for (uid, sink) in connections where !present.contains(uid) {
            connections.removeValue(forKey: uid)
            sink.send(.lost)
        }
    }

    private func sources() -> [MIDIEndpointRef] {
        (0..<MIDIGetNumberOfSources()).map(MIDIGetSource).filter { $0 != 0 }
    }

    private func source(_ uid: MIDIUniqueID) -> MIDIEndpointRef? {
        sources().first { Self.uniqueId($0) == uid }
    }

    /// Where to send to a device: its entity's first destination, or for a
    /// source without an entity, the destination with the same name.
    private func destination(for source: MIDIEndpointRef) -> MIDIEndpointRef? {
        var entity = MIDIEntityRef()
        if MIDIEndpointGetEntity(source, &entity) == noErr, MIDIEntityGetNumberOfDestinations(entity) > 0 {
            let destination = MIDIEntityGetDestination(entity, 0)
            return destination == 0 ? nil : destination
        }
        let name = Self.string(source, kMIDIPropertyDisplayName)
        return (0..<MIDIGetNumberOfDestinations()).map(MIDIGetDestination)
            .first { $0 != 0 && Self.string($0, kMIDIPropertyDisplayName) == name }
    }

    private static func uniqueId(_ object: MIDIObjectRef) -> MIDIUniqueID {
        var uid: MIDIUniqueID = 0
        MIDIObjectGetIntegerProperty(object, kMIDIPropertyUniqueID, &uid)
        return uid
    }

    private static func string(_ object: MIDIObjectRef, _ property: CFString) -> String? {
        var value: Unmanaged<CFString>?
        guard MIDIObjectGetStringProperty(object, property, &value) == noErr else { return nil }
        return value?.takeRetainedValue() as String?
    }

    private static func transport(_ source: MIDIEndpointRef) -> Transport {
        var entity = MIDIEntityRef()
        var device = MIDIDeviceRef()
        guard MIDIEndpointGetEntity(source, &entity) == noErr,
              MIDIEntityGetDevice(entity, &device) == noErr else {
            return .virtual // sources without an entity are created by apps
        }
        let driver = (string(device, kMIDIPropertyDriverOwner) ?? "").lowercased()
        if driver.contains("bluetooth") || driver.contains("ble") { return .bluetooth }
        if driver.contains("usb") { return .usb }
        return .other
    }

    /// MIDI 1.0 bytes (whole messages, no running status, as the core sends
    /// them) as Universal MIDI Packets, group 0, one array of words per
    /// packet: channel-voice messages as one word each, and system
    /// exclusive (`F0 … F7`) as 7-bit SysEx packets of two words carrying
    /// up to six data bytes. Anything else is dropped.
    static func umpMessages(_ bytes: [UInt8]) -> [[UInt32]] {
        var messages: [[UInt32]] = []
        var index = 0
        while index < bytes.count {
            let status = bytes[index]
            if status == 0xF0 {
                guard let end = bytes[index...].firstIndex(of: 0xF7) else { break } // cut short
                messages += sysEx(Array(bytes[(index + 1)..<end]))
                index = end + 1
                continue
            }
            guard (0x80...0xEF).contains(status) else {
                index += 1
                continue
            }
            let length = (0xC0...0xDF).contains(status) ? 1 : 2
            guard index + length < bytes.count else { break } // cut short
            let data1 = UInt32(bytes[index + 1] & 0x7F)
            let data2 = length == 2 ? UInt32(bytes[index + 2] & 0x7F) : 0
            messages.append([0x2000_0000 | UInt32(status) << 16 | data1 << 8 | data2])
            index += 1 + length
        }
        return messages
    }

    /// The 7-bit SysEx packets in UMP words (two words each): each one's
    /// status (0 complete, 1 start, 2 continue, 3 end) and data bytes.
    /// Anything else is skipped.
    nonisolated static func sysExPackets(_ words: [UInt32]) -> [(status: UInt32, data: [UInt8])] {
        var packets: [(status: UInt32, data: [UInt8])] = []
        var index = 0
        while index < words.count {
            let first = words[index]
            // A message's length in words, by its type.
            let length = switch first >> 28 {
            case 0x3, 0x4: 2
            case 0x5: 4
            default: 1
            }
            if first >> 28 == 0x3, index + 1 < words.count {
                let second = words[index + 1]
                let count = Int(min(first >> 16 & 0xF, 6))
                let bytes = [first >> 8, first, second >> 24, second >> 16, second >> 8, second].map { UInt8($0 & 0x7F) }
                packets.append((first >> 20 & 0xF, Array(bytes.prefix(count))))
            }
            index += length
        }
        return packets
    }

    /// A SysEx message's data (without `F0` and `F7`) as UMP 7-bit SysEx
    /// packets: complete, or start, continue … end.
    static func sysEx(_ data: [UInt8]) -> [[UInt32]] {
        let chunks = stride(from: 0, to: max(data.count, 1), by: 6).map {
            Array(data[$0..<min($0 + 6, data.count)])
        }
        return chunks.enumerated().map { number, chunk in
            let status: UInt32 = switch (number == 0, number == chunks.count - 1) {
            case (true, true): 0 // complete in one packet
            case (true, false): 1 // start
            case (false, false): 2 // continue
            case (false, true): 3 // end
            }
            var payload = [UInt8](repeating: 0, count: 6)
            for (offset, byte) in chunk.enumerated() { payload[offset] = byte & 0x7F }
            let first = 0x3000_0000 | status << 20 | UInt32(chunk.count) << 16
                | UInt32(payload[0]) << 8 | UInt32(payload[1])
            let second = UInt32(payload[2]) << 24 | UInt32(payload[3]) << 16
                | UInt32(payload[4]) << 8 | UInt32(payload[5])
            return [first, second]
        }
    }
}
