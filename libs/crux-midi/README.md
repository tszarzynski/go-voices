# crux-midi

MIDI devices for [Crux](https://github.com/redbadger/crux) apps: the shell does the I/O, and the core decides what to connect.

- `effects`: the operations a shell performs. It lists MIDI devices (`WatchDevices`), connects to them (`ConnectMidi`, a stream of `Connected`, `Bytes`, then `Failed` or `Lost`), disconnects and forgets them, scans for Bluetooth LE MIDI (`ScanBluetooth`, or `OpenBluetoothSettings` where the system pairs), and sends bytes (`SendMidi`).
- `Connections`: the part of your model that decides what to connect. It connects every listed device, except those the user disconnected or that failed to connect, and reconnects after a dropped link. Attempt numbers make it ignore late news from connections and scans it gave up on, and it stops scanning once something connects.

Your app keeps a `Connections` in its model, wraps `MidiEvent` in one of its own events, and acts on the `Notice`s it gets back (`Connected`, `Bytes`, `Released`):

```rust,ignore
Event::Midi(event) => {
    let Update { command, changed, notices } = model.midi.update(event);
    for notice in notices { /* feed bytes, release notes, send a program change … */ }
    if changed { command.and(render()) } else { command }
}
```

`no_std` with `alloc` (`default-features = false`).

## Licence

MIT OR Apache-2.0.
