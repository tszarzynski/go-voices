//! The MIDI and Bluetooth operations a shell performs.
//!
//! | operation                 | kind   | output                       |
//! |---------------------------|--------|------------------------------|
//! | [`WatchDevices`]          | stream | [`MidiDevices`], per change  |
//! | [`ConnectMidi`]           | stream | [`Connection`], per event    |
//! | [`DisconnectMidi`]        | notify |                              |
//! | [`ForgetDevice`]          | notify |                              |
//! | [`ScanBluetooth`]         | stream | [`ScanEvent`], per event     |
//! | [`StopBluetoothScan`]     | notify |                              |
//! | [`OpenBluetoothSettings`] | notify |                              |
//! | [`SendMidi`]              | notify |                              |
//!
//! Crux streams can't be cancelled from the core's side, so ending one is a
//! notification: [`DisconnectMidi`] ends a [`ConnectMidi`] stream and
//! [`StopBluetoothScan`] a [`ScanBluetooth`] one. After it, the shell sends
//! nothing more on that stream; anything it does send is ignored.
//!
//! Device ids are chosen by the shell and opaque to the core. They must be
//! stable for as long as the app runs, and the same in [`WatchDevices`] and
//! [`ScanBluetooth`] for the same device.

use alloc::{string::String, vec::Vec};
use core::{fmt, future};

use crux_core::{
    Command, Request,
    command::{NotificationBuilder, StreamBuilder},
    macros::Operation,
};
use facet::Facet;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

/// How a MIDI device is attached.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Transport {
    /// A USB cable.
    Usb,
    /// Bluetooth LE MIDI.
    Bluetooth,
    /// Software (an IAC bus, another app's port).
    Virtual,
    /// Anything the shell can't tell.
    Other,
}

/// A MIDI input the shell can connect to.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MidiDevice {
    /// Chosen by the shell; see the module documentation.
    pub id: String,
    /// What to call it.
    pub name: String,
    /// How it's attached.
    pub transport: Transport,
}

/// Items of the [`WatchDevices`] stream: every input available now.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MidiDevices(pub Vec<MidiDevice>);

/// Why connecting or scanning failed. The core words the message.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum MidiError {
    /// The device isn't there (any more).
    NotFound,
    /// No answer within the shell's timeout.
    NotResponding,
    /// The device has no MIDI output, or no BLE-MIDI service.
    NotMidi,
    /// The link dropped before the connection was ready.
    Disconnected,
    /// The device refused to pair, or pairing broke off.
    PairingFailed,
    /// Bluetooth is switched off.
    BluetoothOff,
    /// The user hasn't allowed Bluetooth access.
    PermissionDenied,
    /// The platform can't do this (no Bluetooth LE, for example).
    Unsupported,
    /// The shell can't take another connection.
    Busy,
    /// Anything else, as the platform reported it.
    Other(String),
}

/// The message to show the user.
impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFound => "Device not found",
            Self::NotResponding => "Not responding",
            Self::NotMidi => "No MIDI from this device",
            Self::Disconnected => "Disconnected while connecting",
            Self::PairingFailed => "Pairing failed",
            Self::BluetoothOff => "Turn on Bluetooth",
            Self::PermissionDenied => "Allow Bluetooth access",
            Self::Unsupported => "Bluetooth isn't available",
            Self::Busy => "Disconnect the other device first",
            Self::Other(message) => message,
        })
    }
}

impl core::error::Error for MidiError {}

/// Items of a [`ConnectMidi`] stream.
///
/// `Connected` once the device is ready, then `Bytes` as they arrive. The
/// stream ends with `Failed` (it never connected) or `Lost` (the link
/// dropped after connecting), or when the core sends [`DisconnectMidi`].
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Connection {
    /// Ready: MIDI follows.
    Connected,
    /// Raw MIDI 1.0 bytes, as the OS delivered them (several messages, or
    /// part of one).
    Bytes(Vec<u8>),
    /// It never connected.
    Failed(MidiError),
    /// The link dropped after connecting.
    Lost,
}

impl Connection {
    /// Whether this is the stream's last item.
    #[must_use]
    pub const fn is_final(&self) -> bool {
        matches!(self, Self::Failed(_) | Self::Lost)
    }
}

/// Items of a [`ScanBluetooth`] stream. It ends with `Stopped` (the scan
/// timed out) or `Failed`, or when the core sends [`StopBluetoothScan`].
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum ScanEvent {
    /// A BLE-MIDI device. Sent again when its name changes (a complete name
    /// after a shortened one, say).
    Found(MidiDevice),
    /// The scan timed out.
    Stopped,
    /// The scan couldn't run.
    Failed(MidiError),
}

impl ScanEvent {
    /// Whether this is the stream's last item.
    #[must_use]
    pub const fn is_final(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed(_))
    }
}

/// Report the available MIDI inputs now and after every change (a device
/// plugged in, removed, paired or forgotten). Requested once, at start.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(stream, output = MidiDevices)]
pub struct WatchDevices;

/// Connect to a device (for a Bluetooth device found by a scan, open the
/// Bluetooth connection, pairing if needed) and send what it plays.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(stream, output = Connection)]
pub struct ConnectMidi {
    /// The device to connect to.
    pub device_id: String,
}

/// Close a connection, or give up connecting.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct DisconnectMidi {
    /// The device to disconnect.
    pub device_id: String,
}

/// Delete what the shell keeps about a device (Bluetooth pairing keys), so
/// it's no longer listed or reconnected. Shells that keep nothing ignore it.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct ForgetDevice {
    /// The device to forget.
    pub device_id: String,
}

/// Scan for Bluetooth LE MIDI devices, where the app can scan itself.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(stream, output = ScanEvent)]
pub struct ScanBluetooth;

/// Stop the scan [`ScanBluetooth`] started.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct StopBluetoothScan;

/// Open the platform's Bluetooth MIDI pairing UI (macOS: Audio MIDI Setup),
/// where the app can't scan itself.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct OpenBluetoothSettings;

/// Send MIDI 1.0 bytes to a connected device, such as program changes for a
/// keyboard's sound engine. The shell finds the device's output from the
/// input [`WatchDevices`] listed.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct SendMidi {
    /// The device, as [`WatchDevices`] lists it.
    pub device_id: String,
    /// Whole messages, with status bytes (no running status).
    pub bytes: Vec<u8>,
}

/// Ends `stream` after the first item for which `is_final` holds, passing
/// that item on (`take_while` would drop it).
fn through_final<S, T>(stream: S, is_final: fn(&T) -> bool) -> impl Stream<Item = T>
where
    S: Stream<Item = T>,
{
    stream.scan(false, move |ended, item| {
        if *ended {
            return future::ready(None);
        }
        *ended = is_final(&item);
        future::ready(Some(item))
    })
}

/// Asks for [`WatchDevices`]: the device list, now and at every change.
#[must_use]
pub fn watch_devices<Effect, Event>()
-> StreamBuilder<Effect, Event, impl Stream<Item = MidiDevices>>
where
    Effect: From<Request<WatchDevices>> + Send + 'static,
    Event: Send + 'static,
{
    Command::stream_from_shell(WatchDevices)
}

/// Asks for [`ConnectMidi`]; the stream ends after its final item.
#[must_use]
pub fn connect<Effect, Event>(
    device_id: impl Into<String>,
) -> StreamBuilder<Effect, Event, impl Stream<Item = Connection>>
where
    Effect: From<Request<ConnectMidi>> + Send + 'static,
    Event: Send + 'static,
{
    let device_id = device_id.into();
    StreamBuilder::new(|ctx| {
        through_final(
            ctx.stream_from_shell(ConnectMidi { device_id }),
            Connection::is_final,
        )
    })
}

/// Sends [`DisconnectMidi`].
#[must_use]
pub fn disconnect<Effect, Event>(
    device_id: impl Into<String>,
) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<DisconnectMidi>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(DisconnectMidi {
        device_id: device_id.into(),
    })
}

/// Sends [`ForgetDevice`].
#[must_use]
pub fn forget<Effect, Event>(
    device_id: impl Into<String>,
) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<ForgetDevice>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(ForgetDevice {
        device_id: device_id.into(),
    })
}

/// Asks for [`ScanBluetooth`]; the stream ends after its final item.
#[must_use]
pub fn scan_bluetooth<Effect, Event>() -> StreamBuilder<Effect, Event, impl Stream<Item = ScanEvent>>
where
    Effect: From<Request<ScanBluetooth>> + Send + 'static,
    Event: Send + 'static,
{
    StreamBuilder::new(|ctx| {
        through_final(ctx.stream_from_shell(ScanBluetooth), ScanEvent::is_final)
    })
}

/// Sends [`StopBluetoothScan`].
#[must_use]
pub fn stop_bluetooth_scan<Effect, Event>()
-> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<StopBluetoothScan>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(StopBluetoothScan)
}

/// Sends [`OpenBluetoothSettings`].
#[must_use]
pub fn open_bluetooth_settings<Effect, Event>()
-> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<OpenBluetoothSettings>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(OpenBluetoothSettings)
}

/// Sends [`SendMidi`].
#[must_use]
pub fn send<Effect, Event>(
    device_id: impl Into<String>,
    bytes: Vec<u8>,
) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<SendMidi>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(SendMidi {
        device_id: device_id.into(),
        bytes,
    })
}
