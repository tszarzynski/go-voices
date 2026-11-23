//! The Layers editor: each part's sound as read from the keyboard, the
//! changes made to it, soloing a layer, and choosing waves.
//!
//! What each setting is lives in [`crate::layer`]; this is what the app does
//! with them. Changes belong to the sound: picking another one forgets them,
//! as the keyboard does.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use catalogue::{Category, Wave};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    app::Model,
    browse, go_keys,
    layer::{self, Bytes, LAYERS, LayerControl, Place, Sound},
};

/// The Edit panel's tabs.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Tab {
    /// The part's and the keyboard's settings.
    #[default]
    Sound,
    /// The sound's layers.
    Layers,
}

/// A part's sound in the editor.
#[derive(Clone, Debug, Default)]
pub struct Layers {
    /// As read from the keyboard, once all of it has come back.
    read: Option<Sound>,
    /// Coming back: what has so far, and which blocks are whole.
    incoming: Sound,
    received: [bool; LAYERS as usize + 1],
    /// Asked for from this keyboard part, not all back yet.
    reading: Option<u8>,
    /// Bytes changed: each one's value, and what it was.
    pub(crate) edits: BTreeMap<Place, (u8, u8)>,
}

impl Layers {
    /// A byte as it is now: changed, or as read.
    pub(crate) fn byte(&self, place: Place) -> Option<u8> {
        self.edits
            .get(&place)
            .map(|&(value, _)| value)
            .or_else(|| self.read.as_ref()?.get(place))
    }

    /// Read from the keyboard: there's something to edit.
    pub(crate) const fn ready(&self) -> bool {
        self.read.is_some()
    }

    /// The changed bytes, to write after the sound.
    pub(crate) fn bytes(&self) -> Bytes {
        self.edits
            .iter()
            .map(|(&place, &(value, _))| (place, value))
            .collect()
    }

    /// A new sound: nothing read, nothing changed.
    pub(crate) fn forget(&mut self) {
        *self = Self::default();
    }

    /// Changes these bytes, remembering what each was. A byte back at what
    /// it was is no longer a change.
    fn change(&mut self, bytes: &[(Place, u8)]) {
        for &(place, value) in bytes {
            let was = self
                .edits
                .get(&place)
                .map(|&(_, was)| was)
                .or_else(|| self.read.as_ref()?.get(place));
            let Some(was) = was else { continue };
            if value == was {
                self.edits.remove(&place);
            } else {
                self.edits.insert(place, (value, was));
            }
        }
    }

    /// What these bytes were before any change.
    fn originals(&self, places: impl IntoIterator<Item = Place>) -> Vec<(Place, u8)> {
        places
            .into_iter()
            .filter_map(|place| Some((place, self.edits.get(&place)?.1)))
            .collect()
    }

    /// Part of the sound came back. True once all of it has.
    fn receive(&mut self, block: u8, index: usize, data: &[u8]) -> bool {
        self.incoming.fill(block, index, data);
        let size = if block == 0 {
            layer::MIX_SIZE
        } else {
            layer::TONE_SIZE
        };
        if index + data.len() >= size {
            self.received[usize::from(block)] = true;
        }
        if self.received.iter().all(|&whole| whole) {
            self.read = Some(core::mem::take(&mut self.incoming));
            self.received = Default::default();
            self.reading = None;
            return true;
        }
        false
    }
}

/// A layer copied, to paste onto another (of this sound or another one).
#[derive(Clone, Debug)]
pub struct Copied {
    /// Its tone, as it was when copied.
    tone: [u8; layer::TONE_SIZE],
    /// `Layer 2 of JUNO Piano 1`.
    pub(crate) from: String,
}

/// The part being edited's sound.
fn current(model: &Model) -> &Layers {
    &model.edited().layers
}

fn current_mut(model: &mut Model) -> &mut Layers {
    &mut model.edited_mut().layers
}

/// The part being edited plays a drum kit, which has no layers.
pub fn drums(model: &Model) -> bool {
    model
        .sound(model.editing())
        .is_some_and(|patch| patch.category == Category::Drums)
}

/// The Layers tab is showing.
pub fn showing(model: &Model) -> bool {
    model.editor && model.tab == Tab::Layers
}

/// Asks for the part being edited's sound, if the Layers tab shows it and
/// it isn't read or being read.
pub fn reads(model: &mut Model) -> Vec<u8> {
    let connected = model.midi.connected().next().is_some();
    let part = model.part(model.editing());
    if !showing(model) || !connected || drums(model) {
        return Vec::new();
    }
    let layers = current_mut(model);
    // Asked for already, from the part it plays on now.
    if layers.ready() || layers.reading == Some(part) {
        return Vec::new();
    }
    layers.reading = Some(part);
    layers.incoming = Sound::default();
    layers.received = Default::default();
    go_keys::read_sound(part)
}

/// Takes what a keyboard sent: parts of the sounds asked for. True if a
/// sound is now whole.
pub fn receive(model: &mut Model, bytes: &[u8]) -> bool {
    let mut whole = false;
    for (address, data) in go_keys::writes(bytes) {
        for part in &mut model.parts {
            if part.layers.reading != Some(part.playing) {
                continue;
            }
            if let Some((block, index)) = go_keys::in_sound(part.playing, address) {
                whole |= part.layers.receive(block, index, &data);
            }
        }
    }
    whole
}

/// Forgets what was asked for, so it's asked again: the keyboard went, or
/// a new one came.
pub fn forget_reads(model: &mut Model) {
    for part in &mut model.parts {
        part.layers.reading = None;
    }
}

/// The selected layer's setting, as it is now.
pub fn value(model: &Model, control: LayerControl) -> Option<i16> {
    let layers = current(model);
    layers
        .ready()
        .then(|| control.value(model.layer, |place| layers.byte(place).unwrap_or_default()))
}

/// The selected layer's setting, as the sound had it.
pub fn original(model: &Model, control: LayerControl) -> Option<i16> {
    let layers = current(model);
    let read = layers.read.as_ref()?;
    Some(control.value(model.layer, |place| {
        layers
            .edits
            .get(&place)
            .map_or_else(|| read.get(place).unwrap_or_default(), |&(_, was)| was)
    }))
}

/// Writes these bytes of the part being edited's sound, keeping them as
/// changes.
fn write(model: &mut Model, bytes: &[(Place, u8)]) -> Vec<u8> {
    let part = model.part(model.editing());
    current_mut(model).change(bytes);
    go_keys::sound_bytes(part, &bytes.iter().copied().collect())
}

/// Changes the selected layer's setting.
pub fn set(model: &mut Model, control: LayerControl, value: i16) -> Vec<u8> {
    if !current(model).ready() {
        return Vec::new();
    }
    let value = in_range(model, control, value);
    let mut bytes = end_solo(model);
    let changes = control.bytes(model.layer, value);
    bytes.extend(write(model, &changes));
    bytes
}

/// A range's ends can't cross; a key range reaching the keyboard's end
/// takes in every key past it, as the sounds' own ranges do.
fn in_range(model: &Model, control: LayerControl, value: i16) -> i16 {
    let other = |control| value_of(model, control);
    let key = u8::try_from(value.clamp(0, 127)).unwrap_or_default();
    match control {
        LayerControl::KeyLow if key <= go_keys::FIRST => i16::from(go_keys::to_ends(key, key).0),
        LayerControl::KeyHigh if key >= go_keys::LAST => i16::from(go_keys::to_ends(key, key).1),
        LayerControl::KeyLow => value.min(other(LayerControl::KeyHigh)),
        LayerControl::KeyHigh => value.max(other(LayerControl::KeyLow)),
        LayerControl::VelocityLow => value.min(other(LayerControl::VelocityHigh)),
        LayerControl::VelocityHigh => value.max(other(LayerControl::VelocityLow)),
        _ => value,
    }
}

fn value_of(model: &Model, control: LayerControl) -> i16 {
    value(model, control).unwrap_or_default()
}

/// Puts the selected layer's setting back as the sound had it.
pub fn reset(model: &mut Model, control: LayerControl) -> Vec<u8> {
    original(model, control).map_or_else(Vec::new, |value| set(model, control, value))
}

/// Edits `layer`, and switches it on or off.
pub fn toggle(model: &mut Model, layer: u8) -> Vec<u8> {
    if layer >= LAYERS {
        return Vec::new();
    }
    model.layer = layer;
    let on = value_of(model, LayerControl::Switch) == 1;
    set(model, LayerControl::Switch, i16::from(!on))
}

/// Puts the selected layer's keys back as the sound had them: both ends
/// at once, so neither is held back by the other's changed end.
pub fn reset_keys(model: &mut Model) -> Vec<u8> {
    let ends = [LayerControl::KeyLow, LayerControl::KeyHigh];
    let Some(originals) = ends
        .iter()
        .map(|&control| original(model, control))
        .collect::<Option<Vec<i16>>>()
    else {
        return Vec::new();
    };
    let mut bytes = end_solo(model);
    let changes: Vec<(Place, u8)> = ends
        .iter()
        .zip(originals)
        .flat_map(|(&control, value)| control.bytes(model.layer, value))
        .collect();
    bytes.extend(write(model, &changes));
    bytes
}

/// Puts a layer (or, with `None`, every layer) back as the sound had it.
pub fn reset_layers(model: &mut Model, layer: Option<u8>) -> Vec<u8> {
    let mut bytes = end_solo(model);
    let layers = current(model);
    let places = layers
        .edits
        .keys()
        .copied()
        .filter(|&place| layer.is_none_or(|layer| layer::owns(layer, place)));
    let originals = layers.originals(places);
    bytes.extend(write(model, &originals));
    bytes
}

/// Copies a layer as it is now, to paste later.
pub fn copy(model: &mut Model, layer: u8) {
    let layers = current(model);
    if !layers.ready() || layer >= LAYERS {
        return;
    }
    let mut tone = [0; layer::TONE_SIZE];
    for (index, byte) in (0u8..).zip(&mut tone) {
        *byte = layers.byte((layer + 1, index)).unwrap_or_default();
    }
    let sound = model
        .sound(model.editing())
        .map_or("this sound", |patch| patch.name.as_str());
    let from = alloc::format!("Layer {} of {sound}", layer + 1);
    model.copied = Some(Copied { tone, from });
}

/// Pastes the layer copied onto `layer`, switched on: its wave and every
/// setting, as changes.
pub fn paste(model: &mut Model, layer: u8) -> Vec<u8> {
    let Some(tone) = model.copied.as_ref().map(|copied| copied.tone) else {
        return Vec::new();
    };
    if !current(model).ready() || layer >= LAYERS {
        return Vec::new();
    }
    let mut bytes = end_solo(model);
    let mut changes: Vec<(Place, u8)> = (0u8..)
        .zip(tone)
        .map(|(index, byte)| ((layer + 1, index), byte))
        .collect();
    changes.extend(LayerControl::Switch.bytes(layer, 1));
    bytes.extend(write(model, &changes));
    bytes
}

/// Plays one layer alone (`Some`), or every one again (`None`), without
/// changing the sound.
pub fn solo(model: &mut Model, layer: Option<u8>) -> Vec<u8> {
    let mut bytes = end_solo(model);
    let part = model.part(model.editing());
    let Some(layer) = layer.filter(|&layer| layer < LAYERS && current(model).ready()) else {
        return bytes;
    };
    model.solo = Some(layer);
    let switches: Bytes = (0..LAYERS)
        .flat_map(|other| LayerControl::Switch.bytes(other, i16::from(other == layer)))
        .collect();
    bytes.extend(go_keys::sound_bytes(part, &switches));
    bytes
}

/// Ends a solo: each layer's switch back as it is.
pub fn end_solo(model: &mut Model) -> Vec<u8> {
    if model.solo.take().is_none() {
        return Vec::new();
    }
    let part = model.part(model.editing());
    let layers = current(model);
    let switches: Bytes = (0..LAYERS)
        .flat_map(|layer| {
            let on =
                LayerControl::Switch.value(layer, |place| layers.byte(place).unwrap_or_default());
            LayerControl::Switch.bytes(layer, on)
        })
        .collect();
    go_keys::sound_bytes(part, &switches)
}

/// The selected layer's wave: its group and number.
pub fn wave(model: &Model) -> Option<(u16, u16)> {
    let layers = current(model);
    layers
        .ready()
        .then(|| layer::wave(model.layer, |place| layers.byte(place).unwrap_or_default()))
}

/// The waves a layer can be given: each stereo pair once, by its left wave.
pub fn choosable(model: &Model) -> impl Iterator<Item = &Wave> {
    model.waves.all().iter().filter(|wave| !wave.is_right())
}

/// Gives the selected layer a wave, and its right one for a stereo pair.
pub fn set_wave(model: &mut Model, group: u16, number: u16) -> Vec<u8> {
    let Some(wave) = model.waves.get(group, number).copied() else {
        return Vec::new();
    };
    if !current(model).ready() {
        return Vec::new();
    }
    let right = model.waves.right_of(&wave).map_or(0, |right| right.number);
    let mut bytes = end_solo(model);
    let changes = layer::wave_bytes(model.layer, group, number, right);
    bytes.extend(write(model, &changes));
    bytes
}

/// The next (or previous) wave for the selected layer, wrapping around.
pub fn step_wave(model: &mut Model, forward: bool) -> Vec<u8> {
    let Some(now) = wave(model) else {
        return Vec::new();
    };
    let waves: Vec<(u16, u16)> = choosable(model).map(|w| (w.group, w.number)).collect();
    browse::step(&waves, Some(now), forward)
        .map_or_else(Vec::new, |(group, number)| set_wave(model, group, number))
}

/// The waves the picker lists, for its search.
pub fn listed_waves<'a>(model: &'a Model, query: &str) -> impl Iterator<Item = &'a Wave> {
    let query = query.trim().to_lowercase();
    choosable(model).filter(move |wave| wave.name.to_lowercase().contains(&query))
}

/// A layer's wave name, from the sound as it is now.
pub fn wave_name(model: &Model, layer: u8) -> String {
    let layers = current(model);
    let (group, number) = layer::wave(layer, |place| layers.byte(place).unwrap_or_default());
    match model.waves.get(group, number) {
        Some(wave) => wave.name.trim_end_matches(" L").into(),
        None if number == 0 => "No wave".into(),
        None => alloc::format!("Wave {group}·{number}"),
    }
}
