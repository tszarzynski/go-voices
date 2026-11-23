//! The core, driven through `crux_core::Core` by a fake shell that records
//! what it was asked to do.

use std::collections::BTreeMap;

use crux_core::{Core, Request};
use shared::{
    Bank, Category, ControlView, Effect, Event, GoVoices, LayersView, Layout, Place, Platform,
    Setting, Tab, ViewModel,
    go_keys::{self, Zone},
    layer::LayerControl,
    midi::{ConnectMidi, Connection, MidiDevice, MidiDevices, Transport, WatchDevices},
    part::{Control, Edits, Scope},
    settings::{LoadSettings, PartSetup, Settings, Setup, StoredSettings},
};

/// A shell that keeps every open request and records what was sent.
struct Shell {
    core: Core<GoVoices>,
    load: Option<Request<LoadSettings>>,
    watch: Option<Request<WatchDevices>>,
    connects: BTreeMap<String, Request<ConnectMidi>>,
    /// `SendMidi` requests: device and bytes.
    sent: Vec<(String, Vec<u8>)>,
    /// The last `SaveSettings`.
    saved: Option<Settings>,
}

impl Shell {
    fn start() -> Self {
        let mut shell = Self {
            core: Core::new(),
            load: None,
            watch: None,
            connects: BTreeMap::new(),
            sent: Vec::new(),
            saved: None,
        };
        shell.send(Event::Start {
            platform: Platform::Ios,
        });
        assert!(shell.load.is_some(), "LoadSettings requested at start");
        assert!(shell.watch.is_some(), "WatchDevices requested at start");
        shell
    }

    /// Starts, and answers the settings request with `stored`.
    fn with_settings(stored: Option<Settings>) -> Self {
        let mut shell = Self::start();
        let mut load = shell.load.take().unwrap();
        let effects = shell
            .core
            .resolve(&mut load, StoredSettings(stored.map(|s| s.to_json())))
            .unwrap();
        shell.handle(effects);
        shell
    }

    fn send(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.handle(effects);
    }

    fn handle(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::LoadSettings(request) => self.load = Some(request),
                Effect::SaveSettings(request) => {
                    self.saved = Settings::from_json(&request.operation.0);
                }
                Effect::WatchDevices(request) => self.watch = Some(request),
                Effect::ConnectMidi(request) => {
                    self.connects
                        .insert(request.operation.device_id.clone(), request);
                }
                Effect::SendMidi(request) => {
                    let op = request.operation;
                    self.sent.push((op.device_id, op.bytes));
                }
                Effect::Render(_)
                | Effect::DisconnectMidi(_)
                | Effect::ForgetDevice(_)
                | Effect::ScanBluetooth(_)
                | Effect::StopBluetoothScan(_)
                | Effect::OpenBluetoothSettings(_) => {}
            }
        }
    }

    fn view(&self) -> ViewModel {
        self.core.view()
    }

    /// Lists the GO:KEYS and has it connect.
    fn connect_keyboard(&mut self) {
        let mut watch = self.watch.take().unwrap();
        let keyboard = MidiDevice {
            id: "go".into(),
            name: "GO:KEYS MIDI".into(),
            transport: Transport::Bluetooth,
        };
        let effects = self
            .core
            .resolve(&mut watch, MidiDevices(vec![keyboard]))
            .unwrap();
        self.watch = Some(watch);
        self.handle(effects);
        let mut connect = self.connects.remove("go").expect("connects by itself");
        let effects = self
            .core
            .resolve(&mut connect, Connection::Connected)
            .unwrap();
        self.connects.insert("go".into(), connect);
        self.handle(effects);
    }

    /// The keyboard sends these bytes.
    fn receive(&mut self, bytes: Vec<u8>) {
        let mut connect = self.connects.remove("go").expect("connected");
        let effects = self
            .core
            .resolve(&mut connect, Connection::Bytes(bytes))
            .unwrap();
        self.connects.insert("go".into(), connect);
        self.handle(effects);
    }

    /// The ids listed, in order.
    fn listed(&self) -> Vec<u32> {
        self.view()
            .sections
            .iter()
            .flat_map(|s| s.rows.iter().map(|r| r.id))
            .collect()
    }

    fn id_of(&self, name: &str) -> u32 {
        self.view()
            .sections
            .iter()
            .flat_map(|s| &s.rows)
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("{name} listed"))
            .id
    }
}

/// `JUNO Piano 1` is JUNO-DS 87·64·004.
const JUNO_PIANO_ON_1: [u8; 8] = [0xB0, 0, 87, 0xB0, 32, 64, 0xC0, 3];

#[test]
fn lists_every_sound_by_category() {
    let shell = Shell::with_settings(None);
    let view = shell.view();
    assert_eq!(view.count, "1,383 sounds");
    assert_eq!(view.search_prompt, "Search all sounds");
    assert_eq!(view.sections[0].title, "Acoustic piano");
    assert_eq!(view.sections[0].rows[0].name, "88StageGrand");
    assert_eq!(view.sections[0].rows[0].address, "87·64·001");
    assert_eq!(view.connection.label, "No keyboard");
}

#[test]
fn selecting_sends_bank_select_and_program_change() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    assert_eq!(shell.view().connection.label, "GO:KEYS MIDI");
    shell.sent.clear();

    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    assert_eq!(shell.sent, [("go".to_string(), JUNO_PIANO_ON_1.to_vec())]);
    let view = shell.view();
    let current = view.current.unwrap();
    assert_eq!(current.name, "JUNO Piano 1");
    assert_eq!(current.address, "87·64·004");
    let display = &view.displays[0];
    assert_eq!(display.heading, "Acoustic piano");
    assert_eq!(display.title, "JUNO Piano 1");
    assert_eq!(
        (current.category.as_str(), current.bank.as_str()),
        ("Acoustic piano", "JUNO-DS")
    );
    assert_eq!(current.address, "87·64·004");
    assert_eq!(view.displays[0].footnote, "Sent on channel 1");
}

#[test]
fn sends_on_the_chosen_channel_and_resends_when_it_changes() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.sent.clear();

    shell.send(Event::SetChannel(4));
    assert_eq!(
        shell.sent,
        [("go".to_string(), vec![0xB3, 0, 87, 0xB3, 32, 64, 0xC3, 3])]
    );
    assert_eq!(shell.view().channel_label, "Channel 4");

    shell.sent.clear();
    shell.send(Event::SetChannel(17));
    shell.send(Event::SetChannel(4));
    assert_eq!(shell.sent, [], "invalid or unchanged channel");
}

#[test]
fn a_pick_made_while_disconnected_is_sent_once_a_keyboard_connects() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    assert_eq!(shell.sent, []);
    assert_eq!(
        shell.view().displays[0].footnote,
        "Sent when a keyboard connects"
    );

    shell.connect_keyboard();
    assert_eq!(shell.sent.len(), 1);
    assert!(
        shell.sent[0]
            .1
            .starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1"))),
        "written to part 1 directly, whatever the channel"
    );
}

#[test]
fn searches_names_and_categories() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::Search("warm pad".into()));
    let view = shell.view();
    assert_eq!(view.sections.len(), 1);
    assert_eq!(view.sections[0].title, "Best matches");
    let names: Vec<&str> = view.sections[0]
        .rows
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert!(names[0].to_lowercase().starts_with("warm pad"), "{names:?}");
    assert!(names.iter().all(|n| n.to_lowercase().contains("warm")));

    shell.send(Event::Search("zzzz".into()));
    let view = shell.view();
    assert_eq!(view.count, "0 matches");
    assert_eq!(view.empty.as_deref(), Some("No sounds match “zzzz”"));
}

#[test]
fn the_tree_lists_banks_and_their_categories_with_counts() {
    let mut shell = Shell::with_settings(None);
    let view = shell.view();
    let places: Vec<(&str, &str, bool)> = view
        .places
        .iter()
        .map(|p| (p.label.as_str(), p.count.as_str(), p.selected))
        .collect();
    assert_eq!(
        places,
        [
            ("All sounds", "1,383", true),
            ("Favourites", "0", false),
            ("Recent", "0", false),
            ("Presets", "0", false)
        ]
    );
    let banks: Vec<&str> = view.banks.iter().map(|b| b.bank.label.as_str()).collect();
    assert_eq!(banks, ["JUNO-DS", "GM2", "GM2 Drums"]);
    let drums = &view.banks[2];
    assert_eq!(drums.bank.count, "2"); // a drum kit and an SFX kit
    assert!(!drums.holds_selection);

    let organ = Place::Category {
        bank: Bank::JunoDs,
        category: Category::Organ,
    };
    shell.send(Event::Open(organ));
    let view = shell.view();
    assert_eq!(view.place_title, "JUNO-DS › Organ");
    assert_eq!(view.search_prompt, "Search Organ");
    assert!(view.banks[0].holds_selection);
    let node = view.banks[0]
        .categories
        .iter()
        .find(|c| c.place == organ)
        .unwrap();
    assert!(node.selected);
    assert_eq!(node.label, "Organ");
    assert_eq!(view.count, format!("{} sounds", node.count));
    assert_eq!(view.sections.len(), 1);
    assert_eq!(view.sections[0].title, "", "the place header names it");
    assert_eq!(
        view.sections[0].rows[0].category, "",
        "grouped: no category"
    );
    shell.send(Event::Search("organ".into()));
    assert_eq!(shell.view().sections[0].rows[0].category, "Organ");
    shell.send(Event::Search(String::new()));

    shell.send(Event::Open(Place::Bank(Bank::Gm2Drums)));
    assert_eq!(shell.view().count, "2 sounds");
}

#[test]
fn next_and_previous_step_through_the_list_and_wrap() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::Search("grand".into()));
    let ids = shell.listed();
    assert!(ids.len() > 2);

    shell.send(Event::Next);
    assert_eq!(shell.view().current.unwrap().id, ids[0]);
    shell.send(Event::Next);
    assert_eq!(shell.view().current.unwrap().id, ids[1]);
    shell.send(Event::Previous);
    shell.send(Event::Previous);
    assert_eq!(shell.view().current.unwrap().id, *ids.last().unwrap());
}

#[test]
fn favourites_and_recents_have_their_tabs() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    let grand = shell.id_of("88StageGrand");
    shell.send(Event::ToggleFavourite(piano));
    shell.send(Event::Select(grand));
    shell.send(Event::Select(piano));

    shell.send(Event::Open(Place::Favourites));
    assert_eq!(shell.listed(), [piano]);
    shell.send(Event::Open(Place::Recent));
    assert_eq!(shell.listed(), [piano, grand], "most recent first");

    shell.send(Event::ToggleFavourite(piano));
    shell.send(Event::Open(Place::Favourites));
    assert_eq!(shell.listed(), [0_u32; 0]);
    assert_eq!(
        shell.view().empty.as_deref(),
        Some("Star a sound to keep it here")
    );
}

#[test]
fn settings_survive_a_relaunch() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    let grand = shell.id_of("88StageGrand");
    shell.send(Event::ToggleFavourite(grand));
    shell.send(Event::Select(piano));
    shell.send(Event::SetChannel(3));
    let saved = shell.saved.clone().unwrap();
    assert_eq!(saved.favourites, ["87.64.1"]);
    assert_eq!(saved.recents, ["87.64.4"]);
    assert_eq!(saved.channel, 3);

    let mut relaunched = Shell::with_settings(Some(saved));
    assert_eq!(relaunched.view().channel_label, "Channel 3");
    assert!(relaunched.view().channels[2].selected);
    relaunched.send(Event::Open(Place::Favourites));
    assert_eq!(relaunched.listed(), [grand]);
    relaunched.send(Event::Open(Place::Recent));
    assert_eq!(relaunched.listed(), [piano]);

    // The sound playing comes back, and goes to the keyboard when it
    // connects.
    assert_eq!(relaunched.view().current.unwrap().name, "JUNO Piano 1");
    relaunched.connect_keyboard();
    assert_eq!(relaunched.sent.len(), 1);
    assert!(
        relaunched.sent[0]
            .1
            .starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1")))
    );
}

#[test]
fn settings_that_load_after_a_keyboard_connects_are_sent_to_it() {
    let mut shell = Shell::start();
    shell.connect_keyboard(); // before the settings have loaded
    shell.sent.clear();
    let mut load = shell.load.take().unwrap();
    let settings = Settings {
        channel: 1,
        setup: Setup {
            parts: vec![PartSetup {
                sound: Some("87.64.4".into()), // JUNO Piano 1
                ..PartSetup::default()
            }],
            ..Setup::default()
        },
        ..Settings::default()
    };
    let effects = shell
        .core
        .resolve(&mut load, StoredSettings(Some(settings.to_json())))
        .unwrap();
    shell.handle(effects);
    assert_eq!(shell.sent.len(), 1);
    assert!(
        shell.sent[0]
            .1
            .starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1")))
    );
}

/// Parts `from` to 8 switched off.
fn off_from(from: u8) -> Vec<u8> {
    (from..=8)
        .flat_map(|part| go_keys::zone(part, Zone::OFF))
        .collect()
}

#[test]
fn a_keyboard_that_connects_with_one_sound_is_put_back_to_one_sound() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let state = &shell.sent[0].1;
    let mut keys = go_keys::zone(1, Zone::FULL);
    keys.extend(off_from(2));
    assert!(
        contains(state, &keys),
        "every other part's keys off, in case it was left split"
    );
}

#[test]
fn only_the_go_keys_connects_by_itself() {
    let mut shell = Shell::with_settings(None);
    let mut watch = shell.watch.take().unwrap();
    let devices = vec![
        MidiDevice {
            id: "p6".into(),
            name: "P-6 MIDI OUT".into(),
            transport: Transport::Usb,
        },
        MidiDevice {
            id: "iac".into(),
            name: "IAC Driver Bus 1".into(),
            transport: Transport::Virtual,
        },
        MidiDevice {
            id: "go".into(),
            name: "GO:KEYS MIDI Bluetooth".into(),
            transport: Transport::Bluetooth,
        },
    ];
    let effects = shell
        .core
        .resolve(&mut watch, MidiDevices(devices))
        .unwrap();
    shell.handle(effects);
    assert_eq!(shell.connects.keys().collect::<Vec<_>>(), ["go"]);
    assert_eq!(shell.view().devices.len(), 3, "the others are listed");
}

/// The catalogue's patch with this name, for building expected bytes.
fn patch(name: &str) -> catalogue::Patch {
    catalogue::Catalogue::builtin()
        .patches()
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("{name} in the catalogue"))
        .clone()
}

/// Every part setting at part `own`'s value, written to `part`; not the
/// octave, which goes with the keys.
fn part_defaults(part: u8, own: u8) -> Vec<u8> {
    let every: Edits = Control::part_controls()
        .filter(|&control| control != Control::Octave)
        .map(|control| (control, control.default(Scope::Part(own))))
        .collect();
    go_keys::settings(Scope::Part(part), &every)
}

/// The upper part from `split` (part 1), the lower below it (part 2).
fn split_zones(split: u8) -> Vec<u8> {
    let mut bytes = go_keys::zone(
        1,
        Zone {
            low: split,
            ..Zone::FULL
        },
    );
    bytes.extend(go_keys::zone(
        2,
        Zone {
            high: split - 1,
            ..Zone::FULL
        },
    ));
    bytes
}

fn contains(bytes: &[u8], part: &[u8]) -> bool {
    bytes.windows(part.len()).any(|w| w == part)
}

#[test]
fn splitting_sends_both_parts_and_the_keys_then_picks_the_lower_one() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.sent.clear();

    shell.send(Event::StartLayout(Layout::Split));
    let mut expected = go_keys::part_patch(1, &patch("JUNO Piano 1"));
    expected.extend(part_defaults(1, 1));
    expected.extend(go_keys::part_patch(2, &patch("JUNO Piano 1")));
    expected.extend(part_defaults(2, 2));
    expected.extend(split_zones(60));
    expected.extend(off_from(3));
    assert_eq!(shell.sent, [("go".to_string(), expected)]);
    let view = shell.view();
    let zones = &view.layout.zones;
    assert_eq!(
        zones
            .iter()
            .map(|z| (z.keys.as_str(), z.editing))
            .collect::<Vec<_>>(),
        [("From C4", false), ("Up to B3", true)],
        "the lower part is next"
    );
    assert_eq!(view.displays[0].part, 1, "lowest keys first");
    assert!(!view.displays[0].dimmed);
    assert!(view.displays[1].dimmed);
    assert_eq!(view.displays[1].heading, "1 · Acoustic piano");

    // A pick now changes the lower part only, written to part 2.
    shell.sent.clear();
    shell.send(Event::Open(Place::All));
    let bass = shell.id_of("88StageGrand");
    shell.send(Event::Select(bass));
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::part_patch(2, &patch("88StageGrand"))
        )]
    );
    let view = shell.view();
    assert_eq!(view.current.unwrap().name, "88StageGrand");
    let [lower, upper] = [&view.displays[0], &view.displays[1]];
    assert_eq!(lower.title, "88StageGrand");
    assert_eq!(lower.footnote, "Up to B3");
    assert_eq!(upper.title, "JUNO Piano 1");
    assert_eq!(upper.footnote, "From C4");

    // Part 1 again: a program change on the channel, and part 1.
    shell.sent.clear();
    shell.send(Event::EditPart(0));
    let grand = shell.id_of("88StgGrand 2");
    shell.send(Event::Select(grand));
    let mut expected = patch("88StgGrand 2").select_bytes(1);
    expected.extend(go_keys::part_patch(1, &patch("88StgGrand 2")));
    assert_eq!(shell.sent, [("go".to_string(), expected)]);
}

#[test]
fn layering_and_going_back_to_one_sound() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::StartLayout(Layout::Layer));
    let layered = shell.sent.pop().unwrap().1;
    let mut keys = go_keys::zone(1, Zone::FULL);
    keys.extend(go_keys::zone(2, Zone::FULL));
    keys.extend(off_from(3));
    assert!(layered.ends_with(&keys));
    assert_eq!(shell.view().displays[0].footnote, "All keys");

    shell.sent.clear();
    shell.send(Event::StartLayout(Layout::One));
    let mut keys = go_keys::zone(1, Zone::FULL);
    keys.extend(off_from(2));
    assert!(shell.sent[0].1.ends_with(&keys));
    let view = shell.view();
    assert_eq!(view.displays.len(), 1);
    assert!(!view.displays[0].several);
}

#[test]
fn each_part_s_keys_move_on_their_own_and_octave_resends_them() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::StartLayout(Layout::Split));
    shell.sent.clear();

    // The lower part up to F#3: only its zone is written.
    shell.send(Event::SetKeys {
        part: 1,
        low: 0,
        high: 54,
    });
    shell.send(Event::SetControl {
        control: Control::Octave,
        value: -1,
    }); // the lower part, being edited
    let mut keys = go_keys::zone(
        1,
        Zone {
            low: 60,
            ..Zone::FULL
        },
    );
    keys.extend(go_keys::zone(
        2,
        Zone {
            high: 54,
            ..Zone::FULL
        },
    ));
    assert_eq!(
        shell.sent,
        [
            ("go".to_string(), keys),
            (
                "go".to_string(),
                go_keys::setting(Scope::Part(2), Control::Octave, -1)
            ),
        ]
    );
    let view = shell.view();
    assert_eq!(view.displays[0].footnote, "Up to F#3 · Oct −1");
    assert_eq!(view.displays[1].footnote, "From C4");

    // The upper part down to C3: layered with the lower one from C3 to
    // F#3; the lower part stays as it was.
    shell.send(Event::SetKeys {
        part: 0,
        low: 48,
        high: 127,
    });
    let zones = shell.view().layout.zones;
    assert_eq!((zones[0].low, zones[1].high), (48, 54));
    assert_eq!(zones[0].keys, "From C3");

    // The keyboard's lowest key takes in every key below.
    shell.send(Event::SetKeys {
        part: 0,
        low: 36,
        high: 127,
    });
    assert_eq!(shell.view().layout.zones[0].keys, "All keys");
}

#[test]
fn a_keyboard_that_connects_gets_the_split() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::StartLayout(Layout::Split));
    assert_eq!(shell.sent, [], "nothing connected");

    shell.connect_keyboard();
    let bytes = &shell.sent[0].1;
    assert!(bytes.starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1"))));
    let mut keys = split_zones(60);
    keys.extend(off_from(3));
    assert!(contains(bytes, &keys));
}

#[test]
fn the_split_survives_a_relaunch() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::StartLayout(Layout::Split));
    let grand = shell.id_of("88StageGrand");
    shell.send(Event::Select(grand));
    shell.send(Event::SetKeys {
        part: 1,
        low: 0,
        high: 47,
    });
    shell.send(Event::SetControl {
        control: Control::Octave,
        value: 1,
    });
    let saved = shell.saved.clone().unwrap();

    let relaunched = Shell::with_settings(Some(saved));
    let displays = relaunched.view().displays;
    assert_eq!(displays[0].title, "88StageGrand");
    assert_eq!(displays[0].footnote, "Up to B2 · Oct +1");
    assert_eq!(displays[1].title, "JUNO Piano 1");
    assert_eq!(
        displays[1].footnote, "From C4",
        "its own keys, as they were"
    );
}

#[test]
fn octave_shifts_one_sound_too_and_stays_through_a_split() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.sent.clear();
    shell.send(Event::ToggleEditor);
    shell.send(Event::SetControl {
        control: Control::Octave,
        value: -1,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::setting(Scope::Part(1), Control::Octave, -1)
        )]
    );
    let play = &shell.view().editor.unwrap().groups[1];
    assert_eq!(play.controls[0].label, "Octave");
    assert_eq!(play.controls[0].text, "−1");

    // Splitting keeps it on the part.
    shell.send(Event::StartLayout(Layout::Split));
    let upper = Zone {
        low: 60,
        octave: -1,
        ..Zone::FULL
    };
    let split = shell.sent.last().unwrap().1.clone();
    assert!(contains(&split, &go_keys::zone(1, upper)), "{split:02X?}");
}

#[test]
fn a_setting_is_sent_to_the_part_being_edited_and_kept_with_new_sounds() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.sent.clear();
    shell.send(Event::ToggleEditor);
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 80,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::setting(Scope::Part(1), Control::Level, 80)
        )]
    );
    let editor = shell.view().editor.unwrap();
    assert_eq!(editor.groups[0].title, "Mix");
    let level = &editor.groups[0].controls[0];
    assert_eq!((level.text.as_str(), level.edited), ("80", true));

    // A new sound: its program change, then the level again.
    shell.sent.clear();
    shell.send(Event::Open(Place::All));
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    let mut expected = JUNO_PIANO_ON_1.to_vec();
    expected.extend(go_keys::setting(Scope::Part(1), Control::Level, 80));
    assert_eq!(shell.sent, [("go".to_string(), expected)]);
}

#[test]
fn each_part_has_its_own_settings() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::StartLayout(Layout::Split)); // editing the lower part
    shell.sent.clear();
    shell.send(Event::SetControl {
        control: Control::Pan,
        value: 44,
    });
    shell.send(Event::SetControl {
        control: Control::Mono,
        value: 0,
    });
    assert_eq!(
        shell.sent,
        [
            (
                "go".to_string(),
                go_keys::setting(Scope::Part(2), Control::Pan, 44)
            ),
            (
                "go".to_string(),
                go_keys::setting(Scope::Part(2), Control::Mono, 0)
            ),
        ]
    );
    shell.send(Event::ToggleEditor);
    let editor = shell.view().editor.unwrap();
    assert!(editor.title.starts_with("Sound modify · Part 2"));
    assert_eq!(editor.groups[0].controls[1].text, "L20");

    shell.send(Event::EditPart(0));
    let editor = shell.view().editor.unwrap();
    assert_eq!(editor.groups[0].controls[1].text, "C", "part 1's own");
    assert!(!editor.edited);
}

#[test]
fn reset_puts_the_part_back() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::SetControl {
        control: Control::ReverbSend,
        value: 20,
    });
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 90,
    });
    shell.sent.clear();

    shell.send(Event::ResetControl(Control::Level));
    shell.send(Event::ResetEditor);
    assert_eq!(
        shell.sent,
        [
            (
                "go".to_string(),
                go_keys::setting(Scope::Part(1), Control::Level, 100)
            ),
            (
                "go".to_string(),
                go_keys::setting(Scope::Part(1), Control::ReverbSend, 50)
            ),
        ],
        "back to the keyboard's own: part 1's reverb is 50"
    );
    shell.send(Event::ToggleEditor);
    assert!(!shell.view().editor.unwrap().edited);
}

#[test]
fn each_part_resets_to_its_own_reverb() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::StartLayout(Layout::Split)); // editing part 2
    shell.send(Event::ToggleEditor);
    let reverb = |shell: &mut Shell| {
        shell
            .view()
            .editor
            .unwrap()
            .groups
            .into_iter()
            .find(|g| g.title == "Effect sends")
            .unwrap()
            .controls[0]
            .clone()
    };
    assert_eq!(reverb(&mut shell).value, 60, "part 2's own");
    assert!(!reverb(&mut shell).edited);
    shell.send(Event::SetControl {
        control: Control::ReverbSend,
        value: 0,
    });
    shell.sent.clear();
    shell.send(Event::ResetControl(Control::ReverbSend));
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::setting(Scope::Part(2), Control::ReverbSend, 60)
        )]
    );
    shell.send(Event::EditPart(0));
    assert_eq!(reverb(&mut shell).value, 50, "part 1's own");
}

#[test]
fn keyboard_settings_are_shared_and_sent_when_it_connects() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::ToggleEditor);
    shell.send(Event::SetControl {
        control: Control::ChorusSwitch,
        value: 0,
    });
    shell.send(Event::SetControl {
        control: Control::ReverbLevel,
        value: 80,
    });
    let keyboard = shell.view().editor.unwrap().keyboard;
    assert_eq!(keyboard[0].title, "Effects");
    assert_eq!(keyboard[0].controls[2].text, "Off");
    assert_eq!(keyboard[0].controls[1].text, "80");

    // The same whichever part is being edited.
    shell.send(Event::StartLayout(Layout::Split));
    shell.send(Event::EditPart(1));
    assert_eq!(
        shell.view().editor.unwrap().keyboard[0].controls[1].text,
        "80"
    );

    shell.sent.clear();
    shell.connect_keyboard();
    let sent: Vec<u8> = shell
        .sent
        .iter()
        .flat_map(|(_, bytes)| bytes.clone())
        .collect();
    for setting in [
        go_keys::setting(Scope::Keyboard, Control::ChorusSwitch, 0),
        go_keys::setting(Scope::Keyboard, Control::ReverbLevel, 80),
    ] {
        assert!(contains(&sent, &setting), "{setting:02X?}");
    }
}

#[test]
fn edits_survive_a_relaunch() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 70,
    });
    shell.send(Event::SetControl {
        control: Control::ReverbLevel,
        value: 100,
    });
    let saved = shell.saved.clone().unwrap();
    let mut relaunched = Shell::with_settings(Some(saved));
    relaunched.send(Event::ToggleEditor);
    let view = relaunched.view();
    assert_eq!(
        view.editor.clone().unwrap().groups[0].controls[0].text,
        "70"
    );
    assert_eq!(view.editor.unwrap().keyboard[0].controls[1].text, "100");
}

#[test]
fn a_part_added_splits_the_part_being_edited_and_one_removed_gives_its_keys_back() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 90,
    });
    shell.sent.clear();

    // C2–C7 split in the middle: the new part 2 plays F#4 up, with the
    // same sound and settings, and is edited next.
    shell.send(Event::AddPart);
    let mut expected = go_keys::part_patch(2, &patch("JUNO Piano 1"));
    let mut settings = Control::part_controls()
        .filter(|&control| control != Control::Octave)
        .map(|control| (control, control.default(Scope::Part(2))))
        .collect::<Edits>();
    settings.insert(Control::Level, 90);
    expected.extend(go_keys::settings(Scope::Part(2), &settings));
    expected.extend(go_keys::zone(
        1,
        Zone {
            high: 65,
            ..Zone::FULL
        },
    ));
    expected.extend(go_keys::zone(
        2,
        Zone {
            low: 66,
            ..Zone::FULL
        },
    ));
    assert_eq!(shell.sent, [("go".to_string(), expected)]);
    let layout = shell.view().layout;
    assert_eq!(
        layout
            .zones
            .iter()
            .map(|z| (z.label.as_str(), z.keys.as_str(), z.editing))
            .collect::<Vec<_>>(),
        [("1", "Up to F4", false), ("2", "From F#4", true)]
    );

    // Up to four, then no more.
    shell.send(Event::AddPart);
    shell.send(Event::AddPart);
    assert!(!shell.view().layout.can_add);
    shell.sent.clear();
    shell.send(Event::AddPart);
    assert_eq!(shell.view().layout.zones.len(), 4);

    // Part 2 taken away: part 1 takes its keys back, and parts 3 and 4
    // move down to 2 and 3, written again.
    shell.sent.clear();
    shell.send(Event::RemovePart(1));
    let zones = shell.view().layout.zones;
    assert_eq!(zones.len(), 3);
    assert_eq!(zones[0].high, 80, "part 1 now ends where part 2 did");
    assert!(contains(
        &shell.sent[0].1,
        &go_keys::part_patch(3, &patch("JUNO Piano 1"))
    ));
    assert!(shell.sent[0].1.ends_with(&off_from(4)));
}

/// Seamless switching on, with the GO:KEYS connected and nothing sent yet.
fn seamless() -> Shell {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::SetSeamless(true));
    shell.sent.clear();
    shell
}

#[test]
fn seamless_switching_is_off_until_turned_on_and_kept() {
    let mut shell = Shell::with_settings(None);
    let switch = shell.view().seamless;
    assert_eq!(switch.label, "Seamless switching");
    assert!(!switch.on);
    shell.send(Event::SetSeamless(true));
    assert!(shell.view().seamless.on);
    let relaunched = Shell::with_settings(shell.saved.clone());
    assert!(relaunched.view().seamless.on);
}

#[test]
fn seamless_switching_plays_a_new_sound_on_the_spare_part() {
    let mut shell = seamless();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));

    // The sound and every setting go to part 5, then part 5's keys go on
    // and part 1's off: notes held on part 1 ring on.
    let mut expected = go_keys::part_patch(5, &patch("JUNO Piano 1"));
    expected.extend(part_defaults(5, 1));
    expected.extend(go_keys::zone(5, Zone::FULL));
    expected.extend(go_keys::zone(1, Zone::OFF));
    assert_eq!(shell.sent, [("go".to_string(), expected)]);

    // Settings now go to part 5; the next sound goes back to part 1.
    shell.sent.clear();
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 80,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::setting(Scope::Part(5), Control::Level, 80)
        )]
    );
    shell.sent.clear();
    shell.send(Event::Next);
    let bytes = &shell.sent[0].1;
    assert!(bytes.starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 2"))));
    assert!(
        contains(bytes, &go_keys::setting(Scope::Part(1), Control::Level, 80)),
        "the level follows the sound"
    );
}

#[test]
fn turning_seamless_switching_off_goes_back_to_part_1() {
    let mut shell = seamless();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano)); // now on part 5
    shell.sent.clear();

    shell.send(Event::SetSeamless(false));
    let bytes = &shell.sent[0].1;
    assert!(bytes.starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1"))));
    for zone in [go_keys::zone(1, Zone::FULL), go_keys::zone(5, Zone::OFF)] {
        assert!(contains(bytes, &zone), "{zone:02X?}");
    }
    // Back to program changes on the channel.
    shell.sent.clear();
    shell.send(Event::Select(piano));
    assert_eq!(shell.sent, [("go".to_string(), JUNO_PIANO_ON_1.to_vec())]);
}

#[test]
fn seamless_switching_in_a_split_uses_part_6_for_the_lower_part() {
    let mut shell = seamless();
    shell.send(Event::StartLayout(Layout::Split)); // editing part 2
    shell.sent.clear();
    let bass = shell.id_of("88StageGrand");
    shell.send(Event::Select(bass));
    let bytes = &shell.sent[0].1;
    assert!(bytes.starts_with(&go_keys::part_patch(6, &patch("88StageGrand"))));
    let lower = go_keys::zone(
        6,
        Zone {
            high: 59,
            ..Zone::FULL
        },
    );
    assert!(contains(bytes, &lower));
}

#[test]
fn a_temperament_tunes_the_part_in_its_key() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::ToggleEditor);
    shell.sent.clear();
    shell.send(Event::SetControl {
        control: Control::Temperament,
        value: 1,
    });
    shell.send(Event::SetControl {
        control: Control::TuningKey,
        value: 2,
    });
    let in_d = Edits::from([(Control::Temperament, 1), (Control::TuningKey, 2)]);
    assert_eq!(
        shell.sent,
        [
            (
                "go".to_string(),
                go_keys::settings(Scope::Part(1), &Edits::from([(Control::Temperament, 1)]))
            ),
            ("go".to_string(), go_keys::settings(Scope::Part(1), &in_d)),
        ]
    );
    let tune = &shell.view().editor.unwrap().groups[2];
    assert_eq!(tune.title, "Tune");
    let texts: Vec<&str> = tune.controls.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["Just", "D", "0", "0", "2"]);

    // Back to equal: every note's offset goes back to 0.
    shell.sent.clear();
    shell.send(Event::ResetControl(Control::Temperament));
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::settings(
                Scope::Part(1),
                &Edits::from([(Control::Temperament, 0), (Control::TuningKey, 2)])
            )
        )]
    );
}

/// A split of 88StageGrand below JUNO Piano 1, the upper level at 80, saved
/// as "Gig".
fn with_a_preset() -> (Shell, u32) {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::StartLayout(Layout::Split));
    let grand = shell.id_of("88StageGrand");
    shell.send(Event::Select(grand));
    shell.send(Event::EditPart(0));
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 80,
    });
    assert_eq!(shell.view().preset_name, "88StageGrand / JUNO Piano 1");
    shell.send(Event::SavePreset { name: "Gig".into() });
    shell.send(Event::Open(Place::Presets));
    let presets = shell.view().presets.unwrap();
    assert_eq!(presets.len(), 1);
    let id = presets[0].id;
    (shell, id)
}

#[test]
fn a_preset_brings_back_the_sounds_keys_and_settings() {
    let (mut shell, id) = with_a_preset();
    let preset = &shell.view().presets.unwrap()[0];
    assert_eq!(preset.name, "Gig");
    assert_eq!(preset.detail, "Split C4 · 88StageGrand / JUNO Piano 1");
    assert!(preset.current, "the one loaded");
    assert!(!preset.edited);
    let loaded = shell.view().loaded.unwrap();
    assert_eq!((loaded.id, loaded.edited), (id, false));

    // Something else entirely, then the preset again.
    shell.send(Event::StartLayout(Layout::One));
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 100,
    });
    shell.send(Event::SetControl {
        control: Control::Pan,
        value: 30,
    });
    let preset = &shell.view().presets.unwrap()[0];
    assert!(preset.current && preset.edited, "still loaded, now changed");
    assert!(shell.view().loaded.unwrap().edited);
    shell.sent.clear();
    shell.send(Event::RecallPreset(id));

    let view = shell.view();
    assert_eq!(view.layout.zones.len(), 2);
    assert_eq!(view.displays[0].title, "88StageGrand");
    assert_eq!(view.displays[1].title, "JUNO Piano 1");
    assert!(!view.presets.unwrap()[0].edited);
    // Every setting is written, so the pan left from before goes back too.
    let bytes = &shell.sent[0].1;
    for expected in [
        go_keys::part_patch(1, &patch("JUNO Piano 1")),
        go_keys::part_patch(2, &patch("88StageGrand")),
        go_keys::setting(Scope::Part(1), Control::Level, 80),
        go_keys::setting(Scope::Part(1), Control::Pan, 64),
        go_keys::zone(
            2,
            Zone {
                high: 59,
                ..Zone::FULL
            },
        ),
    ] {
        assert!(
            bytes.windows(expected.len()).any(|w| w == expected),
            "{expected:02X?}"
        );
    }
}

#[test]
fn presets_are_kept_renamed_updated_and_deleted() {
    let (mut shell, id) = with_a_preset();
    shell.send(Event::RenamePreset {
        id,
        name: "Sunday".into(),
    });
    let relaunched = Shell::with_settings(shell.saved.clone());
    let mut relaunched = relaunched;
    relaunched.send(Event::Open(Place::Presets));
    assert_eq!(relaunched.view().presets.unwrap()[0].name, "Sunday");

    // Update: what's playing now replaces it.
    shell.send(Event::StartLayout(Layout::Layer));
    shell.send(Event::UpdatePreset(id));
    let preset = &shell.view().presets.unwrap()[0];
    assert!(preset.detail.starts_with("Layer"));
    assert!(preset.current);

    shell.send(Event::DeletePreset(id));
    let view = shell.view();
    assert_eq!(view.presets, Some(Vec::new()));
    assert_eq!(
        view.empty.as_deref(),
        Some("Save what you're playing to keep it here")
    );
}

#[test]
fn next_and_previous_step_through_the_presets() {
    let (mut shell, first) = with_a_preset();
    shell.send(Event::StartLayout(Layout::One));
    shell.send(Event::SavePreset {
        name: "Solo".into(),
    });
    let names: Vec<String> = shell
        .view()
        .presets
        .unwrap()
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Gig", "Solo"]);

    shell.send(Event::Next); // after Solo: back to Gig
    let presets = shell.view().presets.unwrap();
    assert_eq!(presets[0].id, first);
    assert!(presets[0].current);
    shell.send(Event::Previous);
    assert!(shell.view().presets.unwrap()[1].current);
}

#[test]
fn save_updates_the_loaded_preset_and_save_new_adds_one() {
    let (mut shell, id) = with_a_preset();
    shell.send(Event::SetControl {
        control: Control::Pan,
        value: 30,
    });
    shell.send(Event::UpdatePreset(id));
    assert!(!shell.view().loaded.unwrap().edited, "saved");

    shell.send(Event::SavePreset {
        name: "Gig 2".into(),
    });
    let view = shell.view();
    let loaded = view.loaded.unwrap();
    assert_eq!(loaded.name, "Gig 2", "the new one is loaded");
    assert_ne!(loaded.id, id);
    assert_eq!(view.presets.unwrap().len(), 2);

    // Kept across launches; gone when deleted.
    let relaunched = Shell::with_settings(shell.saved.clone());
    assert_eq!(relaunched.view().loaded.unwrap().name, "Gig 2");
    shell.send(Event::DeletePreset(loaded.id));
    assert_eq!(shell.view().loaded, None);
}

/// A made-up sound, as part `part` would send it back: four layers on, each
/// at level 100 with cutoff 40, waves 1·291 … 1·294 ("Fng.EB1 …").
fn sound_replies(part: u8) -> Vec<u8> {
    let base = [0x11 + (part - 1) / 4, ((part - 1) % 4) * 0x20];
    let mut mix = [0u8; 0x29];
    for layer in 0..4 {
        // On, every key, every velocity.
        mix[5 + 9 * layer..10 + 9 * layer].copy_from_slice(&[1, 0, 127, 0, 0]);
        mix[10 + 9 * layer..12 + 9 * layer].copy_from_slice(&[1, 127]);
    }
    let mut bytes = go_keys::dt1([base[0], base[1], 0x10, 0], &mix);
    for layer in 0..4u8 {
        let mut tone = [64u8; 154];
        tone[0x00] = 100;
        tone[0x27] = 0;
        let wave = 291 + u16::from(layer);
        tone[0x28..0x34].copy_from_slice(&[
            0,
            0,
            0,
            1,
            0,
            (wave >> 8) as u8,
            (wave >> 4 & 15) as u8,
            (wave & 15) as u8,
            0,
            0,
            0,
            0,
        ]);
        tone[0x49] = 40;
        bytes.extend(go_keys::dt1([base[0], base[1], 0x20 + 2 * layer, 0], &tone));
    }
    bytes
}

/// Connected, JUNO Piano 1 playing, the Layers tab open and read.
fn layers_read() -> Shell {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::ToggleEditor);
    shell.sent.clear();
    shell.send(Event::ShowTab(Tab::Layers));
    assert_eq!(shell.sent, [("go".to_string(), go_keys::read_sound(1))]);
    let layers = shell.view().editor.unwrap().layers.unwrap();
    assert_eq!(
        layers.status.as_deref(),
        Some("Reading the sound from the keyboard…")
    );
    shell.receive(sound_replies(1));
    shell.sent.clear();
    shell
}

fn layers(shell: &Shell) -> LayersView {
    shell.view().editor.unwrap().layers.unwrap()
}

fn layer_control(shell: &Shell, control: LayerControl) -> ControlView {
    layers(shell)
        .groups
        .into_iter()
        .flat_map(|group| group.controls)
        .find(|c| c.setting == Setting::Layer(control))
        .unwrap()
}

#[test]
fn the_layers_tab_reads_the_sound_and_edits_it() {
    let mut shell = layers_read();
    let view = layers(&shell);
    assert_eq!(view.status, None);
    assert_eq!(shell.view().editor.unwrap().title, "Layers · JUNO Piano 1");
    let waves: Vec<&str> = view.cards.iter().map(|c| c.wave.as_str()).collect();
    assert_eq!(
        waves,
        [
            "Fng.EB1 mf A",
            "Fng.EB1 mf B",
            "Fng.EB1 mf C",
            "Fng.EB1 ff A"
        ]
    );
    assert!(view.cards.iter().all(|c| c.on));
    assert!(view.cards[0].selected);
    assert_eq!(view.wave.unwrap().detail, "Group 1 · 291");
    assert_eq!(layer_control(&shell, LayerControl::Cutoff).text, "40");
    let amp = view
        .groups
        .iter()
        .find(|g| g.title == "Amp envelope")
        .unwrap();
    assert_eq!(amp.envelope.len(), 6);

    // Layer 2's cutoff: written to tone 2, shown as changed.
    shell.send(Event::SelectLayer(1));
    shell.send(Event::SetLayerControl {
        control: LayerControl::Cutoff,
        value: 110,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::sound_bytes(1, &BTreeMap::from([((2, 0x49), 110)]))
        )]
    );
    let control = layer_control(&shell, LayerControl::Cutoff);
    assert_eq!((control.text.as_str(), control.edited), ("110", true));
    let view = layers(&shell);
    assert!(view.edited && view.cards[1].edited && !view.cards[0].edited);

    // Back as the sound had it.
    shell.sent.clear();
    shell.send(Event::ResetLayerControl(LayerControl::Cutoff));
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::sound_bytes(1, &BTreeMap::from([((2, 0x49), 40)]))
        )]
    );
    assert!(!layers(&shell).edited);
}

#[test]
fn solo_plays_one_layer_until_the_tab_changes() {
    let mut shell = layers_read();
    shell.send(Event::SoloLayer(Some(2)));
    let solo: BTreeMap<(u8, u8), u8> = (0..4u8)
        .map(|l| ((0, 5 + 9 * l), u8::from(l == 2)))
        .collect();
    assert_eq!(
        shell.sent,
        [("go".to_string(), go_keys::sound_bytes(1, &solo))]
    );
    assert!(layers(&shell).cards[2].soloed);
    assert!(!layers(&shell).edited, "a solo isn't a change");

    shell.sent.clear();
    shell.send(Event::ShowTab(Tab::Sound));
    let all: BTreeMap<(u8, u8), u8> = (0..4u8).map(|l| ((0, 5 + 9 * l), 1)).collect();
    assert_eq!(
        shell.sent,
        [("go".to_string(), go_keys::sound_bytes(1, &all))]
    );
}

#[test]
fn waves_step_and_come_in_stereo_pairs() {
    let mut shell = layers_read();
    shell.send(Event::StepWave(true));
    assert_eq!(layers(&shell).wave.unwrap().name, "Fng.EB1 mf B");

    shell.send(Event::SearchWaves(Some("ac.pno*p".into())));
    let picker = shell.view().waves.unwrap();
    assert!(picker.rows.iter().all(|row| !row.name.ends_with(" R")));
    assert_eq!(picker.rows[0].name, "Ac.Pno*p A");
    shell.sent.clear();
    shell.send(Event::SetWave {
        group: 1,
        number: 1,
    });
    // Internal, group 1, left 1, right 2: one message from 27 on.
    assert_eq!(
        go_keys::writes(&shell.sent[0].1),
        [(
            [0x11, 0x00, 0x20, 0x27],
            Vec::from([0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 2])
        )]
    );
    assert_eq!(layers(&shell).wave.unwrap().detail, "Group 1 · 1–2 stereo");
    shell.send(Event::SearchWaves(None));
    assert_eq!(shell.view().waves, None);
}

#[test]
fn a_layer_copies_onto_another_even_of_another_sound() {
    let mut shell = layers_read();
    assert_eq!(layers(&shell).paste, None, "nothing copied yet");
    shell.send(Event::SelectLayer(1));
    shell.send(Event::SetLayerControl {
        control: LayerControl::Cutoff,
        value: 110,
    });
    shell.send(Event::CopyLayer(1));
    assert_eq!(
        layers(&shell).paste.as_deref(),
        Some("Layer 2 of JUNO Piano 1")
    );

    // Onto layer 4, switched off first: it takes layer 2's tone, as
    // changed, and plays.
    shell.send(Event::SelectLayer(3));
    shell.send(Event::SetLayerControl {
        control: LayerControl::Switch,
        value: 0,
    });
    shell.sent.clear();
    shell.send(Event::PasteLayer(3));
    // Its switch, then its whole tone: 1·292 where it had 1·294, cutoff 110.
    let writes = go_keys::writes(&shell.sent[0].1);
    assert_eq!(writes[0], ([0x11, 0x00, 0x10, 5 + 9 * 3], Vec::from([1])));
    let (address, tone) = &writes[1];
    assert_eq!((*address, tone.len()), ([0x11, 0x00, 0x26, 0x00], 154));
    assert_eq!((tone[0x2F], tone[0x49]), (4, 110));
    let view = layers(&shell);
    assert_eq!(view.cards[3].wave, "Fng.EB1 mf B");
    assert!(view.cards[3].on && view.cards[3].edited);
    assert_eq!(layer_control(&shell, LayerControl::Cutoff).text, "110");

    // Still there for the next sound, once it's read.
    shell.send(Event::Next);
    shell.receive(sound_replies(1));
    assert_eq!(
        layers(&shell).paste.as_deref(),
        Some("Layer 2 of JUNO Piano 1")
    );
}

#[test]
fn a_layer_plays_a_range_of_keys_and_velocities() {
    let mut shell = layers_read();
    let card = |shell: &Shell| layers(shell).cards[0].clone();
    assert_eq!(card(&shell).detail, "All keys");
    let range = layers(&shell)
        .groups
        .into_iter()
        .find_map(|group| group.keys)
        .unwrap();
    assert_eq!((layers(&shell).first, layers(&shell).last), (36, 96));
    assert!(range.low.text == "C-1" && range.high.text == "G9");

    // Up to B3: the left hand.
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyHigh,
        value: 59,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::sound_bytes(1, &BTreeMap::from([((0, 7), 59)]))
        )]
    );
    assert_eq!(card(&shell).detail, "Up to B3");
    assert!(card(&shell).edited);

    // The lowest key can't pass the highest.
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyLow,
        value: 70,
    });
    assert_eq!(card(&shell).detail, "B3");
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyLow,
        value: 48,
    });
    assert_eq!(card(&shell).detail, "C3–B3");

    // The keyboard's lowest key means every key below it too.
    shell.sent.clear();
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyLow,
        value: 36,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::sound_bytes(1, &BTreeMap::from([((0, 6), 0)]))
        )]
    );
    assert_eq!(card(&shell).detail, "Up to B3");

    // Fading in over the six keys below.
    shell.sent.clear();
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyFadeLow,
        value: 6,
    });
    assert_eq!(
        shell.sent,
        [(
            "go".to_string(),
            go_keys::sound_bytes(1, &BTreeMap::from([((0, 8), 6)]))
        )]
    );
    let range = layers(&shell)
        .groups
        .into_iter()
        .find_map(|group| group.keys)
        .unwrap();
    assert_eq!(range.fade_low, 6);

    // Only when played hard.
    shell.send(Event::SetLayerControl {
        control: LayerControl::VelocityLow,
        value: 122,
    });
    assert_eq!(card(&shell).detail, "Up to B3 · Velocity 122–127");

    // Reset puts its ranges back too.
    shell.send(Event::ResetLayers(Some(0)));
    assert_eq!(card(&shell).detail, "All keys");
    assert!(!layers(&shell).edited);
}

#[test]
fn a_new_sound_forgets_layer_changes_and_is_read_again() {
    let mut shell = layers_read();
    shell.send(Event::SetLayerControl {
        control: LayerControl::Cutoff,
        value: 110,
    });
    shell.sent.clear();
    shell.send(Event::Next);
    let bytes = &shell.sent[0].1;
    assert!(bytes.ends_with(&go_keys::read_sound(1)), "read again");
    assert_eq!(
        layers(&shell).status.as_deref(),
        Some("Reading the sound from the keyboard…")
    );
}

#[test]
fn presets_and_relaunches_keep_layer_changes() {
    let mut shell = layers_read();
    shell.send(Event::SetLayerControl {
        control: LayerControl::Cutoff,
        value: 110,
    });
    shell.send(Event::SavePreset {
        name: "Bright".into(),
    });
    let id = shell.view().loaded.unwrap().id;
    let change = go_keys::sound_bytes(1, &BTreeMap::from([((1, 0x49), 110)]));

    // Another sound, then the preset: its sound, then the change.
    shell.send(Event::Next);
    shell.sent.clear();
    shell.send(Event::RecallPreset(id));
    let bytes = &shell.sent[0].1;
    assert!(bytes.windows(change.len()).any(|w| w == change));

    // A relaunch: sent again when the keyboard connects.
    let mut relaunched = Shell::with_settings(shell.saved.clone());
    relaunched.connect_keyboard();
    let bytes = &relaunched.sent[0].1;
    assert!(bytes.windows(change.len()).any(|w| w == change));
}

#[test]
fn the_layers_tab_says_why_it_cant_edit() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::ToggleEditor);
    shell.send(Event::ShowTab(Tab::Layers));
    assert_eq!(
        layers(&shell).status.as_deref(),
        Some("Connect your keyboard to edit this sound's layers")
    );
    shell.send(Event::Open(Place::Category {
        bank: Bank::Gm2Drums,
        category: Category::Drums,
    }));
    let kit = shell.listed()[0];
    shell.send(Event::Select(kit));
    assert_eq!(
        layers(&shell).status.as_deref(),
        Some("Drum kits have no layers to edit here")
    );
}

#[test]
fn a_preset_recalled_with_seamless_switching_goes_to_the_idle_part() {
    let mut shell = seamless();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano)); // now on part 5
    shell.send(Event::SavePreset { name: "P".into() });
    shell.sent.clear();
    shell.send(Event::RecallPreset(shell.view().loaded.unwrap().id));
    let bytes = &shell.sent[0].1;
    assert!(
        bytes.starts_with(&go_keys::part_patch(1, &patch("JUNO Piano 1"))),
        "part 1, not part 5 that's sounding"
    );
    assert!(contains(bytes, &go_keys::zone(5, Zone::OFF)));
}

#[test]
fn a_preset_recalled_ends_a_solo() {
    let mut shell = layers_read();
    shell.send(Event::SoloLayer(Some(1)));
    shell.send(Event::SavePreset { name: "P".into() });
    shell.send(Event::RecallPreset(shell.view().loaded.unwrap().id));
    shell.receive(sound_replies(1));
    assert!(layers(&shell).cards.iter().all(|card| !card.soloed));
    shell.sent.clear();
    shell.send(Event::ShowTab(Tab::Sound));
    assert_eq!(shell.sent, [], "no layer switches written off");
}

#[test]
fn a_new_part_reads_its_own_sound_even_mid_read() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::ToggleEditor);
    shell.send(Event::ShowTab(Tab::Layers)); // part 1 asked for, no reply yet
    shell.sent.clear();
    shell.send(Event::AddPart);
    assert!(shell.sent[0].1.ends_with(&go_keys::read_sound(2)));
}

#[test]
fn a_keyboard_that_connects_again_is_asked_for_the_sound_again() {
    let mut shell = Shell::with_settings(None);
    shell.connect_keyboard();
    shell.send(Event::ToggleEditor);
    shell.send(Event::ShowTab(Tab::Layers)); // asked for, no reply
    shell.sent.clear();
    shell.connect_keyboard();
    let sent: Vec<u8> = shell.sent.iter().flat_map(|(_, b)| b.clone()).collect();
    assert!(sent.ends_with(&go_keys::read_sound(1)));
}

#[test]
fn a_sound_picked_while_settings_load_drops_the_kept_sound_s_layers() {
    let mut shell = Shell::start();
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    let mut load = shell.load.take().unwrap();
    let settings = Settings {
        setup: Setup {
            parts: vec![PartSetup {
                sound: Some("87.64.1".into()),
                layers: vec![shared::settings::LayerEdit {
                    block: 1,
                    index: 0x49,
                    value: 110,
                    was: 40,
                }],
                ..PartSetup::default()
            }],
            ..Setup::default()
        },
        ..Settings::default()
    };
    let effects = shell
        .core
        .resolve(&mut load, StoredSettings(Some(settings.to_json())))
        .unwrap();
    shell.handle(effects);
    assert_eq!(shell.view().current.unwrap().name, "JUNO Piano 1");
    shell.send(Event::SetControl {
        control: Control::Level,
        value: 90,
    });
    let saved = shell.saved.clone().unwrap();
    assert_eq!(saved.setup.parts[0].layers, []);
}

#[test]
fn a_keyboard_that_connects_gets_every_setting_even_back_at_default() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::SetControl {
        control: Control::ReverbSwitch,
        value: 0,
    });
    shell.send(Event::ResetControl(Control::ReverbSwitch));
    shell.connect_keyboard();
    let bytes = &shell.sent[0].1;
    for setting in [
        go_keys::setting(Scope::Keyboard, Control::ReverbSwitch, 1),
        go_keys::setting(Scope::Part(1), Control::Level, 100),
    ] {
        assert!(contains(bytes, &setting), "{setting:02X?}");
    }
}

#[test]
fn a_part_setting_kept_as_the_keyboard_s_is_dropped() {
    let settings = Settings::from_json(
        r#"{"system_edits":[{"control":"Level","value":80},{"control":"ReverbLevel","value":90}]}"#,
    );
    let mut shell = Shell::with_settings(settings);
    shell.connect_keyboard(); // would panic writing Level to the keyboard
    shell.send(Event::ToggleEditor);
    let editor = shell.view().editor.unwrap();
    assert_eq!(editor.keyboard[0].controls[1].text, "90");
    assert_eq!(editor.groups[0].controls[0].text, "100");
}

#[test]
fn removing_an_earlier_part_keeps_editing_the_same_one() {
    let mut shell = Shell::with_settings(None);
    let piano = shell.id_of("JUNO Piano 1");
    shell.send(Event::Select(piano));
    shell.send(Event::AddPart); // part 2, edited
    let grand = shell.id_of("88StageGrand");
    shell.send(Event::Select(grand));
    shell.send(Event::AddPart); // part 3, edited
    shell.send(Event::EditPart(1));
    shell.send(Event::RemovePart(0));
    let zones = shell.view().layout.zones;
    let editing = zones.iter().find(|z| z.editing).unwrap();
    assert_eq!(editing.sound, "88StageGrand");
    assert_eq!(editing.label, "1", "now part 1");
}

#[test]
fn a_solo_of_a_layer_that_isn_t_there_does_nothing() {
    let mut shell = layers_read();
    shell.send(Event::SoloLayer(Some(7)));
    assert_eq!(shell.sent, []);
}

#[test]
fn a_part_of_one_key_can_t_be_split() {
    let mut shell = Shell::with_settings(None);
    shell.send(Event::SetKeys {
        part: 0,
        low: 60,
        high: 60,
    });
    assert!(!shell.view().layout.can_add);
    shell.send(Event::AddPart);
    assert_eq!(shell.view().layout.zones.len(), 1);
}

#[test]
fn the_zone_map_and_displays_say_what_they_can_do() {
    let mut shell = Shell::with_settings(None);
    let layout = shell.view().layout;
    assert_eq!(layout.zones[0].title, "Part 1");
    assert_eq!(layout.zones[0].remove, None, "the only part");
    assert_eq!(
        layout
            .ticks
            .iter()
            .map(|t| t.label.as_str())
            .collect::<Vec<_>>(),
        ["C2", "C3", "C4", "C5", "C6", "C7"]
    );
    shell.send(Event::AddPart);
    let view = shell.view();
    assert_eq!(
        view.layout.zones[1].remove.as_deref(),
        Some("Remove part 2")
    );
    assert_eq!(view.displays[1].remove.as_deref(), Some("Remove part 2"));
}

#[test]
fn a_layer_toggles_and_its_keys_and_the_tab_reset() {
    let mut shell = layers_read();
    shell.send(Event::ToggleLayer(2));
    let view = layers(&shell);
    assert!(view.cards[2].selected && !view.cards[2].on);
    shell.send(Event::ToggleLayer(2));
    assert!(layers(&shell).cards[2].on);

    // Both ends back at once, though the low end can't pass the high one.
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyHigh,
        value: 50,
    });
    shell.send(Event::SetLayerControl {
        control: LayerControl::KeyLow,
        value: 50,
    });
    shell.send(Event::ResetLayerKeys);
    assert_eq!(layers(&shell).cards[2].detail, "All keys");

    // Reset puts back what the tab showing changes.
    shell.send(Event::SetLayerControl {
        control: LayerControl::Cutoff,
        value: 110,
    });
    assert!(shell.view().editor.unwrap().edited);
    shell.send(Event::ResetEditor);
    assert!(!shell.view().editor.unwrap().edited);
    shell.send(Event::ShowTab(Tab::Sound));
    shell.send(Event::SetControl {
        control: Control::Pan,
        value: 30,
    });
    shell.send(Event::ResetEditor);
    assert!(!shell.view().editor.unwrap().edited);
}
