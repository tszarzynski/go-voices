//! The view model: what every shell draws, worded here once.
//!
//! Shells choose fonts, colours, layout and letter case. Everything that
//! depends on the app's state, including the words, comes from here.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use catalogue::Patch;
use crux_midi::{
    LinkState,
    effects::{MidiDevice, Transport},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    app::{CHANNELS, Model, Place, Platform},
    go_keys,
    layer::{self, LayerControl},
    layers::{self, Tab},
    part::{Control, Group},
    parts::{self, Layout},
    settings::{self, Setup},
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
    /// The sound picked for the part being edited.
    pub current: Option<CurrentView>,
    /// The displays: one per part, lowest keys first, as on the keyboard.
    pub displays: Vec<DisplayView>,
    /// The parts on the zone map.
    pub layout: LayoutView,
    /// The Edit panel, while it's showing in place of the sound list.
    pub editor: Option<EditorView>,
    /// The wave picker, while it's showing.
    pub waves: Option<WavePickerView>,
    /// `Channel 1`.
    pub channel_label: String,
    /// For `SetChannel`: every channel.
    pub channels: Vec<ChannelOption>,
    /// For `SetSeamless`, in the keyboards sheet.
    pub seamless: SwitchView,
    /// What the channel setting is for, in the keyboards sheet.
    pub channel_hint: String,
    /// Listed devices, then [`BluetoothView::found`] ones being connected.
    pub devices: Vec<DeviceView>,
    /// What to say while `devices` is empty.
    pub devices_hint: String,
    /// Pairing or scanning for Bluetooth keyboards.
    pub bluetooth: BluetoothView,
    /// The last error, until `DismissStatus` or the next attempt.
    pub status: Option<String>,
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
    /// The patch picked last.
    pub current: bool,
}

/// The patch picked last.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CurrentView {
    /// For `ToggleFavourite`.
    pub id: u32,
    /// The patch's name.
    pub name: String,
    /// Its category's name, `Acoustic piano`.
    pub category: String,
    /// Its bank's name, `JUNO-DS`.
    pub bank: String,
    /// Its address, `87·64·004`.
    pub address: String,
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

/// One display: a part's sound.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DisplayView {
    /// For `EditPart` and `RemovePart`, from 0.
    pub part: u8,
    /// There are other parts: tapping it edits this one.
    pub several: bool,
    /// `Remove part 2`, if it can be taken away.
    pub remove: Option<String>,
    /// `2 · Acoustic piano`, `Acoustic piano`, `GO Voices`.
    pub heading: String,
    /// The sound's name, or what's listed, or `—`.
    pub title: String,
    /// What happens next, or the part's keys: `Up to B3 · Oct −1`.
    pub footnote: String,
    /// Its sound, for the address and the star.
    pub sound: Option<CurrentView>,
    /// Not the part being edited.
    pub dimmed: bool,
    /// The last error, on the display being edited.
    pub status: Option<String>,
}

/// A setting that's on or off.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct SwitchView {
    /// `Seamless switching`.
    pub label: String,
    /// What it does.
    pub hint: String,
    /// On.
    pub on: bool,
}

/// One MIDI channel in the channel menu.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChannelOption {
    /// For `SetChannel`.
    pub channel: u8,
    /// `Channel 1`.
    pub label: String,
    /// In use.
    pub selected: bool,
}

/// The zone map: the parts and their keys.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutView {
    /// One lane per part, in order.
    pub zones: Vec<ZoneView>,
    /// The keyboard's first and last keys, to draw.
    pub first: u8,
    pub last: u8,
    /// `AddPart` can add one.
    pub can_add: bool,
    /// The octaves marked under the map: each C's key and name.
    pub ticks: Vec<NoteTick>,
    /// For `StartLayout`.
    pub layouts: Vec<LayoutOption>,
}

/// One part on the zone map.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ZoneView {
    /// For `EditPart`, `RemovePart` and `SetKeys`, from 0.
    pub part: u8,
    /// `1`.
    pub label: String,
    /// `Part 1`.
    pub title: String,
    /// `Remove part 1`, if it can be taken away (not the only one).
    pub remove: Option<String>,
    /// Its sound's name, or `—`.
    pub sound: String,
    /// Its lowest and highest keys (0 and 127: every key that way).
    pub low: u8,
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

/// A way to start: one sound, a split, a layer.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LayoutOption {
    /// For `StartLayout`.
    pub layout: Layout,
    /// `One sound`, `Split at C4`, `Layer`.
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
    /// `Whole keyboard · every part`.
    pub keyboard_title: String,
    /// Effects: settings shared by every part.
    pub keyboard: Vec<GroupView>,
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
    /// `Sound`, `Layers`.
    pub label: String,
    /// Showing.
    pub selected: bool,
}

/// The Layers tab: a sound's four layers, and the selected one's settings.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LayersView {
    /// Why there's nothing to edit yet: reading, no keyboard, a drum kit.
    pub status: Option<String>,
    /// The layers, for `SelectLayer` and `SoloLayer`. Empty until read.
    pub cards: Vec<LayerCardView>,
    /// The selected layer's wave.
    pub wave: Option<WaveView>,
    /// The selected layer's settings, in groups.
    pub groups: Vec<GroupView>,
    /// Anything changed, to reset.
    pub edited: bool,
    /// The layer `PasteLayer` would paste: `Layer 2 of JUNO Piano 1`.
    pub paste: Option<String>,
    /// The keyboard's first and last keys, to draw layers' keys on.
    pub first: u8,
    pub last: u8,
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
    /// The lowest and highest keys it plays (MIDI notes, may be past the
    /// keyboard's ends).
    pub low: u8,
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
    /// Keys below the lowest and above the highest it fades over.
    pub fade_low: u8,
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
    /// Changed from the sound's.
    pub edited: bool,
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
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceView {
    /// For `ToggleDevice`.
    pub id: String,
    /// What to call it.
    pub name: String,
    /// Where its connection is.
    pub state: DeviceState,
    /// `USB`, `Bluetooth` …, or `Connecting…`, `Connected`.
    pub detail: String,
    /// What `ToggleDevice` does: `Connect`, `Cancel` or `Disconnect`.
    pub action: String,
}

/// Finding Bluetooth keyboards.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct BluetoothView {
    /// Whether a scan is running.
    pub scanning: bool,
    /// What the Bluetooth panel says.
    pub hint: String,
    /// The button: scan, stop (send `StopBluetoothScan`), or pair.
    pub action: String,
    /// Devices found by the scan that aren't listed (or being connected).
    pub found: Vec<DeviceView>,
}

#[must_use]
pub fn view_model(model: &Model) -> ViewModel {
    let (devices, found) = device_views(model);
    let filters = &model.filters;
    let total = model.catalogue.patches().len();
    let presets = (filters.place == Place::Presets).then(|| preset_views(model));
    let listed = presets.as_ref().map_or(model.results.len(), Vec::len);
    let noun = if presets.is_some() { "preset" } else { "sound" };
    let count_text = count(filters.searching(), listed, noun);
    let connected_any = devices.iter().any(|d| d.state == DeviceState::Connected);
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
        loaded: model.loaded().map(|(preset, edited)| LoadedView {
            id: preset.id,
            name: preset.name.clone(),
            edited,
        }),
        count: count_text.clone(),
        empty: (listed == 0).then(|| empty_text(model)),
        current: current_view(model, model.editing()),
        displays: displays(model, connected_any, &count_text),
        layout: layout_view(model),
        editor: model.editor.then(|| editor_view(model)),
        waves: wave_picker(model),
        channel_label: channel_label(model.channel),
        seamless: SwitchView {
            label: "Seamless switching".into(),
            hint: "Notes you're still holding keep their sound when you pick a new one, and \
                   the next notes play the new sound. Uses two more of the keyboard's parts. \
                   Untested with a sustain pedal: notes held by the pedal may keep sounding."
                .into(),
            on: model.seamless,
        },
        channels: CHANNELS
            .map(|channel| ChannelOption {
                channel,
                label: channel_label(channel),
                selected: channel == model.channel,
            })
            .collect(),
        channel_hint: "Sounds are sent on this channel. The keyboard listens on channel 1 \
                       unless its settings were changed."
            .into(),
        devices_hint: devices_hint(model.platform).into(),
        devices,
        bluetooth: bluetooth_view(model, found),
        status: model.midi.status().map(String::from),
    }
}

/// A part's sound, for a display.
fn current_view(model: &Model, index: usize) -> Option<CurrentView> {
    let id = model.picked(index)?;
    let patch = model.patch(id)?;
    Some(CurrentView {
        id: row_id(id),
        name: patch.name.clone(),
        category: patch.category.name().into(),
        bank: patch.bank().name().into(),
        address: patch.address(),
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
fn preset_views(model: &Model) -> Vec<PresetView> {
    let loaded = model.loaded().map(|(preset, edited)| (preset.id, edited));
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
            }
        })
        .collect()
}

/// What a setup plays: `Single · JUNO Piano 1`, `Split C4 · Fretless /
/// JUNO Piano 1`, `Layer · …`, `3 parts · …` (lowest keys first).
fn describe(model: &Model, setup: &Setup) -> String {
    let mut parts: Vec<&settings::PartSetup> = setup.parts.iter().collect();
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
    let full = |part: &&settings::PartSetup| part.low == 0 && part.high == 127;
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

/// A name for a new preset: its sounds', lowest keys first.
fn preset_name(model: &Model) -> String {
    let names: Option<Vec<String>> = by_keys(model)
        .into_iter()
        .map(|index| model.sound(index).map(|patch| patch.name.clone()))
        .collect();
    names.map_or_else(
        || format!("Preset {}", model.presets.len() + 1),
        |names| names.join(" / "),
    )
}

fn channel_label(channel: u8) -> String {
    format!("Channel {channel}")
}

/// One display per part, lowest keys first.
fn displays(model: &Model, connected: bool, count: &str) -> Vec<DisplayView> {
    let status = model.midi.status().map(String::from);
    let editing = model.editing();
    let several = model.parts.len() > 1;
    by_keys(model)
        .into_iter()
        .map(|index| {
            let sound = current_view(model, index);
            let number = parts::number(index);
            let heading = match (&sound, several) {
                (None, false) => "GO Voices".into(),
                (None, true) => number.to_string(),
                (Some(sound), false) => sound.category.clone(),
                (Some(sound), true) => format!("{number} · {}", sound.category),
            };
            let title = match (&sound, several) {
                (Some(sound), _) => sound.name.clone(),
                (None, false) => count.into(),
                (None, true) => "—".into(),
            };
            let footnote = match (sound.is_some(), several, connected) {
                (_, true, _) => keys(model, index),
                (false, false, _) => "Tap a sound to play it on your keyboard".into(),
                (true, false, true) => format!("Sent on channel {}", model.channel),
                (true, false, false) => "Sent when a keyboard connects".into(),
            };
            DisplayView {
                part: u8::try_from(index).unwrap_or_default(),
                several,
                remove: remove_label(model, index),
                heading,
                title,
                footnote,
                sound,
                dimmed: index != editing,
                status: (index == editing).then(|| status.clone()).flatten(),
            }
        })
        .collect()
}

fn control_view(model: &Model, control: Control) -> ControlView {
    let value = model.value(control);
    ControlView {
        setting: Setting::Part(control),
        label: control.label().into(),
        text: control.text(value),
        value,
        min: control.range().0,
        max: control.range().1,
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
    ControlView {
        setting: Setting::Layer(control),
        label: control.label().into(),
        text: control.text(value),
        value,
        min: control.range().0,
        max: control.range().1,
        edited: layers::original(model, control).is_some_and(|original| original != value),
        choices: choices(control.choices()),
    }
}

/// The selected layer's settings, in groups.
fn layer_groups(model: &Model) -> Vec<GroupView> {
    let sounds = &model.edited().layers;
    let byte = |place| sounds.byte(place).unwrap_or_default();
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
            envelope: match title {
                "Filter envelope" | "Amp envelope" => {
                    layer::envelope(model.layer, title == "Amp envelope", byte)
                        .into_iter()
                        .map(|(x, y)| PointView { x, y })
                        .collect()
                }
                _ => Vec::new(),
            },
        })
        .collect()
}

/// The Layers tab.
fn layers_view(model: &Model) -> LayersView {
    let sounds = &model.edited().layers;
    let status = if layers::drums(model) {
        Some("Drum kits have no layers to edit here")
    } else if sounds.ready() {
        None
    } else if model.midi.connected().next().is_none() {
        Some("Connect your keyboard to edit this sound's layers")
    } else {
        Some("Reading the sound from the keyboard…")
    };
    if let Some(status) = status {
        return LayersView {
            status: Some(status.into()),
            cards: Vec::new(),
            wave: None,
            groups: Vec::new(),
            edited: false,
            paste: None,
            first: go_keys::FIRST,
            last: go_keys::LAST,
        };
    }
    let byte = |place| sounds.byte(place).unwrap_or_default();
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
            edited: sounds
                .edits
                .keys()
                .any(|&place| layer::is_wave(model.layer, place)),
        }
    });
    let groups = layer_groups(model);
    LayersView {
        status: None,
        cards,
        wave,
        groups,
        edited: !sounds.edits.is_empty(),
        paste: model.copied.as_ref().map(|copied| copied.from.clone()),
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
            name: wave.name.trim_end_matches(" L").into(),
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
    let groups = |groups: &[Group]| {
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
    };
    EditorView {
        title,
        groups: groups(&Control::GROUPS),
        edited: layers.as_ref().map_or_else(
            || {
                Control::part_controls()
                    .any(|control| model.value(control) != model.default_value(control))
            },
            |layers| layers.edited,
        ),
        keyboard_title: "Whole keyboard · every part".into(),
        keyboard: groups(&Control::KEYBOARD_GROUPS),
        tabs: [(Tab::Sound, "Sound"), (Tab::Layers, "Layers")]
            .into_iter()
            .map(|(tab, label)| TabView {
                tab,
                label: label.into(),
                selected: model.tab == tab,
            })
            .collect(),
        layers,
    }
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
            .map(|(index, part)| ZoneView {
                part: u8::try_from(index).unwrap_or_default(),
                label: parts::number(index).to_string(),
                title: part_label(index),
                remove: remove_label(model, index),
                sound: model
                    .sound(index)
                    .map_or_else(|| "—".into(), |patch| patch.name.clone()),
                low: part.low,
                high: part.high,
                keys: go_keys::keys_text(part.low, part.high),
                editing: index == editing,
            })
            .collect(),
        first: go_keys::FIRST,
        last: go_keys::LAST,
        can_add: parts::can_add(model),
        ticks: go_keys::KEYS
            .filter(|note| note % 12 == 0)
            .map(|note| NoteTick {
                note,
                label: go_keys::note_name(note),
            })
            .collect(),
        layouts: [
            (Layout::One, "One sound".into()),
            (
                Layout::Split,
                format!("Split at {}", go_keys::note_name(parts::SPLIT)),
            ),
            (Layout::Layer, "Layer".into()),
        ]
        .into_iter()
        .map(|(layout, label)| LayoutOption { layout, label })
        .collect(),
    }
}

/// The keys a part plays, and its octave shift if any: `Up to B3 · Oct −1`,
/// `All keys`.
fn keys(model: &Model, index: usize) -> String {
    let part = &model.parts[index];
    let range = go_keys::keys_text(part.low, part.high);
    match part.octave() {
        0 => range,
        shift => format!("{range} · Oct {}", Control::Octave.text(i16::from(shift))),
    }
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
fn search_scope(place: Place) -> String {
    match place {
        Place::All => "all sounds".into(),
        Place::Favourites => "favourites".into(),
        Place::Recent => "recent sounds".into(),
        Place::Presets => "presets".into(),
        Place::Bank(bank) => bank.name().into(),
        Place::Category { category, .. } => category.name().into(),
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
            current: model.picked(model.editing()) == Some(id),
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
        (false, true) => return format!("No presets match “{query}”"),
        (false, false) => return format!("No sounds match “{query}”"),
        (true, true) => return "Save what you're playing to keep it here".into(),
        (true, false) => {}
    }
    match model.filters.place {
        Place::Favourites if model.favourites.is_empty() => "Star a sound to keep it here".into(),
        Place::Recent if model.recents.is_empty() => "Sounds you pick show up here".into(),
        _ => "No sounds here".into(),
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
    let (detail, action) = match state {
        DeviceState::Disconnected => (transport_label(device.transport), "Connect"),
        DeviceState::Connecting => ("Connecting…", "Cancel"),
        DeviceState::Connected => ("Connected", "Disconnect"),
    };
    DeviceView {
        id: device.id.clone(),
        name: device.name.clone(),
        state,
        detail: detail.into(),
        action: action.into(),
    }
}

const fn transport_label(transport: Transport) -> &'static str {
    match transport {
        Transport::Usb => "USB",
        Transport::Bluetooth => "Bluetooth",
        Transport::Virtual => "Virtual",
        Transport::Other => "MIDI",
    }
}

const fn devices_hint(platform: Platform) -> &'static str {
    match platform {
        Platform::Mac => "Switch on your keyboard, or connect it with a USB cable.",
        Platform::Ios => "Pair your keyboard below, or connect it with a USB cable.",
        Platform::Android => "Scan for your keyboard below, or connect it with a USB cable.",
    }
}

fn bluetooth_view(model: &Model, found: Vec<DeviceView>) -> BluetoothView {
    let scanning = model.midi.scanning();
    let (hint, action) = match (model.platform, scanning) {
        (Platform::Android, true) => ("Scanning for Bluetooth MIDI keyboards…", "Stop"),
        (Platform::Android, false) => ("Find Bluetooth MIDI keyboards nearby.", "Scan"),
        (Platform::Ios, _) => (
            "Pair a Bluetooth MIDI keyboard once; after that it connects by itself.",
            "Pair Bluetooth keyboard…",
        ),
        (Platform::Mac, _) => (
            "Pair Bluetooth MIDI keyboards in Audio MIDI Setup: Window › Show MIDI Studio › \
             Configure Bluetooth. They then appear above.",
            "Open Audio MIDI Setup…",
        ),
    };
    BluetoothView {
        scanning,
        hint: hint.into(),
        action: action.into(),
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
