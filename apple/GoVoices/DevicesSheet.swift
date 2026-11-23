import App
import SwiftUI

/// The keyboards: connect and disconnect them, and pair a Bluetooth one.
struct DevicesSheet: View {
    let view: ViewModel
    let send: (Event) -> Void
    let done: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("KEYBOARDS")
                    .font(.inter(13, weight: .heavy))
                    .tracking(1.5)
                    .foregroundStyle(.white)
                Spacer()
                Button("Done", action: done)
                    .buttonStyle(FlatButtonStyle())
                    .keyboardShortcut(.defaultAction)
            }
            .padding(.horizontal, GoKeys.page)
            .padding(.vertical, GoKeys.s(8))
            .background(GoKeys.panel)
            .overlay(alignment: .bottom) { GoKeys.line.frame(height: 1) }

            ScrollView {
                VStack(alignment: .leading, spacing: GoKeys.gap) {
                    if let status = view.status {
                        Display { StatusLine(status: status) { send(.dismissStatus) } }
                    }

                    Heading(text: "Keyboards")
                    if view.devices.isEmpty {
                        Text(view.devicesHint).goKeys(.row, color: GoKeys.dim)
                    }
                    ForEach(view.devices, id: \.id) { device in
                        DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                    }

                    Heading(text: "MIDI channel").padding(.top, GoKeys.gap)
                    Text(view.channelHint).goKeys(.row, color: GoKeys.dim)
                    ChannelMenu(view: view, send: send)

                    Heading(text: view.seamless.label).padding(.top, GoKeys.gap)
                    Text(view.seamless.hint).goKeys(.row, color: GoKeys.dim)
                        .fixedSize(horizontal: false, vertical: true)
                    ToggleGroup {
                        ForEach([false, true], id: \.self) { on in
                            Button(on ? "On" : "Off") { send(.setSeamless(on)) }
                                .buttonStyle(FlatButtonStyle(on: view.seamless.on == on))
                                .accessibilityAddTraits(view.seamless.on == on ? .isSelected : [])
                        }
                    }
                    .fixedSize()

                    Heading(text: "Bluetooth").padding(.top, GoKeys.gap)
                    Text(view.bluetooth.hint).goKeys(.row, color: GoKeys.dim)
                    ForEach(view.bluetooth.found, id: \.id) { device in
                        DeviceRow(device: device) { send(.toggleDevice(deviceId: device.id)) }
                    }
                    Button(view.bluetooth.action) {
                        send(view.bluetooth.scanning ? .stopBluetoothScan : .startBluetoothScan)
                    }
                    .buttonStyle(FlatButtonStyle(on: view.bluetooth.scanning))
                }
                .padding(GoKeys.page)
            }
        }
        .background(GoKeys.panel)
        .tint(GoKeys.accent)
        #if os(macOS)
        .frame(minWidth: 440, minHeight: 360)
        #endif
    }
}

private struct Heading: View {
    let text: String

    var body: some View {
        Text(caps: text).goKeys(.label, color: GoKeys.dim)
    }
}

/// One keyboard: its LED, name and state, and what tapping does.
private struct DeviceRow: View {
    let device: DeviceView
    let toggle: () -> Void

    var body: some View {
        HStack(spacing: GoKeys.gap) {
            LedDot(color: device.state.led)
            VStack(alignment: .leading, spacing: 1) {
                Text(device.name).goKeys(.row).lineLimit(1)
                Text(device.detail).goKeys(.detail, color: GoKeys.dim)
            }
            Spacer()
            Button(device.action, action: toggle)
                .buttonStyle(FlatButtonStyle(on: device.state == .connected))
        }
        .padding(.horizontal, GoKeys.s(10))
        .padding(.vertical, GoKeys.s(8))
        .background(GoKeys.background)
    }
}

/// The MIDI channel sounds are sent on.
private struct ChannelMenu: View {
    let view: ViewModel
    let send: (Event) -> Void

    var body: some View {
        FlatMenu(title: view.channelLabel, label: "MIDI channel") {
            ForEach(view.channels, id: \.channel) { option in
                MenuChoice(title: option.label, selected: option.selected) { send(.setChannel(option.channel)) }
            }
        }
    }
}
