import App
import SwiftUI

/// The keyboard: in cards, as Help is. The keyboards to connect and
/// pairing a Bluetooth one; what's played (the channel, seamless
/// switching); the effects every part shares.
struct DevicesSheet: View {
    let view: ViewModel
    let send: (Event) -> Void
    let done: () -> Void

    #if os(iOS)
    private enum Page: Identifiable {
        case help, about
        var id: Self { self }
    }

    /// Help or About, over the sheet.
    @State private var showing: Page?
    #endif

    var body: some View {
        VStack(spacing: 0) {
            SheetHeader(title: "Keyboard", done: done)
            ScrollView {
                VStack(alignment: .leading, spacing: GoKeys.section) {
                    if let status = view.status {
                        StatusLine(status: status.message) { send(.dismissStatus) }
                    }
                    keyboards
                    playing
                    ControlGrid(groups: view.keyboard ?? [], icon: "dial.medium", send: send)
                    #if os(iOS)
                    // The Mac has these in its menus.
                    Card(title: "Go:Voices", icon: "info.circle") {
                        CardRow(title: "How it works") {
                            Button("Help") { showing = .help }.buttonStyle(FlatButtonStyle())
                        }
                        Hairline()
                        CardRow(title: "Version and credits") {
                            Button("About") { showing = .about }.buttonStyle(FlatButtonStyle())
                        }
                    }
                    #endif
                }
                .padding([.horizontal, .bottom], GoKeys.page)
            }
        }
        .background(GoKeys.panel)
        .tint(GoKeys.accent)
        #if os(macOS)
        .frame(minWidth: 480, idealWidth: 520, minHeight: 480)
        #else
        .sheet(item: $showing) { page in
            switch page {
            case .help: HelpView()
            case .about: AboutView()
            }
        }
        #endif
    }

    /// The keyboards listed, then pairing a Bluetooth one (and what a scan
    /// found).
    private var keyboards: some View {
        Card(title: "Keyboards", icon: "pianokeys") {
            if view.devices.isEmpty {
                Text(Wording.devicesHint)
                    .goKeys(.row, color: GoKeys.dim)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(GoKeys.page)
                Hairline()
            }
            ForEach(view.devices, id: \.id) { device in
                DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                Hairline()
            }
            ForEach(view.bluetooth.found, id: \.id) { device in
                DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                Hairline()
            }
            CardRow(title: "Bluetooth", hint: Wording.bluetoothHint) {
                Button(Wording.bluetoothAction) {
                    send(view.bluetooth.scanning ? .stopBluetoothScan : .startBluetoothScan)
                }
                .buttonStyle(FlatButtonStyle(on: view.bluetooth.scanning))
            }
        }
    }

    /// How sounds are played: the channel, and seamless switching.
    private var playing: some View {
        Card(title: "Playing", icon: "music.note") {
            CardRow(title: "MIDI channel", hint: Wording.channelHint) {
                ChannelMenu(view: view, send: send).fixedSize()
            }
            Hairline()
            CardRow(title: Wording.seamless, hint: Wording.seamlessHint) {
                Button(view.seamless ? "On" : "Off") { send(.setSeamless(!view.seamless)) }
                    .buttonStyle(FlatButtonStyle(on: view.seamless))
                    .help("\(Wording.seamless): \(Wording.tap.lowercased()) to switch")
                    .accessibilityLabel(Wording.seamless)
                    .accessibilityValue(view.seamless ? "On" : "Off")
            }
        }
    }
}

/// One keyboard: its LED, name and state, and what tapping does.
private struct DeviceRow: View {
    let device: DeviceView
    let toggle: () -> Void

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            LedDot(color: device.state.led)
            VStack(alignment: .leading, spacing: 0) {
                Text(device.name).goKeys(.row).lineLimit(1)
                Text(device.detail).goKeys(.detail, color: GoKeys.dim)
            }
            Spacer(minLength: GoKeys.gap)
            Button(device.action, action: toggle)
                .buttonStyle(FlatButtonStyle(on: device.state == .connected))
        }
        .padding(.horizontal, GoKeys.page)
        .padding(.vertical, GoKeys.gap)
        .frame(minHeight: GoKeys.row)
    }
}

/// The MIDI channel sounds are sent on.
private struct ChannelMenu: View {
    let view: ViewModel
    let send: (Event) -> Void

    var body: some View {
        FlatMenu(title: Wording.channel(view.channel), label: "MIDI channel") {
            ForEach(view.channels, id: \.self) { channel in
                MenuChoice(title: Wording.channel(channel), selected: channel == view.channel) { send(.setChannel(channel)) }
            }
        }
    }
}
