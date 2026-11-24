import App
import SwiftUI

/// The keyboard: connect and disconnect keyboards, the channel, seamless
/// switching, the effects every part shares, and pairing a Bluetooth one.
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
                        Display { StatusLine(status: status.message) { send(.dismissStatus) } }
                    }
                    // Each section: its heading beside its control, then a
                    // short line on what it does.
                    Section(title: "Keyboards", hint: view.devices.isEmpty ? Wording.devicesHint : nil) {
                        VStack(spacing: GoKeys.tight) {
                            ForEach(view.devices, id: \.id) { device in
                                DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                            }
                        }
                    }
                    Section(title: "MIDI channel", hint: Wording.channelHint) {
                        ChannelMenu(view: view, send: send).fixedSize()
                    }
                    Section(title: Wording.seamless, hint: Wording.seamlessHint) {
                        Button(view.seamless ? "On" : "Off") { send(.setSeamless(!view.seamless)) }
                            .buttonStyle(FlatButtonStyle(on: view.seamless))
                            .help("\(Wording.seamless): \(Wording.tap.lowercased()) to switch")
                            .accessibilityLabel(Wording.seamless)
                            .accessibilityValue(view.seamless ? "On" : "Off")
                    }
                    ForEach(view.keyboard ?? [], id: \.title) { group in
                        Section(title: group.title, hint: group.note.isEmpty ? nil : group.note) {
                            EmptyView()
                        }
                        ControlGrid(groups: [group], bare: true, send: send)
                    }
                    Section(title: "Bluetooth", hint: Wording.bluetoothHint) {
                        VStack(alignment: .leading, spacing: GoKeys.tight) {
                            ForEach(view.bluetooth.found, id: \.id) { device in
                                DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                            }
                            Button(Wording.bluetoothAction) {
                                send(view.bluetooth.scanning ? .stopBluetoothScan : .startBluetoothScan)
                            }
                            .buttonStyle(FlatButtonStyle(on: view.bluetooth.scanning))
                        }
                    }
                    #if os(iOS)
                    // The Mac has these in its menus.
                    Section(title: "Go:Voices", hint: nil) {
                        ToggleGroup {
                            Button("Help") { showing = .help }
                                .buttonStyle(FlatButtonStyle())
                            Button("About") { showing = .about }
                                .buttonStyle(FlatButtonStyle())
                        }
                        .fixedSize()
                    }
                    #endif
                }
                .padding(GoKeys.page)
            }
        }
        .background(GoKeys.panel)
        .tint(GoKeys.accent)
        #if os(macOS)
        .frame(minWidth: 520, minHeight: 420)
        #else
        .sheet(item: $showing) { page in
            switch page {
            case .help: HelpView()
            case .about: AboutView()
            }
        }
        #endif
    }
}

/// One section of the sheet: its heading on the left, its control on the
/// right, and its hint, dim and short, under them.
private struct Section<Content: View>: View {
    let title: String
    let hint: String?
    @ViewBuilder let content: Content

    /// The headings' column: wide enough for the longest.
    private static var headingWidth: CGFloat { GoKeys.s(190) }

    var body: some View {
        VStack(alignment: .leading, spacing: GoKeys.tight) {
            HStack(alignment: .firstTextBaseline, spacing: GoKeys.gap) {
                Text(title).goKeys(.heading)
                    .frame(width: Self.headingWidth, alignment: .leading)
                content
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            if let hint {
                Text(hint).goKeys(.detail, color: GoKeys.dim)
                    .fixedSize(horizontal: false, vertical: true)
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
            Spacer()
            Button(device.action, action: toggle)
                .buttonStyle(FlatButtonStyle(on: device.state == .connected))
        }
        .padding(.horizontal, GoKeys.gap)
        .frame(minHeight: GoKeys.row)
        .background(GoKeys.background, in: RoundedRectangle(cornerRadius: GoKeys.radius))
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
