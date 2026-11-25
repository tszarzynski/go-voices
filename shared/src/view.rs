//! The view model: what every shell draws, as values.
//!
//! The core gives the state: names, numbers, keys and settings with their
//! values written out (`C4`, `L20`, `+3`). Shells word their own chrome:
//! buttons, hints, prompts and errors.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use catalogue::Patch;
use crux_midi::{
    LinkState,
    effects::{MidiDevice, MidiError, Transport},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    app::{CHANNELS, Model, Place},
    go_keys,
    layer::{self, LayerControl},
    layers::{self, Tab},
    part::{Control, Group},
    parts,
    settings::{self, PartSetup, Preset, Setup},
};

/// Everything a shell draws.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewModel {
    /// The keyboard the sounds go to.
    pub connection: ConnectionView,
    /// The top of the tree: All sounds, Favourites, Recent.
    pub places: Vec<PlaceView>,
    /// The rest of the tree: each bank, and its categories.
    pub banks: Vec<BankNode>,
    /// Where the list is, e.g. `JUNO-DS › Organ`.
    pub place_title: String,
    /// What's typed in the search field.
    pub query: String,
    /// The search field's placeholder, e.g. `Search Organ`.
    pub search_prompt: String,
    /// The list, in sections.
    pub sections: Vec<SectionView>,
    /// In Presets, the list: the presets instead of sounds.
    pub presets: Option<Vec<PresetView>>,
    /// What to call a new preset, to start from: its sounds' names.
    pub preset_name: String,
    /// The preset recalled or saved last, which Save updates.
    pub loaded: Option<LoadedView>,
    /// How many are listed, e.g. `1,383 sounds`, `12 matches`.
    pub count: String,
    /// What to say when the list is empty.
    pub empty: Option<String>,
    /// The selected part's sound, if it has one: where the next pick goes.
    pub current: Option<CurrentView>,
    /// The parts on the zone map.
    pub layout: LayoutView,
    /// The Edit panel, while it's showing beside the sound list.
    pub editor: Option<EditorView>,
    /// Settings shared by every part, while the keyboard sheet is open.
    pub keyboard: Option<Vec<GroupView>>,
    /// The wave picker, while it's showing.
    pub waves: Option<WavePickerView>,
    /// The MIDI channel sounds go on.
    pub channel: u8,
    /// For `SetChannel`: every channel.
    pub channels: Vec<u8>,
    /// Seamless switching is on (`SetSeamless`).
    pub seamless: bool,
    /// Listed devices, then [`BluetoothView::found`] ones being connected.
    pub devices: Vec<DeviceView>,
    /// Pairing or scanning for Bluetooth keyboards.
    pub bluetooth: BluetoothView,
    /// Why the last connection or scan failed, until `DismissStatus` or
    /// the next attempt.
    pub status: Option<MidiError>,
}

/// Where a device's connection is.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum DeviceState {
    /// Not connected.
    #[default]
    Disconnected,
    /// Connecting, or pairing.
    Connecting,
    /// Connected: sounds go to it.
    Connected,
}

/// The connection badge in the toolbar.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectionView {
    /// The best state of any device.
    pub state: DeviceState,
    /// The connected keyboards' names, `Connecting…` or `No keyboard`.
    pub label: String,
}

/// One place in the tree.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PlaceView {
    /// For `Open`.
    pub place: Place,
    /// `All sounds`, `JUNO-DS`, `Acoustic piano` …
    pub label: String,
    /// How many patches it holds, `1,383`.
    pub count: String,
    /// Whether the list shows it.
    pub selected: bool,
}

/// A bank in the tree, with its categories under it.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BankNode {
    /// The bank itself.
    pub bank: PlaceView,
    /// Its categories, in the keyboard's order.
    pub categories: Vec<PlaceView>,
    /// Whether the place the list shows is in this bank, so a shell can
    /// show it open.
    pub holds_selection: bool,
}

/// A run of the list under one heading.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SectionView {
    /// The category's name, or `Best matches`, `Recent`.
    pub title: String,
    /// The patches.
    pub rows: Vec<RowView>,
}

/// One patch in the list.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RowView {
    /// For `Select` and `ToggleFavourite`. Stable while the app runs.
    pub id: u32,
    /// The patch's name.
    pub name: String,
    /// Its address, `87·64·004`.
    pub address: String,
    /// Its category's name, `Acoustic piano`, where the list isn't
    /// grouped by category (search results, Recent); otherwise empty.
    pub category: String,
    /// Starred.
    pub favourite: bool,
}

/// The selected part's sound: marked in the list, starred on its lane.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CurrentView {
    /// For `ToggleFavourite`, and to find it in the list.
    pub id: u32,
    /// Starred.
    pub favourite: bool,
}

/// One preset in the list.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PresetView {
    /// For the preset events.
    pub id: u32,
    /// What the user called it.
    pub name: String,
    /// What it plays: `Split C4 · Fretless / JUNO Piano 1`,
    /// `3 parts · …`.
    pub detail: String,
    /// The one loaded: recalled or saved last.
    pub current: bool,
    /// Loaded, and what's playing has changed since.
    pub edited: bool,
    /// Its parts' keys, in part order, for a small zone map.
    pub zones: Vec<KeysView>,
}

/// A part's keys, for a small zone map.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct KeysView {
    /// Its lowest key (0: every key down).
    pub low: u8,
    /// Its highest key (127: every key up).
    pub high: u8,
}

/// The preset recalled or saved last.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LoadedView {
    /// For `UpdatePreset`.
    pub id: u32,
    /// Its name.
    pub name: String,
    /// What's playing has changed since: there's something to save.
    pub edited: bool,
}

/// The zone map: the parts and their keys.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutView {
    /// One lane per part, in order.
    pub zones: Vec<ZoneView>,
    /// The keyboard's first key, to draw.
    pub first: u8,
    /// Its last key.
    pub last: u8,
    /// `AddPart` can add one.
    pub can_add: bool,
    /// The octaves marked under the map: each C's key and name.
    pub ticks: Vec<NoteTick>,
}

/// One part on the zone map.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ZoneView {
    /// For `EditPart`, `RemovePart` and `SetKeys`, from 0.
    pub part: u8,
    /// `Part 1`.
    pub title: String,
    /// `Remove part 1`, if it can be taken away (not the only one).
    pub remove: Option<String>,
    /// Its sound's name, or `—`.
    pub sound: String,
    /// Its sound's category, `Acoustic piano`, or empty.
    pub category: String,
    /// Its lowest key (0: every key down).
    pub low: u8,
    /// Its highest key (127: every key up).
    pub high: u8,
    /// `Up to B3`, `All keys`.
    pub keys: String,
    /// The part being edited.
    pub editing: bool,
}

/// A key marked under the zone map.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct NoteTick {
    /// The MIDI note.
    pub note: u8,
    /// `C4`.
    pub label: String,
}

/// The Edit panel: the settings of the part being edited.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EditorView {
    /// The tab showing, what it edits: `Sound modify · Part 2 · Fretless`,
    /// `Layers · Fretless`.
    pub title: String,
    /// Mix, Play … for the part being edited.
    pub groups: Vec<GroupView>,
    /// Anything in the tab showing to reset (`ResetEditor`).
    pub edited: bool,
    /// For `ShowTab`: Sound, Layers.
    pub tabs: Vec<TabView>,
    /// In the Layers tab, the sound's layers.
    pub layers: Option<LayersView>,
}

/// One tab of the Edit panel.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TabView {
    /// For `ShowTab`.
    pub tab: Tab,
    /// Showing.
    pub selected: bool,
}

/// The Layers tab: a sound's four layers, and the selected one's settings.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LayersView {
    /// Why there's nothing to edit yet.
    pub status: Option<LayersStatus>,
    /// The layers, for `SelectLayer` and `SoloLayer`. Empty until read.
    pub cards: Vec<LayerCardView>,
    /// The selected layer's wave.
    pub wave: Option<WaveView>,
    /// The selected layer's settings, in groups.
    pub groups: Vec<GroupView>,
    /// The layer `PasteLayer` would paste.
    pub paste: Option<PasteView>,
    /// The keyboard's first key, to draw layers' keys on.
    pub first: u8,
    /// Its last key.
    pub last: u8,
}

/// Why the Layers tab has nothing to edit yet.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum LayersStatus {
    /// A drum kit: no layers to edit here.
    Drums,
    /// No keyboard to read the sound from.
    NotConnected,
    /// Reading the sound from the keyboard.
    Reading,
}

/// A layer copied, to paste onto another.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PasteView {
    /// Its layer, from 0.
    pub layer: u8,
    /// Its sound's name, if it had one.
    pub sound: Option<String>,
}

/// One layer, in the row of layers.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent things a card shows"
)]
pub struct LayerCardView {
    /// 0–3, for `SelectLayer` and `SoloLayer`.
    pub layer: u8,
    /// `Layer 1`.
    pub label: String,
    /// Its wave's name.
    pub wave: String,
    /// It plays.
    pub on: bool,
    /// Its level, `100`.
    pub level: String,
    /// The one being edited.
    pub selected: bool,
    /// Playing alone.
    pub soloed: bool,
    /// Changed from the sound's.
    pub edited: bool,
    /// The lowest key it plays (a MIDI note, may be past the keyboard's
    /// end).
    pub low: u8,
    /// The highest key it plays.
    pub high: u8,
    /// Which keys, and how hard it must be played if not at any velocity:
    /// `Up to B3 · Velocity 122–127`, `All keys`.
    pub detail: String,
}

/// The selected layer's keys, to drag on a keyboard: `SetLayerControl`
/// with `KeyLow` or `KeyHigh`.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct KeyRangeView {
    /// The lowest key.
    pub low: ControlView,
    /// The highest key.
    pub high: ControlView,
    /// Keys below the lowest it fades in over.
    pub fade_low: u8,
    /// Keys above the highest it fades out over.
    pub fade_high: u8,
    /// `Up to B3`.
    pub text: String,
}

/// The selected layer's wave.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WaveView {
    /// `Fng.EB1 mf A`.
    pub name: String,
    /// `Group 1 · 291`, `Group 1 · 1–2 stereo`.
    pub detail: String,
}

/// The wave picker: every wave a layer can have, as searched.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WavePickerView {
    /// What's typed.
    pub query: String,
    /// `Search 1,045 waves`.
    pub prompt: String,
    /// The waves listed, for `SetWave`.
    pub rows: Vec<WaveRowView>,
    /// What to say when none match.
    pub empty: Option<String>,
}

/// One wave in the picker.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WaveRowView {
    /// For `SetWave`.
    pub group: u16,
    /// For `SetWave`.
    pub number: u16,
    /// `Fng.EB1 mf A`.
    pub name: String,
    /// `1 · 291`.
    pub detail: String,
    /// The selected layer's.
    pub current: bool,
}

/// A point of an envelope's shape: 0–1000 across and up.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointView {
    /// Across: time.
    pub x: u16,
    /// Up: level.
    pub y: u16,
}

/// A row of settings under a heading.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GroupView {
    /// `Mix`, `Play`.
    pub title: String,
    /// What the settings do, where the title doesn't say. May be empty.
    pub note: String,
    /// The settings.
    pub controls: Vec<ControlView>,
    /// For an envelope, its shape. Empty otherwise.
    pub envelope: Vec<PointView>,
    /// For a layer's keys, the range to drag (its controls are the rest).
    pub keys: Option<KeyRangeView>,
}

/// Which setting a control changes.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Setting {
    /// A part's or the keyboard's: `SetControl`, `ResetControl`.
    Part(Control),
    /// The selected layer's: `SetLayerControl`, `ResetLayerControl`.
    Layer(LayerControl),
}

/// One setting: a knob, or with `choices`, a choice.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ControlView {
    /// What it changes.
    pub setting: Setting,
    /// `Level`, `Pan`.
    pub label: String,
    /// The value in words: `100`, `L20`, `Sound`.
    pub text: String,
    /// The value.
    pub value: i16,
    /// The lowest value.
    pub min: i16,
    /// The highest value.
    pub max: i16,
    /// Changed from the keyboard's own.
    pub edited: bool,
    /// For a choice: its options, in order. Empty for a knob.
    pub choices: Vec<ChoiceView>,
}

/// One option of a choice.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChoiceView {
    /// For `SetControl`.
    pub value: i16,
    /// `Sound`, `Poly`, `Mono`.
    pub label: String,
}

/// One row of the device list.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeviceView {
    /// For `ToggleDevice`.
    pub id: String,
    /// What to call it.
    pub name: String,
    /// How it's attached.
    pub transport: Transport,
    /// Where its connection is: `ToggleDevice` connects, cancels or
    /// disconnects.
    pub state: DeviceState,
}

/// Finding Bluetooth keyboards.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct BluetoothView {
    /// Whether a scan is running: `StopBluetoothScan` stops it, and
    /// `StartBluetoothScan` scans or, where the system pairs, opens that.
    pub scanning: bool,
    /// Devices found by the scan that aren't listed (or being connected).
    pub found: Vec<DeviceView>,
}

/// What the shells draw, from the model.
#[must_use]
pub fn view_model(model: &Model) -> ViewModel {
    let (devices, found) = device_views(model);
    let filters = &model.filters;
    let total = model.catalogue.patches().len();
    let loaded = model.loaded();
    let presets = (filters.place == Place::Presets).then(|| preset_views(model, loaded));
    let listed = presets.as_ref().map_or(model.results.len(), Vec::len);
    let noun = if presets.is_some() { "preset" } else { "sound" };
    ViewModel {
        connection: connection_view(&devices),
        places: [
            (Place::All, total),
            (Place::Favourites, model.favourites.len()),
            (Place::Recent, model.recents.len()),
            (Place::Presets, model.presets.len()),
        ]
        .into_iter()
        .map(|(place, count)| place_view(model, place, count))
        .collect(),
        banks: bank_nodes(model),
        place_title: place_title(filters.place),
        query: filters.query.clone(),
        search_prompt: format!("Search {}", search_scope(filters.place)),
        sections: sections(model),
        presets,
        preset_name: preset_name(model),
        loaded: loaded.map(|(preset, edited)| LoadedView {
            id: preset.id,
            name: preset.name.clone(),
            edited,
        }),
        count: count(filters.searching(), listed, noun),
        empty: (listed == 0).then(|| empty_text(model)),
        current: current_view(model),
        layout: layout_view(model),
        editor: model.editor.then(|| editor_view(model)),
        keyboard: model
            .keyboard_sheet
            .then(|| groups(model, &Control::KEYBOARD_GROUPS)),
        waves: wave_picker(model),
        channel: model.channel,
        channels: CHANNELS.collect(),
        seamless: model.seamless,
        devices,
        bluetooth: bluetooth_view(model, found),
        status: model.midi.status().cloned(),
    }
}

/// The selected part's sound.
fn current_view(model: &Model) -> Option<CurrentView> {
    let id = model.picked(model.editing())?;
    model.patch(id)?;
    Some(CurrentView {
        id: row_id(id),
        favourite: model.favourites.contains(&id),
    })
}

/// `1 sound`, `1,383 sounds`, `3 presets`, `12 matches`.
fn count(searching: bool, listed: usize, noun: &str) -> String {
    match (searching, listed) {
        (true, 1) => "1 match".into(),
        (true, n) => format!("{} matches", thousands(n)),
        (false, 1) => format!("1 {noun}"),
        (false, n) => format!("{} {noun}s", thousands(n)),
    }
}

/// The presets the list shows.
fn preset_views(model: &Model, loaded: Option<(&Preset, bool)>) -> Vec<PresetView> {
    let loaded = loaded.map(|(preset, edited)| (preset.id, edited));
    model
        .listed_presets()
        .map(|preset| {
            let current = loaded.is_some_and(|(id, _)| id == preset.id);
            PresetView {
                id: preset.id,
                name: preset.name.clone(),
                detail: describe(model, &preset.setup),
                current,
                edited: current && loaded.is_some_and(|(_, edited)| edited),
                zones: preset
                    .setup
                    .parts
                    .iter()
                    .map(|part| KeysView {
                        low: part.low,
                        high: part.high,
                    })
                    .collect(),
            }
        })
        .collect()
}

/// What a setup plays: `Single · JUNO Piano 1`, `Split C4 · Fretless /
/// JUNO Piano 1`, `Layer · …`, `3 parts · …` (lowest keys first).
fn describe(model: &Model, setup: &Setup) -> String {
    let mut parts: Vec<&PartSetup> = setup.parts.iter().collect();
    parts.sort_by_key(|part| (part.low, part.high));
    let names = parts
        .iter()
        .map(|part| {
            part.sound
                .as_deref()
                .and_then(|key| settings::patch(model, key))
                .map_or("—", |patch| patch.name.as_str())
        })
        .collect::<Vec<_>>()
        .join(" / ");
    let full = |part: &&PartSetup| part.low == 0 && part.high == 127;
    match parts.as_slice() {
        [_] => format!("Single · {names}"),
        [lower, upper] if u16::from(lower.high) + 1 == u16::from(upper.low) => {
            format!("Split {} · {names}", go_keys::note_name(upper.low))
        }
        [a, b] if full(a) && full(b) => format!("Layer · {names}"),
        _ => format!("{} parts · {names}", parts.len()),
    }
}

/// The parts' indexes, lowest keys first, as on the keyboard.
fn by_keys(model: &Model) -> Vec<usize> {
    let mut order: Vec<usize> = (0..model.parts.len()).collect();
    order.sort_by_key(|&index| {
        let part = &model.parts[index];
        (part.low, part.high, index)
    });
    order
}

/// What Save new suggests calling a preset: its sounds, lowest keys
/// first.
pub fn preset_name(model: &Model) -> String {
    let names: Option<Vec<String>> = by_keys(model)
        .into_iter()
        .map(|index| model.sound(index).map(|patch| patch.name.clone()))
        .collect();
    names.map_or_else(
        || format!("Preset {}", model.presets.len() + 1),
        |names| names.join(" / "),
    )
}

fn control_view(model: &Model, control: Control) -> ControlView {
    let value = model.value(control);
    let (min, max) = control.range();
    ControlView {
        setting: Setting::Part(control),
        label: control.label().into(),
        text: control.text(value),
        value,
        min,
        max,
        edited: value != model.default_value(control),
        choices: choices(control.choices()),
    }
}

fn choices(choices: Vec<(i16, &'static str)>) -> Vec<ChoiceView> {
    choices
        .into_iter()
        .map(|(value, label)| ChoiceView {
            value,
            label: label.into(),
        })
        .collect()
}

/// A setting of the selected layer.
fn layer_control_view(model: &Model, control: LayerControl) -> ControlView {
    let value = layers::value(model, control).unwrap_or_default();
    let (min, max) = control.range();
    ControlView {
        setting: Setting::Layer(control),
        label: control.label().into(),
        text: control.text(value),
        value,
        min,
        max,
        edited: layers::original(model, control).is_some_and(|original| original != value),
        choices: choices(control.choices()),
    }
}

/// The selected layer's settings, in groups.
fn layer_groups(model: &Model) -> Vec<GroupView> {
    let sounds = &model.edited().layers;
    let byte = |place| sounds.byte_or_zero(place);
    let key =
        |control: LayerControl| u8::try_from(control.value(model.layer, byte)).unwrap_or_default();
    layer::GROUPS
        .iter()
        .map(|&(title, controls)| GroupView {
            title: title.into(),
            note: String::new(),
            // The keys' ends are dragged on the keyboard, not turned.
            controls: controls
                .iter()
                .filter(|&&control| {
                    !matches!(control, LayerControl::KeyLow | LayerControl::KeyHigh)
                })
                .map(|&control| layer_control_view(model, control))
                .collect(),
            keys: controls
                .contains(&LayerControl::KeyLow)
                .then(|| KeyRangeView {
                    low: layer_control_view(model, LayerControl::KeyLow),
                    high: layer_control_view(model, LayerControl::KeyHigh),
                    fade_low: key(LayerControl::KeyFadeLow),
                    fade_high: key(LayerControl::KeyFadeHigh),
                    text: go_keys::keys_text(key(LayerControl::KeyLow), key(LayerControl::KeyHigh)),
                }),
            // The envelopes' groups, by their attack: drawn as a shape.
            envelope: [LayerControl::FilterAttack, LayerControl::AmpAttack]
                .into_iter()
                .find(|attack| controls.contains(attack))
                .map(|attack| {
                    layer::envelope(model.layer, attack == LayerControl::AmpAttack, byte)
                        .into_iter()
                        .map(|(x, y)| PointView { x, y })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

/// The Layers tab.
fn layers_view(model: &Model) -> LayersView {
    let sounds = &model.edited().layers;
    let status = if layers::drums(model) {
        Some(LayersStatus::Drums)
    } else if sounds.ready() {
        None
    } else if model.midi.connected().next().is_none() {
        Some(LayersStatus::NotConnected)
    } else {
        Some(LayersStatus::Reading)
    };
    if status.is_some() {
        return LayersView {
            status,
            cards: Vec::new(),
            wave: None,
            groups: Vec::new(),
            paste: None,
            first: go_keys::FIRST,
            last: go_keys::LAST,
        };
    }
    let byte = |place| sounds.byte_or_zero(place);
    let key =
        |control: LayerControl, layer| u8::try_from(control.value(layer, byte)).unwrap_or_default();
    let cards = (0..layer::LAYERS)
        .map(|layer| LayerCardView {
            layer,
            label: format!("Layer {}", layer + 1),
            wave: layers::wave_name(model, layer),
            on: LayerControl::Switch.value(layer, byte) == 1,
            level: LayerControl::Level.value(layer, byte).to_string(),
            selected: layer == model.layer,
            soloed: model.solo == Some(layer),
            edited: sounds.edits.keys().any(|&place| layer::owns(layer, place)),
            low: key(LayerControl::KeyLow, layer),
            high: key(LayerControl::KeyHigh, layer),
            detail: [
                go_keys::keys_text(
                    key(LayerControl::KeyLow, layer),
                    key(LayerControl::KeyHigh, layer),
                ),
                layer::velocity_text(
                    key(LayerControl::VelocityLow, layer),
                    key(LayerControl::VelocityHigh, layer),
                ),
            ]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        })
        .collect();
    let wave = layers::wave(model).map(|(group, number)| {
        let stereo = model
            .waves
            .get(group, number)
            .and_then(|wave| model.waves.right_of(wave));
        WaveView {
            name: layers::wave_name(model, model.layer),
            detail: stereo.map_or_else(
                || format!("Group {group} · {number}"),
                |right| format!("Group {group} · {number}–{} stereo", right.number),
            ),
        }
    });
    let groups = layer_groups(model);
    LayersView {
        status: None,
        cards,
        wave,
        groups,
        paste: model.copied.as_ref().map(|copied| PasteView {
            layer: copied.layer,
            sound: copied.sound.clone(),
        }),
        first: go_keys::FIRST,
        last: go_keys::LAST,
    }
}

/// The wave picker, while it's showing.
fn wave_picker(model: &Model) -> Option<WavePickerView> {
    let query = model.wave_query.as_deref()?;
    let current = layers::wave(model);
    let rows: Vec<WaveRowView> = layers::listed_waves(model, query)
        .map(|wave| WaveRowView {
            group: wave.group,
            number: wave.number,
            name: wave.shown_name().into(),
            detail: format!("{} · {}", wave.group, wave.number),
            current: current == Some((wave.group, wave.number)),
        })
        .collect();
    Some(WavePickerView {
        query: query.into(),
        prompt: format!(
            "Search {} waves",
            thousands(layers::choosable(model).count())
        ),
        empty: rows
            .is_empty()
            .then(|| format!("No waves match “{}”", query.trim())),
        rows,
    })
}

fn editor_view(model: &Model) -> EditorView {
    let sound = model
        .sound(model.editing())
        .map_or("the keyboard's sound", |patch| patch.name.as_str());
    let what = if model.tab == Tab::Layers {
        "Layers"
    } else {
        "Sound modify"
    };
    let title = if model.parts.len() == 1 {
        format!("{what} · {sound}")
    } else {
        format!("{what} · {} · {sound}", part_label(model.editing()))
    };
    let layers = (model.tab == Tab::Layers).then(|| layers_view(model));
    EditorView {
        title,
        groups: if layers.is_some() {
            Vec::new()
        } else {
            groups(model, &Control::GROUPS)
        },
        edited: if layers.is_some() {
            !model.edited().layers.edits.is_empty()
        } else {
            Control::part_controls()
                .any(|control| model.value(control) != model.default_value(control))
        },
        tabs: [Tab::Sound, Tab::Layers]
            .into_iter()
            .map(|tab| TabView {
                tab,
                selected: model.tab == tab,
            })
            .collect(),
        layers,
    }
}

/// Settings in their groups.
fn groups(model: &Model, groups: &[Group]) -> Vec<GroupView> {
    groups
        .iter()
        .map(|group| GroupView {
            title: group.title.into(),
            note: group.note.into(),
            controls: group
                .controls
                .iter()
                .map(|&control| control_view(model, control))
                .collect(),
            envelope: Vec::new(),
            keys: None,
        })
        .collect()
}

/// `Part 2`.
fn part_label(index: usize) -> String {
    format!("Part {}", parts::number(index))
}

/// `Remove part 2`, unless it's the only part.
fn remove_label(model: &Model, index: usize) -> Option<String> {
    (model.parts.len() > 1).then(|| format!("Remove part {}", parts::number(index)))
}

fn layout_view(model: &Model) -> LayoutView {
    let editing = model.editing();
    LayoutView {
        zones: model
            .parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                let sound = model.sound(index);
                ZoneView {
                    part: u8::try_from(index).unwrap_or_default(),
                    title: part_label(index),
                    remove: remove_label(model, index),
                    sound: sound.map_or_else(|| "—".into(), |patch| patch.name.clone()),
                    category: sound
                        .map(|patch| patch.category.name().into())
                        .unwrap_or_default(),
                    low: part.low,
                    high: part.high,
                    keys: go_keys::keys_text(part.low, part.high),
                    editing: index == editing,
                }
            })
            .collect(),
        first: go_keys::FIRST,
        last: go_keys::LAST,
        can_add: parts::can_add(model),
        ticks: ticks(),
    }
}

/// The octaves marked on a drawn keyboard: each C's key and name.
fn ticks() -> Vec<NoteTick> {
    go_keys::KEYS
        .filter(|note| note % 12 == 0)
        .map(|note| NoteTick {
            note,
            label: go_keys::note_name(note),
        })
        .collect()
}

/// `1383` as `1,383`.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Patch ids fit easily: the catalogue has fewer than 2,000.
#[expect(clippy::cast_possible_truncation, reason = "ids are catalogue indices")]
const fn row_id(id: usize) -> u32 {
    id as u32
}

fn place_view(model: &Model, place: Place, count: usize) -> PlaceView {
    PlaceView {
        place,
        label: place_label(place).into(),
        count: thousands(count),
        selected: model.filters.place == place,
    }
}

fn bank_nodes(model: &Model) -> Vec<BankNode> {
    model
        .tree
        .iter()
        .map(|&(bank, count, ref categories)| BankNode {
            bank: place_view(model, Place::Bank(bank), count),
            categories: categories
                .iter()
                .map(|&(category, count)| {
                    place_view(model, Place::Category { bank, category }, count)
                })
                .collect(),
            holds_selection: model.filters.place.bank() == Some(bank),
        })
        .collect()
}

/// A place's name in the tree.
const fn place_label(place: Place) -> &'static str {
    match place {
        Place::All => "All sounds",
        Place::Favourites => "Favourites",
        Place::Recent => "Recent",
        Place::Presets => "Presets",
        Place::Bank(bank) => bank.name(),
        Place::Category { category, .. } => category.name(),
    }
}

/// Where the list is: the place, with its bank in front for a category.
fn place_title(place: Place) -> String {
    match place {
        Place::Category { bank, category } => format!("{} › {}", bank.name(), category.name()),
        _ => place_label(place).into(),
    }
}

/// What the search looks through, after `Search`.
const fn search_scope(place: Place) -> &'static str {
    match place {
        Place::All => "all sounds",
        Place::Favourites => "favourites",
        Place::Recent => "recent sounds",
        Place::Presets => "presets",
        Place::Bank(bank) => bank.name(),
        Place::Category { category, .. } => category.name(),
    }
}

/// Sections by category while browsing; one section while searching (the
/// best matches first) or for the recents (the latest first), or without a
/// title for one category.
fn sections(model: &Model) -> Vec<SectionView> {
    let recent = model.filters.place == Place::Recent;
    let ungrouped = recent || model.filters.searching();
    let row = |id: usize| {
        model.patch(id).map(|patch: &Patch| RowView {
            id: row_id(id),
            name: patch.name.clone(),
            address: patch.address(),
            category: if ungrouped {
                patch.category.name().into()
            } else {
                String::new()
            },
            favourite: model.favourites.contains(&id),
        })
    };
    let rows = model.results.iter().filter_map(|&id| row(id));
    if ungrouped {
        let rows: Vec<RowView> = rows.collect();
        if rows.is_empty() {
            return Vec::new();
        }
        let title = if recent { "Recent" } else { "Best matches" };
        return Vec::from([SectionView {
            title: title.into(),
            rows,
        }]);
    }
    // One category: the place says what it is, so no heading.
    if model.filters.place.category().is_some() {
        let rows: Vec<RowView> = rows.collect();
        return if rows.is_empty() {
            Vec::new()
        } else {
            Vec::from([SectionView {
                title: String::new(),
                rows,
            }])
        };
    }
    let mut sections: Vec<SectionView> = Vec::new();
    for (row, &id) in rows.zip(&model.results) {
        let title = model.patch(id).map_or("", |p| p.category.name());
        match sections.last_mut() {
            Some(section) if section.title == title => section.rows.push(row),
            _ => sections.push(SectionView {
                title: title.into(),
                rows: Vec::from([row]),
            }),
        }
    }
    sections
}

fn empty_text(model: &Model) -> String {
    let query = model.filters.query.trim();
    let presets = model.filters.place == Place::Presets;
    match (query.is_empty(), presets) {
        (false, true) => format!("No presets match “{query}”"),
        (false, false) => format!("No sounds match “{query}”"),
        (true, true) => "Save what you're playing to keep it here".into(),
        (true, false) => match model.filters.place {
            Place::Favourites if model.favourites.is_empty() => {
                "Star a sound to keep it here".into()
            }
            Place::Recent if model.recents.is_empty() => "Sounds you pick show up here".into(),
            _ => "No sounds here".into(),
        },
    }
}

fn connection_view(devices: &[DeviceView]) -> ConnectionView {
    let connected: Vec<&str> = devices
        .iter()
        .filter(|d| d.state == DeviceState::Connected)
        .map(|d| d.name.as_str())
        .collect();
    if !connected.is_empty() {
        return ConnectionView {
            state: DeviceState::Connected,
            label: connected.join(", "),
        };
    }
    if devices.iter().any(|d| d.state == DeviceState::Connecting) {
        return ConnectionView {
            state: DeviceState::Connecting,
            label: "Connecting…".into(),
        };
    }
    ConnectionView {
        state: DeviceState::Disconnected,
        label: "No keyboard".into(),
    }
}

/// The listed devices (and those being connected from a scan), and the
/// other devices a scan found.
fn device_views(model: &Model) -> (Vec<DeviceView>, Vec<DeviceView>) {
    let midi = &model.midi;
    let listed = |id: &str| midi.devices().iter().any(|d| d.id == id);
    let connecting_from_scan = midi
        .found()
        .iter()
        .filter(|d| !listed(&d.id) && midi.state(&d.id).is_some());
    let devices = midi
        .devices()
        .iter()
        .chain(connecting_from_scan)
        .map(|d| device_view(model, d))
        .collect();
    let found = midi
        .found()
        .iter()
        .filter(|d| !listed(&d.id) && midi.state(&d.id).is_none())
        .map(|d| device_view(model, d))
        .collect();
    (devices, found)
}

fn device_view(model: &Model, device: &MidiDevice) -> DeviceView {
    let state = match model.midi.state(&device.id) {
        Some(LinkState::Connected) => DeviceState::Connected,
        Some(LinkState::Connecting) => DeviceState::Connecting,
        None => DeviceState::Disconnected,
    };
    DeviceView {
        id: device.id.clone(),
        name: device.name.clone(),
        transport: device.transport,
        state,
    }
}

const fn bluetooth_view(model: &Model, found: Vec<DeviceView>) -> BluetoothView {
    BluetoothView {
        scanning: model.midi.scanning(),
        found,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_thousands() {
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(1383), "1,383");
        assert_eq!(thousands(123_456), "123,456");
    }
}
