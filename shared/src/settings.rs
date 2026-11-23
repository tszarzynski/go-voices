//! What the app keeps between launches, and the effects that keep it.
//!
//! | operation        | kind    | output             |
//! |------------------|---------|--------------------|
//! | [`LoadSettings`] | request | [`StoredSettings`] |
//! | [`SaveSettings`] | notify  |                    |

use alloc::{format, string::String, vec::Vec};

use crux_core::{
    Command, Request,
    command::{NotificationBuilder, RequestBuilder},
    macros::Operation,
};
use facet::Facet;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use catalogue::Patch;

use crate::{
    app::{CHANNELS, Model, RECENTS},
    go_keys, layer,
    layers::Layers,
    part::{self, Edit, Scope},
    parts::{self, Part},
};

/// The user's choices. Patches are kept by address (`87.64.4`), which
/// stays the same when the catalogue changes order.
///
/// Kept as JSON ([`Settings::to_json`]) so that they survive the app
/// changing: a field added later takes its default when older settings
/// lack it, and a setting the app no longer has is skipped.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    /// Starred patches.
    #[serde(deserialize_with = "known")]
    pub favourites: Vec<String>,
    /// Patches picked, most recent first.
    #[serde(deserialize_with = "known")]
    pub recents: Vec<String>,
    /// The MIDI channel to send on, 1–16.
    #[serde(deserialize_with = "or_default")]
    pub channel: u8,
    /// Seamless switching is on.
    #[serde(deserialize_with = "or_default")]
    pub seamless: bool,
    /// What's playing.
    #[serde(flatten)]
    pub setup: Setup,
    /// The presets, in the order they were saved.
    #[serde(deserialize_with = "known")]
    pub presets: Vec<Preset>,
    /// The preset recalled or saved last.
    #[serde(deserialize_with = "or_default")]
    pub loaded: Option<u32>,
}

/// What the keyboard plays: its parts, and every changed setting.
///
/// Settings from before parts (one sound, a split or a layer of an upper
/// and a lower half) are read as the parts they were.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(from = "RawSetup")]
pub struct Setup {
    /// The parts, 1 to 4.
    pub parts: Vec<PartSetup>,
    /// The whole keyboard's changed settings.
    pub system_edits: Vec<Edit>,
}

/// One part: its sound, keys, and changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct PartSetup {
    /// Its sound.
    #[serde(deserialize_with = "or_default")]
    pub sound: Option<String>,
    /// Its lowest and highest keys (0 and 127: every key that way).
    #[serde(deserialize_with = "or_default")]
    pub low: u8,
    #[serde(deserialize_with = "or_high")]
    pub high: u8,
    /// Its changed settings.
    #[serde(deserialize_with = "known")]
    pub edits: Vec<Edit>,
    /// Its sound's changed layers.
    #[serde(deserialize_with = "known")]
    pub layers: Vec<LayerEdit>,
}

impl Default for PartSetup {
    fn default() -> Self {
        Self {
            sound: None,
            low: 0,
            high: 127,
            edits: Vec::new(),
            layers: Vec::new(),
        }
    }
}

/// A setup as read: parts, or the halves of older settings.
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawSetup {
    #[serde(deserialize_with = "known")]
    parts: Vec<PartSetup>,
    #[serde(deserialize_with = "known")]
    system_edits: Vec<Edit>,
    /// Before parts: `Single`, `Split` or `Layer`.
    #[serde(deserialize_with = "or_default")]
    mode: String,
    #[serde(deserialize_with = "or_default")]
    upper: Option<String>,
    #[serde(deserialize_with = "or_default")]
    lower: Option<String>,
    #[serde(deserialize_with = "or_default")]
    split: Option<u8>,
    #[serde(deserialize_with = "known")]
    upper_edits: Vec<Edit>,
    #[serde(deserialize_with = "known")]
    lower_edits: Vec<Edit>,
    #[serde(deserialize_with = "known")]
    upper_layers: Vec<LayerEdit>,
    #[serde(deserialize_with = "known")]
    lower_layers: Vec<LayerEdit>,
}

impl From<RawSetup> for Setup {
    fn from(raw: RawSetup) -> Self {
        let mut parts = raw.parts;
        if parts.is_empty() && raw.mode.is_empty() && raw.upper.is_none() {
            // Nothing kept yet: one part, every key.
        } else if parts.is_empty() {
            // The upper half was part 1, the lower part 2.
            let mut upper = PartSetup {
                sound: raw.upper,
                edits: raw.upper_edits,
                layers: raw.upper_layers,
                ..PartSetup::default()
            };
            let mut lower = PartSetup {
                sound: raw.lower,
                edits: raw.lower_edits,
                layers: raw.lower_layers,
                ..PartSetup::default()
            };
            match raw.mode.as_str() {
                "Split" => {
                    // The upper half's lowest key: any but the keyboard's
                    // lowest.
                    let split = raw
                        .split
                        .filter(|split| (go_keys::FIRST + 1..=go_keys::LAST).contains(split))
                        .unwrap_or(parts::SPLIT);
                    upper.low = split;
                    lower.high = split - 1;
                    parts = Vec::from([upper, lower]);
                }
                "Layer" => parts = Vec::from([upper, lower]),
                _ => parts = Vec::from([upper]),
            }
        }
        Self {
            parts,
            system_edits: raw.system_edits,
        }
    }
}

/// One changed byte of a sound's layers.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerEdit {
    /// The tone mix table (0) or a tone (1–4).
    pub block: u8,
    /// Where in it.
    pub index: u8,
    /// Its value.
    pub value: u8,
    /// What the sound had.
    pub was: u8,
}

/// A setup kept under a name, to bring back with one tap.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Preset {
    /// For the preset events. Never reused while it's kept.
    #[serde(deserialize_with = "or_default")]
    pub id: u32,
    /// What the user called it.
    #[serde(deserialize_with = "or_default")]
    pub name: String,
    /// What it plays.
    #[serde(flatten)]
    pub setup: Setup,
}

impl Settings {
    /// As kept.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Read from what was kept, or `None` if it isn't settings at all.
    #[must_use]
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

/// The model's settings, to keep.
pub(crate) fn snapshot(model: &Model) -> Settings {
    let key = |id: &usize| model.patch(*id).map(key);
    Settings {
        favourites: model.favourites.iter().filter_map(key).collect(),
        recents: model.recents.iter().filter_map(key).collect(),
        channel: model.channel,
        seamless: model.seamless,
        setup: setup(model),
        presets: model.presets.clone(),
        loaded: model.loaded,
    }
}

/// What the model plays.
pub(crate) fn setup(model: &Model) -> Setup {
    let key = |id: Option<usize>| id.and_then(|id| model.patch(id)).map(key);
    Setup {
        parts: model
            .parts
            .iter()
            .map(|part| PartSetup {
                sound: key(part.sound),
                low: part.low,
                high: part.high,
                edits: part::to_list(&part.edits),
                layers: part
                    .layers
                    .edits
                    .iter()
                    .map(|(&(block, index), &(value, was))| LayerEdit {
                        block,
                        index,
                        value,
                        was,
                    })
                    .collect(),
            })
            .collect(),
        system_edits: part::to_list(&model.system),
    }
}

/// Puts kept settings back into the model, leaving out what's no longer
/// valid. A sound picked while they loaded stays.
pub(crate) fn restore(model: &mut Model, settings: &Settings) {
    model.favourites = settings
        .favourites
        .iter()
        .filter_map(|key| id(model, key))
        .collect();
    model.recents = settings
        .recents
        .iter()
        .filter_map(|key| id(model, key))
        .take(RECENTS)
        .collect();
    if CHANNELS.contains(&settings.channel) {
        model.channel = settings.channel;
    }
    model.seamless = settings.seamless;
    model.presets.clone_from(&settings.presets);
    model.loaded = settings
        .loaded
        .filter(|&id| model.presets.iter().any(|p| p.id == id));
    let picked = model.parts[0].sound;
    apply(model, &settings.setup);
    // A sound picked while they loaded stays, without the kept sound's
    // layer changes.
    if picked.is_some() && picked != model.parts[0].sound {
        model.parts[0].sound = picked;
        model.parts[0].layers = Layers::default();
    }
    model.refresh();
}

/// Makes the model play `setup`, leaving out what's no longer valid. Each
/// part plays on its own part.
pub(crate) fn apply(model: &mut Model, setup: &Setup) {
    let mut kept: Vec<Part> = setup
        .parts
        .iter()
        .take(parts::MOST)
        .enumerate()
        .map(|(index, kept)| {
            let mut part = Part::new(index, kept.sound.as_ref().and_then(|key| id(model, key)));
            if kept.low <= kept.high && kept.high <= 127 {
                (part.low, part.high) = go_keys::to_ends(kept.low, kept.high);
            }
            part.edits = part::from_list(&kept.edits, parts::scope(index));
            part.layers = Layers::default();
            part.layers.edits = kept
                .layers
                .iter()
                .filter(|edit| edit.block <= layer::LAYERS && edit.value != edit.was)
                .map(|edit| ((edit.block, edit.index), (edit.value, edit.was)))
                .collect();
            part
        })
        .collect();
    if kept.is_empty() {
        kept.push(Part::new(0, None));
    }
    model.parts = kept;
    model.editing = model.editing();
    model.system = part::from_list(&setup.system_edits, Scope::Keyboard);
}

/// The patch with this key.
pub(crate) fn patch<'a>(model: &'a Model, key: &str) -> Option<&'a Patch> {
    id(model, key).and_then(|id| model.patch(id))
}

/// The id of the patch with this key.
fn id(model: &Model, key: &str) -> Option<usize> {
    model
        .catalogue
        .patches()
        .iter()
        .position(|patch| self::key(patch) == key)
}

/// How settings name a patch: its address, e.g. `87.64.4`.
fn key(patch: &Patch) -> String {
    format!("{}.{}.{}", patch.msb, patch.lsb, patch.pc)
}

/// A value, or its default if the app no longer knows it.
fn or_default<'de, D: Deserializer<'de>, T: DeserializeOwned + Default>(
    deserializer: D,
) -> Result<T, D::Error> {
    let value = Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// A part's highest key, or every key up if it can't be read.
fn or_high<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let value = Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(127))
}

/// The items the app still knows.
fn known<'de, D: Deserializer<'de>, T: DeserializeOwned>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    let values = Vec::<Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

/// The answer to [`LoadSettings`]: what [`SaveSettings`] kept, nothing on
/// the first launch.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredSettings(pub Option<String>);

/// Read what [`SaveSettings`] kept last.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(request, output = StoredSettings)]
pub struct LoadSettings;

/// Keep these settings (JSON, see [`Settings`]), replacing the last ones.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct SaveSettings(pub String);

/// Asks for [`LoadSettings`].
#[must_use]
pub fn load<Effect, Event>() -> RequestBuilder<Effect, Event, impl Future<Output = StoredSettings>>
where
    Effect: From<Request<LoadSettings>> + Send + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(LoadSettings)
}

/// Sends [`SaveSettings`].
#[must_use]
pub fn save<Effect, Event>(
    settings: &Settings,
) -> NotificationBuilder<Effect, Event, impl Future<Output = ()> + use<Effect, Event>>
where
    Effect: From<Request<SaveSettings>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(SaveSettings(settings.to_json()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::part::Control;

    #[test]
    fn older_or_newer_settings_still_load() {
        // Older: halves, no `upper`. Newer: a setting this app doesn't
        // know, and a field it doesn't have.
        let json = r#"{"favourites":["87.64.1"],"channel":2,"mode":"Teleport",
            "upper_edits":[{"control":"Level","value":80},{"control":"Wobble","value":3}],
            "colour":"red"}"#;
        let settings = Settings::from_json(json).unwrap();
        assert_eq!(settings.favourites, ["87.64.1"]);
        assert_eq!(settings.channel, 2);
        assert_eq!(
            settings.setup.parts,
            [PartSetup {
                edits: Vec::from([Edit {
                    control: Control::Level,
                    value: 80
                }]),
                ..PartSetup::default()
            }]
        );
        assert_eq!(Settings::from_json(&settings.to_json()), Some(settings));
        assert_eq!(Settings::from_json("not json"), None);
    }

    #[test]
    fn a_split_or_layer_of_halves_reads_as_two_parts() {
        let split = r#"{"mode":"Split","upper":"87.64.1","lower":"87.70.3","split":55,
            "lower_edits":[{"control":"Level","value":80}],
            "presets":[{"id":1,"name":"Duo","mode":"Layer","upper":"87.64.1","lower":"87.70.3"}]}"#;
        let settings = Settings::from_json(split).unwrap();
        let parts = &settings.setup.parts;
        assert_eq!(parts.len(), 2);
        assert_eq!(
            (parts[0].sound.as_deref(), parts[0].low, parts[0].high),
            (Some("87.64.1"), 55, 127)
        );
        assert_eq!(
            (parts[1].sound.as_deref(), parts[1].low, parts[1].high),
            (Some("87.70.3"), 0, 54)
        );
        assert_eq!(parts[1].edits.len(), 1);
        let layer = &settings.presets[0].setup.parts;
        assert_eq!((layer.len(), layer[1].low, layer[1].high), (2, 0, 127));
        // Kept again as parts only.
        let json = settings.to_json();
        assert!(!json.contains("upper") && json.contains("\"parts\""));
        assert_eq!(Settings::from_json(&json), Some(settings));
    }

    #[test]
    fn a_value_the_app_can_t_read_takes_its_default_and_keeps_the_rest() {
        let json = r#"{"channel":300,"seamless":"yes","loaded":"x","favourites":["87.64.1",7],
            "parts":[{"sound":5,"low":"C4","edits":[]}],
            "presets":[{"id":1,"name":"Gig"}]}"#;
        let settings = Settings::from_json(json).unwrap();
        assert_eq!(settings.presets.len(), 1);
        assert_eq!(settings.favourites, ["87.64.1"]);
        assert!(!settings.seamless);
        assert_eq!(settings.loaded, None);
        assert_eq!(settings.setup.parts, [PartSetup::default()]);
    }
}
