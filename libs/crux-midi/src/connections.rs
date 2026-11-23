//! Which devices to connect, and what became of them.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crux_core::{Command, Request, command::AbortHandle};

use crate::effects::{
    self, ConnectMidi, Connection, DisconnectMidi, ForgetDevice, MidiDevice, MidiDevices,
    OpenBluetoothSettings, ScanBluetooth, ScanEvent, StopBluetoothScan, WatchDevices,
};

/// The effects [`Connections`] asks for. An app's `Effect` has them when it
/// has a variant for each of these operations.
pub trait MidiEffect:
    From<Request<WatchDevices>>
    + From<Request<ConnectMidi>>
    + From<Request<DisconnectMidi>>
    + From<Request<ForgetDevice>>
    + From<Request<ScanBluetooth>>
    + From<Request<StopBluetoothScan>>
    + From<Request<OpenBluetoothSettings>>
    + Send
    + Unpin
    + 'static
{
}

impl<T> MidiEffect for T where
    T: From<Request<WatchDevices>>
        + From<Request<ConnectMidi>>
        + From<Request<DisconnectMidi>>
        + From<Request<ForgetDevice>>
        + From<Request<ScanBluetooth>>
        + From<Request<StopBluetoothScan>>
        + From<Request<OpenBluetoothSettings>>
        + Send
        + Unpin
        + 'static
{
}

/// What the shell answers [`Connections`]' requests with. Wrap it in an
/// app event (`impl From<MidiEvent> for Event`) and hand it back to
/// [`Connections::update`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MidiEvent {
    /// The shell's device list changed ([`WatchDevices`]).
    Devices(MidiDevices),
    /// News of a connection ([`ConnectMidi`]).
    Link {
        /// The device connected to.
        device_id: String,
        /// Which attempt at it: a later one makes this one stale.
        attempt: u32,
        /// What happened.
        item: Connection,
    },
    /// News of a scan ([`ScanBluetooth`]).
    Scan {
        /// Which scan: a later one makes this one stale.
        attempt: u32,
        /// What it found.
        item: ScanEvent,
    },
}

/// What the app may want to act on, besides redrawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The device is connected and ready for [`effects::send`].
    Connected(String),
    /// MIDI arrived from the device.
    Bytes {
        /// Where from.
        device_id: String,
        /// Raw MIDI 1.0, as the shell delivered it.
        bytes: Vec<u8>,
    },
    /// The device's connection ended (or it stopped connecting). Notes it
    /// was holding won't be released by note-offs now.
    Released(String),
}

/// The result of a change to [`Connections`].
#[must_use]
pub struct Update<Effect, Event> {
    /// What to ask the shell for.
    pub command: Command<Effect, Event>,
    /// Whether anything the app might show changed.
    pub changed: bool,
    /// What happened to devices, in order.
    pub notices: Vec<Notice>,
}

impl<Effect, Event> Update<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    fn unchanged() -> Self {
        Self {
            command: Command::done(),
            changed: false,
            notices: Vec::new(),
        }
    }

    const fn changed(command: Command<Effect, Event>, notices: Vec<Notice>) -> Self {
        Self {
            command,
            changed: true,
            notices,
        }
    }
}

/// Where a device's connection is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkState {
    /// Connecting, or pairing.
    Connecting,
    /// Connected and ready.
    Connected,
}

/// A connection, from the first `ConnectMidi` until it ends.
struct Link {
    /// Tells this attempt's items from those of an earlier one.
    attempt: u32,
    /// Whether the shell has reported it ready.
    connected: bool,
    handle: AbortHandle,
}

/// A Bluetooth scan, from `ScanBluetooth` until it ends.
struct Scan {
    attempt: u32,
    handle: AbortHandle,
}

/// The MIDI devices, their connections and the Bluetooth scan: keep one in
/// the app's model.
///
/// Every listed device is connected (or only those the app wants, see
/// [`Connections::auto_connect_only`]), except those the user disconnected
/// ([`Connections::toggle`]) or that failed to connect, until toggled again.
/// A dropped link is reconnected while its device is listed. A connection
/// stops the scan, which would only take radio time from MIDI now.
#[derive(Default)]
pub struct Connections {
    /// What the shell reports through `WatchDevices`.
    devices: Vec<MidiDevice>,
    /// Devices connected or connecting, by id.
    links: BTreeMap<String, Link>,
    /// Devices not to connect automatically: disconnected by the user, or
    /// failed to connect.
    no_auto_connect: BTreeSet<String>,
    /// Which devices connect by themselves; all of them if `None`.
    auto_connects: Option<fn(&MidiDevice) -> bool>,
    attempts: u32,
    bluetooth_scan: bool,
    scan: Option<Scan>,
    /// Devices found by the current (or last) scan.
    found: Vec<MidiDevice>,
    status: Option<String>,
}

impl Connections {
    /// Starts watching the shell's devices. `bluetooth_scan` is whether the
    /// app scans for Bluetooth MIDI itself; if not, [`Connections::start_scan`]
    /// opens the system's pairing UI instead.
    pub fn start<Effect, Event>(&mut self, bluetooth_scan: bool) -> Command<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        self.bluetooth_scan = bluetooth_scan;
        effects::watch_devices().then_send(|devices| MidiEvent::Devices(devices).into())
    }

    /// Connects only the devices `wanted` accepts by themselves, so other
    /// MIDI gear (a DAW's ports, another synth) isn't sent the app's MIDI.
    /// The others are still listed and connect when toggled.
    pub fn auto_connect_only(&mut self, wanted: fn(&MidiDevice) -> bool) {
        self.auto_connects = Some(wanted);
    }

    /// Takes the shell's answer to an earlier request.
    pub fn update<Effect, Event>(&mut self, event: MidiEvent) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        match event {
            MidiEvent::Devices(MidiDevices(devices)) => self.set_devices(devices),
            MidiEvent::Link {
                device_id,
                attempt,
                item,
            } => self.on_link(device_id, attempt, item),
            MidiEvent::Scan { attempt, item } => self.on_scan(attempt, item),
        }
    }

    /// Connects to a device, or disconnects it, or stops connecting to it.
    /// A device disconnected here isn't connected again automatically until
    /// toggled again.
    pub fn toggle<Effect, Event>(&mut self, device_id: String) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        let mut notices = Vec::new();
        let command = if self.links.contains_key(&device_id) {
            self.no_auto_connect.insert(device_id.clone());
            self.close_link(&device_id, &mut notices)
        } else {
            self.no_auto_connect.remove(&device_id);
            // Trying again: the last error no longer applies.
            self.status = None;
            self.connect(device_id)
        };
        Update::changed(command, notices)
    }

    /// Disconnects a listed device and has the shell forget it. If it comes
    /// back (paired again), it's connected like a new device.
    pub fn forget<Effect, Event>(&mut self, device_id: String) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        if !self.devices.iter().any(|d| d.id == device_id) {
            return Update::unchanged();
        }
        self.no_auto_connect.remove(&device_id);
        let mut notices = Vec::new();
        let command = self
            .close_link(&device_id, &mut notices)
            .and(effects::forget(device_id).build());
        Update::changed(command, notices)
    }

    /// Starts scanning for Bluetooth MIDI, or starts again with an empty
    /// list if already scanning. Where the app can't scan, opens the
    /// system's pairing UI.
    pub fn start_scan<Effect, Event>(&mut self) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        if !self.bluetooth_scan {
            return Update {
                command: effects::open_bluetooth_settings().build(),
                ..Update::unchanged()
            };
        }
        let stop = self.end_scan();
        // Keep what's being connected to: it's listed from here until the
        // shell's device list has it.
        self.found
            .retain(|device| self.links.contains_key(&device.id));
        self.status = None;
        let attempt = self.next_attempt();
        let scan = effects::scan_bluetooth()
            .then_send(move |item| MidiEvent::Scan { attempt, item }.into());
        self.scan = Some(Scan {
            attempt,
            handle: scan.abort_handle(),
        });
        // In this order: the shell stops the old scan before starting anew.
        Update::changed(stop.and(scan), Vec::new())
    }

    /// Stops scanning.
    pub fn stop_scan<Effect, Event>(&mut self) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        Update::changed(self.end_scan(), Vec::new())
    }

    /// Hides the last error.
    pub fn dismiss_status(&mut self) {
        self.status = None;
    }

    /// The devices the shell lists.
    #[must_use]
    pub fn devices(&self) -> &[MidiDevice] {
        &self.devices
    }

    /// Devices found by the current (or last) scan, listed or not.
    #[must_use]
    pub fn found(&self) -> &[MidiDevice] {
        &self.found
    }

    /// Where the device's connection is, if it has one.
    #[must_use]
    pub fn state(&self, device_id: &str) -> Option<LinkState> {
        self.links.get(device_id).map(|link| {
            if link.connected {
                LinkState::Connected
            } else {
                LinkState::Connecting
            }
        })
    }

    /// The ids of the connected devices.
    pub fn connected(&self) -> impl Iterator<Item = &str> {
        self.links
            .iter()
            .filter(|(_, link)| link.connected)
            .map(|(id, _)| id.as_str())
    }

    /// Whether the app scans for Bluetooth MIDI itself.
    #[must_use]
    pub const fn can_scan(&self) -> bool {
        self.bluetooth_scan
    }

    /// Whether a scan is running.
    #[must_use]
    pub const fn scanning(&self) -> bool {
        self.scan.is_some()
    }

    /// The last error, worded for the user, until dismissed or the next
    /// attempt.
    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    const fn next_attempt(&mut self) -> u32 {
        self.attempts = self.attempts.wrapping_add(1);
        self.attempts
    }

    /// Takes the shell's new device list. Devices that went away are
    /// disconnected; new ones are connected automatically.
    fn set_devices<Effect, Event>(&mut self, devices: Vec<MidiDevice>) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        // Unplugged or switched off: its connection is over, whether or not
        // the shell has said so yet. Devices that were never listed
        // (connecting from a scan) aren't affected.
        let gone: Vec<String> = self
            .devices
            .iter()
            .filter(|old| !devices.iter().any(|new| new.id == old.id))
            .map(|old| old.id.clone())
            .collect();
        let mut notices = Vec::new();
        let mut commands: Vec<_> = gone
            .iter()
            .map(|id| self.close_link(id, &mut notices))
            .collect();
        self.devices = devices;
        commands.extend(self.auto_connect());
        Update::changed(Command::all(commands), notices)
    }

    /// Connects every listed device the app wants that isn't connected,
    /// connecting, or left alone on purpose.
    fn auto_connect<Effect, Event>(&mut self) -> Vec<Command<Effect, Event>>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        #[expect(clippy::needless_collect, reason = "`connect` needs `self` mutably")]
        let ids: Vec<String> = self
            .devices
            .iter()
            .filter(|d| self.auto_connects.is_none_or(|wanted| wanted(d)))
            .map(|d| &d.id)
            .filter(|id| !self.links.contains_key(*id) && !self.no_auto_connect.contains(*id))
            .cloned()
            .collect();
        ids.into_iter().map(|id| self.connect(id)).collect()
    }

    fn connect<Effect, Event>(&mut self, device_id: String) -> Command<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        let attempt = self.next_attempt();
        let id = device_id.clone();
        let command = effects::connect(device_id.clone()).then_send(move |item| {
            MidiEvent::Link {
                device_id: id.clone(),
                attempt,
                item,
            }
            .into()
        });
        self.links.insert(
            device_id,
            Link {
                attempt,
                connected: false,
                handle: command.abort_handle(),
            },
        );
        command
    }

    /// Ends the connection to `device_id`, if there is one.
    fn close_link<Effect, Event>(
        &mut self,
        device_id: &str,
        notices: &mut Vec<Notice>,
    ) -> Command<Effect, Event>
    where
        Effect: MidiEffect,
        Event: Send + Unpin + 'static,
    {
        let Some(link) = self.links.remove(device_id) else {
            return Command::done();
        };
        link.handle.abort();
        notices.push(Notice::Released(device_id.to_string()));
        effects::disconnect(device_id.to_string()).build()
    }

    fn on_link<Effect, Event>(
        &mut self,
        device_id: String,
        attempt: u32,
        item: Connection,
    ) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: From<MidiEvent> + Send + Unpin + 'static,
    {
        let Some(link) = self.links.get_mut(&device_id) else {
            return Update::unchanged();
        };
        if link.attempt != attempt {
            return Update::unchanged();
        }
        match item {
            Connection::Connected => {
                link.connected = true;
                Update::changed(self.end_scan(), vec![Notice::Connected(device_id)])
            }
            Connection::Bytes(bytes) => Update {
                notices: vec![Notice::Bytes { device_id, bytes }],
                ..Update::unchanged()
            },
            Connection::Failed(error) => {
                self.links.remove(&device_id);
                self.status = Some(error.to_string());
                self.no_auto_connect.insert(device_id.clone());
                Update::changed(Command::done(), vec![Notice::Released(device_id)])
            }
            Connection::Lost => {
                self.links.remove(&device_id);
                // Still listed (out of range, say): try again. A device that
                // was removed is connected when it comes back.
                let command = Command::all(self.auto_connect());
                Update::changed(command, vec![Notice::Released(device_id)])
            }
        }
    }

    /// Ends the scan, if one is running.
    fn end_scan<Effect, Event>(&mut self) -> Command<Effect, Event>
    where
        Effect: MidiEffect,
        Event: Send + Unpin + 'static,
    {
        self.scan.take().map_or_else(Command::done, |scan| {
            scan.handle.abort();
            effects::stop_bluetooth_scan().build()
        })
    }

    fn on_scan<Effect, Event>(&mut self, attempt: u32, item: ScanEvent) -> Update<Effect, Event>
    where
        Effect: MidiEffect,
        Event: Send + Unpin + 'static,
    {
        if self
            .scan
            .as_ref()
            .is_none_or(|scan| scan.attempt != attempt)
        {
            return Update::unchanged();
        }
        match item {
            ScanEvent::Found(device) => {
                // Seen again (often with its complete name now): update it.
                match self.found.iter_mut().find(|d| d.id == device.id) {
                    Some(known) if *known == device => return Update::unchanged(),
                    Some(known) => *known = device,
                    None => self.found.push(device),
                }
            }
            ScanEvent::Stopped => self.scan = None,
            ScanEvent::Failed(error) => {
                self.scan = None;
                self.status = Some(error.to_string());
            }
        }
        Update::changed(Command::done(), Vec::new())
    }
}
