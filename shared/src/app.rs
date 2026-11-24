//! The Crux app: browse the sounds, send the one picked.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use core::ops::RangeInclusive;

use catalogue::{Bank, Catalogue, Category, Patch, Waves};
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_midi::{
    Connections, MidiEvent, Notice, Update,
    effects::{
        self as midi, ConnectMidi, DisconnectMidi, ForgetDevice, MidiDevice, OpenBluetoothSettings,
        ScanBluetooth, SendMidi, StopBluetoothScan, WatchDevices,
    },
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    browse::{self, BankTree, Filters},
    go_keys,
    layer::{self, LayerControl},
    layers::{self, Tab},
    part::{Control, Edits, Scope, Target},
    parts::{self, Layout, Part},
    settings::{self, LoadSettings, Preset, SaveSettings, Settings, StoredSettings},
    view::ViewModel,
};

/// How many picks the Recent tab keeps.
pub const RECENTS: usize = 20;

/// The MIDI channels.
pub const CHANNELS: RangeInclusive<u8> = 1..=16;

/// Where the app runs: it decides how Bluetooth keyboards are paired.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Platform {
    /// Paired in Audio MIDI Setup.
    #[default]
    Mac,
    /// Paired in the system's Bluetooth MIDI sheet, opened from the app.
    Ios,
    /// Scanned for and connected in the app.
    Android,
}

/// A place in the sidebar tree: which patches the list holds before
/// searching.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Place {
    /// The whole catalogue.
    #[default]
    All,
    /// Starred patches.
    Favourites,
    /// The last patches picked, most recent first.
    Recent,
    /// The presets: the list shows them instead of sounds.
    Presets,
    /// One bank.
    Bank(Bank),
    /// One category of one bank.
    Category {
        /// The bank.
        bank: Bank,
        /// The category.
        category: Category,
    },
}

impl Place {
    /// The bank it's in, if it's in one.
    #[must_use]
    pub const fn bank(self) -> Option<Bank> {
        match self {
            Self::Bank(bank) | Self::Category { bank, .. } => Some(bank),
            Self::All | Self::Favourites | Self::Recent | Self::Presets => None,
        }
    }

    /// Its category, if it's one.
    #[must_use]
    pub const fn category(self) -> Option<Category> {
        match self {
            Self::Category { category, .. } => Some(category),
            _ => None,
        }
    }
}

/// What the shells tell the core, and what effects answer with.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    // From the shell
    /// Sent once at launch.
    Start {
        /// Where the app runs.
        platform: Platform,
    },
    /// What's typed in the search field.
    Search(String),
    /// List this place in the tree.
    Open(Place),
    /// Pick a patch (by [`crate::RowView::id`]) and send it to the keyboard.
    Select(u32),
    /// Pick the next patch in the list (wrapping around).
    Next,
    /// Pick the previous patch in the list (wrapping around).
    Previous,
    /// Star a patch, or unstar it.
    ToggleFavourite(u32),
    /// The MIDI channel to send on, 1–16.
    SetChannel(u8),
    /// Start again from one sound, a split or a layer.
    StartLayout(Layout),
    /// Which part a pick (or Next, Previous) and the Edit panel change.
    EditPart(u8),
    /// Split the part being edited at its middle key: a new part takes
    /// the upper half.
    AddPart,
    /// Take a part away: a neighbour takes over its keys.
    RemovePart(u8),
    /// Set a part's lowest and highest keys (MIDI notes), on its own: other
    /// parts stay where they are. At the keyboard's ends, every key past
    /// them too.
    SetKeys {
        /// Which part, from 0.
        part: u8,
        /// Its lowest key.
        low: u8,
        /// Its highest key.
        high: u8,
    },
    /// Turn seamless switching on or off: a new sound goes to a spare part,
    /// so notes still sounding keep their sound.
    SetSeamless(bool),
    /// Keep what's playing as a new preset.
    SavePreset {
        /// What to call it.
        name: String,
    },
    /// Play a preset (by [`crate::PresetView::id`]).
    RecallPreset(u32),
    /// Keep what's playing in a preset, in place of what it had.
    UpdatePreset(u32),
    /// Call a preset something else.
    RenamePreset {
        /// Which.
        id: u32,
        /// Its new name.
        name: String,
    },
    /// Forget a preset.
    DeletePreset(u32),
    /// Show a tab of the Edit panel.
    ShowTab(Tab),
    /// Edit this layer (0–3) in the Layers tab.
    SelectLayer(u8),
    /// Change the selected layer's setting.
    SetLayerControl {
        /// Which.
        control: LayerControl,
        /// The new value, in its range.
        value: i16,
    },
    /// Put the selected layer's setting back as the sound had it.
    ResetLayerControl(LayerControl),
    /// Put a layer back as the sound had it, or with `None`, every layer.
    ResetLayers(Option<u8>),
    /// Play one layer alone, or with `None`, all of them again.
    SoloLayer(Option<u8>),
    /// Edit this layer and switch it on, or off.
    ToggleLayer(u8),
    /// Set the selected layer's lowest and highest keys (MIDI notes) at
    /// once, as when its range is moved whole. At the keyboard's ends,
    /// every key past them too.
    SetLayerKeys {
        /// Its lowest key.
        low: u8,
        /// Its highest key.
        high: u8,
    },
    /// Put the selected layer's keys back as the sound had them.
    ResetLayerKeys,
    /// Copy this layer, to paste onto another.
    CopyLayer(u8),
    /// Paste the layer copied onto this one.
    PasteLayer(u8),
    /// Give the selected layer the next wave (or the previous one).
    StepWave(bool),
    /// Give the selected layer this wave.
    SetWave {
        /// Its group.
        group: u16,
        /// Its number.
        number: u16,
    },
    /// Show the wave picker, searching for this; `None` hides it.
    SearchWaves(Option<String>),
    /// Show the Edit panel in place of the sound list, or hide it.
    ToggleEditor,
    /// Opens the Edit panel on part `0` (from 0), whether it was open or
    /// not.
    OpenEditor(u8),
    /// Open the keyboard sheet, or close it.
    ShowKeyboard(bool),
    /// Change a setting: of the part being edited, or of the whole
    /// keyboard.
    SetControl {
        /// Which.
        control: Control,
        /// The new value, in the control's range.
        value: i16,
    },
    /// Put a setting back to the keyboard's own.
    ResetControl(Control),
    /// Put back everything the Edit panel's tab showing changes: the part's
    /// settings, or its sound's layers.
    ResetEditor,
    /// Connect to a device, or disconnect it, or stop connecting to it.
    ToggleDevice {
        /// The device's id, as the shell listed it.
        device_id: String,
    },
    /// Find Bluetooth keyboards: scan, or open the system's pairing UI.
    StartBluetoothScan,
    /// Stop scanning.
    StopBluetoothScan,
    /// Hide the error message.
    DismissStatus,

    // Internal
    /// The shell's answer to a MIDI request.
    #[serde(skip)]
    #[facet(skip)]
    Midi(#[facet(opaque)] MidiEvent),
    /// The settings kept at the last launch.
    #[serde(skip)]
    #[facet(skip)]
    Loaded(#[facet(opaque)] StoredSettings),
}

impl From<MidiEvent> for Event {
    fn from(event: MidiEvent) -> Self {
        Self::Midi(event)
    }
}

/// What the core asks the shell to do.
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    /// Draw the view model again.
    Render(RenderOperation),
    /// See [`WatchDevices`].
    WatchDevices(WatchDevices),
    /// See [`ConnectMidi`].
    ConnectMidi(ConnectMidi),
    /// See [`DisconnectMidi`].
    DisconnectMidi(DisconnectMidi),
    /// See [`ForgetDevice`].
    ForgetDevice(ForgetDevice),
    /// See [`ScanBluetooth`].
    ScanBluetooth(ScanBluetooth),
    /// See [`StopBluetoothScan`].
    StopBluetoothScan(StopBluetoothScan),
    /// See [`OpenBluetoothSettings`].
    OpenBluetoothSettings(OpenBluetoothSettings),
    /// See [`SendMidi`].
    SendMidi(SendMidi),
    /// See [`LoadSettings`].
    LoadSettings(LoadSettings),
    /// See [`SaveSettings`].
    SaveSettings(SaveSettings),
}

/// The app's state, between events.
pub struct Model {
    pub(crate) catalogue: Catalogue,
    /// The sidebar's banks and categories with their counts: the catalogue
    /// doesn't change, so counted once.
    pub(crate) tree: Vec<BankTree>,
    pub(crate) platform: Platform,
    pub(crate) filters: Filters,
    /// What the list shows, from `filters`: patch ids, best match first.
    pub(crate) results: Vec<usize>,
    /// The parts the keyboard plays, 1 to 4: sent to every keyboard that
    /// connects.
    pub(crate) parts: Vec<Part>,
    /// The part picks and the Edit panel change.
    pub(crate) editing: usize,
    /// The whole keyboard's changed settings.
    pub(crate) system: Edits,
    /// The Edit panel is showing.
    pub(crate) editor: bool,
    /// Whether the keyboard sheet is open: its settings are only built
    /// while it is.
    pub(crate) keyboard_sheet: bool,
    /// Its tab.
    pub(crate) tab: Tab,
    /// The layer the Layers tab edits, 0–3.
    pub(crate) layer: u8,
    /// The layer playing alone, if one is.
    pub(crate) solo: Option<u8>,
    /// The layer copied, if one is. Kept across sounds, not relaunches.
    pub(crate) copied: Option<layers::Copied>,
    /// The waves a layer can have.
    pub(crate) waves: Waves,
    /// The wave picker's search, while it's showing.
    pub(crate) wave_query: Option<String>,
    /// New sounds go to a spare part, taking turns with the part's own.
    pub(crate) seamless: bool,
    /// The presets, in the order they were saved.
    pub(crate) presets: Vec<Preset>,
    /// The preset recalled or saved last, which Save updates.
    pub(crate) loaded: Option<u32>,
    pub(crate) favourites: BTreeSet<usize>,
    /// Most recent first.
    pub(crate) recents: Vec<usize>,
    pub(crate) channel: u8,
    /// The MIDI devices and their connections.
    pub(crate) midi: Connections,
}

impl Default for Model {
    fn default() -> Self {
        let catalogue = Catalogue::builtin();
        let results = (0..catalogue.patches().len()).collect();
        Self {
            tree: browse::tree(&catalogue),
            catalogue,
            platform: Platform::default(),
            filters: Filters::default(),
            results,
            parts: Vec::from([Part::new(0, None)]),
            editing: 0,
            system: Edits::new(),
            editor: false,
            keyboard_sheet: false,
            tab: Tab::Sound,
            layer: 0,
            solo: None,
            copied: None,
            waves: Waves::builtin(),
            wave_query: None,
            seamless: false,
            presets: Vec::new(),
            loaded: None,
            favourites: BTreeSet::new(),
            recents: Vec::new(),
            channel: 1,
            midi: Connections::default(),
        }
    }
}

impl Model {
    /// The patch with this id.
    pub(crate) fn patch(&self, id: usize) -> Option<&Patch> {
        self.catalogue.get(id)
    }

    /// The part picks and the Edit panel change.
    pub(crate) fn editing(&self) -> usize {
        self.editing.min(self.parts.len().saturating_sub(1))
    }

    /// The part being edited.
    pub(crate) fn edited(&self) -> &Part {
        &self.parts[self.editing()]
    }

    pub(crate) fn edited_mut(&mut self) -> &mut Part {
        let index = self.editing();
        &mut self.parts[index]
    }

    /// Whose setting `control` is: the part being edited's, or the whole
    /// keyboard's. Its own part's, for defaults: a part sounds the same on
    /// its spare.
    pub(crate) fn scope(&self, control: Control) -> Scope {
        control.scope(parts::number(self.editing()))
    }

    /// The keyboard part part `index` plays on now.
    pub(crate) fn part(&self, index: usize) -> u8 {
        self.parts[index].playing
    }

    /// The part the part being edited plays on now, for writing `control`;
    /// or the whole keyboard.
    fn target(&self, control: Control) -> Scope {
        control.scope(self.edited().playing)
    }

    /// The edits `control` is among.
    fn edits_for(&self, control: Control) -> &Edits {
        match self.scope(control) {
            Scope::Keyboard => &self.system,
            Scope::Part(_) => &self.edited().edits,
        }
    }

    fn edits_for_mut(&mut self, control: Control) -> &mut Edits {
        match self.scope(control) {
            Scope::Keyboard => &mut self.system,
            Scope::Part(_) => &mut self.edited_mut().edits,
        }
    }

    /// A setting's keyboard's own value, for the part being edited.
    pub(crate) fn default_value(&self, control: Control) -> i16 {
        control.default(self.scope(control))
    }

    /// A setting's value: of the part being edited, or of the keyboard.
    pub(crate) fn value(&self, control: Control) -> i16 {
        self.edits_for(control)
            .get(&control)
            .copied()
            .unwrap_or_else(|| self.default_value(control))
    }

    /// The sound picked for part `index`.
    pub(crate) fn picked(&self, index: usize) -> Option<usize> {
        self.parts.get(index)?.sound
    }

    /// Part `index`'s sound.
    pub(crate) fn sound(&self, index: usize) -> Option<&Patch> {
        self.patch(self.picked(index)?)
    }

    /// The preset recalled or saved last, and whether what's playing has
    /// changed since.
    pub(crate) fn loaded(&self) -> Option<(&Preset, bool)> {
        let preset = self.presets.iter().find(|p| Some(p.id) == self.loaded)?;
        Some((preset, preset.setup != settings::setup(self)))
    }

    /// The presets the list shows: those whose name matches the search.
    pub(crate) fn listed_presets(&self) -> impl Iterator<Item = &Preset> {
        let query = self.filters.query.trim().to_lowercase();
        self.presets
            .iter()
            .filter(move |preset| preset.name.to_lowercase().contains(&query))
    }

    pub(crate) fn refresh(&mut self) {
        self.results = browse::results(
            &self.catalogue,
            &self.filters,
            &self.favourites,
            &self.recents,
        );
    }
}

/// Go:Voices: the GO:KEYS sounds, browsed and sent over MIDI.
#[derive(Default)]
pub struct GoVoices;

impl App for GoVoices {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Start { platform } => {
                model.platform = platform;
                model.midi.auto_connect_only(is_go_keys);
                Command::all([
                    settings::load().then_send(Event::Loaded),
                    model.midi.start(platform == Platform::Android),
                    render(),
                ])
            }
            Event::Loaded(StoredSettings(json)) => {
                let Some(settings) = json.as_deref().and_then(Settings::from_json) else {
                    return render();
                };
                settings::restore(model, &settings);
                // A keyboard may have connected while they loaded.
                send(model.midi.connected(), &state(model)).and(render())
            }
            Event::Search(query) => {
                model.filters.query = query;
                model.refresh();
                render()
            }
            Event::Open(place) => open(model, place),
            Event::Select(id) => select(model, id as usize),
            Event::Next | Event::Previous if model.filters.place == Place::Presets => {
                step_presets(model, event == Event::Next)
            }
            Event::Next | Event::Previous => {
                let forward = event == Event::Next;
                let from = model.picked(model.editing());
                browse::step(&model.results, from, forward)
                    .map_or_else(Command::done, |id| select(model, id))
            }
            Event::ToggleFavourite(id) => toggle_favourite(model, id as usize),
            Event::SetChannel(channel) => {
                if !CHANNELS.contains(&channel) || channel == model.channel {
                    return Command::done();
                }
                model.channel = channel;
                commit(model, &sound(model, 0))
            }
            Event::StartLayout(_)
            | Event::AddPart
            | Event::RemovePart(_)
            | Event::SetKeys { .. } => edit_parts(model, &event),
            Event::SetSeamless(on) => set_seamless(model, on),
            Event::RecallPreset(id) => recall(model, id),
            Event::SavePreset { .. }
            | Event::UpdatePreset(_)
            | Event::RenamePreset { .. }
            | Event::DeletePreset(_) => edit_presets(model, event),
            Event::EditPart(_)
            | Event::ToggleEditor
            | Event::OpenEditor(_)
            | Event::ShowKeyboard(_)
            | Event::ShowTab(_)
            | Event::SelectLayer(_)
            | Event::SoloLayer(_)
            | Event::CopyLayer(_)
            | Event::SearchWaves(_) => show(model, event),
            Event::SetLayerControl { .. }
            | Event::ResetLayerControl(_)
            | Event::ResetLayers(_)
            | Event::PasteLayer(_)
            | Event::ToggleLayer(_)
            | Event::SetLayerKeys { .. }
            | Event::ResetLayerKeys
            | Event::StepWave(_)
            | Event::SetWave { .. } => edit_layers(model, &event),
            Event::SetControl { control, value } => set_control(model, control, value),
            Event::ResetControl(control) => {
                let value = model.default_value(control);
                set_control(model, control, value)
            }
            Event::ResetEditor if model.tab == Tab::Layers => {
                edit_layers(model, &Event::ResetLayers(None))
            }
            Event::ResetEditor => reset_part(model),
            Event::ToggleDevice { device_id } => {
                let update = model.midi.toggle(device_id);
                apply(model, update)
            }
            Event::StartBluetoothScan | Event::StopBluetoothScan => {
                let update = if event == Event::StartBluetoothScan {
                    model.midi.start_scan()
                } else {
                    model.midi.stop_scan()
                };
                apply(model, update)
            }
            Event::DismissStatus => {
                model.midi.dismiss_status();
                render()
            }
            Event::Midi(event) => {
                let update = model.midi.update(event);
                apply(model, update)
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        crate::view::view_model(model)
    }
}

/// Stars a patch, or unstars it.
fn toggle_favourite(model: &mut Model, id: usize) -> Command<Effect, Event> {
    if model.patch(id).is_none() {
        return Command::done();
    }
    if !model.favourites.remove(&id) {
        model.favourites.insert(id);
    }
    model.refresh();
    commit(model, &[])
}

/// Changes the parts on the zone map: starts again from a layout, adds or
/// takes away a part, or moves a part's keys.
fn edit_parts(model: &mut Model, event: &Event) -> Command<Effect, Event> {
    if let Event::SetKeys { part, low, high } = *event {
        if !parts::set_keys(model, usize::from(part), low, high) {
            return Command::done();
        }
        return commit(model, &parts::zones(model));
    }
    // A solo ends on the part it's on, before parts move.
    let mut bytes = layers::end_solo(model);
    match *event {
        Event::StartLayout(layout) => {
            parts::start(model, layout);
            bytes.extend(everything(model));
        }
        Event::AddPart if parts::add(model) => {
            bytes.extend(whole_part(model, model.editing()));
            bytes.extend(parts::zones(model));
        }
        Event::RemovePart(part) if parts::remove(model, usize::from(part)) => {
            bytes.extend(everything(model));
        }
        _ => {}
    }
    commit(model, &bytes)
}

/// Changes a setting and sends it, so it's heard straight away.
fn set_control(model: &mut Model, control: Control, value: i16) -> Command<Effect, Event> {
    let value = control.clamp(value);
    if value == model.value(control) {
        return Command::done();
    }
    let default = model.default_value(control);
    let edits = model.edits_for_mut(control);
    if value == default {
        edits.remove(&control);
    } else {
        edits.insert(control, value);
    }
    let target = model.target(control);
    let bytes = if control.target() == Target::Scale {
        // The temperament and its key tune the notes together.
        let both = [Control::Temperament, Control::TuningKey]
            .map(|control| (control, model.value(control)));
        go_keys::settings(target, &Edits::from(both))
    } else {
        go_keys::setting(target, control, value)
    };
    commit(model, &bytes)
}

/// Puts every setting of the part being edited back to the keyboard's own.
fn reset_part(model: &mut Model) -> Command<Effect, Event> {
    let index = model.editing();
    let defaults: Edits = core::mem::take(&mut model.edited_mut().edits)
        .into_keys()
        .map(|control| (control, control.default(parts::scope(index))))
        .collect();
    commit(
        model,
        &go_keys::settings(Scope::Part(model.part(index)), &defaults),
    )
}

/// Picks a patch for the part being edited: sends it to every connected
/// keyboard and puts it first in the recents.
fn select(model: &mut Model, id: usize) -> Command<Effect, Event> {
    if model.patch(id).is_none() {
        return Command::done();
    }
    let index = model.editing();
    model.edited_mut().sound = Some(id);
    // A new sound: its layers are the keyboard's again.
    model.solo = None;
    model.edited_mut().layers.forget();
    model.recents.retain(|&recent| recent != id);
    model.recents.insert(0, id);
    model.recents.truncate(RECENTS);
    // Recent reorders; elsewhere the list stays as it is.
    if model.filters.place == Place::Recent {
        model.refresh();
    }
    if !model.seamless {
        return commit(model, &sound(model, index));
    }
    // The spare takes over: notes still sounding on the other part ring on.
    model.parts[index].playing = parts::idle(model, index);
    let mut bytes = whole_part(model, index);
    bytes.extend(parts::zones(model));
    commit(model, &bytes)
}

/// Saves, updates, renames or deletes a preset.
fn edit_presets(model: &mut Model, event: Event) -> Command<Effect, Event> {
    let setup = settings::setup(model);
    match event {
        Event::SavePreset { name } => {
            let id = model.presets.iter().map(|p| p.id + 1).max().unwrap_or(1);
            model.presets.push(Preset {
                id,
                name: name.trim().into(),
                setup,
            });
            model.loaded = Some(id);
        }
        Event::UpdatePreset(id) => {
            if let Some(preset) = model.presets.iter_mut().find(|p| p.id == id) {
                preset.setup = setup;
                model.loaded = Some(id);
            }
        }
        Event::RenamePreset { id, name } => {
            if let Some(preset) = model.presets.iter_mut().find(|p| p.id == id) {
                preset.name = name.trim().into();
            }
        }
        Event::DeletePreset(id) => {
            model.presets.retain(|p| p.id != id);
            if model.loaded == Some(id) {
                model.loaded = None;
            }
        }
        _ => return Command::done(),
    }
    commit(model, &[])
}

/// Plays the next (or previous) preset listed, wrapping around.
fn step_presets(model: &mut Model, forward: bool) -> Command<Effect, Event> {
    let ids: Vec<u32> = model.listed_presets().map(|p| p.id).collect();
    browse::step(&ids, model.loaded, forward).map_or_else(Command::done, |id| recall(model, id))
}

/// Plays a preset. Every setting is written, so nothing is left from
/// before; with seamless switching it goes to the spare parts.
fn recall(model: &mut Model, id: u32) -> Command<Effect, Event> {
    let Some(setup) = model
        .presets
        .iter()
        .find(|preset| preset.id == id)
        .map(|preset| preset.setup.clone())
    else {
        return Command::done();
    };
    // A solo ends on the sound it's on, before that sound goes.
    let mut bytes = layers::end_solo(model);
    let was_playing: Vec<u8> = model.parts.iter().map(|part| part.playing).collect();
    settings::apply(model, &setup);
    model.loaded = Some(id);
    if model.seamless {
        // Each part goes to whichever of its two it isn't sounding on.
        for (index, &playing) in was_playing.iter().enumerate().take(model.parts.len()) {
            model.parts[index].playing = playing;
        }
        for index in 0..model.parts.len() {
            model.parts[index].playing = parts::idle(model, index);
        }
    }
    bytes.extend(everything(model));
    bytes.extend(go_keys::settings(
        Scope::Keyboard,
        &every_keyboard_setting(model),
    ));
    commit(model, &bytes)
}

/// Turns seamless switching on or off. Off, each part goes back to its own
/// part, with its sound and settings, and the spares' keys go off.
fn set_seamless(model: &mut Model, on: bool) -> Command<Effect, Event> {
    if on == model.seamless {
        return Command::done();
    }
    model.seamless = on;
    if on {
        return commit(model, &[]);
    }
    let mut bytes = Vec::new();
    for index in 0..model.parts.len() {
        if model.part(index) != parts::number(index) {
            model.parts[index].playing = parts::number(index);
            bytes.extend(whole_part(model, index));
        }
    }
    bytes.extend(parts::zones(model));
    bytes.extend(parts::others_off(model));
    commit(model, &bytes)
}

/// Every setting of part `index`, changed or the keyboard's own; not its
/// octave, which is in its zone and goes with the keys.
fn every_setting(model: &Model, index: usize) -> Edits {
    let edits = &model.parts[index].edits;
    Control::part_controls()
        .filter(|&control| control != Control::Octave)
        .map(|control| {
            let value = edits
                .get(&control)
                .copied()
                .unwrap_or_else(|| control.default(parts::scope(index)));
            (control, value)
        })
        .collect()
}

/// Every setting of the whole keyboard: changed, or its own.
fn every_keyboard_setting(model: &Model) -> Edits {
    Control::KEYBOARD_GROUPS
        .iter()
        .flat_map(|group| group.controls.iter().copied())
        .map(|control| (control, model.value(control)))
        .collect()
}

/// A part's sound (as `patch`), these settings, then its changed layers,
/// written to the part it plays on now. The settings go after the sound:
/// the keyboard may reset a part's settings when its patch changes.
fn part_bytes(model: &Model, index: usize, patch: Vec<u8>, settings: &Edits) -> Vec<u8> {
    let mut bytes = patch;
    bytes.extend(go_keys::settings(Scope::Part(model.part(index)), settings));
    let layers = model.parts[index].layers.bytes();
    if !layers.is_empty() {
        bytes.extend(go_keys::sound_bytes(model.part(index), &layers));
    }
    bytes
}

/// A part's sound and every one of its settings, written to the part it
/// plays on now, which may have been left with another sound's.
fn whole_part(model: &Model, index: usize) -> Vec<u8> {
    let patch = model
        .sound(index)
        .map(|patch| go_keys::part_patch(model.part(index), patch))
        .unwrap_or_default();
    part_bytes(model, index, patch, &every_setting(model, index))
}

/// A part's new sound and its changed settings. The first part's sound goes
/// as bank select and program change on the channel, which reaches part 1;
/// with more parts, or with seamless switching, it's written to its part
/// directly, like the others.
fn sound(model: &Model, index: usize) -> Vec<u8> {
    let mut patch = Vec::new();
    if let Some(sound) = model.sound(index) {
        if index == 0 && !model.seamless {
            patch.extend(sound.select_bytes(model.channel));
        }
        if model.parts.len() > 1 || model.seamless {
            patch.extend(go_keys::part_patch(model.part(index), sound));
        }
    }
    part_bytes(model, index, patch, &model.parts[index].edits)
}

/// After parts move: every part's sound and settings where it plays now,
/// the keys, and every other part off.
fn everything(model: &Model) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..model.parts.len())
        .flat_map(|index| whole_part(model, index))
        .collect();
    bytes.extend(parts::zones(model));
    bytes.extend(parts::others_off(model));
    bytes
}

/// Everything a keyboard needs to play as the app has it: every part and
/// every setting, as they may have changed while it was away, and every
/// other part off, as it may still be split from before.
fn state(model: &Model) -> Vec<u8> {
    let mut bytes = everything(model);
    bytes.extend(go_keys::settings(
        Scope::Keyboard,
        &every_keyboard_setting(model),
    ));
    bytes
}

/// Sends these bytes, if any, to these devices.
fn send<'a>(devices: impl Iterator<Item = &'a str>, bytes: &[u8]) -> Command<Effect, Event> {
    if bytes.is_empty() {
        return Command::done();
    }
    Command::all(devices.map(|device| midi::send(device.to_string(), bytes.to_vec()).build()))
}

/// After a change: keeps the settings, sends these bytes (if any) to every
/// connected keyboard and redraws.
/// Also asks for the sound the Layers tab shows, if it needs reading.
fn commit(model: &mut Model, bytes: &[u8]) -> Command<Effect, Event> {
    let mut bytes = bytes.to_vec();
    bytes.extend(layers::reads(model));
    settings::save(&settings::snapshot(model))
        .build()
        .and(send(model.midi.connected(), &bytes))
        .and(render())
}

/// Lists `place`.
fn open(model: &mut Model, place: Place) -> Command<Effect, Event> {
    model.filters.place = place;
    model.refresh();
    render()
}

/// After a change only of what's shown: sends these bytes (if any), asks
/// for the sound the Layers tab shows if it needs reading, and redraws.
fn look(model: &mut Model, mut bytes: Vec<u8>) -> Command<Effect, Event> {
    bytes.extend(layers::reads(model));
    send(model.midi.connected(), &bytes).and(render())
}

/// Changes what the Edit panel shows (and plays, for a solo). A solo
/// ends when the panel shows something else.
fn show(model: &mut Model, event: Event) -> Command<Effect, Event> {
    let bytes = match event {
        Event::EditPart(part) | Event::OpenEditor(part) => {
            let bytes = layers::end_solo(model);
            model.editing = usize::from(part).min(model.parts.len() - 1);
            model.editor |= matches!(event, Event::OpenEditor(_));
            bytes
        }
        Event::ToggleEditor => {
            let bytes = layers::end_solo(model);
            model.editor = !model.editor;
            bytes
        }
        Event::ShowTab(tab) => {
            let bytes = layers::end_solo(model);
            model.tab = tab;
            bytes
        }
        Event::ShowKeyboard(open) => {
            model.keyboard_sheet = open;
            Vec::new()
        }
        Event::SelectLayer(layer) => {
            model.layer = layer.min(layer::LAYERS - 1);
            Vec::new()
        }
        Event::SoloLayer(layer) => layers::solo(model, layer),
        Event::CopyLayer(layer) => {
            layers::copy(model, layer);
            Vec::new()
        }
        Event::SearchWaves(query) => {
            model.wave_query = query;
            Vec::new()
        }
        _ => Vec::new(),
    };
    look(model, bytes)
}

/// Changes the selected layer, and keeps the change.
fn edit_layers(model: &mut Model, event: &Event) -> Command<Effect, Event> {
    let bytes = match *event {
        Event::SetLayerControl { control, value } => layers::set(model, control, value),
        Event::ResetLayerControl(control) => layers::reset(model, control),
        Event::ResetLayers(layer) => layers::reset_layers(model, layer),
        Event::PasteLayer(layer) => layers::paste(model, layer),
        Event::ToggleLayer(layer) => layers::toggle(model, layer),
        Event::SetLayerKeys { low, high } => layers::set_keys(model, low, high),
        Event::ResetLayerKeys => layers::reset_keys(model),
        Event::StepWave(forward) => layers::step_wave(model, forward),
        Event::SetWave { group, number } => layers::set_wave(model, group, number),
        _ => return Command::done(),
    };
    commit(model, &bytes)
}

/// Whether a MIDI device is a GO:KEYS, which the app connects to by itself:
/// `GO:KEYS MIDI` over Bluetooth, `GO:KEYS` over USB. Other gear is only
/// connected from the keyboards sheet.
fn is_go_keys(device: &MidiDevice) -> bool {
    device.name.to_ascii_uppercase().contains("GO:KEYS")
}

/// Acts on what happened to the devices: a keyboard that connects gets the
/// sounds (and the split or layer). Redraws if anything shown changed.
/// What a keyboard sends back (the sounds the Layers tab asked for) is
/// read in.
fn apply(model: &mut Model, update: Update<Effect, Event>) -> Command<Effect, Event> {
    let Update {
        command,
        mut changed,
        notices,
    } = update;
    let mut connected = Vec::new();
    for notice in &notices {
        match notice {
            Notice::Connected(device_id) => {
                // Anything asked for before is asked again.
                layers::forget_reads(model);
                connected.push(device_id.as_str());
            }
            Notice::Bytes { bytes, .. } => changed |= layers::receive(model, bytes),
            Notice::Released(_) => layers::forget_reads(model),
        }
    }
    let command = if connected.is_empty() {
        command
    } else {
        let mut bytes = state(model);
        bytes.extend(layers::reads(model));
        command.and(send(connected.into_iter(), &bytes))
    };
    if changed {
        command.and(render())
    } else {
        command
    }
}
