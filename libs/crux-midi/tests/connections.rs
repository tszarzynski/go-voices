//! `Connections` in a minimal app: what it reports to the app, and sending.

use crux_core::{Command, Request, macros::effect, render::RenderOperation};
use crux_midi::{
    Connections, MidiEvent, Notice,
    effects::{
        self, ConnectMidi, Connection, DisconnectMidi, ForgetDevice, MidiDevice, MidiDevices,
        OpenBluetoothSettings, ScanBluetooth, SendMidi, StopBluetoothScan, Transport, WatchDevices,
    },
};

#[effect]
#[derive(Debug)]
enum Effect {
    Render(RenderOperation),
    WatchDevices(WatchDevices),
    ConnectMidi(ConnectMidi),
    DisconnectMidi(DisconnectMidi),
    ForgetDevice(ForgetDevice),
    ScanBluetooth(ScanBluetooth),
    StopBluetoothScan(StopBluetoothScan),
    OpenBluetoothSettings(OpenBluetoothSettings),
    SendMidi(SendMidi),
}

/// The app's events: here, only what `Connections` sends itself.
#[derive(Debug)]
struct Event(#[expect(dead_code, reason = "the tests drive `Connections` directly")] MidiEvent);

impl From<MidiEvent> for Event {
    fn from(event: MidiEvent) -> Self {
        Self(event)
    }
}

fn keyboard() -> MidiDevice {
    MidiDevice {
        id: "1".into(),
        name: "GO:KEYS MIDI".into(),
        transport: Transport::Bluetooth,
    }
}

/// Lists the keyboard and returns the connect request it triggers.
fn listed(midi: &mut Connections) -> Request<ConnectMidi> {
    let _ = midi.start::<Effect, Event>(false);
    let mut update =
        midi.update::<Effect, Event>(MidiEvent::Devices(MidiDevices(vec![keyboard()])));
    assert!(update.changed);
    let Some(Effect::ConnectMidi(request)) = update.command.effects().next() else {
        panic!("a listed device is connected");
    };
    request
}

#[test]
fn reports_a_connection_and_its_bytes() {
    let mut midi = Connections::default();
    let _request = listed(&mut midi);
    assert_eq!(midi.connected().count(), 0);

    // What the connect command turns the shell's items into.
    let attempt_event = |item| MidiEvent::Link {
        device_id: "1".into(),
        attempt: 1,
        item,
    };

    let update = midi.update::<Effect, Event>(attempt_event(Connection::Connected));
    assert_eq!(update.notices, [Notice::Connected("1".into())]);
    assert_eq!(midi.connected().collect::<Vec<_>>(), ["1"]);

    let update = midi.update::<Effect, Event>(attempt_event(Connection::Bytes(vec![0x90, 60, 1])));
    assert!(!update.changed, "bytes are the app's to judge");
    assert_eq!(
        update.notices,
        [Notice::Bytes {
            device_id: "1".into(),
            bytes: vec![0x90, 60, 1]
        }]
    );

    let update = midi.update::<Effect, Event>(attempt_event(Connection::Lost));
    assert_eq!(update.notices, [Notice::Released("1".into())]);
}

#[test]
fn toggling_off_releases_the_device() {
    let mut midi = Connections::default();
    let _request = listed(&mut midi);
    let mut update = midi.toggle::<Effect, Event>("1".into());
    assert_eq!(update.notices, [Notice::Released("1".into())]);
    assert!(matches!(
        update.command.effects().next(),
        Some(Effect::DisconnectMidi(_))
    ));
}

#[test]
fn sends_bytes_to_a_device() {
    let mut command: Command<Effect, Event> = effects::send("1", vec![0xC0, 4]).build();
    let Some(Effect::SendMidi(request)) = command.effects().next() else {
        panic!("SendMidi");
    };
    assert_eq!(
        request.operation,
        SendMidi {
            device_id: "1".into(),
            bytes: vec![0xC0, 4]
        }
    );
}

#[test]
fn auto_connects_only_the_devices_the_app_wants() {
    let mut midi = Connections::default();
    midi.auto_connect_only(|device| device.name.starts_with("GO:KEYS"));
    let _ = midi.start::<Effect, Event>(false);
    let other = MidiDevice {
        id: "2".into(),
        name: "P-6 MIDI OUT".into(),
        transport: Transport::Usb,
    };
    let mut update =
        midi.update::<Effect, Event>(MidiEvent::Devices(MidiDevices(vec![other, keyboard()])));
    let connects: Vec<String> = update
        .command
        .effects()
        .filter_map(|effect| match effect {
            Effect::ConnectMidi(request) => Some(request.operation.device_id),
            _ => None,
        })
        .collect();
    assert_eq!(connects, ["1"], "only the keyboard");
    assert_eq!(midi.devices().len(), 2, "the other is still listed");

    // Toggled, it connects all the same.
    let mut update = midi.toggle::<Effect, Event>("2".into());
    assert!(matches!(
        update.command.effects().next(),
        Some(Effect::ConnectMidi(_))
    ));
}
